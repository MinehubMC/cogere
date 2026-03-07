#[derive(Debug, Clone)]
pub struct InstanceSettings {
    pub instance_name: String,
    pub allow_user_group_creation: bool,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        Self {
            // https://en.wiktionary.org/wiki/cogere
            instance_name: "cōgere".to_string(),
            allow_user_group_creation: true,
        }
    }
}
