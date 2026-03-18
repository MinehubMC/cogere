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
        auth::{MachineKeyPermission, PublicMachineKey, PublicUser, User},
        groups::{GroupMachineKey, GroupMember},
        plugins::GroupPluginSummary,
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

impl From<models::groups::Group> for GroupEntry {
    fn from(value: models::groups::Group) -> Self {
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
        .require(PermissionCheck::new(ResourceType::Group, Action::List))
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
        .require(PermissionCheck::new(ResourceType::Group, Action::Create))
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
    resource_check: Option<PermissionCheck>,
) -> Result<(GroupEntry, User), AppError> {
    let user: User = auth.user().await.ok_or(Error::Unauthorized)?;
    let entity = AuthenticatedEntity::User(user.clone());
    let checker = PermissionChecker::new(&state.db, &entity);

    checker
        .require(PermissionCheck::new(ResourceType::Group, Action::Get).in_group(group_id))
        .await?;

    if let Some(check) = resource_check {
        checker.require(check.in_group(group_id)).await?;
    }

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
    members: Vec<GroupMember>,
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
    members: Vec<GroupMember>,
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
    let (group, user) = load_group_context(&state, &auth, group_id, None).await?;

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
    let (group, user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(ResourceType::User, Action::List)),
    )
    .await?;

    let members = get_group_members(&state.db, group_id).await?;

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

#[derive(Template)]
#[template(path = "groups/plugins.jinja")]
struct GroupPluginsTemplate {
    group: GroupEntry,
    plugins: Vec<GroupPluginSummary>,
    settings: InstanceSettings,
    messages: Vec<Message>,
    current_user: Option<PublicUser>,
    active_tab: &'static str,
    is_htmx: bool,
}

#[derive(Template)]
#[template(path = "groups/partials/plugins_content.jinja")]
struct GroupPluginsPartialTemplate {
    group: GroupEntry,
    plugins: Vec<GroupPluginSummary>,
    active_tab: &'static str,
    is_htmx: bool,
}

pub async fn groups_plugins(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let (group, user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(ResourceType::Plugin, Action::List)),
    )
    .await?;

    let plugins = database::groups::get_group_plugins(&state.db, group_id).await?;

    let html = if headers.contains_key("hx-request") {
        GroupPluginsPartialTemplate {
            group,
            plugins,
            active_tab: "plugins",
            is_htmx: true,
        }
        .render()?
    } else {
        GroupPluginsTemplate {
            group,
            plugins,
            settings: state.settings.read().await.clone(),
            messages: messages.into_iter().collect(),
            current_user: Some(user.into()),
            active_tab: "plugins",
            is_htmx: false,
        }
        .render()?
    };

    Ok(Html(html))
}

#[derive(Template)]
#[template(path = "groups/machinekeys.jinja")]
struct GroupMachineKeysTemplate {
    group: GroupEntry,
    keys: Vec<GroupMachineKey>,
    settings: InstanceSettings,
    messages: Vec<Message>,
    current_user: Option<PublicUser>,
    active_tab: &'static str,
    is_htmx: bool,
}

#[derive(Template)]
#[template(path = "groups/partials/machinekeys_content.jinja")]
struct GroupMachineKeysPartialTemplate {
    group: GroupEntry,
    keys: Vec<GroupMachineKey>,
    active_tab: &'static str,
    is_htmx: bool,
}

