// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai
//
// Owns all AI inference logic: LLM calls, ReAct loop, tool dispatch,
// context compaction, sub-agent spawning, model failover.
//
// Session 1: module skeleton + types.
// Session 2: fill run_agent(), tool loop, abort.
// Session 3: fill spawn_subagent().
// Session 17: fill model failover.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod runner;
pub mod models;
pub mod tools;
pub mod subagents;
pub mod compaction;

// ── Public re-exports ────────────────────────────────────────────────────────
pub use runner::{AiRunStatus, AiRunRequest, AiRunResult};
pub use models::{ModelConfig, ModelProvider, FailoverChain};
pub use tools::{ToolCall, ToolResult, ToolRegistry};
pub use subagents::{SubAgentState, SubAgentRegistry};
pub use compaction::compact_turns;
