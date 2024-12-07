use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientSecret {
    pub web: WebClientSecret,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WebClientSecret {
    pub client_id: String,
    pub project_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_secret: String,
}

impl ClientSecret {
    #[tracing::instrument(skip_all)]
    pub async fn read_from_file<F>(mut file: F) -> anyhow::Result<Self>
    where
        F: tokio::io::AsyncRead + Unpin,
    {
        use tokio::io::AsyncReadExt;

        let mut buf = String::new();
        let len = file.read_to_string(&mut buf).await?;
        tracing::debug!("read {len} bytes");
        let s: Self = serde_json::from_str(&buf)?;
        Ok(s)
    }

    pub fn override_from_env(self, infix: Option<&str>) -> Self {
        let Self { web } = self;
        let web = web.override_from_env(infix);
        Self { web }
    }
}

impl WebClientSecret {
    /// `OVERRIDE_{INFIX_}CLIENT_ID`, etc
    pub fn override_from_env(self, infix: Option<&str>) -> Self {
        macro_rules! var_name {

            ($($i:ident),+) => { ::paste::paste! { ( $(
                concat!("OVERRIDE_", stringify!( [< $i:snake:upper >] )).to_string()
            ),+ ) } };

            ($in:expr; $($i:ident),+) => { ::paste::paste! { ( $(
                format!(concat!("OVERRIDE_{}_", stringify!( [< $i:snake:upper >] )), $in)
            ),+ ) } };
        }

        let var_names = if let Some(infix) = infix {
            var_name!(infix; client_id, project_id, auth_uri, token_uri, auth_provider_x509_cert_url, client_secret)
        } else {
            var_name!(
                client_id,
                project_id,
                auth_uri,
                token_uri,
                auth_provider_x509_cert_url,
                client_secret
            )
        };
        let (
            client_id_key,
            project_id_key,
            auth_uri_key,
            token_uri_key,
            auth_provider_x509_cert_url_key,
            client_secret_key,
        ) = var_names;

        macro_rules! let_var_or {
            { $($i:ident;)+ } => {
                ::paste::paste! { $(
                    let $i = ::std::env::var(&[< $i _key >]).unwrap_or($i);
                )+ }
            }
        }

        let Self {
            client_id,
            project_id,
            auth_uri,
            token_uri,
            auth_provider_x509_cert_url,
            client_secret,
        } = self;
        let_var_or! {
            client_id;
            project_id;
            auth_uri;
            token_uri;
            auth_provider_x509_cert_url;
            client_secret;
        }
        Self {
            client_id,
            project_id,
            auth_uri,
            token_uri,
            auth_provider_x509_cert_url,
            client_secret,
        }
    }
}
