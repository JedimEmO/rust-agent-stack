use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{
    io::{self, IsTerminal},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;

use crate::{
    auth::get_current_token,
    client::ChatClient,
    config::Config,
};

pub mod state;
pub mod widgets;

use state::{AppState, InputMode};

pub async fn run_chat_ui(config: &Config) -> Result<()> {
    // Check if user is logged in
    let token = get_current_token()?
        .ok_or_else(|| anyhow::anyhow!("Not logged in. Please login first with: bidirectional-chat-client login -u <username>"))?;
    
    // Check if we're running in a terminal
    if !io::stdout().is_terminal() {
        return Err(anyhow::anyhow!(
            "This command requires an interactive terminal. Please run it directly in a terminal, not through pipes or redirection."
        ));
    }
    
    // Setup terminal
    enable_raw_mode().map_err(|e| {
        anyhow::anyhow!("Failed to enable raw mode. Make sure you're running in a proper terminal: {}", e)
    })?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture).map_err(|e| {
        disable_raw_mode().ok();
        anyhow::anyhow!("Failed to setup terminal: {}", e)
    })?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let app_state = Arc::new(Mutex::new(AppState::new(token.username.clone())));
    
    // Create chat client
    let client = ChatClient::new(config.websocket_url.clone(), token.token, app_state.clone()).await?;
    
    // Join the default room
    client.join_room("general".to_string()).await?;
    
    // Start the message handler task
    let message_handler = tokio::spawn({
        let client = client.clone();
        async move {
            client.handle_pending_messages().await;
        }
    });
    
    // Run the UI
    let res = run_app(&mut terminal, app_state).await;
    
    // Cancel the message handler
    message_handler.abort();
    
    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    
    if let Err(err) = res {
        println!("Error: {:?}", err);
    }
    
    Ok(())
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: Arc<Mutex<AppState>>,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app_state))?;
        
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let mut state = app_state.lock().await;
                    
                    match state.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') => return Ok(()),
                            KeyCode::Char('i') => {
                                state.input_mode = InputMode::Editing;
                            }
                            KeyCode::Char('?') => {
                                state.show_help = !state.show_help;
                            }
                            KeyCode::Up => {
                                state.scroll_up();
                            }
                            KeyCode::Down => {
                                state.scroll_down();
                            }
                            KeyCode::PageUp => {
                                state.scroll_up_page();
                            }
                            KeyCode::PageDown => {
                                state.scroll_down_page();
                            }
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
                                if !state.input.is_empty() {
                                    let message = state.input.drain(..).collect::<String>();
                                    state.add_outgoing_message(message);
                                }
                            }
                            KeyCode::Char(c) => {
                                state.input.push(c);
                            }
                            KeyCode::Backspace => {
                                state.input.pop();
                            }
                            KeyCode::Esc => {
                                state.input_mode = InputMode::Normal;
                            }
                            _ => {}
                        },
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app_state: &Arc<Mutex<AppState>>) {
    let state = app_state.blocking_lock();
    
    // Create main layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Header
            Constraint::Min(10),        // Messages area
            Constraint::Length(3),      // Input area
            Constraint::Length(1),      // Status bar
        ])
        .split(f.area());
    
    // Header
    let header = Paragraph::new(format!(" 🐱 Bidirectional Chat - {} ", state.username))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, chunks[0]);
    
    // Main content area (messages + users)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(50),        // Messages
            Constraint::Length(30),     // User list
        ])
        .split(chunks[1]);
    
    // Messages area
    render_messages(f, content_chunks[0], &state);
    
    // User list
    render_user_list(f, content_chunks[1], &state);
    
    // Input area
    render_input(f, chunks[2], &state);
    
    // Status bar
    render_status_bar(f, chunks[3], &state);
    
    // Help overlay
    if state.show_help {
        render_help_overlay(f);
    }
}

fn render_messages(f: &mut Frame, area: Rect, state: &AppState) {
    let messages: Vec<ListItem> = state
        .messages
        .iter()
        .map(|msg| {
            let content = match msg {
                state::Message::Incoming { username, text, timestamp } => {
                    vec![
                        Span::styled(
                            format!("[{}] ", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            format!("{}: ", username),
                            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(text),
                    ]
                }
                state::Message::Outgoing { text, timestamp } => {
                    vec![
                        Span::styled(
                            format!("[{}] ", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            "You: ",
                            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(text),
                    ]
                }
                state::Message::System { text, timestamp } => {
                    vec![
                        Span::styled(
                            format!("[{}] ", timestamp.format("%H:%M:%S")),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(
                            text,
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
                        ),
                    ]
                }
            };
            ListItem::new(Line::from(content))
        })
        .collect();
    
    let messages_list = List::new(messages)
        .block(Block::default().borders(Borders::ALL).title(" Messages "))
        .style(Style::default().fg(Color::White));
        
    f.render_widget(messages_list, area);
}

fn render_user_list(f: &mut Frame, area: Rect, state: &AppState) {
    let users: Vec<ListItem> = state
        .users
        .iter()
        .map(|user| {
            let style = if user == &state.username {
                Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!(" 🐱 {}", user)).style(style)
        })
        .collect();
    
    let user_list = List::new(users)
        .block(Block::default().borders(Borders::ALL).title(" Online Users "))
        .style(Style::default().fg(Color::White));
        
    f.render_widget(user_list, area);
}

fn render_input(f: &mut Frame, area: Rect, state: &AppState) {
    let input = Paragraph::new(state.input.as_str())
        .style(match state.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Style::default().fg(Color::Yellow),
        })
        .block(Block::default().borders(Borders::ALL).title(match state.input_mode {
            InputMode::Normal => " Press 'i' to type ",
            InputMode::Editing => " Type your message (ESC to cancel, Enter to send) ",
        }));
        
    f.render_widget(input, area);
    
    if state.input_mode == InputMode::Editing {
        f.set_cursor_position((
            area.x + state.input.len() as u16 + 1,
            area.y + 1,
        ));
    }
}

fn render_status_bar(f: &mut Frame, area: Rect, state: &AppState) {
    let status = match state.connection_status {
        state::ConnectionStatus::Connected => {
            Span::styled(" ● Connected ", Style::default().fg(Color::Green))
        }
        state::ConnectionStatus::Connecting => {
            Span::styled(" ◌ Connecting... ", Style::default().fg(Color::Yellow))
        }
        state::ConnectionStatus::Disconnected => {
            Span::styled(" ○ Disconnected ", Style::default().fg(Color::Red))
        }
    };
    
    let help = Span::styled(
        " Press '?' for help | 'q' to quit ",
        Style::default().fg(Color::DarkGray),
    );
    
    let status_bar = Paragraph::new(Line::from(vec![status, help]))
        .style(Style::default().bg(Color::Black));
        
    f.render_widget(status_bar, area);
}

fn render_help_overlay(f: &mut Frame) {
    let area = centered_rect(60, 40, f.area());
    
    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("  Keyboard Shortcuts", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  i        - Enter typing mode"),
        Line::from("  ESC      - Exit typing mode"),
        Line::from("  Enter    - Send message"),
        Line::from("  ↑/↓      - Scroll messages"),
        Line::from("  PgUp/Dn  - Scroll page"),
        Line::from("  ?        - Toggle this help"),
        Line::from("  q        - Quit application"),
        Line::from(""),
    ];
    
    let help = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Help ")
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);
        
    f.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}