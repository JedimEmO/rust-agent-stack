use axum::Router;
use basic_jsonrpc_api::{
    CreateTaskRequest, DashboardStats, MyServiceBuilder, SignInRequest, SignInResponse, Task,
    TaskListResponse, TaskPriority, UpdateTaskRequest, UserProfile,
};
use chrono::Utc;
use ras_jsonrpc_core::{AuthFuture, AuthProvider, AuthenticatedUser};
use ras_observability_core::{MethodDurationTracker, RequestContext, UsageTracker};
use ras_observability_otel::OtelSetupBuilder;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tracing::info;
use uuid::Uuid;

// Example auth provider
pub struct MyAuthProvider;

impl AuthProvider for MyAuthProvider {
    fn authenticate(&self, token: String) -> AuthFuture<'_> {
        Box::pin(async move {
            // Simple example - in real implementation, validate JWT
            if token == "valid_token" {
                let mut permissions = HashSet::new();
                permissions.insert("user".to_string());

                Ok(AuthenticatedUser {
                    user_id: "user123".to_string(),
                    permissions,
                    metadata: None,
                })
            } else if token == "admin_token" {
                let mut permissions = HashSet::new();
                permissions.insert("user".to_string());
                permissions.insert("admin".to_string());

                Ok(AuthenticatedUser {
                    user_id: "admin123".to_string(),
                    permissions,
                    metadata: None,
                })
            } else {
                Err(ras_jsonrpc_core::AuthError::InvalidToken)
            }
        })
    }
}

// Simple in-memory task storage
#[derive(Clone)]
struct TaskStorage {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
}

