use uuid::Uuid;

use crate::assembler::ArtifactCoordinate;

#[derive(Clone)]
pub struct AssemblyJob {
    pub id: Uuid,
    pub group_id: Uuid,
    pub artifacts: Vec<ArtifactCoordinate>,
}
