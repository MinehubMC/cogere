#[derive(Debug, Clone)]
pub struct InstanceSettings {
    pub instance_name: String,
    pub allow_user_group_creation: bool,
    pub assembly_timeout_secs: u64,
    pub assembly_expiry_secs: u64,
    pub assembly_cleanup_interval_secs: u64,
    pub assembly_max_age_days: u64,
}

impl Default for InstanceSettings {
    fn default() -> Self {
        Self {
            // https://en.wiktionary.org/wiki/cogere
            instance_name: "cōgere".to_string(),
            allow_user_group_creation: true,
            assembly_timeout_secs: 60 * 15,     // 15 minutes
            assembly_expiry_secs: 60 * 30,      // 30 minutes
            assembly_cleanup_interval_secs: 60, // minute
            assembly_max_age_days: 30,          // 30 days
        }
    }
}
