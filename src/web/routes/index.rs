use crate::render_template;
use askama::Template;
use axum::response::Html;

#[derive(Template)]
#[template(path = "pages/index.html")]
pub struct IndexTemplate {}

pub async fn index() -> Html<String> {
    let template = IndexTemplate {};
    render_template!(template)
}
