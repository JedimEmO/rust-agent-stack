use chrono::{DateTime, Local};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub enum Message {
    Incoming {
        username: String,
        text: String,
        timestamp: DateTime<Local>,
    },
    Outgoing {
        text: String,
        timestamp: DateTime<Local>,
    },
    System {
        text: String,
        timestamp: DateTime<Local>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
}

pub struct AppState {
    pub username: String,
    pub messages: VecDeque<Message>,
    pub users: Vec<String>,
    pub input: String,
    pub input_mode: InputMode,
    pub connection_status: ConnectionStatus,
    pub show_help: bool,
    pub scroll_offset: usize,
    pub pending_messages: Vec<String>,
}

impl AppState {
    pub fn new(username: String) -> Self {
        let mut state = Self {
            username: username.clone(),
            messages: VecDeque::new(),
            users: vec![username],
            input: String::new(),
            input_mode: InputMode::Normal,
            connection_status: ConnectionStatus::Connecting,
            show_help: false,
            scroll_offset: 0,
            pending_messages: Vec::new(),
        };
        
        state.add_system_message("Welcome to Bidirectional Chat! Press '?' for help.".to_string());
        state
    }
    
    pub fn add_incoming_message(&mut self, username: String, text: String) {
        self.messages.push_back(Message::Incoming {
            username,
            text,
            timestamp: Local::now(),
        });
        self.trim_messages();
    }
    
    pub fn add_outgoing_message(&mut self, text: String) {
        self.pending_messages.push(text.clone());
        self.messages.push_back(Message::Outgoing {
            text,
            timestamp: Local::now(),
        });
        self.trim_messages();
    }
    
    pub fn add_system_message(&mut self, text: String) {
        self.messages.push_back(Message::System {
            text,
            timestamp: Local::now(),
        });
        self.trim_messages();
    }
    
    pub fn set_users(&mut self, users: Vec<String>) {
        self.users = users;
    }
    
    pub fn add_user(&mut self, username: String) {
        if !self.users.contains(&username) {
            self.users.push(username.clone());
            self.add_system_message(format!("{} joined the chat", username));
        }
    }
    
    pub fn remove_user(&mut self, username: &str) {
        self.users.retain(|u| u != username);
        self.add_system_message(format!("{} left the chat", username));
    }
    
    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
        match status {
            ConnectionStatus::Connected => {
                self.add_system_message("Connected to server".to_string());
            }
            ConnectionStatus::Disconnected => {
                self.add_system_message("Disconnected from server".to_string());
            }
            _ => {}
        }
    }
    
    pub fn scroll_up(&mut self) {
        if self.scroll_offset < self.messages.len().saturating_sub(1) {
            self.scroll_offset += 1;
        }
    }
    
    pub fn scroll_down(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }
    
    pub fn scroll_up_page(&mut self) {
        self.scroll_offset = (self.scroll_offset + 10).min(self.messages.len().saturating_sub(1));
    }
    
    pub fn scroll_down_page(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(10);
    }
    
    pub fn get_next_pending_message(&mut self) -> Option<String> {
        if self.pending_messages.is_empty() {
            None
        } else {
            Some(self.pending_messages.remove(0))
        }
    }
    
    fn trim_messages(&mut self) {
        const MAX_MESSAGES: usize = 1000;
        while self.messages.len() > MAX_MESSAGES {
            self.messages.pop_front();
            if self.scroll_offset > 0 {
                self.scroll_offset -= 1;
            }
        }
    }
}