pub async fn group_machine_keys(
    State(state): State<AppState>,
    auth: AuthSession,
    messages: Messages,
    headers: HeaderMap,
    Path(group_id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    let (group, user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(ResourceType::MachineKey, Action::List)),
    )
    .await?;

    let mut tx = state.db.begin().await?;

    let keys = database::machine_keys::get_group_machine_keys(&mut *tx, group_id).await?;

    tx.commit().await?;

    let html = if headers.contains_key("hx-request") {
        GroupMachineKeysPartialTemplate {
            group,
            keys,
            active_tab: "machinekeys",
            is_htmx: true,
        }
        .render()?
    } else {
        GroupMachineKeysTemplate {
            group,
            keys,
            settings: state.settings.read().await.clone(),
            messages: messages.into_iter().collect(),
            current_user: Some(user.into()),
            active_tab: "machinekeys",
            is_htmx: false,
        }
        .render()?
    };

    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct CreateMachineKeyForm {
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct AddPermissionForm {
    pub resource_type: ResourceType,
    pub action: Action,
    pub resource_id: Option<Uuid>,
}

#[derive(Template)]
#[template(path = "groups/partials/machine_key_created.jinja")]
struct NewMachineKeyCreatedTemplate {
    group: GroupEntry,
    key: PublicMachineKey,
    secret: String,
    key_id: Uuid,
    permissions: Vec<MachineKeyPermission>,
    keys: Vec<GroupMachineKey>,
}

#[derive(Template)]
#[template(path = "groups/partials/machine_key_permissions.jinja")]
struct PermissionsListTemplate {
    group: GroupEntry,
    key_id: Uuid,
    permissions: Vec<MachineKeyPermission>,
}

pub async fn create_group_machine_key(
    State(state): State<AppState>,
    auth: AuthSession,
    Path(group_id): Path<Uuid>,
    Form(form): Form<CreateMachineKeyForm>,
) -> Result<Html<String>, AppError> {
    let (group, _user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(
            ResourceType::MachineKey,
            Action::Create,
        )),
    )
    .await?;

    let mut tx = state.db.begin().await?;
    let created =
        database::machine_keys::create_machine_key(&mut *tx, group_id, &form.description).await?;
    tx.commit().await?;

    let mut conn = state.db.acquire().await?;
    let keys = database::machine_keys::get_group_machine_keys(&mut *conn, group_id).await?;

    let html = NewMachineKeyCreatedTemplate {
        group,
        key_id: created.key.id,
        key: created.key,
        secret: created.secret,
        permissions: vec![],
        keys,
    }
    .render()?;

    Ok(Html(html))
}

pub async fn add_group_machine_key_permission(
    State(state): State<AppState>,
    auth: AuthSession,
    Path((group_id, key_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<AddPermissionForm>,
) -> Result<Html<String>, AppError> {
    let (group, _user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(
            ResourceType::MachineKey,
            Action::Manage,
        )),
    )
    .await?;

    let mut tx = state.db.begin().await?;

    database::machine_keys::add_machine_key_permission(
        &mut *tx,
        key_id,
        form.resource_type,
        form.resource_id,
        form.action,
    )
    .await?;

    let permissions = database::machine_keys::get_machine_key_permissions(&mut *tx, key_id).await?;

    tx.commit().await?;

    let html = PermissionsListTemplate {
        group,
        key_id,
        permissions,
    }
    .render()?;

    Ok(Html(html))
}

pub async fn delete_group_machine_key(
    State(state): State<AppState>,
    auth: AuthSession,
    Path((group_id, key_id)): Path<(Uuid, Uuid)>,
) -> Result<Html<String>, AppError> {
    load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(
            ResourceType::MachineKey,
            Action::Delete,
        )),
    )
    .await?;

    let mut tx = state.db.begin().await?;
    database::machine_keys::delete_machine_key(&mut *tx, key_id).await?;
    tx.commit().await?;

    Ok(Html(String::new()))
}

pub async fn remove_group_machine_key_permission(
    State(state): State<AppState>,
    auth: AuthSession,
    Path((group_id, key_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<AddPermissionForm>,
) -> Result<Html<String>, AppError> {
    let (group, _user) = load_group_context(
        &state,
        &auth,
        group_id,
        Some(PermissionCheck::new(
            ResourceType::MachineKey,
            Action::Manage,
        )),
    )
    .await?;

    let mut tx = state.db.begin().await?;

    database::machine_keys::remove_machine_key_permission(
        &mut *tx,
        key_id,
        form.resource_type,
        form.resource_id,
        form.action,
    )
    .await?;

    let permissions = database::machine_keys::get_machine_key_permissions(&mut *tx, key_id).await?;

    tx.commit().await?;

    let html = PermissionsListTemplate {
        group,
        key_id,
        permissions,
    }
    .render()?;

    Ok(Html(html))
}
