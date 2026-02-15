use askama::Template;
use axum::{extract::State, http::StatusCode, response::Html};
use axum_messages::{Message, Messages};

use crate::{database::machine_keys::get_all_machinekeys, server::AppState};

#[derive(Debug)]
struct MachineKeyEntry {
    id: String,
    description: String,
    group_id: String,
}

#[derive(Template)]
#[template(path = "machine_keys.jinja")]
struct MachineKeysTemplate {
    keys: Vec<MachineKeyEntry>,
    messages: Vec<Message>,
}

pub async fn machinekeys_index(
    State(state): State<AppState>,
    messages: Messages,
) -> Result<Html<String>, (StatusCode, String)> {
    let keys = get_all_machinekeys(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}")))?
        .into_iter()
        .map(|k| MachineKeyEntry {
            id: k.id.to_string(),
            description: k.description,
            group_id: k.group_id.to_string(),
        })
        .collect();

    let html = MachineKeysTemplate {
        keys,
        messages: messages.into_iter().collect(),
    }
    .render()
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Template error: {e}"),
        )
    })?;

    Ok(Html(html))
}
