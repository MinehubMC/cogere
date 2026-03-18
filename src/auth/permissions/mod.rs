use core::fmt;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod check;

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum InstanceRole {
    User,
    InstanceAdmin,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
pub enum GroupRole {
    Viewer = 0,
    Editor = 1,
    Admin = 2,
    Owner = 3,
}

impl fmt::Display for GroupRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupRole::Viewer => write!(f, "Viewer"),
            GroupRole::Editor => write!(f, "Editor"),
            GroupRole::Admin => write!(f, "Admin"),
            GroupRole::Owner => write!(f, "Owner"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid group role: {0}")]
pub struct GroupRoleParseError(String);

impl std::str::FromStr for GroupRole {
    type Err = GroupRoleParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "editor" => Ok(Self::Editor),
            "viewer" => Ok(Self::Viewer),
            other => Err(GroupRoleParseError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Plugin,
    Artifact,
    Group,
    MachineKey,
    User,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid resource type: {0}")]
pub struct ResourceTypeParseError(String);

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceType::Plugin => write!(f, "plugin"),
            ResourceType::Artifact => write!(f, "artifact"),
            ResourceType::Group => write!(f, "group"),
            ResourceType::MachineKey => write!(f, "machine_key"),
            ResourceType::User => write!(f, "user"),
        }
    }
}

impl std::str::FromStr for ResourceType {
    type Err = ResourceTypeParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "plugin" => Ok(Self::Plugin),
            "artifact" => Ok(Self::Artifact),
            "group" => Ok(Self::Group),
            "machine_key" => Ok(Self::MachineKey),
            "user" => Ok(Self::User),
            other => Err(ResourceTypeParseError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::Type, Deserialize, Serialize)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Create,
    Get,
    List,
    Delete,
    Manage,
}

#[derive(Debug, thiserror::Error)]
#[error("invalid action: {0}")]
pub struct ActionParseError(String);

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::Create => write!(f, "create"),
            Action::Get => write!(f, "get"),
            Action::List => write!(f, "list"),
            Action::Delete => write!(f, "delete"),
            Action::Manage => write!(f, "manage"),
        }
    }
}

impl std::str::FromStr for Action {
    type Err = ActionParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "create" => Ok(Self::Create),
            "get" => Ok(Self::Get),
            "list" => Ok(Self::List),
            "delete" => Ok(Self::Delete),
            "manage" => Ok(Self::Manage),
            other => Err(ActionParseError(other.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PermissionCheck {
    pub resource_type: ResourceType,
    pub resource_id: Option<Uuid>,
    pub action: Action,
    pub group_id: Option<Uuid>,
}

impl PermissionCheck {
    pub fn new(resource_type: ResourceType, action: Action) -> Self {
        Self {
            resource_type,
            resource_id: None,
            action,
            group_id: None,
        }
    }

    pub fn with_resource_id(mut self, id: Uuid) -> Self {
        self.resource_id = Some(id);
        self
    }

    pub fn in_group(mut self, group_id: Uuid) -> Self {
        self.group_id = Some(group_id);
        self
    }
}
