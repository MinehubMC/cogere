use sqlx::SqlitePool;
use uuid::Uuid;

use crate::auth::extractor::AuthenticatedEntity;
use crate::auth::permissions::{Action, GroupRole, InstanceRole, PermissionCheck, ResourceType};
use crate::database::groups::get_membership_by_user_and_group_id;
use crate::database::machine_keys::{
    machine_key_has_specific_permission, machine_key_has_wide_permission,
};
use crate::errors::Error;

pub struct PermissionChecker<'a> {
    db: &'a SqlitePool,
    entity: &'a AuthenticatedEntity,
}

impl<'a> PermissionChecker<'a> {
    pub fn new(db: &'a SqlitePool, entity: &'a AuthenticatedEntity) -> Self {
        Self { db, entity }
    }

    pub async fn can(&self, check: PermissionCheck) -> Result<bool, Error> {
        match self.entity {
            AuthenticatedEntity::User(user) => {
                if user.role == InstanceRole::InstanceAdmin {
                    return Ok(true);
                }

                self.check_user(user.id, &check).await
            }
            AuthenticatedEntity::Machine(key) => {
                self.check_machine(key.id, key.group_id, &check).await
            }
        }
    }

    pub async fn require(&self, check: PermissionCheck) -> Result<(), Error> {
        if self.can(check).await? {
            Ok(())
        } else {
            Err(Error::Forbidden)
        }
    }

    async fn check_user(&self, user_id: Uuid, check: &PermissionCheck) -> Result<bool, Error> {
        let group_id = match check.group_id {
            Some(id) => id,
            None => return Ok(false),
        };

        let membership = get_membership_by_user_and_group_id(self.db, user_id, group_id).await?;

        let role = match membership {
            Some(role) => role,
            None => return Ok(false),
        };

        Ok(self.group_role_allows(&role, &check.resource_type, &check.action))
    }

    async fn check_machine(
        &self,
        key_id: Uuid,
        key_group_id: Uuid,
        check: &PermissionCheck,
    ) -> Result<bool, Error> {
        if let Some(group_id) = check.group_id {
            if key_group_id != group_id {
                return Ok(false);
            }
        }

        let resource_type = check.resource_type.as_str();
        let action = check.action.as_str();

        if let Some(resource_id) = check.resource_id {
            if machine_key_has_specific_permission(
                self.db,
                key_id,
                resource_type,
                resource_id,
                action,
            )
            .await?
            {
                return Ok(true);
            }
        }

        if machine_key_has_wide_permission(self.db, key_id, resource_type, action).await? {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }

    fn group_role_allows(
        &self,
        role: &GroupRole,
        resource: &ResourceType,
        action: &Action,
    ) -> bool {
        match action {
            Action::List | Action::Download => true,
            Action::Create => *role >= GroupRole::Editor,
            Action::Delete => *role >= GroupRole::Admin,
            Action::Manage => match resource {
                ResourceType::Group
                | ResourceType::Plugin
                | ResourceType::Artifact
                | ResourceType::MachineToken => *role >= GroupRole::Admin,
                ResourceType::User => *role >= GroupRole::Owner,
            },
        }
    }
}
