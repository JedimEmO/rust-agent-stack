use ras_rest_macro::rest_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UsersResponse {
    pub users: Vec<User>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TasksResponse {
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub user_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
}

rest_service!({
    service_name: UserService,
    base_path: "/api/v1",
    openapi: true,
    serve_docs: true,
    docs_path: "/docs",
    endpoints: [
        // Public endpoints
        GET UNAUTHORIZED users() -> UsersResponse,
        GET UNAUTHORIZED users/{id: String}() -> User,

        // Admin endpoints
        POST WITH_PERMISSIONS(["admin"]) users(CreateUserRequest) -> User,
        PUT WITH_PERMISSIONS(["admin"]) users/{id: String}(UpdateUserRequest) -> User,
        DELETE WITH_PERMISSIONS(["admin"]) users/{id: String}() -> (),

        // User endpoints for tasks
        GET WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks() -> TasksResponse,
        POST WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks(CreateTaskRequest) -> Task,
        PUT WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks/{task_id: String}(UpdateTaskRequest) -> Task,
        DELETE WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks/{task_id: String}() -> (),

        // New endpoints with query parameters for pagination and search
        GET UNAUTHORIZED search/users ? q: String & limit: Option<u32> & offset: Option<u32> () -> UsersResponse,
        GET WITH_PERMISSIONS(["user"]) users/{user_id: String}/tasks/search ? completed: Option<bool> & page: Option<u32> & per_page: Option<u32> () -> TasksResponse,
    ]
});

// Note: WASM client generation has been removed from ras-rest-macro
// You'll need to implement your own WASM bindings if needed
