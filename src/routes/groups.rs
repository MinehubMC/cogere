use askama::Template;
use axum::{Form, extract::State, response::Html};
use axum_messages::{Message, Messages};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::{
        auth::AuthSession,
        extractor::AuthenticatedEntity,
        permissions::{
            Action, InstanceRole, PermissionCheck, ResourceType, check::PermissionChecker,
        },
    },
    database::{self, groups::get_memberships_by_user_id},
    errors::{AppError, Error},
    models::{
        self,
        auth::{CurrentUser, User},
    },
    server::AppState,
};

#[derive(Debug)]
struct GroupEntry {
    id: Uuid,
    name: String,
    description: String,
}

impl From<models::Group> for GroupEntry {
    fn from(value: models::Group) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
        }
    }
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
        .map(GroupEntry::from)
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

#[derive(Deserialize)]
pub struct CreateGroupForm {
    pub name: String,
    pub description: String,
}

#[derive(Template)]
#[template(path = "partials/group_card.jinja")]
struct GroupCardTemplate {
    group: GroupEntry,
}

pub async fn create_group(
    State(state): State<AppState>,
    auth: AuthSession,
    Form(form): Form<CreateGroupForm>,
) -> Result<Html<String>, AppError> {
    let settings = state.settings.read().await.clone();
    let user: User = auth.user().await.ok_or(Error::Unauthorized)?;
    let entity = AuthenticatedEntity::User(user.clone());

    if !settings.allow_user_group_creation && user.role != InstanceRole::InstanceAdmin {
        return Err(
            Error::NotAllowed("group creation is disabled on this instance".to_string()).into(),
        );
    }

    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(
            ResourceType::Group,
            Action::Create,
        ))
        .await?;

    let group =
        database::groups::create_group(&state.db, form.name, form.description, user.id).await?;

    let html = GroupCardTemplate {
        group: group.into(),
    }
    .render()?;

    Ok(Html(html))
}
