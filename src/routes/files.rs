use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};
use std::fs;

use crate::{Config, server::AppState};

#[derive(Debug)]
struct FileEntry {
    name: String,
    size: u64,
}

#[derive(Template)]
#[template(path = "files.jinja")]
struct FilesTemplate {
    files: Vec<FileEntry>,
}

pub async fn files_index(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let files = read_files(&state.config).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let html = FilesTemplate { files }.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {e}"),
        )
    })?;

    Ok(Html(html))
}

fn read_files(config: &Config) -> Result<Vec<FileEntry>, String> {
    let mut files = fs::read_dir(&config.data_folder)
        .map_err(|e| format!("Failed to read data folder: {e}"))?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name() != ".cogere")
        .filter_map(|e| {
            let path = e.path();
            let metadata = path.metadata().ok()?;
            metadata.is_file().then(|| FileEntry {
                name: e.file_name().to_string_lossy().into_owned(),
                size: metadata.len(),
            })
        })
        .collect::<Vec<_>>();

    files.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(files)
}
