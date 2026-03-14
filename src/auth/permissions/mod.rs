use core::fmt;

use askama::filters::HtmlSafe;
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

impl HtmlSafe for GroupRole {}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceType {
    Plugin,
    Artifact,
    Group,
    MachineToken,
    User,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Artifact => "artifact",
            Self::Group => "group",
            Self::MachineToken => "machine_token",
            Self::User => "user",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Create,
    Get,
    List,
    Delete,
    Manage,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Get => "get",
            Self::List => "list",
            Self::Delete => "delete",
            Self::Manage => "manage",
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
    pub fn on_type(resource_type: ResourceType, action: Action) -> Self {
        Self {
            resource_type,
            resource_id: None,
            action,
            group_id: None,
        }
    }

    pub fn on_instance(resource_type: ResourceType, id: Uuid, action: Action) -> Self {
        Self {
            resource_type,
            resource_id: Some(id),
            action,
            group_id: None,
        }
    }

    pub fn in_group(mut self, group_id: Uuid) -> Self {
        self.group_id = Some(group_id);
        self
    }
}
