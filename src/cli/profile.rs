use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub pat: String,
    pub login: String,
}

impl Profile {
    pub fn load() -> anyhow::Result<Self> {
        let auth_path = Profile::build_config_path()?;

        let profile_json = fs::read_to_string(auth_path)?;

        let profile: Profile = serde_json::from_str(&profile_json)?;

        Ok(profile)
    }

    fn build_config_path() -> anyhow::Result<std::path::PathBuf> {
        let mut path = dirs::home_dir().unwrap();

        path.push(".fabriq");

        fs::create_dir_all(&path).unwrap();

        path.push("auth.yaml");

        Ok(path)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let auth_path = Profile::build_config_path()?;

        let profile_json = serde_json::to_string(&self)?;

        Ok(fs::write(&auth_path, profile_json)?)
    }
}
