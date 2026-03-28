use std::sync::Arc;
use crate::agents::base::base_agent;
use crate::core::model_provider;

#[allow(non_camel_case_types)]
pub struct judge_agent {
    pub base: base_agent,
}

#[allow(non_camel_case_types)]
impl judge_agent {
    pub fn new(provider: Arc<dyn model_provider>, model: &str, temp: f32, tokens: u32) -> Self {
        let base = base_agent::new(
            "lead engineer",
            "chief judge & lead engineer",
            "review the workflow progress conversationally. determine if the task is finished, needs to continue, or should be paused. synthesize the final engineering verdict casually but firmly.",
            provider,
            model,
            temp,
            tokens,
        );
        Self { base }
    }

    pub async fn get_final_verdict(&self, history: &str) -> Result<(String, String), anyhow::Error> {
        let prompt = format!(
            "review the conversation and the workflow progress.\n\ncurrent history:\n{}\n\nbe critical but conversational in your reasoning. if the implementation is incomplete, buggy, or lacks proper structure, mark it as CONTINUE. provide a final decision in JSON format with the following keys:\n- \"summary\": a casual brief overview of the work done.\n- \"best_answer\": the most effective solution provided.\n- \"key_disagreements\": a list of points of contention.\n- \"final_decision\": choose one of [\"FINISHED\", \"CONTINUE\", \"PAUSED\"].\n- \"reasoning\": a paragraph explaining why you made this decision, written naturally like a human lead engineer talking to their team.",
            history
        );

        self.base.provider.call_model(
            &self.base.model_name,
            &prompt,
            Some(&self.base.build_full_system_prompt()),
            self.base.temperature,
            self.base.max_tokens,
            None,  // Judge doesn't use tools
        ).await
    }
}
