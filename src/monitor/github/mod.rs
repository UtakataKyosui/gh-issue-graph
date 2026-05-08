pub mod branches;
pub mod graphql;

use anyhow::Result;
use octocrab::Octocrab;

pub fn build_client(token: &str) -> Result<Octocrab> {
    let client = Octocrab::builder()
        .personal_token(token.to_string())
        .build()?;
    Ok(client)
}
