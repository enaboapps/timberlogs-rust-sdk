use thiserror::Error;

#[derive(Debug, Error)]
pub enum TimberlogsError {
    #[error("validation error: {0}")]
    Validation(String),

    #[error("HTTP error {status}: {body}")]
    Http { status: u16, body: String },

    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("client is not connected")]
    NotConnected,
}
