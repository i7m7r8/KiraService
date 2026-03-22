// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: compaction
//
// Context window compaction — trim old turns when approaching token limit.
// Mirrors OpenClaw: src/agents/compaction.ts
//
// Strategy: keep system prompt + last N turns + summarize dropped turns.
// Session 1: types + basic trim.  Session 4: full summary-based compaction.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::runner::Turn;

pub struct CompactionConfig {
    pub context_limit_tokens:  u32,  // model's max context window
    pub compact_at_fraction:   f32,  // trigger at this fraction (e.g. 0.85)
    pub min_turns_to_keep:     usize, // always keep last N turns regardless
}

impl Default for CompactionConfig {
    fn default() -> Self {
        CompactionConfig {
            context_limit_tokens: 128_000,
            compact_at_fraction:  0.85,
            min_turns_to_keep:    10,
        }
    }
}

/// Estimate token count for a slice of turns (rough: 4 chars per token)
pub fn estimate_tokens(turns: &[Turn]) -> u32 {
    turns.iter().map(|t| (t.content.len() as u32 / 4).max(1)).sum()
}

/// Trim turns to fit within token budget.
/// Returns (kept_turns, dropped_count).
/// Keeps: system prompt turn (role=system) always, then as many recent turns as fit.
pub fn compact_turns(
    turns: &[Turn],
    config: &CompactionConfig,
) -> (Vec<Turn>, usize) {
    let budget = (config.context_limit_tokens as f32 * config.compact_at_fraction) as u32;
    let total_tokens = estimate_tokens(turns);

    if total_tokens <= budget {
        return (turns.to_vec(), 0);
    }

    // Always keep system turns
    let (system_turns, non_system): (Vec<&Turn>, Vec<&Turn>) =
        turns.iter().partition(|t| t.role == "system");

    // Take from the end (most recent first) until we hit budget
    let system_tokens: u32 = system_turns.iter()
        .map(|t| (t.content.len() as u32 / 4).max(1))
        .sum();
    let available = budget.saturating_sub(system_tokens);

    let mut kept_non_system: Vec<&Turn> = Vec::new();
    let mut used_tokens: u32 = 0;

    for turn in non_system.iter().rev() {
        let t_tokens = (turn.content.len() as u32 / 4).max(1);
        if used_tokens + t_tokens > available && kept_non_system.len() >= config.min_turns_to_keep {
            break;
        }
        kept_non_system.push(turn);
        used_tokens += t_tokens;
    }
    kept_non_system.reverse();

    let dropped = non_system.len().saturating_sub(kept_non_system.len());

    let mut result: Vec<Turn> = system_turns.into_iter().cloned().collect();
    result.extend(kept_non_system.into_iter().cloned());

    (result, dropped)
}

/// Check if compaction is needed
pub fn needs_compaction(turns: &[Turn], config: &CompactionConfig) -> bool {
    let budget = (config.context_limit_tokens as f32 * config.compact_at_fraction) as u32;
    estimate_tokens(turns) > budget
}
