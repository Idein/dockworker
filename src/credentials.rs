///! Access credentials for accessing any docker daemon endpoints
///!
///! Currently, any values of these types are only used for `/images/{name}/push` api.
use crate::system::AuthToken;
use serde::{Deserialize, Serialize};

/// Access credential
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Credential {
    /// identity token issued by docker registry
    Token(IdentityToken),
    /// user password
    Password(UserPassword),
}

impl Credential {
    pub fn with_token(token: IdentityToken) -> Self {
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
pub struct IdentityToken {
    identitytoken: String,
}

impl IdentityToken {
    #[allow(dead_code)]
    pub fn token(&self) -> String {
        self.identitytoken.clone()
    }
    #[allow(dead_code)]
    pub fn from_auth_token(auth_token: &AuthToken) -> Self {
        Self {
            identitytoken: auth_token.token(),
        }
    }

    pub fn from_bare_token(token: String) -> Self {
        Self {
            identitytoken: token,
        }
    }
}
