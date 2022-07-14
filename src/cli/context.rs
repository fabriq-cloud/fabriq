use crate::profile::Profile;

pub struct Context<'a> {
    pub endpoint: &'a str,
    pub profile: Profile,
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        let profile = Profile::load().unwrap();

        Self {
            endpoint: "http://localhost:50051",
            profile,
        }
    }
}
