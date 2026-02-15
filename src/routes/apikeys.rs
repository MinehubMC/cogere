use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};

use crate::{AppState, database::apikeys::get_all_apikeys};

#[derive(Debug)]
struct APIKeyEntry {
    id: String,
    description: String,
    role: String,
}

#[derive(Template)]
#[template(path = "apikeys.jinja")]
struct APIKeysTemplate {
    keys: Vec<APIKeyEntry>,
}

pub async fn apikeys_index(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let keys = get_all_apikeys(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .into_iter()
        .map(|k| APIKeyEntry {
            id: k.id.to_string(),
            description: k.description,
            role: k.role.to_string(),
        })
        .collect();

    let html = APIKeysTemplate { keys }.render().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {e}"),
        )
    })?;

    Ok(Html(html))
}
