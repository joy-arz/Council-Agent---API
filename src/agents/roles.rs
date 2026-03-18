use std::sync::Arc;
use crate::agents::base::base_agent;
use crate::core::model_provider;

pub fn strategist(provider: Arc<dyn model_provider>, model: &str, temp: f32, tokens: u32) -> base_agent {
    base_agent::new(
        "architect",
        "lead architect & workflow lead",
        "your absolute primary directive is to fulfill the user's exact request immediately and precisely. talk through your high-level technical design and workflow casually, then immediately implement the foundational files and core logic using your tools.",
        provider,
        model,
        temp,
        tokens,
    )
}

pub fn critic(provider: Arc<dyn model_provider>, model: &str, temp: f32, tokens: u32) -> base_agent {
    base_agent::new(
        "reviewer",
        "security & QA specialist",
        "review the architect's changes conversationally. point out any bugs, safety issues, or edge cases like a peer would in a code review. suggest fixes or use your tools to correct them to ensure the implementation is robust.",
        provider,
        model,
        temp,
        tokens,
    )
}

pub fn optimizer(provider: Arc<dyn model_provider>, model: &str, temp: f32, tokens: u32) -> base_agent {
    base_agent::new(
        "refactorer",
        "performance & refactoring engineer",
        "chat about how to optimize the code for performance and readability. refactor any redundant logic and apply finishing touches to the workspace files to make them production ready.",
        provider,
        model,
        temp,
        tokens,
    )
}

pub fn contrarian(provider: Arc<dyn model_provider>, model: &str, temp: f32, tokens: u32) -> base_agent {
    base_agent::new(
        "maintainer",
        "maintenance & technical debt specialist",
        "discuss the long-term maintainability of the current approach. warn the team casually about any architectural debt and suggest simplifications to ensure the solution scales well.",
        provider,
        model,
        temp,
        tokens,
    )
}
