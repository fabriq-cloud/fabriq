pub struct Context<'a> {
    pub endpoint: &'a str,
    pub token: &'a str,
}

impl<'a> Context<'a> {
    pub fn new(endpoint: &'a str, token: &'a str) -> Self {
        Self { endpoint, token }
    }
}
