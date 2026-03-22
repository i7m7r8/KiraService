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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai  (Session 2: runner fully implemented)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod runner;
pub mod models;
pub mod tools;
pub mod subagents;
pub mod compaction;

pub use runner::{
    AiRunStatus, AiRunRequest, AiRunResult,
    RunState, RunStatus, RUN_STATE,
    AgentRunConfig, run_agent,
    register_dispatch, register_llm_call,
    parse_tool_calls_json, build_messages_json,
    JsonToolCall,
};
pub use models::{ModelConfig, ModelProvider, FailoverChain};
pub use tools::{ToolCall, ToolResult, ToolRegistry};
pub use subagents::{
    SubAgentState, SubAgentRegistry, SubAgentPhase,
    SpawnRequest, SUBAGENT_REGISTRY,
    spawn_subagent, register_subagent_fns,
};
pub use compaction::compact_turns;
