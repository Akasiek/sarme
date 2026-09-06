use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum LrclibError {
    #[error("could not build the LRCLIB HTTP client: {0}")]
    Client(#[source] reqwest::Error),
    #[error("LRCLIB has no lyrics for this track")]
    NoResult,
    #[error("LRCLIB rate limit was exceeded")]
    RateLimited,
    #[error("LRCLIB returned server error {0}")]
    Server(StatusCode),
    #[error("LRCLIB returned unexpected HTTP status {0}")]
    UnexpectedStatus(StatusCode),
    #[error("LRCLIB request failed: {0}")]
    Request(#[source] reqwest::Error),
    #[error("LRCLIB concurrency limiter was closed")]
    LimiterClosed,
}
