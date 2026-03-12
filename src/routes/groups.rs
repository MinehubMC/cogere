use askama::Template;
use axum::{
    Form,
    extract::{Path, State},
    http::HeaderMap,
    response::Html,
};
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
    database::{
        self,
        groups::{get_group_by_id_and_user_id, get_group_members, get_memberships_by_user_id},
    },
    errors::{AppError, Error},
    models::{
        self,
        auth::{PublicUser, User},
        settings::InstanceSettings,
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
#[template(path = "groups/index.jinja")]
struct GroupsTemplate {
    groups: Vec<GroupEntry>,
    messages: Vec<Message>,
    settings: crate::models::settings::InstanceSettings,
    current_user: Option<PublicUser>,
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
#[template(path = "groups/partials/group_card.jinja")]
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

async fn load_group_context(
    state: &AppState,
    auth: &AuthSession,
    group_id: Uuid,
) -> Result<(GroupEntry, User), AppError> {
    let user: User = auth.user().await.ok_or(Error::Unauthorized)?;
    let entity = AuthenticatedEntity::User(user.clone());
    PermissionChecker::new(&state.db, &entity)
        .require(PermissionCheck::on_type(ResourceType::Group, Action::Get))
        .await?;
    let group = get_group_by_id_and_user_id(&state.db, group_id, entity.raw_uuid())
        .await?
        .into();
    Ok((group, user))
}

#[derive(Template)]
#[template(path = "groups/overview.jinja")]
struct GroupOverviewTemplate {
    group: GroupEntry,
    settings: InstanceSettings,
    messages: Vec<Message>,
    current_user: Option<PublicUser>,
    active_tab: &'static str,
    is_htmx: bool,
}

#[derive(Template)]
#[template(path = "groups/members.jinja")]
struct GroupMembersTemplate {
    group: GroupEntry,
    members: Vec<PublicUser>,
    settings: InstanceSettings,
    messages: Vec<Message>,
    current_user: Option<PublicUser>,
    active_tab: &'static str,
    is_htmx: bool,
}

#[derive(Template)]
#[template(path = "groups/partials/overview_content.jinja")]
struct GroupOverviewPartialTemplate {
    group: GroupEntry,
    active_tab: &'static str,
    is_htmx: bool,
}

#[derive(Template)]
#[template(path = "groups/partials/members_content.jinja")]
struct GroupMembersPartialTemplate {
    group: GroupEntry,
    members: Vec<PublicUser>,
    active_tab: &'static str,
    is_htmx: bool,
}

pub async fn groups_detail(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let (group, user) = load_group_context(&state, &auth, group_id).await?;

    let html = if headers.contains_key("hx-request") {
        GroupOverviewPartialTemplate {
            group,
            active_tab: "overview",
            is_htmx: true,
        }
        .render()?
    } else {
        GroupOverviewTemplate {
            group,
            settings: state.settings.read().await.clone(),
            messages: messages.into_iter().collect(),
            current_user: Some(user.into()),
            active_tab: "overview",
            is_htmx: false,
        }
        .render()?
    };

    Ok(Html(html))
}

pub async fn groups_members(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let (group, user) = load_group_context(&state, &auth, group_id).await?;
    let members = get_group_members(&state.db, group_id)
        .await?
        .into_iter()
        .map(PublicUser::from)
        .collect();

    let html = if headers.contains_key("hx-request") {
        GroupMembersPartialTemplate {
            group,
            members,
            active_tab: "members",
            is_htmx: true,
        }
        .render()?
    } else {
        GroupMembersTemplate {
            group,
            members,
            settings: state.settings.read().await.clone(),
            messages: messages.into_iter().collect(),
            current_user: Some(user.into()),
            active_tab: "members",
            is_htmx: false,
        }
        .render()?
    };

    Ok(Html(html))
}
