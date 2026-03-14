use core::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginSource {
    Local,
    External {
        provider: String,
        external_id: String,
    },
}

impl fmt::Display for PluginSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginSource::Local => f.write_str("local"),
            PluginSource::External {
                provider,
                external_id,
            } => {
                write!(f, "external:{provider}:{external_id}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Private,
}

impl fmt::Display for Visibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Visibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Private => "private",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub id: Uuid,
    pub plugin_group_id: String,
    pub plugin_artifact_id: String,
    pub source: PluginSource,
}

#[derive(Debug, Clone)]
pub struct PluginVersion {
    pub id: Uuid,
    pub plugin_id: Uuid,
    pub version: String,
    // None = not in cache yet
    pub blob_id: Option<Uuid>,
}

impl PluginVersion {
    pub fn is_cached(&self) -> bool {
        self.blob_id.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Blob {
    pub id: Uuid,
    pub sha256: String,
    pub size_bytes: u64,
    pub ref_count: u32,
}

impl sqlx::FromRow<'_, sqlx::sqlite::SqliteRow> for Blob {
    fn from_row(row: &'_ sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;

        let id_str: String = row.try_get("id")?;
        let id = uuid::Uuid::parse_str(&id_str).map_err(|e| sqlx::Error::ColumnDecode {
            index: "id".to_string(),
            source: Box::new(e),
        })?;

        Ok(Blob {
            id,
            sha256: row.try_get("sha256")?,
            size_bytes: row.try_get("size_bytes")?,
            ref_count: row.try_get("ref_count")?,
        })
    }
}

pub struct GroupPluginSummary {
    pub id: Uuid,
    pub plugin_artifact_id: String,
    pub plugin_group_id: String,
    pub source: PluginSource,
    pub visibility: Visibility,
    pub is_owner: bool,
    pub latest_version: Option<String>,
    pub is_cached: bool,
}
