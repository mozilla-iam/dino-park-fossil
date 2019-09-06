use cis_client::settings::CisSettings;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct AvatarSettings {
    pub s3_bucket: String,
    pub retrieve_by_id_path: String,
    pub picture_api_url: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub cis: CisSettings,
    pub avatar: AvatarSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let file = env::var("DPF_SETTINGS").unwrap_or_else(|_| String::from(".settings.json"));
        let mut s = Config::new();
        s.merge(File::with_name(&file))?;
        s.merge(Environment::new().separator("__"))?;
        s.try_into()
    }
}
