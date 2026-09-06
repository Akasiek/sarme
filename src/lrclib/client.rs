use std::sync::Arc;
use std::time::Duration;

use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::sync::Semaphore;

use super::error::LrclibError;
use super::model::{LyricsCandidate, TrackQuery};

const DEFAULT_BASE_URL: &str = "https://lrclib.net";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAY: Duration = Duration::from_millis(300);
const MAX_RETRIES: u32 = 2;
const USER_AGENT: &str = concat!("Sarme/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Debug)]
pub(crate) struct LrclibClient {
    http: reqwest::Client,
    base_url: String,
    concurrency: Arc<Semaphore>,
}

impl LrclibClient {
    pub(crate) fn new() -> Result<Self, LrclibError> {
        Self::with_base_url(DEFAULT_BASE_URL)
    }

    pub(crate) fn with_base_url(base_url: impl Into<String>) -> Result<Self, LrclibError> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent(USER_AGENT)
            .build()
            .map_err(LrclibError::Client)?;

        Ok(Self {
            http,
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            concurrency: Arc::new(Semaphore::new(1)),
        })
    }

    pub(crate) async fn exact(
        &self,
        query: TrackQuery<'_>,
    ) -> Result<LyricsCandidate, LrclibError> {
        let mut parameters = vec![("track_name", query.title), ("artist_name", query.artist)];
        if let Some(album) = query.album {
            parameters.push(("album_name", album));
        }
        let duration = query.duration_seconds.map(|value| value.to_string());
        if let Some(duration) = duration.as_deref() {
            parameters.push(("duration", duration));
        }

        self.get("/api/get", &parameters).await
    }

    pub(crate) async fn search(
        &self,
        query: TrackQuery<'_>,
    ) -> Result<Vec<LyricsCandidate>, LrclibError> {
        let mut parameters = vec![("track_name", query.title), ("artist_name", query.artist)];
        if let Some(album) = query.album {
            parameters.push(("album_name", album));
        }

        self.get("/api/search", &parameters).await
    }

    async fn get<T: DeserializeOwned>(
        &self,
        path: &str,
        parameters: &[(&str, &str)],
    ) -> Result<T, LrclibError> {
        let _permit = self
            .concurrency
            .acquire()
            .await
            .map_err(|_| LrclibError::LimiterClosed)?;
        let url = format!("{}{path}", self.base_url);

        let mut attempt = 0;
        loop {
            match self.http.get(&url).query(parameters).send().await {
                Ok(response) if should_retry(response.status()) && attempt < MAX_RETRIES => {
                    backoff(attempt).await;
                    attempt += 1;
                }
                Ok(response) => return decode(response).await,
                Err(error) if is_retryable(&error) && attempt < MAX_RETRIES => {
                    backoff(attempt).await;
                    attempt += 1;
                }
                Err(error) => return Err(LrclibError::Request(error)),
            }
        }
    }
}

fn should_retry(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn is_retryable(error: &reqwest::Error) -> bool {
    error.is_timeout() || error.is_connect()
}

async fn backoff(attempt: u32) {
    tokio::time::sleep(RETRY_DELAY.saturating_mul(2_u32.saturating_pow(attempt))).await;
}

async fn decode<T: DeserializeOwned>(response: Response) -> Result<T, LrclibError> {
    match response.status() {
        StatusCode::OK => response.json().await.map_err(LrclibError::Request),
        StatusCode::NOT_FOUND => Err(LrclibError::NoResult),
        StatusCode::TOO_MANY_REQUESTS => Err(LrclibError::RateLimited),
        status if status.is_server_error() => Err(LrclibError::Server(status)),
        status => Err(LrclibError::UnexpectedStatus(status)),
    }
}
