use anyhow::{Context, Result};
use async_trait::async_trait;
use rust_identity_core::{IdentityProvider, IdentityResult, UserPermissions, VerifiedIdentity};
use rust_identity_local::LocalUserProvider;
use rust_identity_session::{JwtAuthProvider, SessionConfig, SessionService};
use rust_rest_macro::rest_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

// Custom provider that implements IdentityProvider and can be shared
#[derive(Clone)]
struct SharedUserProvider {
    inner: Arc<LocalUserProvider>,
}

impl SharedUserProvider {
    fn new() -> Self {
        Self {
            inner: Arc::new(LocalUserProvider::new()),
        }
    }

    async fn add_user(
        &self,
        username: String,
        password: String,
        email: Option<String>,
        display_name: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner
            .add_user(username, password, email, display_name)
            .await
            .map_err(|e| e.into())
    }
}

// Implement IdentityProvider for SharedUserProvider so it can be registered with SessionService
#[async_trait]
impl IdentityProvider for SharedUserProvider {
    fn provider_id(&self) -> &str {
        self.inner.provider_id()
    }

    async fn verify(&self, auth_payload: serde_json::Value) -> IdentityResult<VerifiedIdentity> {
        self.inner.verify(auth_payload).await
    }
}

// Example data types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct CreateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UpdateUserRequest {
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UserResponse {
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UsersResponse {
    users: Vec<UserResponse>,
}

// Authentication data types
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct RegisterUserRequest {
    username: String,
    password: String,
    email: Option<String>,
    display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct AuthResponse {
    token: String,
    user_info: AuthUserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct AuthUserInfo {
    subject: String,
    email: Option<String>,
    display_name: Option<String>,
    permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct UserInfoResponse {
    user_id: String,
    permissions: Vec<String>,
    metadata: Option<serde_json::Value>,
}

// Generate the REST service
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    docs_path: "/docs",
    ui_theme: "default",
    endpoints: [
        // Authentication endpoints
        POST UNAUTHORIZED auth/register(RegisterUserRequest) -> AuthResponse,
        POST UNAUTHORIZED auth/login(LoginRequest) -> AuthResponse,
        POST WITH_PERMISSIONS([]) auth/logout(()) -> (),
        GET WITH_PERMISSIONS([]) auth/me(()) -> UserInfoResponse,

        // User management endpoints
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> UserResponse,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> UserResponse,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: i32}(UpdateUserRequest) -> UserResponse,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),
    ]
});

// Application configuration
#[derive(Debug, Clone)]
struct AppConfig {
    pub jwt_secret: String,
    pub server_host: String,
    pub server_port: u16,
}

impl AppConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-me-in-production-please".to_string()),
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: std::env::var("SERVER_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("SERVER_PORT must be a valid port number")?,
        })
    }
}

// Simple permissions implementation for this example
#[derive(Clone)]
struct ExamplePermissions;

#[async_trait]
impl UserPermissions for ExamplePermissions {
    async fn get_permissions(&self, identity: &VerifiedIdentity) -> IdentityResult<Vec<String>> {
        // For this example, give admin permissions to 'admin' users and user permissions to others
        if identity.subject == "admin" {
            Ok(vec!["admin".to_string(), "user".to_string()])
        } else {
            Ok(vec!["user".to_string()])
        }
    }
}

// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    session_service: Arc<SessionService>,
    shared_provider: SharedUserProvider,
}

// Example in-memory storage
type UserStore = Arc<Mutex<HashMap<i32, UserResponse>>>;

// Example handlers
#[derive(Clone)]
struct UserHandlers {
    store: UserStore,
}

impl UserHandlers {
    fn new() -> Self {
        let mut initial_users = HashMap::new();
        initial_users.insert(
            1,
            UserResponse {
                id: 1,
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
            },
        );
        initial_users.insert(
            2,
            UserResponse {
                id: 2,
                name: "Jane Smith".to_string(),
                email: "jane@example.com".to_string(),
            },
        );

        Self {
            store: Arc::new(Mutex::new(initial_users)),
        }
    }

    async fn get_users(&self) -> Result<UsersResponse, Box<dyn std::error::Error + Send + Sync>> {
        let store = self.store.lock().unwrap();
        let users = store.values().cloned().collect();
        Ok(UsersResponse { users })
    }

    async fn create_user(
        &self,
        _user: rust_auth_core::AuthenticatedUser,
        request: CreateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut store = self.store.lock().unwrap();
        let id = store.len() as i32 + 1;
        let user = UserResponse {
            id,
            name: request.name,
            email: request.email,
        };
        store.insert(id, user.clone());
        Ok(user)
    }

    async fn get_user(
        &self,
        _user: &rust_auth_core::AuthenticatedUser,
        id: i32,
    ) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>> {
        let store = self.store.lock().unwrap();
        store
            .get(&id)
            .cloned()
            .ok_or_else(|| "User not found".into())
    }

