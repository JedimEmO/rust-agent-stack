mod simple_auth;

use anyhow::Result;
use std::collections::HashMap;
use std::future::IntoFuture;
use std::sync::{Arc, Mutex};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use ras_auth_core::AuthenticatedUser;
use ras_rest_core::{RestError, RestResponse, RestResult};

use rest_api::*;
use simple_auth::SimpleAuthProvider;

// Simple in-memory storage
#[derive(Clone)]
struct AppState {
    users: Arc<Mutex<HashMap<String, User>>>,
    tasks: Arc<Mutex<HashMap<String, Vec<Task>>>>,
}

struct UserServiceImpl {
    state: AppState,
}

#[async_trait::async_trait]
impl UserServiceTrait for UserServiceImpl {
    async fn get_users(&self) -> RestResult<UsersResponse> {
        let users = self.state.users.lock().unwrap();
        let users_vec: Vec<User> = users.values().cloned().collect();
        let total = users_vec.len();

        Ok(RestResponse::ok(UsersResponse {
            users: users_vec,
            total,
        }))
    }

    async fn get_users_by_id(&self, id: String) -> RestResult<User> {
        let users = self.state.users.lock().unwrap();

        users
            .get(&id)
            .cloned()
            .map(|user| RestResponse::ok(user))
            .ok_or_else(|| RestError::not_found("User not found"))
    }

    async fn post_users(
        &self,
        _user: &AuthenticatedUser,
        request: CreateUserRequest,
    ) -> RestResult<User> {
        let mut users = self.state.users.lock().unwrap();

        let user = User {
            id: Uuid::new_v4().to_string(),
            name: request.name,
            email: request.email,
            role: "user".to_string(),
        };

        users.insert(user.id.clone(), user.clone());

        Ok(RestResponse::created(user))
    }

    async fn put_users_by_id(
        &self,
        _user: &AuthenticatedUser,
        id: String,
        request: UpdateUserRequest,
    ) -> RestResult<User> {
        let mut users = self.state.users.lock().unwrap();

        let user = users
            .get_mut(&id)
            .ok_or_else(|| RestError::not_found("User not found"))?;

        if let Some(name) = request.name {
            user.name = name;
        }
        if let Some(email) = request.email {
            user.email = email;
        }

        Ok(RestResponse::ok(user.clone()))
    }

    async fn delete_users_by_id(&self, _user: &AuthenticatedUser, id: String) -> RestResult<()> {
        let mut users = self.state.users.lock().unwrap();

        users
            .remove(&id)
            .map(|_| RestResponse::ok(()))
            .ok_or_else(|| RestError::not_found("User not found"))
    }

    async fn get_users_by_user_id_tasks(
        &self,
        _user: &AuthenticatedUser,
        user_id: String,
    ) -> RestResult<TasksResponse> {
        let tasks = self.state.tasks.lock().unwrap();

        let user_tasks = tasks.get(&user_id).cloned().unwrap_or_default();

        Ok(RestResponse::ok(TasksResponse { tasks: user_tasks }))
    }

    async fn post_users_by_user_id_tasks(
        &self,
        _user: &AuthenticatedUser,
        user_id: String,
        request: CreateTaskRequest,
    ) -> RestResult<Task> {
        let mut tasks = self.state.tasks.lock().unwrap();

        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: request.title,
            description: request.description,
            completed: false,
            user_id: user_id.clone(),
        };

        tasks
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(task.clone());

        Ok(RestResponse::created(task))
    }

    async fn put_users_by_user_id_tasks_by_task_id(
        &self,
        _user: &AuthenticatedUser,
        user_id: String,
        task_id: String,
        request: UpdateTaskRequest,
    ) -> RestResult<Task> {
        let mut tasks = self.state.tasks.lock().unwrap();

        let user_tasks = tasks
            .get_mut(&user_id)
            .ok_or_else(|| RestError::not_found("User tasks not found"))?;

        let task = user_tasks
            .iter_mut()
            .find(|t| t.id == task_id)
            .ok_or_else(|| RestError::not_found("Task not found"))?;

        if let Some(title) = request.title {
            task.title = title;
        }
        if let Some(description) = request.description {
            task.description = description;
        }
        if let Some(completed) = request.completed {
            task.completed = completed;
        }

        Ok(RestResponse::ok(task.clone()))
    }

    async fn delete_users_by_user_id_tasks_by_task_id(
        &self,
        _user: &AuthenticatedUser,
        user_id: String,
        task_id: String,
    ) -> RestResult<()> {
        let mut tasks = self.state.tasks.lock().unwrap();

        let user_tasks = tasks
            .get_mut(&user_id)
            .ok_or_else(|| RestError::not_found("User tasks not found"))?;

        let pos = user_tasks
            .iter()
            .position(|t| t.id == task_id)
            .ok_or_else(|| RestError::not_found("Task not found"))?;

        user_tasks.remove(pos);

        Ok(RestResponse::ok(()))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rest_backend=debug,rest_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize state
    let state = AppState {
        users: Arc::new(Mutex::new(HashMap::new())),
        tasks: Arc::new(Mutex::new(HashMap::new())),
    };

    // Add demo users
    {
        let mut users = state.users.lock().unwrap();
        users.insert(
            "1".to_string(),
            User {
                id: "1".to_string(),
                name: "John Doe".to_string(),
                email: "john@example.com".to_string(),
                role: "user".to_string(),
            },
        );
        users.insert(
            "2".to_string(),
            User {
                id: "2".to_string(),
                name: "Admin User".to_string(),
                email: "admin@example.com".to_string(),
                role: "admin".to_string(),
            },
        );
    }

    // Setup simple authentication
    let auth_provider = SimpleAuthProvider;

    // Create service
    let service = UserServiceImpl { state };

    // Build router
    let api_router = UserServiceBuilder::new(service)
        .auth_provider(auth_provider)
        .build();

    // Setup CORS for WASM client
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = api_router.layer(cors);

    let addr = "127.0.0.1:3000";
    tracing::info!("Server running at http://{}", addr);
    tracing::info!("API docs at http://{}/api/v1/docs", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    let server = axum::serve(listener, app.into_make_service());
    server.into_future().await?;

    Ok(())
}
