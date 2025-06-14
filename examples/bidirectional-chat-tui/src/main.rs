mod app;
mod auth;
mod ui;

use anyhow::Result;
use app::{AppEvent, AppScreen, AppState, AuthField, ChatClient};
use auth::AuthClient;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io::{self, Write},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc, Mutex},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    dotenvy::dotenv().ok();
    let server_url = std::env::var("SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.show_cursor()?;

    // Create app state and event channel
    let app_state = Arc::new(Mutex::new(AppState::default()));
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AppEvent>();
    
    // Create clients
    let auth_client = AuthClient::new(server_url.clone());
    let chat_client = Arc::new(Mutex::new(ChatClient::new(event_tx.clone())));

    // Run the app
    let res = run_app(&mut terminal, app_state.clone(), &auth_client, chat_client.clone(), &server_url, &mut event_rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app_state: Arc<Mutex<AppState>>,
    auth_client: &AuthClient,
    chat_client: Arc<Mutex<ChatClient>>,
    server_url: &str,
    event_rx: &mut mpsc::UnboundedReceiver<AppEvent>,
) -> Result<()> {
    let mut jwt_token: Option<String> = None;

    loop {
        // Draw UI
        {
            let app = app_state.lock().await;
            terminal.draw(|f| ui::draw(f, &app))?;
            terminal.backend_mut().flush()?;
        }

        // Check for terminal events
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                let mut app = app_state.lock().await;
                
                match &app.screen {
                    AppScreen::Login | AppScreen::Register => {
                        match key.code {
                            KeyCode::Tab => {
                                app.auth_field_focus = match app.auth_field_focus {
                                    AuthField::Username => AuthField::Password,
                                    AuthField::Password => AuthField::Username,
                                };
                            }
                            KeyCode::Enter => {
                                let username = app.auth_username_input.clone();
                                let password = app.auth_password_input.clone();
                                drop(app); // Release lock before async operation
                                
                                // Skip if empty
                                if username.is_empty() || password.is_empty() {
                                    app_state.lock().await.error_message = Some("Username and password cannot be empty".to_string());
                                    continue;
                                }
                                
                                let result = if app_state.lock().await.screen == AppScreen::Login {
                                    auth_client.login(username.clone(), password).await
                                } else {
                                    auth_client.register(username.clone(), password).await
                                };
                                
                                match result {
                                    Ok(auth_response) => {
                                        jwt_token = Some(auth_response.token);
                                        let mut app = app_state.lock().await;
                                        app.username = Some(username);
                                        app.screen = AppScreen::RoomList;
                                        app.error_message = None;
                                        drop(app);
                                        
                                        // Connect to WebSocket
                                        let mut client = chat_client.lock().await;
                                        if let Err(e) = client.connect(server_url, jwt_token.clone().unwrap()).await {
                                            app_state.lock().await.error_message = Some(format!("Failed to connect: {}", e));
                                        } else {
                                            // Load room list
                                            match client.list_rooms().await {
                                                Ok(rooms) => {
                                                    app_state.lock().await.rooms = rooms;
                                                }
                                                Err(e) => {
                                                    app_state.lock().await.error_message = Some(format!("Failed to load rooms: {}", e));
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        app_state.lock().await.error_message = Some(e.to_string());
                                    }
                                }
                            }
                            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                app.screen = AppScreen::Register;
                                app.error_message = None;
                            }
                            KeyCode::Esc => {
                                if app.screen == AppScreen::Register {
                                    app.screen = AppScreen::Login;
                                    app.error_message = None;
                                } else {
                                    return Ok(());
                                }
                            }
                            KeyCode::Backspace => {
                                match app.auth_field_focus {
                                    AuthField::Username => { app.auth_username_input.pop(); }
                                    AuthField::Password => { app.auth_password_input.pop(); }
                                }
                            }
                            KeyCode::Char(c) => {
                                match app.auth_field_focus {
                                    AuthField::Username => app.auth_username_input.push(c),
                                    AuthField::Password => app.auth_password_input.push(c),
                                }
                                tracing::debug!("Input char: {}, username: {}, password len: {}", c, app.auth_username_input, app.auth_password_input.len());
                            }
                            _ => {}
                        }
                    }
                    AppScreen::RoomList => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                let mut client = chat_client.lock().await;
                                let _ = client.disconnect().await;
                                return Ok(());
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                drop(app);
                                let client = chat_client.lock().await;
                                match client.list_rooms().await {
                                    Ok(rooms) => {
                                        app_state.lock().await.rooms = rooms;
                                    }
                                    Err(e) => {
                                        app_state.lock().await.error_message = Some(format!("Failed to refresh rooms: {}", e));
                                    }
                                }
                            }
                            KeyCode::Char(c) if c.is_digit(10) => {
                                let index = c.to_digit(10).unwrap() as usize - 1;
                                if index < app.rooms.len() {
                                    let room_name = app.rooms[index].room_name.clone();
                                    drop(app);
                                    
                                    let client = chat_client.lock().await;
                                    match client.join_room(room_name.clone()).await {
                                        Ok((room_id, _)) => {
                                            let mut app = app_state.lock().await;
                                            app.current_room = Some((room_id.clone(), room_name.clone()));
                                            app.screen = AppScreen::Chat { room_id, room_name };
                                            app.messages.clear();
                                        }
                                        Err(e) => {
                                            app_state.lock().await.error_message = Some(format!("Failed to join room: {}", e));
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    AppScreen::Chat { room_id, .. } => {
                        match key.code {
                            KeyCode::Esc => {
                                let room_id = room_id.clone();
                                drop(app);
                                
                                let client = chat_client.lock().await;
                                if let Err(e) = client.leave_room(room_id).await {
                                    app_state.lock().await.error_message = Some(format!("Failed to leave room: {}", e));
                                }
                                
                                let mut app = app_state.lock().await;
                                app.screen = AppScreen::RoomList;
                                app.current_room = None;
                                app.input_buffer.clear();
                            }
                            KeyCode::Enter => {
                                if !app.input_buffer.is_empty() {
                                    let text = app.input_buffer.clone();
                                    app.input_buffer.clear();
                                    
                                    // Check for slash commands
                                    if text.starts_with('/') {
                                        let command = text.trim_start_matches('/').to_lowercase();
                                        match command.as_str() {
                                            "quit" | "exit" => {
                                                drop(app);
                                                let mut client = chat_client.lock().await;
                                                let _ = client.disconnect().await;
                                                return Ok(());
                                            }
                                            _ => {
                                                app.error_message = Some(format!("Unknown command: /{}", command));
                                            }
                                        }
                                    } else {
                                        drop(app);
                                        
                                        let client = chat_client.lock().await;
                                        if let Err(e) = client.send_message(text).await {
                                            app_state.lock().await.error_message = Some(format!("Failed to send message: {}", e));
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace => {
                                app.input_buffer.pop();
                            }
                            KeyCode::Char(c) => {
                                app.input_buffer.push(c);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Handle app events (non-blocking)
        if let Ok(event) = event_rx.try_recv() {
                let mut app = app_state.lock().await;
                match event {
                    AppEvent::MessageReceived(message) => {
                        app.messages.push(message);
                    }
                    AppEvent::UserJoined { username, room_id } => {
                        let msg = app::Message {
                            id: 0,
                            username: "System".to_string(),
                            text: format!("{} joined the room", username),
                            timestamp: chrono::Local::now(),
                            room_id,
                        };
                        app.messages.push(msg);
                    }
                    AppEvent::UserLeft { username, room_id } => {
                        let msg = app::Message {
                            id: 0,
                            username: "System".to_string(),
                            text: format!("{} left the room", username),
                            timestamp: chrono::Local::now(),
                            room_id,
                        };
                        app.messages.push(msg);
                    }
                    AppEvent::SystemAnnouncement { message } => {
                        if let Some((room_id, _)) = &app.current_room {
                            let msg = app::Message {
                                id: 0,
                                username: "System".to_string(),
                                text: message,
                                timestamp: chrono::Local::now(),
                                room_id: room_id.clone(),
                            };
                            app.messages.push(msg);
                        }
                    }
                    AppEvent::Connected => {
                        app.connected = true;
                    }
                    AppEvent::Disconnected => {
                        app.connected = false;
                        app.screen = AppScreen::Login;
                        app.error_message = Some("Disconnected from server".to_string());
                    }
                    AppEvent::Error(e) => {
                        app.error_message = Some(e);
                    }
                    _ => {}
                }
        }
        
        // Small delay to prevent busy loop
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}