use uuid::Uuid;

pub enum BlobEntityType {
    Plugin { id: Uuid },
    Assembly { id: Uuid },
}

impl BlobEntityType {
    pub fn as_type_str(&self) -> &'static str {
        match self {
            BlobEntityType::Plugin { .. } => "plugin",
            BlobEntityType::Assembly { .. } => "assembly",
        }
    }

    pub fn id(&self) -> Uuid {
        match self {
            BlobEntityType::Plugin { id } => *id,
            BlobEntityType::Assembly { id } => *id,
        }
    }
}
