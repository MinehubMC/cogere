use axum::{
    extract::State,
    http::{StatusCode, Uri, header},
    response::IntoResponse,
};
use rust_embed::RustEmbed;

use crate::server::AppState;

#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct EmbeddedAssets;

pub async fn serve_asset(State(state): State<AppState>, uri: Uri) -> impl IntoResponse {
    let path = uri
        .path()
        .trim_start_matches("/assets/")
        .trim_start_matches('/');

    let fs_path = state
        .config
        .data_folder
        .join(".cogere")
        .join("assets")
        .join(path);

    if fs_path.exists() {
        match tokio::fs::read(&fs_path).await {
            Ok(bytes) => {
                let mime = mime_guess::from_path(&fs_path).first_or_octet_stream();
                return ([(header::CONTENT_TYPE, mime.as_ref().to_string())], bytes)
                    .into_response();
            }
            Err(_) => {}
        }
    }

    match EmbeddedAssets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                content.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
