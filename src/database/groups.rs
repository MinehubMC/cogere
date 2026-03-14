use crate::{
    auth::permissions::GroupRole,
    models::{
        groups::{Group, GroupMachineKey, GroupMember},
        plugins::{GroupPluginSummary, PluginSource, Visibility},
    },
};
use sqlx::SqlitePool;
use uuid::Uuid;

pub async fn get_membership_by_user_and_group_id(
    pool: &SqlitePool,
    user_id: Uuid,
    group_id: Uuid,
) -> Result<Option<GroupRole>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        group_role: GroupRole,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT group_role FROM group_members WHERE user_id = ? AND group_id = ?",
    )
    .bind(user_id.to_string())
    .bind(group_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| r.group_role))
}

pub async fn get_memberships_by_user_id(
    pool: &SqlitePool,
    user_id: Uuid,
) -> Result<Vec<Group>, sqlx::Error> {
    sqlx::query_as::<_, Group>("SELECT g.* FROM groups g LEFT JOIN group_members gm ON g.id = gm.group_id WHERE gm.user_id = ?")
        .bind(user_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn create_group(
    pool: &SqlitePool,
    name: String,
    description: String,
    user_id: Uuid,
) -> Result<Group, sqlx::Error> {
    let group_id = Uuid::now_v7();

    let mut tx = pool.begin().await?;

    let group = sqlx::query_as::<_, Group>(
        "INSERT INTO groups (id, name, description) VALUES (?, ?, ?) RETURNING *",
    )
    .bind(group_id.to_string())
    .bind(&name)
    .bind(&description)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO group_members (user_id, group_id, group_role) VALUES (?, ?, 'owner')")
        .bind(user_id.to_string())
        .bind(group_id.to_string())
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(group)
}

pub async fn get_group_by_id_and_user_id(
    pool: &SqlitePool,
    group_id: Uuid,
    user_id: Uuid,
) -> Result<Group, sqlx::Error> {
    sqlx::query_as::<_, Group>("SELECT g.* FROM groups g LEFT JOIN group_members gm ON g.id = gm.group_id WHERE g.id = ? AND gm.user_id = ?")
        .bind(group_id.to_string())
        .bind(user_id.to_string())
        .fetch_one(pool)
        .await
}

pub async fn get_group_by_id(
    pool: &SqlitePool,
    group_id: Uuid,
) -> Result<Option<Group>, sqlx::Error> {
    sqlx::query_as::<_, Group>("SELECT * FROM groups WHERE id = ?")
        .bind(group_id.to_string())
        .fetch_optional(pool)
        .await
}

pub async fn get_group_members(
    pool: &SqlitePool,
    group_id: Uuid,
) -> Result<Vec<GroupMember>, sqlx::Error> {
    sqlx::query_as::<_, GroupMember>("SELECT u.id, u.username, gm.group_role FROM group_members gm LEFT JOIN users u ON u.id = gm.user_id WHERE gm.group_id = ?")
        .bind(group_id.to_string())
        .fetch_all(pool)
        .await
}

pub async fn get_group_plugins(
    pool: &SqlitePool,
    group_id: Uuid,
) -> Result<Vec<GroupPluginSummary>, sqlx::Error> {
    let group_id_str = group_id.to_string();
    let rows = sqlx::query!(
        r#"
        SELECT
            p.id AS "id!",
            p.plugin_artifact_id AS "plugin_artifact_id!",
            p.plugin_group_id AS "plugin_group_id!",
            p.source AS "source!",
            p.external_provider AS external_provider,
            p.external_id AS external_id,
            gp.visibility AS "visibility!",
            gp.is_owner AS "is_owner!: i64",
            pv.version AS latest_version,
            pv.blob_id AS latest_blob_id
        FROM group_plugins gp
        JOIN plugins p ON p.id = gp.plugin_id
        LEFT JOIN plugin_versions pv ON pv.id = (
            SELECT id FROM plugin_versions
            WHERE plugin_id = p.id
            ORDER BY version DESC
            LIMIT 1
        )
        WHERE gp.group_id = ?
        ORDER BY gp.attached_at DESC
        "#,
        group_id_str,
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|r| {
            let id = Uuid::parse_str(&r.id).map_err(|e| sqlx::Error::ColumnDecode {
                index: "id".to_string(),
                source: Box::new(e),
            })?;

            let source = match r.source.as_str() {
                "local" => PluginSource::Local,
                _ => PluginSource::External {
                    provider: r.external_provider.unwrap_or_default(),
                    external_id: r.external_id.unwrap_or_default(),
                },
            };

            let visibility = match r.visibility.as_str() {
                "public" => Visibility::Public,
                _ => Visibility::Private,
            };

            let is_cached = r.latest_blob_id.is_some();

            Ok(GroupPluginSummary {
                id,
                plugin_artifact_id: r.plugin_artifact_id,
                plugin_group_id: r.plugin_group_id,
                source,
                visibility,
                is_owner: r.is_owner != 0,
                latest_version: Some(r.latest_version),
                is_cached,
            })
        })
        .collect()
}

pub async fn get_group_machine_keys(
    pool: &SqlitePool,
    group_id: Uuid,
) -> Result<Vec<GroupMachineKey>, sqlx::Error> {
    sqlx::query_as::<_, GroupMachineKey>("SELECT * FROM machine_keys WHERE group_id = ?")
        .bind(group_id.to_string())
        .fetch_all(pool)
        .await
}
