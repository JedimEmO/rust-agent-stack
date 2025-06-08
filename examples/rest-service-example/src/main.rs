use rust_rest_macro::rest_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

// Generate the REST service
rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    endpoints: [
        GET UNAUTHORIZED users() -> UsersResponse,
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> UserResponse,
        GET WITH_PERMISSIONS(["user"]) users/{id: i32}() -> UserResponse,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: i32}(UpdateUserRequest) -> UserResponse,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: i32}() -> (),
    ]
});

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
        _user: rust_jsonrpc_core::AuthenticatedUser,
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
        _user: &rust_jsonrpc_core::AuthenticatedUser,
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
        _user: rust_jsonrpc_core::AuthenticatedUser,
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
        _user: rust_jsonrpc_core::AuthenticatedUser,
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let handlers = UserHandlers::new();

    // Build the service router with separate handler instances for each closure
    let handlers1 = handlers.clone();
    let handlers2 = handlers.clone();
    let handlers3 = handlers.clone();
    let handlers4 = handlers.clone();
    let handlers5 = handlers.clone();

    let app = UserServiceBuilder::new()
        .get_users_handler(move || {
            let handlers = handlers1.clone();
            async move { handlers.get_users().await }
        })
        .post_users_handler(move |user, request| {
            let handlers = handlers2.clone();
            async move { handlers.create_user(user, request).await }
        })
        .get_users_by_id_handler(move |user, id| {
            let handlers = handlers3.clone();
            async move { handlers.get_user(&user, id).await }
        })
        .put_users_by_id_handler(move |user, id, request| {
            let handlers = handlers4.clone();
            async move { handlers.update_user(user, id, request).await }
        })
        .delete_users_by_id_handler(move |user, id| {
            let handlers = handlers5.clone();
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
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("REST service running on http://0.0.0.0:3000");
    println!("Available endpoints:");
    println!("  GET    /api/v1/users          - List all users (no auth required)");
    println!("  POST   /api/v1/users          - Create user (requires admin permission)");
    println!("  GET    /api/v1/users/:id      - Get user by ID (requires user permission)");
    println!("  PUT    /api/v1/users/:id      - Update user (requires admin permission)");
    println!("  DELETE /api/v1/users/:id      - Delete user (requires admin permission)");

    axum::serve(listener, app).await.unwrap();
}
