pub struct Context<'a> {
    pub endpoint: &'a str,
    pub token: &'a str,
}

impl<'a> Default for Context<'a> {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:50051",
            token: "tim",
        }
    }
}
