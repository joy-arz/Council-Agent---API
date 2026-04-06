//! Constants used throughout the application
//! All magic numbers should be defined here

/// Maximum tool arguments size in bytes (5MB - allows writing reasonable file sizes)
pub const MAX_TOOL_ARGUMENT_SIZE: usize = 5_000_000;

/// Maximum estimated context tokens before triggering compaction warning
/// ~150K tokens fits within most LLM context windows (32K-200K)
pub const MAX_CONTEXT_TOKENS: usize = 150_000;

/// Maximum context history messages before triggering summarization warning
pub const MAX_CONTEXT_MESSAGES: usize = 30;

/// Maximum summaries to keep in memory
pub const MAX_SUMMARIES: usize = 10;

/// Maximum pinned messages to prevent memory leak
pub const MAX_PINNED_MESSAGES: usize = 20;

/// Maximum messages in sliding window
pub const MAX_MESSAGES: usize = 50;

/// Default max rounds for deliberation
pub const DEFAULT_MAX_ROUNDS: usize = 7;

/// Minimum rounds before judge auto-decision
pub const MIN_ROUNDS_BEFORE_JUDGE: usize = 3;

/// Tool iterations per agent response
pub const MAX_TOOL_ITERATIONS: usize = 5;

/// Infinite loop detection threshold
pub const INFINITE_LOOP_THRESHOLD: usize = 3;

/// Rate limiting: requests per second
pub const RATE_LIMIT_REFILL_RATE: f64 = 10.0;

/// Rate limiting: burst size
pub const RATE_LIMIT_MAX_TOKENS: usize = 50;

/// Graceful shutdown timeout in seconds
pub const SHUTDOWN_TIMEOUT_SECS: u64 = 30;

/// SSE channel buffer size
pub const SSE_CHANNEL_BUFFER: usize = 100;

/// Agent response timeout in seconds
pub const AGENT_TIMEOUT_SECS: u64 = 120;

/// Shell command timeout in seconds
pub const SHELL_TIMEOUT_SECS: u64 = 30;
