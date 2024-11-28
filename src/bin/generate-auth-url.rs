use google_oauth::{ClientSecret, UnauthorizedClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let secret_file = tokio::fs::File::open("tmp/client_secret.json").await?;
    let secret = ClientSecret::read_from_file(secret_file).await?;
    let client = UnauthorizedClient::builder()
        .redirect_uri("http://localhost:8080/oauth2/callback")
        .add_scope(google_oauth::scope::Calendar)
        .secret(&secret.web)
        .build()?;
    println!("{}", client.generate_url());
    Ok(())
}
