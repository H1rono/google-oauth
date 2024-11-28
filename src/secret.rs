use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct ClientSecret {
    pub web: WebClientSecret,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct WebClientSecret {
    pub client_id: String,
    pub project_id: String,
    pub auth_url: String,
    pub token_url: String,
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
}
