//! Authentication Service

pub struct AuthService;

impl AuthService {
    pub fn authenticateUser(&self, username: &str) -> bool {
        !username.is_empty()
    }

    pub fn validate_token(&self, token: &str) -> bool {
        token.starts_with("bearer_")
    }
}
