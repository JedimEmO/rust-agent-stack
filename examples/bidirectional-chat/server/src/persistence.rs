//! Persistence layer for chat state
//!
//! This module handles saving and loading chat room state and message history
//! to/from disk. Uses JSON files for simplicity.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedRoom {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub users: HashSet<String>, // Current users (for recovery after restart)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedMessage {
    pub id: u64,
    pub room_id: String,
    pub username: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    pub rooms: HashMap<String, PersistedRoom>,
    pub messages: Vec<PersistedMessage>,
    pub next_message_id: u64,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            rooms: HashMap::new(),
            messages: Vec::new(),
            next_message_id: 1,
        }
    }
}

pub struct PersistenceManager {
    data_dir: PathBuf,
    state_file: PathBuf,
    messages_dir: PathBuf,
}

impl PersistenceManager {
    pub fn new(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        let state_file = data_dir.join("chat_state.json");
        let messages_dir = data_dir.join("messages");

        Self {
            data_dir,
            state_file,
            messages_dir,
        }
    }

    pub async fn init(&self) -> Result<()> {
        // Create directories if they don't exist
        fs::create_dir_all(&self.data_dir).await?;
        fs::create_dir_all(&self.messages_dir).await?;
        Ok(())
    }

    pub async fn save_state(&self, state: &PersistedState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        fs::write(&self.state_file, json).await?;
        Ok(())
    }

    pub async fn load_state(&self) -> Result<PersistedState> {
        if !self.state_file.exists() {
            return Ok(PersistedState::default());
        }

        let json = fs::read_to_string(&self.state_file).await?;
        let state = serde_json::from_str(&json)?;
        Ok(state)
    }

    pub async fn append_message(&self, room_id: &str, message: &PersistedMessage) -> Result<()> {
        // Save messages per room in separate files for better performance
        let message_file = self.messages_dir.join(format!("{}.jsonl", room_id));

        let json = serde_json::to_string(message)?;
        let mut content = json;
        content.push('\n');

        // Append to file
        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&message_file)
            .await?;
        file.write_all(content.as_bytes()).await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn load_room_messages(
        &self,
        room_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<PersistedMessage>> {
        let message_file = self.messages_dir.join(format!("{}.jsonl", room_id));

        if !message_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&message_file).await?;
        let mut messages: Vec<PersistedMessage> = Vec::new();

        for line in content.lines() {
            if !line.trim().is_empty() {
                if let Ok(msg) = serde_json::from_str::<PersistedMessage>(line) {
                    messages.push(msg);
                }
            }
        }

        // Apply limit if specified (return most recent messages)
        if let Some(limit) = limit {
            let start = messages.len().saturating_sub(limit);
            messages = messages[start..].to_vec();
        }

        Ok(messages)
    }

    pub async fn delete_room_messages(&self, room_id: &str) -> Result<()> {
        let message_file = self.messages_dir.join(format!("{}.jsonl", room_id));
        if message_file.exists() {
            fs::remove_file(message_file).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_persistence_manager() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let persistence = PersistenceManager::new(temp_dir.path());
        persistence.init().await?;

        // Test saving and loading state
        let mut state = PersistedState::default();
        let mut room = PersistedRoom {
            id: "general".to_string(),
            name: "General".to_string(),
            created_at: Utc::now(),
            users: HashSet::new(),
        };
        room.users.insert("alice".to_string());
        state.rooms.insert("general".to_string(), room);
        state.next_message_id = 42;

        persistence.save_state(&state).await?;
        let loaded_state = persistence.load_state().await?;

        assert_eq!(loaded_state.next_message_id, 42);
        assert!(loaded_state.rooms.contains_key("general"));

        // Test message persistence
        let msg = PersistedMessage {
            id: 1,
            room_id: "general".to_string(),
            username: "alice".to_string(),
            text: "Hello, world!".to_string(),
            timestamp: Utc::now(),
        };

        persistence.append_message("general", &msg).await?;
        let messages = persistence.load_room_messages("general", None).await?;

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "Hello, world!");

        Ok(())
    }
}
