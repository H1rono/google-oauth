use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::scope::{self, Scope, SpaceDelimitedScope};
use crate::secret::WebClientSecret;

pub mod calendar;
mod misc;

pub use misc::{AuthorizationCode, Bearer, RefreshToken};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub redirect_uri: String,
    pub scope: SpaceDelimitedScope,
}

#[derive(Clone)]
pub struct UnauthorizedClient {
    secret: WebClientSecret,
    config: ClientConfig,
    client: reqwest::Client,
}

impl UnauthorizedClient {
    pub fn new(secret: WebClientSecret, config: ClientConfig) -> Self {
        Self {
            secret,
            config,
            client: Default::default(),
        }
    }

    pub fn builder() -> UnauthorizedClientBuilder {
        UnauthorizedClientBuilder::default()
    }

    pub fn generate_url(&self) -> String {
        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

        let Self { secret, config, .. } = self;
        let WebClientSecret {
            client_id,
            auth_uri,
            ..
        } = secret;
        let ClientConfig {
            redirect_uri,
            scope,
        } = config;
        let client_id = utf8_percent_encode(client_id, NON_ALPHANUMERIC);
        let redirect_uri = utf8_percent_encode(redirect_uri, NON_ALPHANUMERIC);
        let scope = scope.to_string();
        let scope = utf8_percent_encode(&scope, NON_ALPHANUMERIC);
        let query = [
            format!("client_id={client_id}"),
            format!("redirect_uri={redirect_uri}"),
            format!("scope={scope}"),
            "response_type=code".to_string(),
            "access_type=offline".to_string(),
            // TODO: add state
        ]
        .join("&");
        format!("{auth_uri}?{query}")
    }

    pub async fn acquire_token_with<'a, S>(&'a self, code: S) -> reqwest::Result<Token>
    where
        S: Into<Cow<'a, str>>,
    {
        let Self {
            secret,
            config: ClientConfig { redirect_uri, .. },
            ..
        } = self;
        let WebClientSecret {
            client_id,
            token_uri,
            client_secret,
            ..
        } = secret;
        let request = TokenRequest {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            code: code.into(),
            grant_type: AuthorizationCode::new(),
            redirect_uri: redirect_uri.into(),
        };
        let request = self
            .client
            .post(token_uri)
            .header(
                http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(request.urlencoded());
        let response: Token = request.send().await?.json().await?;
        Ok(response)
    }

    pub async fn authorize_with_code<'a, S>(&'a self, code: S) -> reqwest::Result<AuthorizedClient>
    where
        S: Into<Cow<'a, str>>,
    {
        let token = self.acquire_token_with(code).await?;
        Ok(self.autorize_with_token(token))
    }

    #[inline]
    pub fn autorize_with_token(&self, token: Token) -> AuthorizedClient {
        AuthorizedClient::new(self.secret.clone(), token)
    }
}

#[derive(Clone)]
pub struct UnauthorizedClientBuilder<S = scope::NoScope> {
    redirect_uri: Option<String>,
    scope: S,
    secret: Option<WebClientSecret>,
}

impl UnauthorizedClientBuilder<scope::NoScope> {
    pub fn new() -> Self {
        Self {
            redirect_uri: None,
            scope: scope::NoScope,
            secret: None,
        }
    }
}

impl Default for UnauthorizedClientBuilder<scope::NoScope> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S1> UnauthorizedClientBuilder<S1> {
    pub fn redirect_uri<'s, S>(self, uri: S) -> Self
    where
        S: Into<Cow<'s, str>>,
    {
        let uri = uri.into().into_owned();
        Self {
            redirect_uri: Some(uri),
            ..self
        }
    }

    pub fn add_scope<S2>(self, s2: S2) -> UnauthorizedClientBuilder<scope::With<S1, S2>>
    where
        S1: Scope,
        S2: Scope,
    {
        let Self {
            redirect_uri,
            scope,
            secret,
        } = self;
        let scope = scope.with(s2);
        UnauthorizedClientBuilder {
            redirect_uri,
            scope,
            secret,
        }
    }

    pub fn scope<S>(self, scope: S) -> UnauthorizedClientBuilder<S>
    where
        S: Scope + Clone,
    {
        let Self {
            redirect_uri,
            secret,
            ..
        } = self;
        UnauthorizedClientBuilder {
            redirect_uri,
            scope,
            secret,
        }
    }

