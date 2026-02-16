use crate::models::auth::{MachineKey, Role, User};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    UploadPlugin,
    DeletePlugin,
    ManageUsers,
}

pub trait HasPermissions {
    fn has_permission(&self, permission: Permission) -> bool;
    fn identifier(&self) -> String;
}

impl HasPermissions for User {
    fn has_permission(&self, permission: Permission) -> bool {
        match permission {
            Permission::UploadPlugin => matches!(self.role, Role::Admin | Role::User),
            Permission::DeletePlugin => matches!(self.role, Role::Admin),
            Permission::ManageUsers => matches!(self.role, Role::Admin),
        }
    }

    fn identifier(&self) -> String {
        format!("user:{}", self.username)
    }
}

impl HasPermissions for MachineKey {
    fn has_permission(&self, permission: Permission) -> bool {
        match permission {
            Permission::UploadPlugin => true,
            Permission::DeletePlugin => false,
            Permission::ManageUsers => false,
        }
    }

    fn identifier(&self) -> String {
        format!("machine:{}", self.description)
    }
}
