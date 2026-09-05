use axum::Router;
use axum::extract::MatchedPath;
use axum::http::Request;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use tracing::info_span;

use super::routes::index;

pub fn get_app_router() -> Router {
    Router::new()
        .merge(pages_router())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let matched_path = request
                    .extensions()
                    .get::<MatchedPath>()
                    .map(MatchedPath::as_str);

                info_span!(
                    "http_request",
                    method = ?request.method(),
                    matched_path,
                    some_other_field = tracing::field::Empty,
                )
            }),
        )
}

/// Routes that render full pages navigated to by the user.
fn pages_router() -> Router {
    Router::new().route("/", get(index))
}
