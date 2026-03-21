use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;
use chrono::Local;

#[allow(non_camel_case_types)]
pub struct session_logger {
    pub log_path: PathBuf,
}

#[allow(non_camel_case_types)]
impl session_logger {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            log_path: workspace_dir.join("last_session_log.md"),
        }
    }

    pub async fn clear(&self) -> tokio::io::Result<()> {
        File::create(&self.log_path).await?;
        self.log("\n# enclave session log\n").await
    }

    pub async fn log(&self, message: &str) -> tokio::io::Result<()> {
        // ensure parent directory exists
        if let Some(parent) = self.log_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let mut file = match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("[logger error] failed to open log file at {:?}: {}", self.log_path, e);
                    return Err(e);
                }
            };

        let entry = if message.starts_with("#") || message.starts_with("---") {
            format!("{}\n", message)
        } else {
            format!("[{}] {}\n", timestamp, message)
        };

        if let Err(e) = file.write_all(entry.as_bytes()).await {
            eprintln!("[logger error] failed to write to log file: {}", e);
            return Err(e);
        }
        let _ = file.flush().await;
        
        // also log to server console for visibility
        println!("[session log] {}", message);
        Ok(())
    }
}
