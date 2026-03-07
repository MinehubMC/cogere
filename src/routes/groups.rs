use askama::Template;
use axum::{extract::State, response::Html};
use axum_messages::{Message, Messages};
use uuid::Uuid;

use crate::{
    auth::{
        extractor::AuthenticatedEntity,
        permissions::{Action, PermissionCheck, ResourceType, check::PermissionChecker},
    },
    database::groups::get_memberships_by_user_id,
    errors::AppError,
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
}

pub async fn groups_index(
    State(state): State<AppState>,
    entity: AuthenticatedEntity,
    messages: Messages,
) -> Result<Html<String>, AppError> {
    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(
            ResourceType::Plugin,
            Action::Create,
        ))
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
        messages: messages.into_iter().collect(),
    }
    .render()?;

    Ok(Html(html))
}
