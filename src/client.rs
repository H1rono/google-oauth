use std::{borrow::Cow, fmt, str::FromStr};

use serde::{de, Deserialize, Serialize};

use crate::secret::WebClientSecret;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientConfig {
    pub redirect_uri: String,
    pub scope: Vec<String>,
}

#[derive(Clone)]
pub struct UnauthorizedClient {
    secret: WebClientSecret,
    config: ClientConfig,
}

impl UnauthorizedClient {
    pub fn new(secret: WebClientSecret, config: ClientConfig) -> Self {
        Self { secret, config }
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
        } = self;
        let client_id = encode!(client_id);
        let redirect_uri = encode!(redirect_uri);
        let scope = encode!(scope.join(" "));
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
}

#[derive(Clone, Default)]
pub struct UnauthorizedClientBuilder {
    redirect_uri: Option<String>,
    scope: Vec<String>,
    secret: Option<WebClientSecret>,
}

impl UnauthorizedClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn add_scope<'s, S>(mut self, scope: S) -> Self
    where
        S: Into<Cow<'s, str>>,
    {
        let scope = scope.into().into_owned();
        self.scope.push(scope);
        self
    }

    pub fn scope<'s, S, E>(self, scope: S) -> Self
    where
        S: IntoIterator<Item = E> + 's,
        E: Into<Cow<'s, str>>,
    {
        let scope = scope.into_iter().map(|e| e.into().into_owned()).collect();
        Self { scope, ..self }
    }

    pub fn secret(self, secret: &WebClientSecret) -> Self {
        let secret = secret.clone();
        Self {
            secret: Some(secret),
            ..self
        }
    }

    pub fn build(self) -> anyhow::Result<UnauthorizedClient> {
        use anyhow::anyhow;

        let Self {
            redirect_uri,
            scope,
            secret,
        } = self;
        let redirect_uri = redirect_uri.ok_or_else(|| anyhow!("redirect_uri is required"))?;
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
pub struct TokenRequest<'a> {
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
    scope: String,
    token_type: Bearer,
}

#[derive(Clone)]
pub struct AuthorizedClient {
    secrets: WebClientSecret,
    token: TokenResponse,
}
