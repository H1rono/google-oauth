use std::{future::IntoFuture, net::SocketAddr, sync::Arc};

use anyhow::anyhow;
use axum::extract::{Query, State};
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Notify};
use tracing_subscriber::EnvFilter;

use google_oauth::{AuthorizedClient, ClientSecret, UnauthorizedClient};

#[derive(Clone)]
struct AppState {
    pub code_tx: mpsc::UnboundedSender<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let client = unauthorized_client().await?;
    tracing::info!("authorize url: {}", client.generate_url());

    let (code_tx, code_rx) = mpsc::unbounded_channel();
    let state = AppState { code_tx };
    let layer = tower::ServiceBuilder::new().layer(tower_http::trace::TraceLayer::new_for_http());
    let router = make_router(state).layer(layer);
    let addr = bind_addr()?;
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let shutdown_notify = Arc::new(Notify::new());
    let wait_code = wait_code_with_notify(code_rx, Arc::clone(&shutdown_notify));
    let shutdown = async move { shutdown_notify.notified().await };
    let serve = axum::serve(listener, router).with_graceful_shutdown(shutdown);
    let serve = serve.into_future().map_err(anyhow::Error::new);
    let (code, ()) = tokio::try_join!(wait_code, serve)?;

    let client = client.authorize_with(code).await?;
    let export = export_token(&client);
    let check = check_client(&client);
    tokio::try_join!(export, check)?;
    Ok(())
}

fn bind_addr() -> anyhow::Result<SocketAddr> {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    Ok(addr)
}

fn make_router(state: AppState) -> axum::Router {
    use axum::routing;

    axum::Router::new()
        .route("/oauth2/callback", routing::get(callback))
        .with_state(state)
}

async fn wait_code_with_notify(
    mut code_rx: mpsc::UnboundedReceiver<String>,
    notify: Arc<Notify>,
) -> anyhow::Result<String> {
    let code = code_rx
        .recv()
        .await
        .ok_or_else(|| anyhow!("received no authorization code"))?;
    notify.notify_one();
    Ok(code)
}

#[tracing::instrument]
async fn unauthorized_client() -> anyhow::Result<UnauthorizedClient> {
    let secret_file = tokio::fs::File::open("tmp/client_secret.json").await?;
    let scope = google_oauth::combine_scope![calendar, calendar.readonly];
    let secret = ClientSecret::read_from_file(secret_file).await?;
    let client = UnauthorizedClient::builder()
        .redirect_uri("http://localhost:8080/oauth2/callback")
        .scope(scope)
        .secret(&secret.web)
        .build()?;
    Ok(client)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CallbackParam {
    code: String,
}

#[tracing::instrument(skip_all)]
async fn callback(
    State(state): State<AppState>,
    Query(param): Query<CallbackParam>,
) -> (http::StatusCode, &'static str) {
    let CallbackParam { code } = param;
    tracing::info!("authorized with code: {code}");
    let Ok(()) = state.code_tx.send(code) else {
        tracing::error!("mpsc channel error");
        return (http::StatusCode::INTERNAL_SERVER_ERROR, "channel error");
    };
    (http::StatusCode::OK, "authorized")
}

#[tracing::instrument(skip_all)]
async fn export_token(client: &AuthorizedClient) -> anyhow::Result<()> {
    let token = client.token();
    let token = serde_json::to_string_pretty(token)?;
    tokio::fs::write("tmp/authorized_token.json", token).await?;
    tracing::info!("exported token to tmp/authorized_token.json");
    Ok(())
}

#[tracing::instrument(skip_all)]
async fn check_client(client: &AuthorizedClient) -> anyhow::Result<()> {
    let res = client
        .get("/calendar/v3/users/me/calendarList")
        .send()
        .await?;
    tracing::info!(status = ?res.status(), headers = ?res.headers());
    let body: serde_json::Value = res.json().await?;
    let body = serde_json::to_string_pretty(&body)?;
    tracing::info!("{body}");
    Ok(())
}
