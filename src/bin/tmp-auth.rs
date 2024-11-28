use std::{future::IntoFuture, net::SocketAddr};

use axum::{
    extract::{Query, State},
    routing,
};
use serde::{Deserialize, Serialize};
use tokio::{
    net::TcpListener,
    sync::{mpsc, oneshot},
};
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

    let secret_file = tokio::fs::File::open("tmp/client_secret.json").await?;
    let secret = ClientSecret::read_from_file(secret_file).await?;
    let client = UnauthorizedClient::builder()
        .redirect_uri("http://localhost:8080/oauth2/callback")
        .add_scope("https://www.googleapis.com/auth/calendar")
        .add_scope("https://www.googleapis.com/auth/calendar.readonly")
        .secret(&secret.web)
        .build()?;
    tracing::info!("authorize url: {}", client.generate_url());

    let (code_tx, mut code_rx) = mpsc::unbounded_channel();
    let state = AppState { code_tx };
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse()?;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let layer = tower::ServiceBuilder::new().layer(tower_http::trace::TraceLayer::new_for_http());
    let router = axum::Router::new()
        .route("/oauth2/callback", routing::get(callback))
        .with_state(state)
        .layer(layer);
    let listener = TcpListener::bind(addr).await?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let wait_code = async {
        let code = code_rx.recv().await;
        if let Err(()) = shutdown_tx.send(()) {
            tracing::error!("failed to send shutdown");
            unreachable!()
        }
        code
    };
    let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
        match shutdown_rx.await {
            Ok(()) => {}
            Err(e) => tracing::error!(%e, "shutdown signal error"),
        }
    });
    let serve = tokio::spawn(serve.into_future());
    let Some(code) = wait_code.await else {
        anyhow::bail!("received no authorization code");
    };
    serve.await??;

    let client = client.authorize_with(code).await?;
    check_client(&client).await?;
    Ok(())
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
async fn check_client(client: &AuthorizedClient) -> anyhow::Result<()> {
    let res = client
        .new_request(http::Method::GET, "/calendar/v3/users/me/calendarList")
        .send()
        .await?;
    tracing::info!(status = ?res.status(), headers = ?res.headers());
    let body: serde_json::Value = res.json().await?;
    let body = serde_json::to_string_pretty(&body)?;
    tracing::info!("{body}");
    Ok(())
}
