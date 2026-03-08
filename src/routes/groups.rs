use askama::Template;
use axum::{extract::State, response::Html};
use axum_messages::{Message, Messages};
use uuid::Uuid;

use crate::{
    auth::{
        auth::AuthSession,
        extractor::AuthenticatedEntity,
        permissions::{
            Action, InstanceRole, PermissionCheck, ResourceType, check::PermissionChecker,
        },
    },
    database::groups::get_memberships_by_user_id,
    errors::{AppError, Error},
    models::auth::{CurrentUser, User},
    server::AppState,
};

#[derive(Debug)]
struct GroupEntry {
    id: Uuid,
    name: String,
}

#[derive(Template)]
#[template(path = "groups.jinja")]
struct GroupsTemplate {
    groups: Vec<GroupEntry>,
    messages: Vec<Message>,
    settings: crate::models::settings::InstanceSettings,
    current_user: Option<CurrentUser>,
}

pub async fn groups_index(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
) -> Result<Html<String>, AppError> {
    let settings = state.settings.read().await.clone();
    let user: User = auth.user().await.ok_or(Error::Unauthorized)?;
    let entity = AuthenticatedEntity::User(user.clone());

    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(ResourceType::Group, Action::List))
        .await?;

    let groups = get_memberships_by_user_id(&state.db, entity.raw_uuid())
        .await?
        .into_iter()
        .map(|r| GroupEntry {
            id: r.id,
            name: r.name,
        })
        .collect();

    let html = GroupsTemplate {
        groups,
        settings,
        messages: messages.into_iter().collect(),
        current_user: Some(user.into()),
    }
    .render()?;

    Ok(Html(html))
}
