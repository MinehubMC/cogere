use sqlx::SqlitePool;
use uuid::Uuid;

use crate::models::auth::User;

pub async fn get_user_by_username(
    pool: &SqlitePool,
    username: String,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("select * from users where username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
}

pub async fn get_user_by_id(pool: &SqlitePool, id: &Uuid) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("select * from users where id = ?")
        .bind(id.to_string())
        .fetch_optional(pool)
        .await
}
