use std::collections::{HashMap, HashSet};

use ras_auth_core::{AuthError, AuthFuture, AuthProvider, AuthenticatedUser};

/// A small fixed-token auth provider for tests.
///
/// The default token table:
/// - `"user-token"`     → user `user-1`,  perms `["user"]`
/// - `"admin-token"`    → user `admin-1`, perms `["admin", "user"]`
/// - `"readonly-token"` → user `ro-1`,    perms `["read"]`
///
/// Any other (or empty) token returns [`AuthError::InvalidToken`].
#[derive(Clone, Debug)]
pub struct MockAuthProvider {
    table: HashMap<String, AuthenticatedUser>,
}

impl Default for MockAuthProvider {
    fn default() -> Self {
        let mut table = HashMap::new();
        table.insert("user-token".to_string(), mock_user("user-1", &["user"]));
        table.insert(
            "admin-token".to_string(),
            mock_user("admin-1", &["admin", "user"]),
        );
        table.insert("readonly-token".to_string(), mock_user("ro-1", &["read"]));
        Self { table }
    }
}

impl MockAuthProvider {
    /// New empty auth provider with no recognized tokens.
    pub fn empty() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    /// Insert or replace a token → user mapping. Useful for adding bespoke
    /// fixtures on top of the default table.
    pub fn with_token(mut self, token: impl Into<String>, user: AuthenticatedUser) -> Self {
        self.table.insert(token.into(), user);
        self
    }
}

impl AuthProvider for MockAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        let result = self
            .table
            .get(&token)
            .cloned()
            .ok_or(AuthError::InvalidToken);
        Box::pin(async move { result })
    }
}

/// Build an [`AuthenticatedUser`] from a string id and a slice of permission
/// names. Convenience for tests that need to construct a user by hand.
pub fn mock_user(user_id: &str, perms: &[&str]) -> AuthenticatedUser {
    AuthenticatedUser {
        user_id: user_id.to_string(),
        permissions: perms
            .iter()
            .map(|p| (*p).to_string())
            .collect::<HashSet<_>>(),
        metadata: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_user_builds_expected_fields() {
        let u = mock_user("alice", &["a", "b"]);
        assert_eq!(u.user_id, "alice");
        assert!(u.permissions.contains("a"));
        assert!(u.permissions.contains("b"));
        assert!(u.metadata.is_none());
    }

    #[tokio::test]
    async fn default_provider_resolves_well_known_tokens() {
        let p = MockAuthProvider::default();
        let user = p.authenticate("user-token".to_string()).await.unwrap();
        assert_eq!(user.user_id, "user-1");
        assert!(user.permissions.contains("user"));

        let admin = p.authenticate("admin-token".to_string()).await.unwrap();
        assert!(admin.permissions.contains("admin"));
        assert!(admin.permissions.contains("user"));

        let ro = p.authenticate("readonly-token".to_string()).await.unwrap();
        assert!(ro.permissions.contains("read"));

        let err = p
            .authenticate("totally-bogus".to_string())
            .await
            .unwrap_err();
        assert!(matches!(err, ras_auth_core::AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn empty_provider_rejects_everything() {
        let p = MockAuthProvider::empty();
        let err = p.authenticate("user-token".to_string()).await.unwrap_err();
        assert!(matches!(err, ras_auth_core::AuthError::InvalidToken));
    }

    #[tokio::test]
    async fn with_token_extends_table() {
        let p = MockAuthProvider::empty().with_token("custom", mock_user("zed", &["god"]));
        let user = p.authenticate("custom".to_string()).await.unwrap();
        assert_eq!(user.user_id, "zed");
        assert!(user.permissions.contains("god"));
    }

    #[test]
    fn check_permissions_returns_specific_error() {
        let p = MockAuthProvider::default();
        let user = mock_user("u", &["read"]);
        // Has the permission → ok.
        p.check_permissions(&user, &["read".into()]).unwrap();
        // Missing → InsufficientPermissions.
        let err = p.check_permissions(&user, &["admin".into()]).unwrap_err();
        assert!(matches!(
            err,
            ras_auth_core::AuthError::InsufficientPermissions { .. }
        ));
    }
}
