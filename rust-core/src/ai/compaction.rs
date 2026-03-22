// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: compaction
//
// Context window compaction — trim old turns, summarise with LLM.
// Mirrors OpenClaw: src/agents/compaction.ts
//
// Session 1: types + basic trim.
// Session 4: full summary-based compaction wired into SessionStore.
//   - compact_session(): collect dropped turns, call LLM, store summary
//   - estimate_tokens(): rough 4-chars-per-token heuristic
//   - COMPACTION_PROMPT: prompt for generating the summary
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::gateway::sessions::SessionStore;

// ── Config ────────────────────────────────────────────────────────────────────

pub struct CompactionConfig {
    pub context_limit_tokens: u64,  // model's max context window
    pub compact_at_fraction:  f32,  // trigger at this fraction (e.g. 0.85)
    pub min_turns_to_keep:    usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        CompactionConfig {
            context_limit_tokens: 128_000,
            compact_at_fraction:  0.85,
            min_turns_to_keep:    12,
        }
    }
}

impl CompactionConfig {
    pub fn token_threshold(&self) -> u64 {
        (self.context_limit_tokens as f32 * self.compact_at_fraction) as u64
    }
}

// ── Token estimation ──────────────────────────────────────────────────────────

/// Rough token count: 4 chars ≈ 1 token (works well for English + code).
pub fn estimate_tokens_str(s: &str) -> u32 {
    ((s.len() as u32) / 4).max(1)
}

pub fn estimate_tokens_turns(turns: &[(String, String)]) -> u64 {
    turns.iter().map(|(_, c)| estimate_tokens_str(c) as u64).sum()
}

// ── Compaction prompt ─────────────────────────────────────────────────────────

const COMPACTION_PROMPT: &str = "\
You are summarising a conversation history to save context space. \
Write a concise but complete summary of the key facts, decisions, tool results, \
and user preferences established so far. Include: \
(1) what the user asked for, \
(2) what was accomplished, \
(3) any important facts or values discovered, \
(4) ongoing tasks or open questions. \
Be specific. Use plain prose, 3-6 sentences maximum. \
Do NOT start with 'The conversation' — start with the content directly.";

// ── Main compaction entry point ───────────────────────────────────────────────

/// Compact a session: trim old turns, call LLM to summarise them, store result.
///
/// Returns `Ok(summary)` on success, `Err(reason)` if nothing needed doing or LLM failed.
///
/// Caller is responsible for:
///   1. Calling `store.save_transcript(session_id)` after success.
///   2. Calling `store.save_index()` after success.
pub fn compact_session(
    store:      &mut SessionStore,
    session_id: &str,
    api_key:    &str,
    base_url:   &str,
    model:      &str,
    force:      bool,
) -> Result<String, String> {
    // Check threshold unless forced
    if !force && !store.needs_compact(session_id) {
        return Err("below threshold".to_string());
    }

    let turn_count = store.turn_count(session_id);
    if turn_count <= store.min_turns_keep {
        return Err(format!("only {} turns, nothing to compact", turn_count));
    }

    // Collect the turns that will be dropped
    let dropped = store.compact_collect_dropped(session_id);
    if dropped.is_empty() {
        return Err("no turns dropped".to_string());
    }

    // Build a mini-transcript for the LLM to summarise
    let transcript_text: String = dropped.iter()
        .map(|(role, content)| {
            let label = if role == "assistant" { "Kira" } else { "User" };
            format!("{}: {}", label, content)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let user_msg = format!(
        "Please summarise this conversation segment:\n\n{}", transcript_text
    );

    // Call LLM — use the same infrastructure already in lib.rs
    let summary = call_llm_for_summary(api_key, base_url, model, &user_msg)?;

    // Store the summary back into the session
    store.apply_compact_summary(session_id, &summary);

    Ok(summary)
}

/// Minimal LLM call for summarisation — plain POST, no tool loop, no history.
fn call_llm_for_summary(
    api_key:  &str,
    base_url: &str,
    model:    &str,
    user_msg: &str,
) -> Result<String, String> {
    use crate::call_llm_sync;

    let history = vec![("user".to_string(), user_msg.to_string())];
    let raw = call_llm_sync(api_key, base_url, model, COMPACTION_PROMPT, &history)?;
    Ok(extract_content_text(&raw))
}

/// Pull the text content from an OpenAI-format response JSON.
fn extract_content_text(raw: &str) -> String {
    // Try choices[0].message.content
    if let Some(idx) = raw.find("\"content\":") {
        let rest = raw[idx + 10..].trim_start();
        if rest.starts_with('"') {
            let inner = &rest[1..];
            let mut out = String::new();
            let mut escape = false;
            for c in inner.chars() {
                if escape {
                    match c {
                        '"'  => out.push('"'),
                        'n'  => out.push('\n'),
                        '\\' => out.push('\\'),
                        _    => { out.push('\\'); out.push(c); }
                    }
                    escape = false;
                } else if c == '\\' {
                    escape = true;
                } else if c == '"' {
                    break;
                } else {
                    out.push(c);
                }
            }
            if !out.is_empty() { return out; }
        }
    }
    // Fallback: return raw trimmed
    raw.trim().to_string()
}

// ── Legacy helper (used by older /ai/agent + /ai/chain paths) ─────────────────

/// Simple turn struct for callers that don't use SessionStore yet.
#[derive(Clone, Debug)]
pub struct Turn {
    pub role:    String,
    pub content: String,
}

/// Trim turns to fit within a token budget. Returns (kept, dropped_count).
/// Kept always includes system turns first, then most-recent non-system.
pub fn compact_turns(turns: &[Turn], config: &CompactionConfig) -> (Vec<Turn>, usize) {
    let budget = config.token_threshold();
    let total: u64 = turns.iter().map(|t| estimate_tokens_str(&t.content) as u64).sum();

    if total <= budget { return (turns.to_vec(), 0); }

    let (sys, non_sys): (Vec<&Turn>, Vec<&Turn>) =
        turns.iter().partition(|t| t.role == "system");

    let sys_tokens: u64 = sys.iter()
        .map(|t| estimate_tokens_str(&t.content) as u64)
        .sum();
    let available = budget.saturating_sub(sys_tokens);

    let mut kept: Vec<&Turn> = Vec::new();
    let mut used: u64 = 0;
    for turn in non_sys.iter().rev() {
        let t = estimate_tokens_str(&turn.content) as u64;
        if used + t > available && kept.len() >= config.min_turns_to_keep { break; }
        kept.push(turn);
        used += t;
    }
    kept.reverse();

    let dropped = non_sys.len().saturating_sub(kept.len());
    let mut result: Vec<Turn> = sys.into_iter().cloned().collect();
    result.extend(kept.into_iter().cloned());
    (result, dropped)
}

pub fn needs_compaction(turns: &[Turn], config: &CompactionConfig) -> bool {
    let total: u64 = turns.iter().map(|t| estimate_tokens_str(&t.content) as u64).sum();
    total > config.token_threshold()
}

