use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../fboot/dist"]
struct Assets;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = Assets::get(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        return ([(header::CONTENT_TYPE, mime)], file.data).into_response();
    }

    // SPA fallback: paths without a file extension are client-side routes
    if !path.contains('.') {
        if let Some(index) = Assets::get("index.html") {
            return ([(header::CONTENT_TYPE, "text/html")], index.data).into_response();
        }
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}
