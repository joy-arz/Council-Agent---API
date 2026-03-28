use std::sync::Arc;
use std::path::PathBuf;
use crate::core::model_provider;
use crate::core::providers_mod::StreamChunk;
use crate::core::tools::{execute_tool, parse_tool_calls};

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct agent_result {
    pub response: String,
    pub tool_calls: Vec<tool_call_result>,
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub struct tool_call_result {
    pub name: String,
    pub status: String,  // "running", "success", "error"
    pub output: Option<String>,
}

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

        // Include actual tool definitions in JSON format
        let tools_json = crate::core::tools::get_tools_json();

        let tools_instruction = format!(r#"AVAILABLE TOOLS (MUST use JSON format when calling tools):
{}

RESPONSE RULES (CRITICAL):
1. Keep responses SHORT - maximum 2-3 sentences
2. If using a tool, output ONLY the JSON object and nothing else
3. Do NOT explain what you are about to do or what you did - just do it
4. NEVER start with "Based on...", "Looking at...", "The user wants me to..." - just answer directly
5. Output JSON in this exact format: {{"name": "tool_name", "arguments": {{"param": "value"}}}}

Examples of GOOD responses:
- "The project has Cargo.toml, src/, and tests/."
- {{"name": "list_directory", "arguments": {{"path": "."}}}}
- "There's a bug in line 42 - missing null check."

Examples of BAD responses (do not do these):
- "Based on my analysis of the workspace, I can see that..."
- "The user wants me to list files, so let me do that by calling..."
- "Looking at the previous tool results, I notice that...""#, tools_json);

        format!(
            "you are a {}.\n{}\n\nresponsibilities:\n{}\n\n{}",
            self.role, safety_instruction, self.system_prompt, tools_instruction
        )
    }

    /// Execute a response with tool calls, continuing until no more tool calls or max iterations reached
    pub async fn get_response_with_tools(&self, history: &str) -> Result<agent_result, anyhow::Error> {
        let mut current_history = history.to_string();
        let mut iterations = 0;
        let mut final_text = String::new();
        let mut all_tool_calls: Vec<tool_call_result> = Vec::new();

        // Get tools JSON for API providers
        let tools_json = crate::core::tools::get_tools_json();

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

            // Use streaming API to get real-time tool events
            let mut rx = self.provider.call_model_streaming(
                &self.model_name,
                &prompt,
                Some(&self.build_full_system_prompt()),
                self.temperature,
                self.max_tokens,
                Some(&tools_json),
            ).await?;

            // Streaming state
            let mut current_tool_input = String::new();
            let mut text_buffer = String::new();
            let mut tool_calls_from_stream: Vec<(String, String)> = Vec::new(); // (name, input)

            // Process stream
            while let Some(chunk) = rx.recv().await {
                match chunk {
                    StreamChunk::TextDelta(text) => {
                        text_buffer.push_str(&text);
                    }
                    StreamChunk::ToolStart { name: _, .. } => {
                        // Tool started - we'll capture the name in ToolEnd
                        current_tool_input.clear();
                    }
                    StreamChunk::ToolInputDelta(delta) => {
                        current_tool_input.push_str(&delta);
                    }
                    StreamChunk::ToolEnd { name, input, .. } => {
                        let final_input = if input.is_empty() { current_tool_input.clone() } else { input };
                        tool_calls_from_stream.push((name, final_input));
                        current_tool_input.clear();
                    }
                    StreamChunk::Usage { .. } => {}
                    StreamChunk::Done => break,
                    StreamChunk::Error(e) => {
                        tracing::warn!("Streaming error: {}", e);
                        break;
                    }
                }
            }

            // Add accumulated text to final text
            let trimmed_text = text_buffer.trim();
            if !trimmed_text.is_empty() {
                if !final_text.is_empty() {
                    final_text.push('\n');
                }
                final_text.push_str(trimmed_text);
            }

            // Execute tool calls found in stream
            if tool_calls_from_stream.is_empty() {
                // No tool calls - this is the final response
                break;
            }

            // Execute each tool and collect results
            let mut tool_results = Vec::new();
            for (name, input) in &tool_calls_from_stream {
                let args: serde_json::Map<String, serde_json::Value> = if input.is_empty() {
                    serde_json::Map::new()
                } else {
                    serde_json::from_str(input).unwrap_or_else(|_| serde_json::Map::new())
                };
                
                let call = crate::core::tools::ToolCall {
                    name: name.clone(),
                    arguments: serde_json::Value::Object(args),
                };
                
                let result = execute_tool(&call, &self.workspace_dir).await;
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

            // Build tool info for display
            for (i, result) in tool_results.iter().enumerate() {
                all_tool_calls.push(tool_call_result {
                    name: tool_calls_from_stream[i].0.clone(),
                    status: if result.success { "success".to_string() } else { "error".to_string() },
                    output: Some(if result.success { result.output.clone() } else { result.error.clone().unwrap_or_default() }),
                });
            }

            // For display, summarize what was done
            if !final_text.is_empty() {
                final_text.push('\n');
            }
            let summaries: Vec<String> = tool_results.iter().map(|r| {
                if r.success {
                    format!("✓ {}", r.name)
                } else {
                    format!("✗ {}", r.name)
                }
            }).collect();
            final_text.push_str(&summaries.join(" "));
        }

        Ok(agent_result {
            response: final_text,
            tool_calls: all_tool_calls,
        })
    }

    /// Execute a response with tool calls using streaming API
    /// Tool calls are executed as they arrive from the stream
    pub async fn get_response_with_tools_streaming<F>(
        &self,
        history: &str,
        mut on_chunk: F,
    ) -> Result<agent_result, anyhow::Error>
    where
        F: FnMut(String) + Send,
    {
        let mut current_history = history.to_string();
        let mut iterations = 0;
        let mut final_text = String::new();
        let mut all_tool_calls: Vec<tool_call_result> = Vec::new();

        let tools_json = crate::core::tools::get_tools_json();

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

            let mut rx = self.provider.call_model_streaming(
                &self.model_name,
                &prompt,
                Some(&self.build_full_system_prompt()),
                self.temperature,
                self.max_tokens,
                Some(&tools_json),
            ).await?;

            // Streaming state
            let mut _current_tool_id: Option<String> = None;
            let mut _current_tool_name: Option<String> = None;
            let mut current_tool_input: String = String::new();
            let mut text_buffer = String::new();
            let mut _tool_started = false;

            // Process stream
            while let Some(chunk) = rx.recv().await {
                match chunk {
                    StreamChunk::TextDelta(text) => {
                        text_buffer.push_str(&text);
                        on_chunk(text);
                    }
                    StreamChunk::ToolStart { id, name } => {
                        _current_tool_id = Some(id);
                        _current_tool_name = Some(name);
                        _tool_started = true;
                    }
                    StreamChunk::ToolInputDelta(delta) => {
                        current_tool_input.push_str(&delta);
                    }
                    StreamChunk::ToolEnd { id: _, name, input } => {
                        // Execute the tool immediately
                        let tool_input = if input.is_empty() { current_tool_input.clone() } else { input };
                        
                        let call = crate::core::tools::ToolCall {
                            name: name.clone(),
                            arguments: if tool_input.is_empty() {
                                serde_json::Value::Object(serde_json::Map::new())
                            } else {
                                serde_json::from_str(&tool_input).unwrap_or(serde_json::Value::Null)
                            },
                        };
                        
                        let result = execute_tool(&call, &self.workspace_dir).await;
                        
                        all_tool_calls.push(tool_call_result {
                            name: name.clone(),
                            status: if result.success { "success".to_string() } else { "error".to_string() },
                            output: Some(if result.success { result.output.clone() } else { result.error.clone().unwrap_or_default() }),
                        });

                        // Add result to history for next iteration
                        let tool_result_text = if result.success { result.output.clone() } else { result.error.clone().unwrap_or_else(|| "Unknown error".to_string()) };
                        current_history.push_str(&format!(
                            "\n\n[Tool: {}]\n{}\n[End Tool]",
                            name,
                            tool_result_text
                        ));

                        // Clear streaming state
                        _current_tool_id = None;
                        _current_tool_name = None;
                        current_tool_input.clear();
                        _tool_started = false;
                    }
                    StreamChunk::Usage { .. } => {
                        // Usage stats - could be logged
                    }
                    StreamChunk::Done => {
                        break;
                    }
                    StreamChunk::Error(e) => {
                        tracing::error!("Streaming error: {}", e);
                        break;
                    }
                }
            }

            // After stream ends, check if we have tool calls to process
            if all_tool_calls.len() == iterations - 1 {
                // No new tool calls this iteration - this is the final response
                let cleaned = text_buffer.trim().to_string();
                if !cleaned.is_empty() {
                    if !final_text.is_empty() {
                        final_text.push('\n');
                        final_text.push_str(&cleaned);
                    } else {
                        final_text = cleaned;
                    }
                }
                break;
            } else if iterations > all_tool_calls.len() {
                // We had tool calls but they weren't completed
                break;
            }
        }

        Ok(agent_result {
            response: final_text,
            tool_calls: all_tool_calls,
        })
    }

    /// Simple response without tool execution
    #[allow(dead_code)]
    pub async fn get_response(&self, history: &str) -> Result<agent_result, anyhow::Error> {
        let prompt = format!(
            "current conversation history:\n{}\n\nrespond as the {}. provide your actual response, actions taken, or findings - not just a plan of what you will do.",
            history, self.name
        );

        let (response, _) = self.provider.call_model(
            &self.model_name,
            &prompt,
            Some(&self.build_full_system_prompt()),
            self.temperature,
            self.max_tokens,
            None,  // No tools for simple responses
        ).await?;

        Ok(agent_result {
            response,
            tool_calls: vec![],
        })
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
