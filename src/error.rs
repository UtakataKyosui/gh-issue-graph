use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("GitHub API error: {0}")]
    Api(String),

    #[error("Rate limit exceeded. Resets at {reset_at}")]
    RateLimit { reset_at: String },

    #[error("Repository not found: {0}")]
    RepoNotFound(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Sub-issues API not available for this repository")]
    SubIssuesUnavailable,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
