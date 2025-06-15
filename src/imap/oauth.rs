use crate::imap::config::OAuthConfig;

pub struct OAuthConfigWithUser<'a> {
    user: &'a str,
    config: &'a OAuthConfig,
}

impl<'a> OAuthConfigWithUser<'a> {
    pub const fn new(user: &'a str, config: &'a OAuthConfig) -> Self {
        Self { user, config }
    }
}

impl imap::Authenticator for OAuthConfigWithUser<'_> {
    type Response = String;
    fn process(&self, _: &[u8]) -> Self::Response {
        format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.user,
            self.config.access_token.secret(),
        )
    }
}
