use axum::{Router, extract::State, routing::get};
use basic_jsonrpc_api::{
    SignInRequest, SignInResponse, MyServiceBuilder, Task, TaskPriority, CreateTaskRequest,
    UpdateTaskRequest, TaskListResponse, UserProfile, DashboardStats,
};
use opentelemetry::{
    KeyValue, global,
    metrics::{Counter, Meter},
};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use prometheus::{Encoder, TextEncoder};
use ras_jsonrpc_core::{AuthFuture, AuthProvider, AuthenticatedUser};
use std::{collections::{HashMap, HashSet}, sync::{Arc, Mutex}};
use chrono::Utc;
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

// Metrics struct to hold OpenTelemetry instruments
#[derive(Clone)]
struct Metrics {
    rpc_requests_started: Counter<u64>,
    rpc_requests_completed: Counter<u64>,
    rpc_method_duration: opentelemetry::metrics::Histogram<f64>,
    active_users: Counter<f64>,
}

impl Metrics {
    fn new(meter: &Meter) -> Self {
        Self {
            rpc_requests_started: meter
                .u64_counter("rpc_requests_started_total")
                .with_description("Total number of RPC requests started")
                .with_unit("requests")
                .build(),
            rpc_requests_completed: meter
                .u64_counter("rpc_requests_completed_total")
                .with_description("Total number of RPC requests completed (Note: This tracks usage_tracker completion, not actual method execution)")
                .with_unit("requests")
                .build(),
            rpc_method_duration: meter
                .f64_histogram("rpc_method_duration_seconds")
                .with_description("Duration of RPC method execution in seconds")
                .with_unit("seconds")
                .build(),
            active_users: meter
                .f64_counter("active_users")
                .with_description("Number of active users")
                .with_unit("users")
                .build(),
        }
    }
}


