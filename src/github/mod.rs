pub mod fetcher;
pub mod graphql;
pub mod queries;

use anyhow::Result;
use octocrab::Octocrab;

/// octocrab クライアントを設定して返す
pub fn build_client(token: &str) -> Result<Octocrab> {
    let client = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;
    Ok(client)
}
