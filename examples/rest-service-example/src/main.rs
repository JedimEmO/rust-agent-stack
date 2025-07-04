use anyhow::{Context, Result};
use async_trait::async_trait;
use ras_identity_core::{IdentityProvider, IdentityResult, UserPermissions, VerifiedIdentity};
use ras_identity_local::LocalUserProvider;
use ras_identity_session::{JwtAuthProvider, SessionConfig, SessionService};
use ras_rest_macro::rest_service;
use ras_rest_core::{RestResult, RestResponse, RestError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::info;

// OpenTelemetry imports
use axum::{body::Body, extract::State, http::StatusCode, response::Response};
use opentelemetry::KeyValue;
use opentelemetry::metrics::{Counter, Histogram, Meter};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, TextEncoder};

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
        POST WITH_PERMISSIONS([]) auth/logout() -> (),
        GET WITH_PERMISSIONS([]) auth/me() -> UserInfoResponse,

        // User management endpoints
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> UserResponse,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> UserResponse,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: i32}(UpdateUserRequest) -> UserResponse,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),
    ]
});

// Metrics structure to hold OpenTelemetry instruments
#[derive(Clone)]
struct Metrics {
    rest_requests_started: Counter<u64>,
    rest_requests_completed: Counter<u64>,
    rest_method_duration: Histogram<f64>,
    active_users: Counter<f64>,
}

impl Metrics {
    fn new(meter: &Meter) -> Self {
        Self {
            rest_requests_started: meter
                .u64_counter("rest_requests_started_total")
                .with_description("Total number of REST API requests started")
                .with_unit("requests")
                .build(),
            rest_requests_completed: meter
                .u64_counter("rest_requests_completed_total")
                .with_description("Total number of REST API requests completed")
                .with_unit("requests")
                .build(),
            rest_method_duration: meter
                .f64_histogram("rest_method_duration_seconds")
                .with_description("Duration of REST API method execution in seconds")
                .with_unit("seconds")
                .build(),
            active_users: meter
                .f64_counter("active_users")
                .with_description("Number of currently active users")
                .with_unit("users")
                .build(),
        }
    }
}

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
    metrics: Metrics,
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

    async fn get_users(&self) -> RestResult<UsersResponse> {
        let store = self.store.lock().unwrap();
        let users = store.values().cloned().collect();
        Ok(RestResponse::ok(UsersResponse { users }))
    }

    async fn create_user(
        &self,
        _user: &ras_auth_core::AuthenticatedUser,
        request: CreateUserRequest,
    ) -> RestResult<UserResponse> {
        let mut store = self.store.lock().unwrap();
        let id = store.len() as i32 + 1;
        let user = UserResponse {
            id,
            name: request.name,
            email: request.email,
        };
        store.insert(id, user.clone());
        Ok(RestResponse::created(user))
    }

    async fn get_user(
        &self,
        _user: &ras_auth_core::AuthenticatedUser,
        id: i32,
    ) -> RestResult<UserResponse> {
        let store = self.store.lock().unwrap();
        store
            .get(&id)
            .cloned()
            .map(RestResponse::ok)
            .ok_or_else(|| RestError::not_found("User not found"))
    }

    async fn update_user(
        &self,
        _user: &ras_auth_core::AuthenticatedUser,
        id: i32,
        request: UpdateUserRequest,
    ) -> RestResult<UserResponse> {
        let mut store = self.store.lock().unwrap();
        if let Some(user) = store.get_mut(&id) {
            user.name = request.name;
            user.email = request.email;
            Ok(RestResponse::ok(user.clone()))
        } else {
            Err(RestError::not_found("User not found"))
        }
    }

    async fn delete_user(
        &self,
        _user: &ras_auth_core::AuthenticatedUser,
        id: i32,
    ) -> RestResult<()> {
        let mut store = self.store.lock().unwrap();
        if store.remove(&id).is_some() {
            Ok(RestResponse::no_content())
        } else {
            Err(RestError::not_found("User not found"))
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
    ) -> RestResult<AuthResponse> {
        info!("Registering new user: {}", request.username);

        // Track user registration
        self.app_state
            .metrics
            .active_users
            .add(1.0, &[KeyValue::new("user_type", "regular")]);

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
            .map_err(|e| RestError::conflict(format!("Failed to register user: {}", e)))?;

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
            .map_err(|e| RestError::internal_server_error(format!("Failed to create session: {}", e)))?;

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
            .map_err(|e| RestError::internal_server_error(format!("Failed to get permissions: {}", e)))?;

        Ok(RestResponse::created(AuthResponse {
            token,
            user_info: AuthUserInfo {
                subject: identity.subject,
                email: identity.email,
                display_name: identity.display_name,
                permissions,
            },
        }))
    }

    async fn login_user(
        &self,
        request: LoginRequest,
    ) -> RestResult<AuthResponse> {
        info!("User login attempt: {}", request.username);

        // Track user login
        self.app_state.metrics.active_users.add(
            1.0,
            &[KeyValue::new(
                "user_type",
                if request.username == "admin" {
                    "admin"
                } else {
                    "regular"
                },
            )],
        );

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
            .map_err(|_e| {
                // Return 401 Unauthorized for authentication failures
                RestError::unauthorized("Invalid credentials")
            })?;

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
            .map_err(|e| RestError::internal_server_error(format!("Failed to get permissions: {}", e)))?;

        Ok(RestResponse::ok(AuthResponse {
            token,
            user_info: AuthUserInfo {
                subject: identity.subject,
                email: identity.email,
                display_name: identity.display_name,
                permissions,
            },
        }))
    }

    async fn logout_user(
        &self,
        user: &ras_auth_core::AuthenticatedUser,
    ) -> RestResult<()> {
        info!("User logout: {}", user.user_id);

        // Track user logout
        self.app_state.metrics.active_users.add(
            -1.0,
            &[KeyValue::new(
                "user_type",
                if user.user_id == "admin" {
                    "admin"
                } else {
                    "regular"
                },
            )],
        );

        // Revoke session using the JTI from the JWT metadata
        if let Some(metadata) = &user.metadata {
            if let Some(jti) = metadata.get("jti").and_then(|v| v.as_str()) {
                self.app_state.session_service.end_session(jti).await;
            }
        }

        Ok(RestResponse::no_content())
    }

    async fn get_user_info(
        &self,
        user: &ras_auth_core::AuthenticatedUser,
    ) -> RestResult<UserInfoResponse> {
        Ok(RestResponse::ok(UserInfoResponse {
            user_id: user.user_id.clone(),
            permissions: user.permissions.iter().cloned().collect(),
            metadata: user.metadata.clone(),
        }))
    }
}

