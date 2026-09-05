#[macro_export]
macro_rules! render_template {
    ($template:expr) => {{
        match $template.render() {
            Ok(html) => axum::response::Html(html),
            Err(_) => $crate::web::routes::error::server_error().await,
        }
    }};
}

#[macro_export]
macro_rules! html_error {
    ($($arg:tt)+) => {
        axum::response::Html(format!(
            r#"<p class="text-red-500 text-sm">{}</p>"#,
            format_args!($($arg)+)
        ))
    };
}
