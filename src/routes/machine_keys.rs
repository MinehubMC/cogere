use askama::Template;
use axum::{extract::State, response::Html};
use axum_messages::{Message, Messages};

use crate::{
    auth::auth::AuthSession,
    database::machine_keys::get_all_machinekeys,
    errors::{AppError, Error},
    models::auth::CurrentUser,
    server::AppState,
};

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
    settings: crate::models::settings::InstanceSettings,
    current_user: Option<CurrentUser>,
}

pub async fn machinekeys_index(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
) -> Result<Html<String>, AppError> {
    let settings = state.settings.read().await.clone();
    let user = auth.user().await.ok_or(Error::Unauthorized)?;

    let keys = get_all_machinekeys(&state.db)
        .await?
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
        settings,
        current_user: Some(user.into()),
    }
    .render()?;

    Ok(Html(html))
}
