use base64::Engine;
use password_auth::generate_hash;
use sqlx::{SqliteConnection, SqlitePool};
use uuid::Uuid;

use crate::{
    auth::permissions::{Action, ResourceType},
    models::{
        auth::{MachineKey, MachineKeyPermission, PublicMachineKey},
        groups::GroupMachineKey,
    },
};

struct Found;

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for Found {
    fn from_row(_: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(Found)
    }
}

pub async fn get_machinekey_by_id(
    pool: &SqlitePool,
    id: Uuid,
) -> Result<Option<MachineKey>, sqlx::Error> {
    sqlx::query_as::<_, MachineKey>("SELECT * FROM machine_keys WHERE id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn machine_key_has_specific_permission(
    pool: &SqlitePool,
    key_id: Uuid,
    resource_type: ResourceType,
    resource_id: Uuid,
    action: Action,
) -> Result<bool, sqlx::Error> {
    let key_id = key_id.to_string();
    let resource_id = resource_id.to_string();

    let row = sqlx::query_as::<_, Found>(
        "SELECT 1 FROM machine_key_permissions
             WHERE key_id = ? AND resource_type = ?
               AND resource_id = ? AND action = ?",
    )
    .bind(key_id)
    .bind(resource_type)
    .bind(resource_id)
    .bind(action)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub async fn machine_key_has_wide_permission(
    pool: &SqlitePool,
    key_id: Uuid,
    resource_type: ResourceType,
    action: Action,
) -> Result<bool, sqlx::Error> {
    let key_id = key_id.to_string();

    let row = sqlx::query_as::<_, Found>(
        "SELECT 1 FROM machine_key_permissions
         WHERE key_id = ? AND resource_type = ?
           AND resource_id IS NULL AND action = ?",
    )
    .bind(key_id)
    .bind(resource_type)
    .bind(action)
    .fetch_optional(pool)
    .await?;

    Ok(row.is_some())
}

pub struct CreatedMachineKey {
    pub key: PublicMachineKey,
    pub secret: String,
}

pub async fn create_machine_key(
    executor: &mut SqliteConnection,
    group_id: Uuid,
    description: &str,
) -> Result<CreatedMachineKey, sqlx::Error> {
    let id = Uuid::now_v7();
    let id_str = id.to_string();
    let group_id_str = group_id.to_string();

    let raw: [u8; 32] = rand::random();
    let secret = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let plaintext = secret.clone();
    let key_hash = tokio::task::spawn_blocking(move || generate_hash(&secret))
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;

    sqlx::query!(
        "INSERT INTO machine_keys (id, group_id, description, key_hash)
         VALUES (?, ?, ?, ?)",
        id_str,
        group_id_str,
        description,
        key_hash,
    )
    .execute(&mut *executor)
    .await?;

    let key = sqlx::query_as::<_, MachineKey>(
        "SELECT id, group_id, description, key_hash FROM machine_keys WHERE id = ?",
    )
    .bind(&id_str)
    .fetch_one(&mut *executor)
    .await?
    .into();

    Ok(CreatedMachineKey {
        key,
        secret: plaintext,
    })
}

pub async fn add_machine_key_permission(
    executor: &mut SqliteConnection,
    key_id: Uuid,
    resource_type: ResourceType,
    resource_id: Option<Uuid>,
    action: Action,
) -> Result<(), sqlx::Error> {
    let key_id_str = key_id.to_string();
    let resource_id_str = resource_id.map(|id| id.to_string());

    sqlx::query!(
        "INSERT OR IGNORE INTO machine_key_permissions (key_id, resource_type, resource_id, action) VALUES (?, ?, ?, ?)",
        key_id_str,
        resource_type,
        resource_id_str,
        action,
    )
    .execute(&mut *executor)
    .await?;

    Ok(())
}

pub async fn remove_machine_key_permission(
    executor: &mut SqliteConnection,
    key_id: Uuid,
    resource_type: ResourceType,
    resource_id: Option<Uuid>,
    action: Action,
) -> Result<(), sqlx::Error> {
    let key_id_str = key_id.to_string();
    let resource_id_str = resource_id.map(|id| id.to_string());

    sqlx::query!(
        "DELETE FROM machine_key_permissions
         WHERE key_id = ?
           AND resource_type = ?
           AND resource_id IS ?
           AND action = ?",
        key_id_str,
        resource_type,
        resource_id_str,
        action,
    )
    .execute(&mut *executor)
    .await?;

    Ok(())
}

pub async fn get_machine_key_permissions(
    executor: &mut SqliteConnection,
    key_id: Uuid,
) -> Result<Vec<MachineKeyPermission>, sqlx::Error> {
    let key_id_str = key_id.to_string();

    sqlx::query_as::<_, MachineKeyPermission>(
        "SELECT key_id, resource_type, resource_id, action FROM machine_key_permissions WHERE key_id = ?",
    )
    .bind(&key_id_str)
    .fetch_all(&mut *executor)
    .await
}

pub async fn delete_machine_key(
    executor: &mut SqliteConnection,
    key_id: Uuid,
) -> Result<(), sqlx::Error> {
    let key_id_str = key_id.to_string();

    sqlx::query!("DELETE FROM machine_keys WHERE id = ?", key_id_str)
        .execute(&mut *executor)
        .await?;

    Ok(())
}

pub async fn get_group_machine_keys(
    executor: &mut SqliteConnection,
    group_id: Uuid,
) -> Result<Vec<GroupMachineKey>, sqlx::Error> {
    let group_id_str = group_id.to_string();

    let mut keys = sqlx::query_as::<_, GroupMachineKey>(
        "SELECT id, description FROM machine_keys WHERE group_id = ?",
    )
    .bind(&group_id_str)
    .fetch_all(&mut *executor)
    .await?;

    for key in &mut keys {
        key.permissions = get_machine_key_permissions(&mut *executor, key.id).await?;
    }

    Ok(keys)
}
