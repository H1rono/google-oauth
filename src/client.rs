use std::{borrow::Cow, fmt, str::FromStr};

use serde::{de, Deserialize, Serialize};

use crate::scope::{self, Scope, SpaceDelimitedScope};
use crate::secret::WebClientSecret;

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
        macro_rules! encode {
            ($e:expr) => {{
                let e = $e;
                ::percent_encoding::utf8_percent_encode(&e, ::percent_encoding::NON_ALPHANUMERIC)
                    .to_string()
            }};
        }

        let Self {
            secret:
                WebClientSecret {
                    client_id,
                    auth_uri,
                    ..
                },
            config:
                ClientConfig {
                    redirect_uri,
                    scope,
                },
            ..
        } = self;
        let client_id = encode!(client_id);
        let redirect_uri = encode!(redirect_uri);
        let scope = encode!(scope.to_string());
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

    pub async fn authorize_with<'a, S>(&'a self, code: S) -> anyhow::Result<AuthorizedClient>
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
        let response: TokenResponse = request.send().await?.json().await?;
        let authorized = AuthorizedClient::new(secret.clone(), response);
        Ok(authorized)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AuthorizationCode(());

struct AuthorizationCodeVisitor;

impl AuthorizationCode {
    pub const STR: &'static str = "authorization_code";

    pub fn new() -> Self {
        Self(())
    }
}

impl fmt::Display for AuthorizationCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::STR)
    }
}

impl FromStr for AuthorizationCode {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::STR {
            return Ok(Self::new());
        }
        Err("not authorization_code")
    }
}

impl Serialize for AuthorizationCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::STR)
    }
}

impl<'de> Deserialize<'de> for AuthorizationCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(AuthorizationCodeVisitor)
    }
}

impl<'de> de::Visitor<'de> for AuthorizationCodeVisitor {
    type Value = AuthorizationCode;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a str "authorization_code""#)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bearer(());

struct BearerVisitor;

impl Bearer {
    pub const STR: &'static str = "Bearer";

    fn new() -> Self {
        Self(())
    }
}

impl fmt::Display for Bearer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(Self::STR)
    }
}

impl FromStr for Bearer {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == Self::STR {
            Ok(Self::new())
        } else {
            Err("not Bearer")
        }
    }
}

impl Serialize for Bearer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::STR)
    }
}

impl<'de> Deserialize<'de> for Bearer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(BearerVisitor)
    }
}

impl<'de> de::Visitor<'de> for BearerVisitor {
    type Value = Bearer;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(r#"a str "Bearer""#)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        v.parse().map_err(E::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TokenResponse {
    access_token: String,
    expires_in: u32,
    #[serde(default)]
    refresh_token: Option<String>,
    scope: SpaceDelimitedScope,
    token_type: Bearer,
}

#[derive(Clone)]
pub struct AuthorizedClient {
    #[allow(dead_code)]
    secret: WebClientSecret,
    token: TokenResponse,
    inner: reqwest::Client,
}

macro_rules! request_fn {
    ($m:ident) => {
        ::paste::paste! {
            pub fn [<  $m:snake:lower >] (&self, uri: &str) -> ::reqwest::RequestBuilder {
                self.request(::http::Method::[< $m:snake:upper >], uri)
            }
        }
    };
}

impl AuthorizedClient {
    pub const BASE_URL: &'static str = "https://www.googleapis.com";

    fn new(secret: WebClientSecret, token: TokenResponse) -> Self {
        let inner = reqwest::Client::new();
        Self {
            secret,
            token,
            inner,
        }
    }

    pub fn token(&self) -> &TokenResponse {
        &self.token
    }

    pub fn request(&self, method: http::Method, uri: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{uri}", Self::BASE_URL);
        self.inner
            .request(method, url)
            .bearer_auth(&self.token.access_token)
    }

    request_fn! {get}
    request_fn! {post}
    request_fn! {patch}
    request_fn! {put}
    request_fn! {delete}
}
