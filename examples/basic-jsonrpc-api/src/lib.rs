use ras_jsonrpc_macro::jsonrpc_service;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub enum SignInRequest {
    WithCredentials { username: String, password: String },
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub enum SignInResponse {
    Success { jwt: String },
    Failure { msg: String },
}

impl Default for SignInResponse {
    fn default() -> Self {
        Self::Success { jwt: String::new() }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub priority: TaskPriority,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateTaskRequest {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub completed: Option<bool>,
    pub priority: Option<TaskPriority>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct TaskListResponse {
    pub tasks: Vec<Task>,
    pub total: usize,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct UserProfile {
    pub username: String,
    pub email: String,
    pub permissions: Vec<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug)]
pub struct UpdateProfileRequest {
    pub email: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct DashboardStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub pending_tasks: usize,
    pub high_priority_tasks: usize,
}

jsonrpc_service!({
    service_name: MyService,
    openrpc: true,
    explorer: true,
    methods: [
        UNAUTHORIZED sign_in(SignInRequest) -> SignInResponse,
        WITH_PERMISSIONS([]) sign_out(()) -> (),
        WITH_PERMISSIONS(["admin"]) delete_everything(()) -> (),

        // Task management
        WITH_PERMISSIONS([]) list_tasks(()) -> TaskListResponse,
        WITH_PERMISSIONS([]) create_task(CreateTaskRequest) -> Task,
        WITH_PERMISSIONS([]) update_task(UpdateTaskRequest) -> Task,
        WITH_PERMISSIONS([]) delete_task(String) -> bool,
        WITH_PERMISSIONS([]) get_task(String) -> Option<Task>,

        // User profile
        WITH_PERMISSIONS([]) get_profile(()) -> UserProfile,
        WITH_PERMISSIONS([]) update_profile(UpdateProfileRequest) -> UserProfile,

        // Dashboard
        WITH_PERMISSIONS([]) get_dashboard_stats(()) -> DashboardStats,
    ]
});
