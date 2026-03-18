use crate::agents::{base_agent, judge_agent};
use crate::core::memory::shared_memory;
use crate::utils::logger_mod::session_logger;
use std::sync::Arc;
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::task::JoinSet;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub struct agent_response {
    pub agent: String,
    pub content: String,
    pub terminal_output: String,
    pub round: usize,
}

#[allow(non_camel_case_types)]
pub struct orchestrator {
    pub agents: Vec<base_agent>,
    pub judge: judge_agent,
    pub max_rounds: usize,
    pub memory: Arc<Mutex<shared_memory>>,
    pub workspace_dir: PathBuf,
    pub logger: Arc<session_logger>,
}

#[allow(non_camel_case_types)]
impl orchestrator {
    pub fn new(
        agents: Vec<base_agent>,
        judge: judge_agent,
        max_rounds: usize,
        memory_size: usize,
        workspace_dir: PathBuf,
    ) -> Self {
        Self {
            agents,
            judge,
            max_rounds,
            memory: Arc::new(Mutex::new(shared_memory::new(memory_size))),
            workspace_dir: workspace_dir.clone(),
            logger: Arc::new(session_logger::new(workspace_dir)),
        }
    }

    async fn get_state_path(&self) -> PathBuf {
        self.workspace_dir.join(".council_state.md")
    }

    pub async fn run_council<F, Fut>(&self, query: &str, mut on_message: F) -> Result<String, anyhow::Error>
    where
        F: FnMut(agent_response) -> Fut,
        Fut: std::future::Future<Output = Result<(), ()>>,
    {
        // init session log
        let _ = self.logger.clear().await;
        let _ = self.logger.log(&format!("--- session started ---\nquery: {}", query)).await;

        {
            let mut mem = self.memory.lock().await;
            mem.clear();
            mem.set_original_query(query.to_string());

            // load previous project state if it exists (refinement: project continuation)
            let state_path = self.get_state_path().await;
            if state_path.exists() {
                if let Ok(state_content) = fs::read_to_string(state_path).await {
                    let _ = self.logger.log("restoring project state from .council_state.md").await;
                    mem.add_message("system".to_string(), format!("previous project state:\n{}", state_content), true);
                }
            }
        }

        for round in 1..=self.max_rounds {
            let _ = self.logger.log(&format!("--- round {} ---", round)).await;
            
            // sequential phase: the strategist always sets the baseline
            let strategist = &self.agents[0];
            let history = self.memory.lock().await.get_formatted_history();
            
            let _ = self.logger.log(&format!("asking {}...", strategist.name)).await;

            let (strategy, terminal) = match strategist.get_response(&history).await {
                Ok((s, t)) => {
                    let _ = self.logger.log(&format!("{} response received.", strategist.name)).await;
                    (s, t)
                },
                Err(e) => {
                    let _ = self.logger.log(&format!("error from {}: {}", strategist.name, e)).await;
                    (format!("(failed to respond due to error: {})", e), format!("error: {}", e))
                }
            };

            // pin the first strategist response (refinement 1)            self.memory.lock().await.add_message(strategist.name.clone(), strategy.clone(), round == 1);
            if on_message(agent_response {
                agent: strategist.name.clone(),
                content: strategy,
                terminal_output: terminal,
                round,
            }).await.is_err() {
                let _ = self.logger.log("client disconnected. aborting council.").await;
                return Err(anyhow::anyhow!("client disconnected"));
            }

            // parallel phase (refinement 2): critic, optimizer, contrarian analyze the strategy concurrently
            let mut set = JoinSet::new();
            let history = self.memory.lock().await.get_formatted_history();
            
            // note: we skip the strategist (index 0) as it already spoke
            for agent in self.agents.iter().skip(1) {
                let agent_arc = Arc::new(agent.clone_for_parallel());
                let history_clone = history.clone();
                let logger_arc = self.logger.clone();
                set.spawn(async move {
                    let _ = logger_arc.log(&format!("asking {} in parallel...", agent_arc.name)).await;
                    let res = agent_arc.get_response(&history_clone).await;
                    (agent_arc.name.clone(), res)
                });
            }

            while let Some(result) = set.join_next().await {
                let (name, res) = match result {
                    Ok(r) => r,
                    Err(e) => {
                        let _ = self.logger.log(&format!("task panic or thread error: {}", e)).await;
                        continue;
                    }
                };
                
                let (content, terminal) = match res {
                    Ok(c) => {
                        let _ = self.logger.log(&format!("{} parallel response received.", name)).await;
                        c
                    },
                    Err(e) => {
                        let _ = self.logger.log(&format!("error from {}: {}", name, e)).await;
                        // soft fail: let the council continue instead of crashing the whole session
                        (format!("(failed to respond due to error: {})", e), format!("error: {}", e))
                    }
                };
                
                self.memory.lock().await.add_message(name.clone(), content.clone(), false);
                if on_message(agent_response {
                    agent: name,
                    content,
                    terminal_output: terminal,
                    round,
                }).await.is_err() {
                    let _ = self.logger.log("client disconnected. aborting council.").await;
                    return Err(anyhow::anyhow!("client disconnected"));
                }
            }
        }

        // final judge verdict
        let history = self.memory.lock().await.get_formatted_history();
        let _ = self.logger.log("--- final verdict phase ---").await;
        let (verdict, terminal) = self.judge.get_final_verdict(&history).await?;
        let _ = self.logger.log("lead engineer verdict received.").await;

        if on_message(agent_response {
            agent: self.judge.base.name.clone(),
            content: verdict.clone(),
            terminal_output: terminal,
            round: self.max_rounds + 1,
        }).await.is_err() {
            let _ = self.logger.log("client disconnected. aborting council.").await;
            return Err(anyhow::anyhow!("client disconnected"));
        }

        // update project state file for future sessions
        let state_path = self.get_state_path().await;
        let _ = fs::write(state_path, &verdict).await;
        let _ = self.logger.log("project state updated. session complete.").await;

        Ok(verdict)
    }
}
