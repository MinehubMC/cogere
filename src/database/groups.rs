use crate::{
    auth::permissions::GroupRole,
    models::{auth::User, groups::Group},
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
) -> Result<Vec<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("SELECT u.* FROM group_members gm LEFT JOIN users u ON u.id = gm.user_id WHERE gm.group_id = ?")
        .bind(group_id.to_string())
        .fetch_all(pool)
        .await
}
