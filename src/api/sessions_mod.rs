use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::core::agent_response;
use std::path::PathBuf;
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub first_message: String,
    pub message_count: usize,
}

#[allow(non_camel_case_types)]
pub struct session_store {
    // map from session id to a list of messages
    pub sessions: Mutex<HashMap<String, Vec<agent_response>>>,
    pub workspace_dir: PathBuf,
}

#[allow(non_camel_case_types)]
impl session_store {
    pub fn new(workspace_dir: PathBuf) -> Self {
        let mut store = Self {
            sessions: Mutex::new(HashMap::new()),
            workspace_dir,
        };
        store.load_from_disk();
        store
    }

    fn get_history_path(&self) -> PathBuf {
        self.workspace_dir.join(".enclave_history.json")
    }

    fn load_from_disk(&mut self) {
        let path = self.get_history_path();
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(data) => {
                    match serde_json::from_str::<HashMap<String, Vec<agent_response>>>(&data) {
                        Ok(loaded_sessions) => {
                            *self.sessions.get_mut() = loaded_sessions;
                        }
                        Err(e) => {
                            eprintln!("Warning: failed to parse session history: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: failed to read session history file: {}", e);
                }
            }
        }
    }

    pub async fn add_message(&self, session_id: &str, msg: agent_response) {
        let mut sessions = self.sessions.lock().await;
        sessions.entry(session_id.to_string()).or_default().push(msg);
        
        // Save while still holding the lock to prevent race conditions
        let data = serde_json::to_string_pretty(&*sessions)
            .unwrap_or_else(|e| {
                eprintln!("Warning: failed to serialize session: {}", e);
                String::new()
            });
        
        if data.is_empty() {
            return;
        }
        
        drop(sessions); // Release lock before I/O
        
        if let Err(e) = tokio::fs::write(self.get_history_path(), data).await {
            eprintln!("Warning: failed to persist session: {}", e);
        }
    }

    pub async fn get_history(&self, session_id: &str) -> Vec<agent_response> {
        let sessions: tokio::sync::MutexGuard<'_, HashMap<String, Vec<agent_response>>> = self.sessions.lock().await;
        sessions.get(session_id).cloned().unwrap_or_default()
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let sessions: tokio::sync::MutexGuard<'_, HashMap<String, Vec<agent_response>>> = self.sessions.lock().await;
        sessions.iter()
            .map(|(session_id, messages)| {
                let first_message = messages.first()
                    .map(|m| {
                        // Get the user query (first message with "User" agent)
                        if m.agent == "User" {
                            m.content.clone()
                        } else {
                            // If no user message found, get first message content truncated
                            m.content.chars().take(100).collect::<String>()
                        }
                    })
                    .unwrap_or_default();

                SessionSummary {
                    session_id: session_id.clone(),
                    first_message,
                    message_count: messages.len(),
                }
            })
            .collect()
    }

    pub async fn delete_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.lock().await;
        let removed = sessions.remove(session_id).is_some();
        
        if removed {
            // Save while still holding the lock
            let data = serde_json::to_string_pretty(&*sessions)
                .unwrap_or_else(|e| {
                    eprintln!("Warning: failed to serialize session: {}", e);
                    String::new()
                });
            
            drop(sessions);
            
            if !data.is_empty() {
                if let Err(e) = tokio::fs::write(self.get_history_path(), data).await {
                    eprintln!("Warning: failed to persist session after deletion: {}", e);
                }
            }
            true
        } else {
            false
        }
    }
}
