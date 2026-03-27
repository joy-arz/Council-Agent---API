use std::sync::Arc;
use std::path::PathBuf;
use crate::core::model_provider;
use crate::core::tools::{execute_tool, parse_tool_calls};

#[allow(non_camel_case_types)]
pub struct base_agent {
    pub name: String,
    pub role: String,
    pub system_prompt: String,
    pub provider: Arc<dyn model_provider>,
    pub model_name: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub is_autonomous: bool,
    pub workspace_dir: PathBuf,
    pub max_tool_iterations: usize,
}

#[allow(non_camel_case_types)]
impl base_agent {
    pub fn new(
        name: &str,
        role: &str,
        system_prompt: &str,
        provider: Arc<dyn model_provider>,
        model_name: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            role: role.to_string(),
            system_prompt: system_prompt.to_string(),
            provider,
            model_name: model_name.to_string(),
            temperature,
            max_tokens,
            is_autonomous: false,
            workspace_dir: PathBuf::from("."),
            max_tool_iterations: 5,
        }
    }

    pub fn set_autonomous(&mut self, value: bool) {
        self.is_autonomous = value;
    }

    #[allow(dead_code)]
    pub fn set_workspace(&mut self, dir: PathBuf) {
        self.workspace_dir = dir;
    }

    pub fn build_full_system_prompt(&self) -> String {
        let safety_instruction = if self.is_autonomous {
            "you are in AUTONOMOUS mode. you have full permission to use tools to directly modify the codebase. your goal is to drive the workflow forward until the task is complete. you have full access to the workspace root and all its subfolders recursively. do not just talk; ACT by reading and editing any necessary files in the workspace (including nested folders). assume ALL changes are pre-accepted and proceed immediately with implementation."
        } else {
            "you are in PROPOSAL mode. you must not modify files directly. instead, you should provide your suggested changes in your response using the following format:\n\n[PROPOSE_CHANGE:path/to/file]\n[new content of the file]\n[/PROPOSE_CHANGE]\n\nthe user will review these proposals and choose whether to apply them. you can propose multiple file changes in a single response."
        };

        let tools_instruction = "RESPONSE RULES (CRITICAL):\n1. Keep responses SHORT - maximum 2-3 sentences\n2. If using tools, output JSON and NOTHING else\n3. Do NOT explain what you are about to do or what you did - just do it or give the answer\n4. NEVER start with \"Based on...\", \"Looking at...\", \"The user wants me to...\" - just answer directly\n5. Valid tools: list_directory, read_file, write_file, run_shell_command, grep\n\nExamples of GOOD responses:\n- \"The project has Cargo.toml, src/, and tests/.\"\n- {\"name\": \"list_directory\", \"arguments\": {\"path\": \".\"}}\n- \"There's a bug in line 42 - missing null check.\"\n\nExamples of BAD responses (do not do these):\n- \"Based on my analysis of the workspace, I can see that...\"\n- \"The user wants me to list files, so let me do that by calling...\"\n- \"Looking at the previous tool results, I notice that...\"";

        format!(
            "you are a {}.\n{}\n\nresponsibilities:\n{}\n\n{}",
            self.role, safety_instruction, self.system_prompt, tools_instruction
        )
    }

    /// Execute a response with tool calls, continuing until no more tool calls or max iterations reached
    pub async fn get_response_with_tools(&self, history: &str) -> Result<(String, String), anyhow::Error> {
        let mut current_history = history.to_string();
        let mut iterations = 0;
        let mut final_text = String::new();

        loop {
            iterations += 1;
            if iterations > self.max_tool_iterations {
                final_text.push_str("\n[Max tool iterations reached]");
                break;
            }

            let prompt = format!(
                "current conversation history:\n{}\n\nrespond as the {}. use tools if needed to complete the task.",
                current_history, self.name
            );

            let (response, _raw) = self.provider.call_model(
                &self.model_name,
                &prompt,
                Some(&self.build_full_system_prompt()),
                self.temperature,
                self.max_tokens,
            ).await?;

            // Parse tool calls from response
            let tool_calls = parse_tool_calls(&response);

            if tool_calls.is_empty() {
                // No tool calls detected - this is the final response
                // Only use meaningful content (skip verbose explanations)
                let cleaned = response.trim().to_string();
                if !cleaned.is_empty() && !final_text.is_empty() {
                    final_text.push_str("\n");
                    final_text.push_str(&cleaned);
                } else if final_text.is_empty() {
                    final_text = cleaned;
                }
                break;
            }

            // Execute tool calls and collect results
            let mut tool_results = Vec::new();
            for call in &tool_calls {
                let result = execute_tool(call, &self.workspace_dir).await;
                tool_results.push(result);
            }

            // Format tool results for the next iteration
            let tool_results_str = tool_results.iter()
                .map(|r| {
                    if r.success {
                        format!("[{}]\n{}", r.name, r.output)
                    } else {
                        format!("[{} Error]\n{}", r.name, r.error.as_ref().unwrap_or(&"Unknown error".to_string()))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n---\n");

            current_history.push_str(&format!(
                "\n\n[Tool Results]\n{}\n\n[End Results]",
                tool_results_str
            ));

            // For display, summarize what was done
            if !final_text.is_empty() {
                final_text.push_str("\n");
            }
            let summaries: Vec<String> = tool_results.iter().map(|r| {
                if r.success {
                    format!("{}: done", r.name)
                } else {
                    format!("{}: failed", r.name)
                }
            }).collect();
            final_text.push_str(&summaries.join(", "));
        }

        Ok((final_text.clone(), final_text))
    }

    /// Simple response without tool execution
    #[allow(dead_code)]
    pub async fn get_response(&self, history: &str) -> Result<(String, String), anyhow::Error> {
        let prompt = format!(
            "current conversation history:\n{}\n\nrespond as the {}. provide your actual response, actions taken, or findings - not just a plan of what you will do.",
            history, self.name
        );

        self.provider.call_model(
            &self.model_name,
            &prompt,
            Some(&self.build_full_system_prompt()),
            self.temperature,
            self.max_tokens,
        ).await
    }

    pub fn clone_for_parallel(&self) -> Self {
        Self {
            name: self.name.clone(),
            role: self.role.clone(),
            system_prompt: self.system_prompt.clone(),
            provider: self.provider.clone(),
            model_name: self.model_name.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            is_autonomous: self.is_autonomous,
            workspace_dir: self.workspace_dir.clone(),
            max_tool_iterations: self.max_tool_iterations,
        }
    }
}

/// Extract text content before any tool call block
#[allow(dead_code)]
fn extract_text_before_tools(response: &str) -> String {
    // Find first tool call marker
    let markers = ["```json", "<tool_call>", "<function>", "read_file(", "write_file(", "run_shell_command(", "list_directory(", "grep("];

    let mut earliest = usize::MAX;
    for marker in &markers {
        if let Some(pos) = response.find(marker) {
            if pos < earliest {
                earliest = pos;
            }
        }
    }

    if earliest == usize::MAX {
        response.trim().to_string()
    } else {
        response[..earliest].trim().to_string()
    }
}