async fn metrics_handler(State(prometheus_registry): State<Arc<prometheus::Registry>>) -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus_registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
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
        
        self.tasks.lock().unwrap().insert(task.id.clone(), task.clone());
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
        let high_priority_tasks = tasks.values()
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

    // Set as global meter provider - this is important!
    global::set_meter_provider(meter_provider.clone());

    // Keep the meter provider alive by storing it
    let _meter_provider_handle = Arc::new(meter_provider);
    let prometheus_registry = Arc::new(prometheus_registry);

    // Get meter and create metrics
    let meter = global::meter("basic-jsonrpc-service");
    let metrics = Arc::new(Metrics::new(&meter));

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
            let metrics = metrics.clone();
            move |headers, user, payload| {
                let user_agent = headers
                    .get("user-agent")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                let method = payload.method.clone();
                let metrics = metrics.clone();
                let user_info = user.map(|u| (u.user_id.clone(), u.permissions.clone()));

                async move {
                    // Record metrics
                    // NOTE: The usage_tracker is called BEFORE the actual RPC method executes,
                    // so we can only track that a request was started here.
                    let attributes = vec![
                        KeyValue::new("method", method.clone()),
                        KeyValue::new("user_agent", user_agent.clone()),
                    ];

                    let authenticated_attributes = match &user_info {
                        Some((user_id, permissions)) => {
                            let mut attrs = attributes.clone();
                            attrs.push(KeyValue::new("user_id", user_id.clone()));
                            attrs.push(KeyValue::new("authenticated", "true"));
                            attrs.push(KeyValue::new(
                                "has_admin",
                                permissions.contains("admin").to_string(),
                            ));

                            tracing::info!(
                                "RPC call: method={}, user={}, permissions={:?}, user_agent={}",
                                method,
                                user_id,
                                permissions,
                                user_agent
                            );

                            attrs
                        }
                        None => {
                            let mut attrs = attributes.clone();
                            attrs.push(KeyValue::new("authenticated", "false"));

                            tracing::info!(
                                "RPC call: method={}, user=anonymous, user_agent={}",
                                method,
                                user_agent
                            );

                            attrs
                        }
                    };

                    // Increment request started counter
                    metrics
                        .rpc_requests_started
                        .add(1, &authenticated_attributes);

                    // Mark usage tracker completion (not the actual method completion)
                    metrics
                        .rpc_requests_completed
                        .add(1, &authenticated_attributes);
                }
            }
        })
        .with_method_duration_tracker({
            let metrics = metrics.clone();
            move |method: &str,
                  user: Option<&ras_jsonrpc_core::AuthenticatedUser>,
                  duration: std::time::Duration| {
                let metrics = metrics.clone();
                let method = method.to_string();
                let user_id = user.map(|u| u.user_id.clone());
                let is_admin = user
                    .map(|u| u.permissions.contains("admin"))
                    .unwrap_or(false);

                async move {
                    let mut attributes = vec![KeyValue::new("method", method.clone())];

                    if let Some(ref user_id) = user_id {
                        attributes.push(KeyValue::new("user_id", user_id.clone()));
                        attributes.push(KeyValue::new("authenticated", "true"));
                        attributes.push(KeyValue::new("has_admin", is_admin.to_string()));
                    } else {
                        attributes.push(KeyValue::new("authenticated", "false"));
                    }

                    // Record the duration in seconds
                    metrics
                        .rpc_method_duration
                        .record(duration.as_secs_f64(), &attributes);

                    tracing::info!(
                        "RPC method completed: method={}, duration={:?}, user={}",
                        method,
                        duration,
                        user_id.as_deref().unwrap_or("anonymous")
                    );
                }
            }
        })
        .auth_provider(MyAuthProvider)
        .sign_in_handler({
            let metrics = metrics.clone();
            move |request| {
                let metrics = metrics.clone();
                async move {
                    println!("{request:?}");
                    match request {
                        SignInRequest::WithCredentials { username, password } => {
                            if username == "admin" && password == "secret" {
                                // Track user sign-in
                                metrics.active_users.add(
                                    1.0,
                                    &[
                                        KeyValue::new("user_type", "admin"),
                                        KeyValue::new("action", "sign_in"),
                                    ],
                                );

                                Ok(SignInResponse::Success {
                                    jwt: "admin_token".to_string(),
                                })
                            } else if username == "user" && password == "password" {
                                // Track user sign-in
                                metrics.active_users.add(
                                    1.0,
                                    &[
                                        KeyValue::new("user_type", "user"),
                                        KeyValue::new("action", "sign_in"),
                                    ],
                                );

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
            }
        })
        .sign_out_handler({
            let metrics = metrics.clone();
            move |user, _request| {
                let metrics = metrics.clone();
                async move {
                    tracing::info!("User {} signed out", user.user_id);

                    // Track user sign-out
                    let user_type = if user.permissions.contains("admin") {
                        "admin"
                    } else {
                        "user"
                    };
                    metrics.active_users.add(
                        -1.0,
                        &[
                            KeyValue::new("user_type", user_type),
                            KeyValue::new("action", "sign_out"),
                        ],
                    );

                    Ok(())
                }
            }
        })
        .delete_everything_handler(|user, _request| async move {
            tracing::warn!("Admin {} is deleting everything!", user.user_id);
            Ok(())
        })
        // Task management handlers
        .list_tasks_handler({
            let storage = task_storage.clone();
            move |_user, _request| {
                let storage = storage.clone();
                async move {
                    Ok(storage.list_tasks())
                }
            }
        })
        .create_task_handler({
            let storage = task_storage.clone();
            move |_user, request| {
                let storage = storage.clone();
                async move {
                    Ok(storage.create_task(request))
                }
            }
        })
        .update_task_handler({
            let storage = task_storage.clone();
            move |_user, request| {
                let storage = storage.clone();
                async move {
                    storage.update_task(request)
                        .ok_or_else(|| "Task not found".into())
                }
            }
        })
        .delete_task_handler({
            let storage = task_storage.clone();
            move |_user, task_id| {
                let storage = storage.clone();
                async move {
                    Ok(storage.delete_task(task_id))
                }
            }
        })
        .get_task_handler({
            let storage = task_storage.clone();
            move |_user, task_id| {
                let storage = storage.clone();
                async move {
                    Ok(storage.get_task(task_id))
                }
            }
        })
        // User profile handlers
        .get_profile_handler(|user, _request| async move {
            let email = if user.permissions.contains("admin") {
                "admin@example.com"
            } else {
                "user@example.com"
            };
            
            Ok(UserProfile {
                username: user.user_id.clone(),
                email: email.to_string(),
                permissions: user.permissions.iter().cloned().collect(),
                created_at: Utc::now().to_rfc3339(),
            })
        })
        .update_profile_handler(|user, request| async move {
            let email = request.email.unwrap_or_else(|| {
                if user.permissions.contains("admin") {
                    "admin@example.com".to_string()
                } else {
                    "user@example.com".to_string()
                }
            });
            
            Ok(UserProfile {
                username: user.user_id.clone(),
                email,
                permissions: user.permissions.iter().cloned().collect(),
                created_at: Utc::now().to_rfc3339(),
            })
        })
        // Dashboard handler
        .get_dashboard_stats_handler({
            let storage = task_storage.clone();
            move |_user, _request| {
                let storage = storage.clone();
                async move {
                    Ok(storage.get_stats())
                }
            }
        })
        .build();

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(prometheus_registry)
        .nest("/api", rpc_router);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    println!("JSON-RPC endpoint: http://0.0.0.0:3000/api/rpc");
    println!("JSON-RPC Explorer: http://0.0.0.0:3000/api/explorer");
    println!("Prometheus metrics: http://0.0.0.0:3000/metrics");
    println!();
    println!("OpenTelemetry Metrics:");
    println!("  - Metrics are exposed in Prometheus format at /metrics");
    println!("  - {}", otlp_note);
    println!();
    println!("NOTE: Method duration tracking is now available!");
    println!("      The new with_method_duration_tracker captures actual method execution time.");
    println!("      Check the rpc_method_duration_seconds histogram in the /metrics endpoint.");
    println!();
    println!("Example credentials:");
    println!("  - Username: user, Password: password (basic user)");
    println!("  - Username: admin, Password: secret (admin user)");
    axum::serve(listener, app).await.unwrap();
}
