///! Access credentials for accessing any docker daemon endpoints
///!
///! Currently, any values of these types are only used for `/images/{name}/push` api.

use serde_json;
use header::XRegistryAuth;

/// Access credential
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum Credential {
    /// identity token issued by docker registry
    Token(AuthToken),
    /// user password
    Password(UserPassword),
}

impl Credential {
    pub fn with_token(token: AuthToken) -> Self {
        Credential::Token(token)
    }

    pub fn with_password(password: UserPassword) -> Self {
        Credential::Password(password)
    }
}

/// User informations for accessing apis
///
/// At least, this value is required for accessing `/images/{name}/push` api.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UserPassword {
    username: String,
    password: String,
    email: String,
    serveraddress: String,
}

impl UserPassword {
    pub fn new(username: String, password: String, email: String, serveraddress: String) -> Self {
        Self {
            username,
            password,
            email,
            serveraddress,
        }
    }
}

/// Access token for accessing apis
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct AuthToken {
    Status: String,
    IdentityToken: String,
}

impl AuthToken {
    #[allow(dead_code)]
    fn token(&self) -> String {
        self.IdentityToken.clone()
    }
}

impl From<Credential> for XRegistryAuth {
    fn from(credential: Credential) -> Self {
        XRegistryAuth::new(serde_json::to_string(&credential).unwrap())
    }
}