// Metrics handler for Prometheus
async fn metrics_handler(
    State(prometheus_registry): State<prometheus::Registry>,
) -> Result<Response<Body>, StatusCode> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus_registry.gather();
    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?)
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenvy::dotenv().ok();

    // Load configuration
    let config = AppConfig::from_env()?;
    info!("Starting REST service with JWT authentication");

    // Initialize OpenTelemetry
    info!("Initializing OpenTelemetry...");

    // Initialize Prometheus registry
    let prometheus_registry = prometheus::Registry::new();

    // Create Prometheus exporter as a reader
    let prometheus_exporter = opentelemetry_prometheus::exporter()
        .with_registry(prometheus_registry.clone())
        .build()
        .expect("Failed to create Prometheus exporter");

    // Build the SdkMeterProvider with the Prometheus exporter as the reader
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(prometheus_exporter)
        .build();

    // Set as global meter provider
    opentelemetry::global::set_meter_provider(meter_provider.clone());

    // Create meter
    let meter = opentelemetry::global::meter("rest-service-example");

    // Create metrics
    let metrics = Metrics::new(&meter);

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
        metrics: metrics.clone(),
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

    let metrics_for_usage = metrics.clone();
    let metrics_for_duration = metrics.clone();

    let app = UserServiceBuilder::new()
        .auth_provider(jwt_auth_provider)
        // Add usage tracker
        .with_usage_tracker(move |headers, user, method, path| {
            let metrics = metrics_for_usage.clone();
            let user_agent = headers
                .get("user-agent")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("Unknown")
                .to_string();

            let user_id = user
                .map(|u| u.user_id.clone())
                .unwrap_or_else(|| "anonymous".to_string());
            let authenticated = user.is_some();
            let permissions = user
                .map(|u| u.permissions.iter().cloned().collect::<Vec<_>>().join(","))
                .unwrap_or_else(|| "none".to_string());
            let method = method.to_string();
            let path = path.to_string();

            async move {
                info!(
                    method = method.as_str(),
                    path = path.as_str(),
                    user_id = user_id.as_str(),
                    user_agent = user_agent.as_str(),
                    "REST API request"
                );

                // Record metrics
                metrics.rest_requests_started.add(
                    1,
                    &[
                        KeyValue::new("method", method.clone()),
                        KeyValue::new("path", path),
                        KeyValue::new("user_id", user_id),
                        KeyValue::new("authenticated", authenticated),
                        KeyValue::new("permissions", permissions),
                        KeyValue::new("user_agent", user_agent),
                    ],
                );
            }
        })
        // Add method duration tracker
        .with_method_duration_tracker(move |method, path, user, duration| {
            let metrics = metrics_for_duration.clone();
            let user_id = user
                .map(|u| u.user_id.clone())
                .unwrap_or_else(|| "anonymous".to_string());
            let authenticated = user.is_some();
            let method = method.to_string();
            let path = path.to_string();
            let duration_ms = duration.as_millis() as u64;

            async move {
                info!(
                    method = method.as_str(),
                    path = path.as_str(),
                    user_id = user_id.as_str(),
                    duration_ms = duration_ms,
                    "REST API request completed"
                );

                // Record method duration in seconds
                metrics.rest_method_duration.record(
                    duration.as_secs_f64(),
                    &[
                        KeyValue::new("method", method.clone()),
                        KeyValue::new("path", path.clone()),
                        KeyValue::new("user_id", user_id.clone()),
                        KeyValue::new("authenticated", authenticated),
                    ],
                );

                // Record completion
                metrics.rest_requests_completed.add(
                    1,
                    &[
                        KeyValue::new("method", method),
                        KeyValue::new("path", path),
                        KeyValue::new("user_id", user_id),
                    ],
                );
            }
        })
        // Authentication handlers
        .post_auth_register_handler(move |request| {
            let handlers = auth_handlers1.clone();
            async move { handlers.register_user(request).await }
        })
        .post_auth_login_handler(move |request| {
            let handlers = auth_handlers2.clone();
            async move { handlers.login_user(request).await }
        })
        .post_auth_logout_handler(move |user| {
            let handlers = auth_handlers3.clone();
            let user = user.clone();
            async move { handlers.logout_user(&user).await }
        })
        .get_auth_me_handler(move |user| {
            let handlers = auth_handlers4.clone();
            let user = user.clone();
            async move { handlers.get_user_info(&user).await }
        })
        // User management handlers
        .get_users_handler(move || {
            let handlers = user_handlers1.clone();
            async move { handlers.get_users().await }
        })
        .post_users_handler(move |user, request| {
            let handlers = user_handlers2.clone();
            let user = user.clone();
            async move { handlers.create_user(&user, request).await }
        })
        .get_users_by_id_handler(move |user, id| {
            let handlers = user_handlers3.clone();
            let user = user.clone();
            async move { handlers.get_user(&user, id).await }
        })
        .put_users_by_id_handler(move |user, id, request| {
            let handlers = user_handlers4.clone();
            let user = user.clone();
            async move { handlers.update_user(&user, id, request).await }
        })
        .delete_users_by_id_handler(move |user, id| {
            let handlers = user_handlers5.clone();
            let user = user.clone();
            async move { handlers.delete_user(&user, id).await }
        })
        .build();

    // Add metrics endpoint
    let metrics_router = axum::Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .with_state(prometheus_registry);

    let app = axum::Router::new().merge(app).merge(metrics_router);

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
    println!("📊 Metrics:");
    println!("  GET    /metrics               - Prometheus metrics endpoint");
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
