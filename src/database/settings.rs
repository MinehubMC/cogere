use sqlx::{Row, SqlitePool};

use crate::models::settings::InstanceSettings;

pub async fn load_instance_settings(db: &SqlitePool) -> Result<InstanceSettings, sqlx::Error> {
    let rows = sqlx::query("SELECT key, value FROM instance_settings")
        .fetch_all(db)
        .await?;

    let mut settings = InstanceSettings::default();
    for row in rows {
        let key: &str = row.try_get("key")?;
        let value: String = row.try_get("value")?;
        match key {
            "instance_name" => settings.instance_name = value,
            "allow_user_group_creation" => settings.allow_user_group_creation = value == "true",
            _ => {}
        }
    }

    Ok(settings)
}