    async fn update_user(
        &self,
        _user: rust_auth_core::AuthenticatedUser,
        id: i32,
        request: UpdateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error + Send + Sync>> {
        let mut store = self.store.lock().unwrap();
        if let Some(user) = store.get_mut(&id) {
            user.name = request.name;
            user.email = request.email;
            Ok(user.clone())
        } else {
            Err("User not found".into())
        }
    }

    async fn delete_user(
        &self,
        _user: rust_auth_core::AuthenticatedUser,
        id: i32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut store = self.store.lock().unwrap();
        if store.remove(&id).is_some() {
            Ok(())
        } else {
            Err("User not found".into())
        }
    }
}

// Authentication handlers
#[derive(Clone)]
struct AuthHandlers {
    app_state: AppState,
}

impl AuthHandlers {
    fn new(app_state: AppState) -> Self {
        Self { app_state }
    }

    async fn register_user(
        &self,
        request: RegisterUserRequest,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("Registering new user: {}", request.username);

        // Add user to the shared provider
        self.app_state
            .shared_provider
            .add_user(
                request.username.clone(),
                request.password.clone(),
                request.email.clone(),
                request.display_name.clone(),
            )
            .await
            .map_err(|e| format!("Failed to register user: {}", e))?;

        // Since we have the same issue with two separate provider instances,
        // we need to manually keep them in sync. This is not ideal but fixes the immediate bug.
        // TODO: Refactor to use a single shared provider instance.

        // Create auth payload for automatic login after registration
        let auth_payload = serde_json::json!({
            "username": request.username,
            "password": request.password
        });

        // Begin session using the session service
        let token = self
            .app_state
            .session_service
            .begin_session("local", auth_payload)
            .await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        // Create identity for permissions lookup
        let identity = VerifiedIdentity {
            provider_id: "local".to_string(),
            subject: request.username,
            email: request.email,
            display_name: request.display_name,
            metadata: None,
        };

        let permissions = ExamplePermissions
            .get_permissions(&identity)
            .await
            .map_err(|e| format!("Failed to get permissions: {}", e))?;

        Ok(AuthResponse {
            token,
            user_info: AuthUserInfo {
                subject: identity.subject,
                email: identity.email,
                display_name: identity.display_name,
                permissions,
            },
        })
    }

    async fn login_user(
        &self,
        request: LoginRequest,
    ) -> Result<AuthResponse, Box<dyn std::error::Error + Send + Sync>> {
        info!("User login attempt: {}", request.username);

        // Create auth payload
        let auth_payload = serde_json::json!({
            "username": request.username,
            "password": request.password
        });

        // Begin session using the session service (this will verify credentials internally)
        let token = self
            .app_state
            .session_service
            .begin_session("local", auth_payload)
            .await
            .map_err(|e| format!("Authentication failed: {}", e))?;

        // Create identity for permissions lookup
        let identity = VerifiedIdentity {
            provider_id: "local".to_string(),
            subject: request.username,
            email: None, // We could look this up from the user provider if needed
            display_name: None,
            metadata: None,
        };

        let permissions = ExamplePermissions
            .get_permissions(&identity)
            .await
            .map_err(|e| format!("Failed to get permissions: {}", e))?;

        Ok(AuthResponse {
            token,
            user_info: AuthUserInfo {
                subject: identity.subject,
                email: identity.email,
                display_name: identity.display_name,
                permissions,
            },
        })
    }

    async fn logout_user(
        &self,
        user: rust_auth_core::AuthenticatedUser,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("User logout: {}", user.user_id);

        // Revoke session using the JTI from the JWT metadata
        if let Some(metadata) = &user.metadata {
            if let Some(jti) = metadata.get("jti").and_then(|v| v.as_str()) {
                self.app_state.session_service.end_session(jti).await;
            }
        }

        Ok(())
    }

