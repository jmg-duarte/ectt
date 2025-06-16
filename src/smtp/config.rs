use oauth2::{
    basic::BasicClient, AccessToken, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet,
    RefreshToken, TokenUrl,
};

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum SendBackend {
    Smtp(SmtpConfig),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub login: String,
    pub auth: Auth,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    Password(PasswordConfig),
    OAuth(OAuthConfig),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PasswordConfig {
    pub raw: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct OAuthConfig {
    pub client_id: ClientId,
    pub client_secret: ClientSecret,
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
    #[serde(alias = "auth_uri")]
    pub auth_url: AuthUrl,
    #[serde(alias = "token_uri")]
    pub token_url: TokenUrl,
}

impl OAuthConfig {
    pub fn get_client(
        self,
    ) -> BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet> {
        BasicClient::new(self.client_id)
            .set_client_secret(self.client_secret)
            .set_auth_uri(self.auth_url)
            .set_token_uri(self.token_url)
    }
}

#[cfg(test)]
mod test {
    use oauth2::{AccessToken, AuthUrl, ClientId, ClientSecret, RefreshToken, TokenUrl};
    use serde_json::json;

    use super::{Auth, OAuthConfig, PasswordConfig, SendBackend, SmtpConfig};

    /// Compilation will fail if for some reason the types stop implementing serde::Deserialize
    #[test]
    fn ensure_deserialize() {
        fn impls_deserialize<'de, T>()
        where
            T: serde::Deserialize<'de>,
        {
        }
        impls_deserialize::<SendBackend>();
        impls_deserialize::<Auth>();
    }

    #[test]
    fn ensure_smtp_password_format() {
        let json = json!({
            "type": "smtp",
            "host": "smtp.example.com",
            "port": 465,
            "login": "jose@example.com",
            "auth": {
                "type": "password",
                "raw": "super-secret"
            }
        });
        let SendBackend::Smtp(SmtpConfig {
            host,
            port,
            login,
            auth,
        }) = serde_json::from_value(json).unwrap();

        assert_eq!(host, "smtp.example.com".to_string());
        assert_eq!(port, 993);
        assert_eq!(login, "jose@example.com");
        // Defer the auth to the other tests
        assert!(matches!(auth, Auth::Password { .. }));
    }

    #[test]
    fn ensure_smtp_oath_format() {
        let json = json!({
            "type": "smtp",
            "host": "smtp.example.com",
            "port": 993,
            "login": "jose@example.com",
            "auth": {
                "type": "oauth",
                "client_id": "client-id",
                "client_secret": "client-secret",
                "auth_url": "https://localhost",
                "token_url": "https://localhost",
                "access_token": "access-token",
                "refresh_token": "refresh-token",
            }
        });
        let SendBackend::Smtp(SmtpConfig {
            host,
            port,
            login,
            auth,
        }) = serde_json::from_value(json).unwrap();

        assert_eq!(host, "smtp.example.com".to_string());
        assert_eq!(port, 993);
        assert_eq!(login, "jose@example.com");
        // Defer the auth to the other tests
        assert!(matches!(auth, Auth::OAuth { .. }));
    }

    #[test]
    fn ensure_auth_password_format() {
        let json = json!({
            "type": "password",
            "raw": "super-secret"
        });
        let Auth::Password(PasswordConfig { raw }) = serde_json::from_value::<Auth>(json).unwrap()
        else {
            panic!("wrong format")
        };
        assert_eq!(raw, "super-secret");
    }

    #[test]
    fn ensure_auth_oauth_format() {
        // Test values, this GApp has been deleted already
        let json = json!({
            "type": "oauth",
            "client_id": "client-id",
            "client_secret": "client-secret",
            "auth_url": "https://localhost",
            "token_url": "https://localhost",
            "access_token": "access-token",
            "refresh_token": "refresh-token",
        });
        let Auth::OAuth(OAuthConfig {
            client_id,
            client_secret,
            access_token,
            refresh_token,
            auth_url,
            token_url,
        }) = serde_json::from_value::<Auth>(json).unwrap()
        else {
            panic!("wrong format");
        };

        assert_eq!(client_id, ClientId::new("client-id".to_string()));
        assert_eq!(
            client_secret.into_secret(),
            ClientSecret::new("client-secret".to_string()).into_secret()
        );
        assert_eq!(
            auth_url,
            AuthUrl::new("https://localhost".to_string()).unwrap()
        );
        assert_eq!(
            token_url,
            TokenUrl::new("https://localhost".to_string()).unwrap()
        );
        assert_eq!(
            access_token.into_secret(),
            AccessToken::new("access-token".to_string()).into_secret()
        );
        assert_eq!(
            refresh_token.into_secret(),
            RefreshToken::new("refresh-token".to_string()).into_secret()
        );
    }
}
