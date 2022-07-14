use serde::{Deserialize, Serialize};
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub pat: String,
    pub login: String,
}

impl Profile {
    pub async fn load() -> anyhow::Result<Self> {
        let auth_path = Profile::build_config_path().await?;

        let profile_json = fs::read_to_string(auth_path).await?;

        let profile: Profile = serde_json::from_str(&profile_json)?;

        Ok(profile)
    }

    async fn build_config_path() -> anyhow::Result<std::path::PathBuf> {
        let mut path = dirs::home_dir().unwrap();

        path.push(".akira");

        fs::create_dir_all(&path).await?;

        path.push("auth.yaml");

        Ok(path)
    }

    pub async fn save(&self) -> anyhow::Result<()> {
        let auth_path = Profile::build_config_path().await?;

        let profile_json = serde_json::to_string(&self)?;

        Ok(fs::write(&auth_path, profile_json).await?)
    }
}
