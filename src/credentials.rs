/// credential from for accessing the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
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
