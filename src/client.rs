use std::borrow::Cow;

use serde::{Deserialize, Serialize};

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
