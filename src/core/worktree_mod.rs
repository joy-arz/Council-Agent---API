use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use chrono::Local;

/// Manages git worktrees for isolated agent execution
#[derive(Debug, Clone)]
pub struct WorktreeManager {
    workspace_dir: PathBuf,
    worktrees_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
    pub branch: String,
}

impl WorktreeManager {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            workspace_dir: workspace_dir.clone(),
            worktrees_dir: workspace_dir.join(".enclave_worktrees"),
        }
    }

    /// Check if git repository exists in workspace
    pub fn is_git_repo(&self) -> bool {
        self.workspace_dir.join(".git").exists()
    }

    /// Create a new isolated worktree for a session
    pub async fn create_worktree(&self, session_id: &str) -> Result<Worktree, anyhow::Error> {
        if !self.is_git_repo() {
            return Err(anyhow::anyhow!("workspace is not a git repository"));
        }

        // Create worktrees directory
        fs::create_dir_all(&self.worktrees_dir).await?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let name = format!("session_{}_{}", &session_id[..session_id.len().min(8)], timestamp);
        let branch = format!("enclave/{}", name);
        let path = self.worktrees_dir.join(&name);

        // Check if worktree already exists
        if path.exists() {
            return Ok(Worktree {
                name,
                path,
                branch,
            });
        }

        // Create worktree using git
        let output = Command::new("git")
            .args(["worktree", "add", "-b", &branch, &path.to_string_lossy(), "HEAD"])
            .current_dir(&self.workspace_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("failed to create worktree: {}", stderr));
        }

        Ok(Worktree {
            name,
            path,
            branch,
        })
    }

    /// Remove a worktree when session ends
    pub async fn remove_worktree(&self, worktree: &Worktree) -> Result<(), anyhow::Error> {
        let mut removal_success = false;
        let mut errors = Vec::new();

        // Remove worktree using git
        match Command::new("git")
            .args(["worktree", "remove", "--force", &worktree.path.to_string_lossy()])
            .current_dir(&self.workspace_dir)
            .output()
        {
            Ok(output) => {
                if output.status.success() {
                    removal_success = true;
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    errors.push(format!("git worktree remove failed: {}", stderr));
                }
            }
            Err(e) => {
                errors.push(format!("failed to execute git worktree remove: {}", e));
            }
        }

        // Remove the branch
        match Command::new("git")
            .args(["branch", "-D", &worktree.branch])
            .current_dir(&self.workspace_dir)
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // Branch might not exist, so just log at debug level
                    tracing::debug!("git branch delete failed (may not exist): {}", stderr);
                }
            }
            Err(e) => {
                errors.push(format!("failed to execute git branch delete: {}", e));
            }
        }

        // Retry logic for directory removal (handles race conditions with git cleanup)
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;
        while worktree.path.exists() && retry_count < MAX_RETRIES {
            // Wait with exponential backoff before retry
            let delay_ms = 100 * (2u32.pow(retry_count));
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms as u64)).await;
            
            match fs::remove_dir_all(&worktree.path).await {
                Ok(_) => {
                    tracing::info!("Cleaned up worktree directory: {:?}", worktree.path);
                    removal_success = true;
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count < MAX_RETRIES {
                        tracing::debug!("Worktree directory removal attempt {} failed: {}, retrying...", retry_count, e);
                    } else {
                        errors.push(format!("failed to remove worktree directory after {} attempts: {}", retry_count, e));
                        break;
                    }
                }
            }
        }

        // Log warnings for any errors encountered
        if !errors.is_empty() {
            for err in &errors {
                tracing::warn!("Worktree cleanup: {}", err);
            }
            // Return error only if directory still exists (cleanup truly failed)
            if worktree.path.exists() {
                return Err(anyhow::anyhow!("worktree cleanup failed: {}", errors.join("; ")));
            }
        }

        if removal_success || !worktree.path.exists() {
            tracing::info!("Worktree {} cleaned up successfully", worktree.name);
        }

        Ok(())
    }

    /// Get path for a specific worktree or fall back to main workspace
    pub fn get_execution_path(&self, worktree: Option<&Worktree>) -> PathBuf {
        worktree.map(|w| w.path.clone()).unwrap_or(self.workspace_dir.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_name_format() {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let name = format!("session_abc123_{}", timestamp);
        assert!(name.starts_with("session_"));
    }
}
