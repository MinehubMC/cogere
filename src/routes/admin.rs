use crate::{
    auth::auth::AuthSession,
    auth::permissions::InstanceRole,
    errors::{AppError, Error},
    models::auth::PublicUser,
    server::{AppState, reload_settings},
};
use askama::Template;
use axum::{extract::State, response::IntoResponse};
use axum_messages::{Message, Messages};

#[derive(Template)]
#[template(path = "admin/settings.jinja")]
struct SettingsTemplate {
    settings: crate::models::settings::InstanceSettings,
    current_user: Option<PublicUser>,
    messages: Vec<Message>,
}

pub async fn settings_index(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
) -> Result<impl IntoResponse, AppError> {
    let settings = state.settings.read().await.clone();

    let html = SettingsTemplate {
        settings,
        current_user: Some(auth.user().await.ok_or(Error::Unauthorized)?.into()),
        messages: messages.into_iter().collect(),
    }
    .render()?;

    Ok(axum::response::Html(html))
}

pub async fn settings_reload(
    State(state): State<AppState>,
    messages: Messages,
) -> Result<impl IntoResponse, AppError> {
    reload_settings(&state).await?;

    messages.success("Settings reloaded.");

    Ok(axum::response::Redirect::to("/admin/settings"))
}
