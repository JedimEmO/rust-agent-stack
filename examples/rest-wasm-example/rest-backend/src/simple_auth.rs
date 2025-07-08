use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};
use std::collections::HashSet;

/// Simple mock auth provider that accepts "validtoken" for user and "admintoken" for admin
#[derive(Clone)]
pub struct SimpleAuthProvider;

impl AuthProvider for SimpleAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            match token.as_str() {
                "validtoken" => {
                    let mut permissions = HashSet::new();
                    permissions.insert("user".to_string());

                    Ok(AuthenticatedUser {
                        user_id: "testuser".to_string(),
                        permissions,
                        metadata: None,
                    })
                }
                "admintoken" => {
                    let mut permissions = HashSet::new();
                    permissions.insert("admin".to_string());
                    permissions.insert("user".to_string());

                    Ok(AuthenticatedUser {
                        user_id: "admin".to_string(),
                        permissions,
                        metadata: None,
                    })
                }
                _ => Err(AuthError::InvalidToken),
            }
        })
    }
}
