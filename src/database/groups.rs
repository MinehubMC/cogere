use crate::auth::permissions::GroupRole;
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