    pub fn secret(self, secret: &WebClientSecret) -> Self {
        let secret = secret.clone();
        Self {
            secret: Some(secret),
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<UnauthorizedClient>
    where
        S1: Scope + Clone,
    {
        use anyhow::anyhow;

        let Self {
            redirect_uri,
            scope,
            secret,
        } = self;
        let redirect_uri = redirect_uri.ok_or_else(|| anyhow!("redirect_uri is required"))?;
        let scope = scope.space_delimited();
        let secret = secret.ok_or_else(|| anyhow!("secret is required"))?;
        let config = ClientConfig {
            redirect_uri,
            scope,
        };
        let client = UnauthorizedClient::new(secret, config);
        Ok(client)
    }
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize)]
struct TokenRequest<'a> {
    #[serde(borrow)]
    client_id: Cow<'a, str>,
    #[serde(borrow)]
    client_secret: Cow<'a, str>,
    #[serde(borrow)]
    code: Cow<'a, str>,
    grant_type: AuthorizationCode,
    #[serde(borrow)]
    redirect_uri: Cow<'a, str>,
}

impl<'a> TokenRequest<'a> {
    pub fn urlencoded(self) -> String {
        macro_rules! encode_queries {
            [ $($i:ident),+ ] => {
                [$(
                    format!(
                        concat!(stringify!($i), "={}"),
                        ::percent_encoding::utf8_percent_encode(& $i, ::percent_encoding::NON_ALPHANUMERIC)
                    )
                ),+]
            };
        }

        let Self {
            client_id,
            client_secret,
            code,
            grant_type,
            redirect_uri,
        } = self;
        let grant_type = grant_type.to_string();
        let params = encode_queries![client_id, client_secret, code, grant_type, redirect_uri];
        params.join("&")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Token {
    access_token: String,
    expires_in: u32,
    #[serde(default)]
    refresh_token: Option<String>,
    scope: SpaceDelimitedScope,
    token_type: Bearer,
}

impl Token {
    pub fn refresh_with(self, other: Token) -> Self {
        let Self { refresh_token, .. } = self;
        Self {
            refresh_token,
            ..other
        }
    }
}

#[derive(Clone)]
pub struct AuthorizedClient {
    #[allow(dead_code)]
    secret: WebClientSecret,
    token: Token,
    inner: reqwest::Client,
}

macro_rules! request_fn {
    (
        $(#[$m:meta])*
        $v:vis $n:ident
    ) => { ::paste::paste! {
        $(#[$m])*
        $v fn [< $n:snake:lower >] (&self, uri: &str) -> ::reqwest::RequestBuilder {
            self.request(::http::Method::[< $n:snake:upper >], uri)
        }
    } };
}

impl AuthorizedClient {
    pub const BASE_URL: &'static str = "https://www.googleapis.com";

    #[inline]
    pub fn new(secret: WebClientSecret, token: Token) -> Self {
        Self {
            secret,
            token,
            inner: reqwest::Client::new(),
        }
    }

    #[inline]
    pub fn token(&self) -> &Token {
        &self.token
    }

    pub fn request(&self, method: http::Method, uri: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{uri}", Self::BASE_URL);
        let req = self.inner.request(method, url);
        self.decorate_request(req)
    }

    request_fn! {pub get}
    request_fn! {pub post}
    request_fn! {pub patch}
    request_fn! {pub put}
    request_fn! {pub delete}

    #[inline]
    pub(crate) fn decorate_request(
        &self,
        request: reqwest::RequestBuilder,
    ) -> reqwest::RequestBuilder {
        request.bearer_auth(&self.token.access_token)
    }

    #[tracing::instrument(skip_all)]
    pub async fn refresh(self) -> anyhow::Result<Self> {
        use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

        let Self {
            secret,
            token,
            inner,
        } = self;
        let WebClientSecret {
            client_id,
            client_secret,
            token_uri,
            ..
        } = &secret;
        let Some(refresh_token) = &token.refresh_token else {
            anyhow::bail!("refresh_token is not provided");
        };
        let grant_type = RefreshToken::new();
        let client_id = utf8_percent_encode(client_id, NON_ALPHANUMERIC);
        let client_secret = utf8_percent_encode(client_secret, NON_ALPHANUMERIC);
        let refresh_token = utf8_percent_encode(refresh_token, NON_ALPHANUMERIC);
        let query = [
            format!("client_id={client_id}"),
            format!("client_secret={client_secret}"),
            format!("refresh_token={refresh_token}"),
            format!("grant_type={grant_type}"),
        ];
        let body = query.join("&");
        let request = inner
            .post(token_uri)
            .header(
                http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body);
        let response: Token = request
            .send()
            .await
            .inspect_err(|err| {
                let err = err as &dyn std::error::Error;
                tracing::error!(err, "could not send request");
            })?
            .json()
            .await
            .inspect_err(|err| {
                let err = err as &dyn std::error::Error;
                tracing::error!(err, "could not parse response body as JSON");
            })?;
        let token = token.refresh_with(response);
        Ok(Self {
            secret,
            token,
            inner,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
#[error("insufficient scope to perform request")]
pub struct InsufficientScopeError(());

impl InsufficientScopeError {
    pub(crate) fn new() -> Self {
        Self(())
    }
}
