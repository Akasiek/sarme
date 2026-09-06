use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::Router;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::get;

use super::client::LrclibClient;
use super::error::LrclibError;
use super::model::TrackQuery;

const RESPONSE: &str = r#"{
    "id": 123,
    "trackName": "Song",
    "artistName": "Artist",
    "albumName": "Album",
    "duration": 180.0,
    "instrumental": false,
    "plainLyrics": "Plain lyrics",
    "syncedLyrics": "[00:01.00]Synced lyrics"
}"#;

#[tokio::test]
async fn exact_lookup_sends_track_signature_and_decodes_lyrics()
-> Result<(), Box<dyn std::error::Error>> {
    let base_url = serve(Router::new().route("/api/get", get(exact_handler))).await?;
    let client = LrclibClient::with_base_url(base_url)?;

    let candidate = client
        .exact(TrackQuery {
            title: "Song",
            artist: "Artist",
            album: Some("Album"),
            duration_seconds: Some(180),
        })
        .await?;

    assert_eq!(candidate.id, 123);
    assert_eq!(
        candidate.synced_lyrics.as_deref(),
        Some("[00:01.00]Synced lyrics")
    );
    Ok(())
}

#[tokio::test]
async fn distinguishes_no_result_rate_limit_and_server_errors()
-> Result<(), Box<dyn std::error::Error>> {
    let base_url = serve(Router::new().route("/api/get", get(error_handler))).await?;
    let client = LrclibClient::with_base_url(base_url)?;

    for (title, expected) in [
        ("missing", "no-result"),
        ("limited", "rate-limited"),
        ("broken", "server"),
    ] {
        let error = client
            .exact(query(title))
            .await
            .err()
            .ok_or("expected LRCLIB error")?;
        let actual = match error {
            LrclibError::NoResult => "no-result",
            LrclibError::RateLimited => "rate-limited",
            LrclibError::Server(_) => "server",
            other => return Err(format!("unexpected error: {other}").into()),
        };
        assert_eq!(actual, expected);
    }

    Ok(())
}

#[tokio::test]
async fn retries_server_errors_with_a_limit() -> Result<(), Box<dyn std::error::Error>> {
    let attempts = Arc::new(AtomicUsize::new(0));
    let app = Router::new()
        .route("/api/get", get(retry_handler))
        .with_state(attempts.clone());
    let client = LrclibClient::with_base_url(serve(app).await?)?;

    let candidate = client.exact(query("Song")).await?;

    assert_eq!(candidate.id, 123);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    Ok(())
}

fn query(title: &str) -> TrackQuery<'_> {
    TrackQuery {
        title,
        artist: "Artist",
        album: None,
        duration_seconds: None,
    }
}

async fn exact_handler(
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> (StatusCode, &'static str) {
    let valid_user_agent = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("Sarme/"));
    let valid_query = query.get("track_name").is_some_and(|value| value == "Song")
        && query
            .get("artist_name")
            .is_some_and(|value| value == "Artist")
        && query
            .get("album_name")
            .is_some_and(|value| value == "Album")
        && query.get("duration").is_some_and(|value| value == "180");

    if valid_user_agent && valid_query {
        (StatusCode::OK, RESPONSE)
    } else {
        (StatusCode::BAD_REQUEST, "{}")
    }
}

async fn error_handler(Query(query): Query<HashMap<String, String>>) -> (StatusCode, &'static str) {
    match query.get("track_name").map(String::as_str) {
        Some("missing") => (StatusCode::NOT_FOUND, "{}"),
        Some("limited") => (StatusCode::TOO_MANY_REQUESTS, "{}"),
        _ => (StatusCode::SERVICE_UNAVAILABLE, "{}"),
    }
}

async fn retry_handler(State(attempts): State<Arc<AtomicUsize>>) -> (StatusCode, &'static str) {
    if attempts.fetch_add(1, Ordering::SeqCst) == 0 {
        (StatusCode::SERVICE_UNAVAILABLE, "{}")
    } else {
        (StatusCode::OK, RESPONSE)
    }
}

async fn serve(app: Router) -> Result<String, std::io::Error> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://{address}"))
}
