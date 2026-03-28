use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub struct message {
    pub agent: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
pub struct summary {
    pub content: String,
    pub round: usize,
}

#[derive(Debug, Clone, Default)]
#[allow(non_camel_case_types)]
pub struct shared_memory {
    pub original_query: String,
    pub pinned_messages: Vec<message>,
    pub messages: Vec<message>,
    pub summaries: Vec<summary>,
    pub max_messages: usize,
    pub max_pinned_messages: usize,
    pub max_summaries: usize,
}

#[allow(non_camel_case_types)]
impl shared_memory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            original_query: String::new(),
            pinned_messages: Vec::new(),
            messages: Vec::new(),
            summaries: Vec::new(),
            max_messages,
            max_pinned_messages: 20, // Limit pinned messages to prevent memory leak
            max_summaries: 10,       // Limit summaries to prevent memory leak
        }
    }

    pub fn set_original_query(&mut self, query: String) {
        self.original_query = query;
    }

    pub fn add_message(&mut self, agent: String, content: String, pinned: bool) {
        let msg = message { agent, content };
        if pinned {
            self.pinned_messages.push(msg);
            // Sliding window for pinned messages
            if self.pinned_messages.len() > self.max_pinned_messages {
                let overflow = self.pinned_messages.len() - self.max_pinned_messages;
                self.pinned_messages.drain(0..overflow);
            }
        } else {
            self.messages.push(msg);
            // sliding window logic for non-pinned messages
            if self.messages.len() > self.max_messages {
                let overflow = self.messages.len() - self.max_messages;
                self.messages.drain(0..overflow);
            }
        }
    }

    pub fn add_summary(&mut self, content: String, round: usize) {
        self.summaries.push(summary { content, round });
        // Sliding window for summaries
        if self.summaries.len() > self.max_summaries {
            let overflow = self.summaries.len() - self.max_summaries;
            self.summaries.drain(0..overflow);
        }
    }

    pub fn get_formatted_history(&self) -> String {
        let mut history = format!("user query: {}\n\n", self.original_query);

        history.push_str("--- summaries ---\n");
        for s in &self.summaries {
            history.push_str(&format!("[summary from round {}]: {}\n\n", s.round, s.content));
        }

        history.push_str("--- core context ---\n");
        for msg in &self.pinned_messages {
            history.push_str(&format!("[{}]: {}\n\n", msg.agent, msg.content));
        }

        history.push_str("--- recent debate ---\n");
        for msg in &self.messages {
            history.push_str(&format!("[{}]: {}\n\n", msg.agent, msg.content));
        }
        history
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.pinned_messages.clear();
        self.summaries.clear();
        self.original_query.clear();
    }
}
