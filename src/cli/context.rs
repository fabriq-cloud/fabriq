use tonic::metadata::MetadataValue;

use crate::profile::Profile;

pub struct Context {
    pub endpoint: String,
    pub profile: Profile,
}

impl Context {
    pub fn get_pat(&self) -> String {
        let pat = self
            .profile
            .pat
            .as_ref()
            .expect("No user context - use `login` command to login with Github first.");

        pat.clone()
    }

    pub fn make_token(&self) -> anyhow::Result<MetadataValue<tonic::metadata::Ascii>> {
        let pat = self.get_pat();
        let token: MetadataValue<_> = pat.parse()?;

        Ok(token)
    }
}

impl Context {
    pub fn new(endpoint: &str) -> Self {
        let profile = Profile::load().unwrap();

        Self {
            endpoint: endpoint.to_owned(),
            profile,
        }
    }
}