impl TaskStorage {
    fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn create_task(&self, req: CreateTaskRequest) -> Task {
        let task = Task {
            id: Uuid::new_v4().to_string(),
            title: req.title,
            description: req.description,
            completed: false,
            priority: req.priority,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        self.tasks
            .lock()
            .unwrap()
            .insert(task.id.clone(), task.clone());
        task
    }

    fn update_task(&self, req: UpdateTaskRequest) -> Option<Task> {
        let mut tasks = self.tasks.lock().unwrap();

        tasks.get_mut(&req.id).map(|task| {
            if let Some(title) = req.title {
                task.title = title;
            }
            if let Some(description) = req.description {
                task.description = description;
            }
            if let Some(completed) = req.completed {
                task.completed = completed;
            }
            if let Some(priority) = req.priority {
                task.priority = priority;
            }
            task.updated_at = Utc::now().to_rfc3339();
            task.clone()
        })
    }

    fn delete_task(&self, id: String) -> bool {
        self.tasks.lock().unwrap().remove(&id).is_some()
    }

    fn get_task(&self, id: String) -> Option<Task> {
        self.tasks.lock().unwrap().get(&id).cloned()
    }

    fn list_tasks(&self) -> TaskListResponse {
        let tasks = self.tasks.lock().unwrap();
        let task_vec: Vec<Task> = tasks.values().cloned().collect();
        let total = task_vec.len();

        TaskListResponse {
            tasks: task_vec,
            total,
        }
    }

    fn get_stats(&self) -> DashboardStats {
        let tasks = self.tasks.lock().unwrap();
        let total_tasks = tasks.len();
        let completed_tasks = tasks.values().filter(|t| t.completed).count();
        let pending_tasks = total_tasks - completed_tasks;
        let high_priority_tasks = tasks
            .values()
            .filter(|t| matches!(t.priority, TaskPriority::High))
            .count();

        DashboardStats {
            total_tasks,
            completed_tasks,
            pending_tasks,
            high_priority_tasks,
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize observability with the new crates
    info!("Initializing OpenTelemetry with unified observability...");
    let otel = OtelSetupBuilder::new("basic-jsonrpc-service")
        .build()
        .expect("Failed to set up OpenTelemetry");

    // Note about OTLP: For OTLP export, you would typically run this service
    // alongside an OpenTelemetry Collector that scrapes the /metrics endpoint
    // and forwards to your OTLP backend
    let otlp_note = std::env::var("OTLP_ENDPOINT")
        .map(|endpoint| format!("Configure your OpenTelemetry Collector to scrape metrics from http://localhost:3000/metrics and forward to {}", endpoint))
        .unwrap_or_else(|_| "To use OTLP, run an OpenTelemetry Collector that scrapes http://localhost:3000/metrics".to_string());

    // Initialize task storage
    let task_storage = Arc::new(TaskStorage::new());

    let rpc_router = MyServiceBuilder::new("/rpc")
        .with_usage_tracker({
            let usage_tracker = otel.usage_tracker();
            move |headers, user, payload| {
                let method = payload.method.clone();
                let context = RequestContext::jsonrpc(method);
                let usage_tracker = usage_tracker.clone();
                let headers_clone = headers.clone();
                let user_clone = user.cloned();

                async move {
                    // Log the request
                    match &user_clone {
                        Some(u) => {
                            info!(
                                "RPC call: method={}, user={}, permissions={:?}",
                                context.method, u.user_id, u.permissions,
                            );
                        }
                        None => {
                            info!("RPC call: method={}, user=anonymous", context.method,);
                        }
                    }

                    // Track the request
                    usage_tracker
                        .track_request(&headers_clone, user_clone.as_ref(), &context)
                        .await;
                }
            }
        })
        .with_method_duration_tracker({
            let duration_tracker = otel.method_duration_tracker();
            move |method: &str,
                  user: Option<&ras_jsonrpc_core::AuthenticatedUser>,
                  duration: std::time::Duration| {
                let context = RequestContext::jsonrpc(method.to_string());
                let duration_tracker = duration_tracker.clone();
                let user_clone = user.cloned();

                async move {
                    duration_tracker
                        .track_duration(&context, user_clone.as_ref(), duration)
                        .await;
                }
            }
        })
        .auth_provider(MyAuthProvider)
        .sign_in_handler({
            move |request| async move {
                println!("{request:?}");
                match request {
                    SignInRequest::WithCredentials { username, password } => {
                        if username == "admin" && password == "secret" {
                            Ok(SignInResponse::Success {
                                jwt: "admin_token".to_string(),
                            })
                        } else if username == "user" && password == "password" {
                            Ok(SignInResponse::Success {
                                jwt: "valid_token".to_string(),
                            })
                        } else {
                            Ok(SignInResponse::Failure {
                                msg: "Invalid credentials".to_string(),
                            })
                        }
                    }
                }
            }
        })
        .sign_out_handler({
            move |user, _request| async move {
                info!("User {} signed out", user.user_id);
                Ok(())
            }
        })
        .delete_everything_handler(|user, _request| async move {
            tracing::warn!("Admin {} is deleting everything!", user.user_id);
            Ok(())
        })
        .create_task_handler({
            let storage = task_storage.clone();
            move |_user, request| {
                let storage = storage.clone();
                async move {
                    let task = storage.create_task(request);
                    Ok(task)
                }
            }
        })
        .update_task_handler({
            let storage = task_storage.clone();
            move |_user, request| {
                let storage = storage.clone();
                async move {
                    storage.update_task(request).ok_or_else(|| {
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Task not found",
                        )) as Box<dyn std::error::Error + Send + Sync>
                    })
                }
            }
        })
        .delete_task_handler({
            let storage = task_storage.clone();
            move |_user, id| {
                let storage = storage.clone();
                async move { Ok(storage.delete_task(id)) }
            }
        })
        .get_task_handler({
            let storage = task_storage.clone();
            move |_user, id| {
                let storage = storage.clone();
                async move { Ok(storage.get_task(id)) }
            }
        })
        .list_tasks_handler({
            let storage = task_storage.clone();
            move |_user, _request| {
                let storage = storage.clone();
                async move { Ok(storage.list_tasks()) }
            }
        })
        .get_profile_handler({
            move |user, _request| async move {
                Ok(UserProfile {
                    username: if user.user_id == "admin123" {
                        "admin"
                    } else {
                        "user"
                    }
                    .to_string(),
                    email: format!("{}@example.com", user.user_id),
                    permissions: user.permissions.iter().cloned().collect(),
                    created_at: "2024-01-01T00:00:00Z".to_string(),
                })
            }
        })
        .get_dashboard_stats_handler({
            let storage = task_storage.clone();
            move |_user, _request| {
                let storage = storage.clone();
                async move { Ok(storage.get_stats()) }
            }
        })
        .build();

    // Create the main app with metrics endpoint
    let app = Router::new().merge(rpc_router).merge(otel.metrics_router());

    println!("Basic JSON-RPC Service");
    println!("===================");
    println!();
    println!("Available at: http://localhost:3000/rpc");
    println!();
    println!("Test credentials:");
    println!("  Admin: username='admin', password='secret'");
    println!("  User:  username='user', password='password'");
    println!();
    println!("Metrics available at: http://localhost:3000/metrics");
    println!();
    println!("{}", otlp_note);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