    async fn get_user_info(
        &self,
        user: &rust_auth_core::AuthenticatedUser,
    ) -> Result<UserInfoResponse, Box<dyn std::error::Error + Send + Sync>> {
        Ok(UserInfoResponse {
            user_id: user.user_id.clone(),
            permissions: user.permissions.iter().cloned().collect(),
            metadata: user.metadata.clone(),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Starting REST service with JWT authentication");

    // Initialize authentication components
    let shared_provider = SharedUserProvider::new();

    // Add test users to the shared provider
    info!("Adding test users");
    shared_provider
        .add_user(
            "admin".to_string(),
            "admin123".to_string(),
            Some("admin@example.com".to_string()),
            Some("Administrator".to_string()),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add admin user: {}", e))?;

    shared_provider
        .add_user(
            "user".to_string(),
            "user123".to_string(),
            Some("user@example.com".to_string()),
            Some("Regular User".to_string()),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add regular user: {}", e))?;

    // Create session configuration
    let session_config = SessionConfig {
        jwt_secret: config.jwt_secret.clone(),
        jwt_ttl: chrono::Duration::hours(24),
        refresh_enabled: true,
        algorithm: jsonwebtoken::Algorithm::HS256,
    };

    // Create session service with permissions provider
    let session_service = Arc::new(
        SessionService::new(session_config)
            .with_permissions(Arc::new(ExamplePermissions) as Arc<dyn UserPermissions>),
    );

    // KEY FIX: Use the shared provider directly for the session service
    // Since SharedUserProvider implements IdentityProvider and is cloneable,
    // we can register a clone that shares the same underlying storage
    session_service
        .register_provider(Box::new(shared_provider.clone()) as Box<dyn IdentityProvider>)
        .await;

    // Create application state
    let app_state = AppState {
        session_service: session_service.clone(),
        shared_provider: shared_provider.clone(),
    };

    // Create handlers
    let user_handlers = UserHandlers::new();
    let auth_handlers = AuthHandlers::new(app_state);

    // Create JWT auth provider for the service
    let jwt_auth_provider = JwtAuthProvider::new(session_service);

    // Build the service router with authentication handlers
    let user_handlers1 = user_handlers.clone();
    let user_handlers2 = user_handlers.clone();
    let user_handlers3 = user_handlers.clone();
    let user_handlers4 = user_handlers.clone();
    let user_handlers5 = user_handlers.clone();

    let auth_handlers1 = auth_handlers.clone();
    let auth_handlers2 = auth_handlers.clone();
    let auth_handlers3 = auth_handlers.clone();
    let auth_handlers4 = auth_handlers.clone();

    let app = UserServiceBuilder::new()
        .auth_provider(jwt_auth_provider)
        // Authentication handlers
        .post_auth_register_handler(move |request| {
            let handlers = auth_handlers1.clone();
            async move { handlers.register_user(request).await }
        })
        .post_auth_login_handler(move |request| {
            let handlers = auth_handlers2.clone();
            async move { handlers.login_user(request).await }
        })
        .post_auth_logout_handler(move |user, _| {
            let handlers = auth_handlers3.clone();
            async move { handlers.logout_user(user).await }
        })
        .get_auth_me_handler(move |user, _| {
            let handlers = auth_handlers4.clone();
            async move { handlers.get_user_info(&user).await }
        })
        // User management handlers
        .get_users_handler(move || {
            let handlers = user_handlers1.clone();
            async move { handlers.get_users().await }
        })
        .post_users_handler(move |user, request| {
            let handlers = user_handlers2.clone();
            async move { handlers.create_user(user, request).await }
        })
        .get_users_by_id_handler(move |user, id| {
            let handlers = user_handlers3.clone();
            async move { handlers.get_user(&user, id).await }
        })
        .put_users_by_id_handler(move |user, id, request| {
            let handlers = user_handlers4.clone();
            async move { handlers.update_user(user, id, request).await }
        })
        .delete_users_by_id_handler(move |user, id| {
            let handlers = user_handlers5.clone();
            async move { handlers.delete_user(user, id).await }
        })
        .build();

    // Generate OpenAPI documentation
    if let Err(e) = generate_userservice_openapi_to_file() {
        eprintln!("Failed to generate OpenAPI documentation: {}", e);
    } else {
        println!("OpenAPI documentation generated successfully!");
    }

    // Start the server
    let bind_addr = format!("{}:{}", config.server_host, config.server_port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .context(format!("Failed to bind to {}", bind_addr))?;

    println!("REST service running on http://{}", bind_addr);
    println!();
    println!("📖 API Documentation:");
    println!("  GET    /api/v1/docs           - Interactive API documentation (Swagger UI)");
    println!("  GET    /api/v1/docs/openapi.json - OpenAPI 3.0 specification");
    println!();
    println!("🔗 Available endpoints:");
    println!("  POST   /api/v1/auth/register  - Register new user");
    println!("  POST   /api/v1/auth/login     - Login user");
    println!("  POST   /api/v1/auth/logout    - Logout user (requires auth)");
    println!("  GET    /api/v1/auth/me        - Get user info (requires auth)");
    println!("  GET    /api/v1/users          - List all users (no auth required)");
    println!("  POST   /api/v1/users          - Create user (requires admin permission)");
    println!("  GET    /api/v1/users/:id      - Get user by ID (requires user permission)");
    println!("  PUT    /api/v1/users/:id      - Update user (requires admin permission)");
    println!("  DELETE /api/v1/users/:id      - Delete user (requires admin permission)");
    println!();
    println!("👤 Test users:");
    println!("  Username: admin, Password: admin123 (has admin permissions)");
    println!("  Username: user,  Password: user123  (has user permissions)");

    axum::serve(listener, app)
        .await
        .context("Failed to start server")?;

    Ok(())
}
