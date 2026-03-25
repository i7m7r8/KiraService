// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: routing
//
// Route inbound messages to the correct agent + session.
// Mirrors OpenClaw: src/routing/resolve-route.ts
//
// Session 1: types.  Session 16: full multi-agent routing.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The resolved key for a session (channel + peer identifier)
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RouteKey(pub String);

impl RouteKey {
    pub fn from_parts(channel: &str, chat_id: &str) -> Self {
        RouteKey(format!("{}:{}", channel, chat_id))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Agent configuration  -  which model/persona/skills to use for a session
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub id:          String,
    pub name:        String,
    pub persona:     String,
    pub model:       Option<String>,   // None = use global default
    pub skill_ids:   Vec<String>,
    pub memory_scope: String,          // "global" | session_id
    pub channels:    Vec<String>,      // which channels this agent handles
    pub enabled:     bool,
}

impl AgentConfig {
    pub fn default_agent() -> Self {
        AgentConfig {
            id:           "default".to_string(),
            name:         "Kira".to_string(),
            persona:      "You are Kira, a powerful Android AI agent.".to_string(),
            model:        None,
            skill_ids:    vec![],
            memory_scope: "global".to_string(),
            channels:     vec!["*".to_string()],
            enabled:      true,
        }
    }

    pub fn to_json(&self) -> String {
        let _skills_json: Vec<String> = self.skill_ids.iter()
            .map(|s| format!("\"{}\"", s))
            .collect();
        format!(
            r#"{{"id":"{}","name":"{}","model":{},"memory_scope":"{}","enabled":{}}}"#,
            self.id, self.name,
            self.model.as_deref()
                .map(|m| format!("\"{}\"", m))
                .unwrap_or_else(|| "null".to_string()),
            self.memory_scope, self.enabled
        )
    }
}

/// Route a message to the correct agent config.
/// Returns the agent_id to use.
pub fn resolve_agent<'a>(
    agents: &'a [AgentConfig],
    channel: &str,
    _sender: &str,
) -> &'a AgentConfig {
    // Find first enabled agent that handles this channel
    agents.iter().find(|a| {
        a.enabled && (
            a.channels.contains(&"*".to_string()) ||
            a.channels.iter().any(|c| c == channel)
        )
    }).unwrap_or_else(|| {
        // Fallback: return first enabled agent
        agents.iter().find(|a| a.enabled)
            .expect("No enabled agents configured")
    })
}

// ── S3: ACP HTTP Route Handlers ───────────────────────────────────────────────
// These functions are called from lib.rs route_http() for /sessions/* endpoints.
// Mirrors OpenClaw: src/gateway/server-methods/

use crate::acp::SessionPatch;

/// Handle GET /sessions/list  -  return all sessions as JSON array.
/// Mirrors OpenClaw: sessions.list method.
pub fn handle_sessions_list(
    store: &crate::gateway::sessions::SessionStore,
    _params: &str,
) -> String {
    store.list_sessions_json()
}

/// Handle GET /sessions/get?key=<session_id>  -  return one session.
pub fn handle_sessions_get(
    store: &crate::gateway::sessions::SessionStore,
    params: &str,
) -> String {
    let key = extract_param(params, "key").unwrap_or_default();
    if key.is_empty() {
        return r#"{"error":"missing key param"}"#.to_string();
    }
    store.get_json(&key)
}

/// Handle POST /sessions/patch  -  apply a SessionPatch to a session.
/// Body: {"key":"session_id", "model":"gpt-4o", "thinking_level":"high", ...}
/// Mirrors OpenClaw: sessions.patch method (src/gateway/server-methods/sessions-patch.ts).
pub fn handle_sessions_patch(
    store: &mut crate::gateway::sessions::SessionStore,
    body: &str,
) -> String {
    // Extract the mandatory key field
    let key = match extract_json_str(body, "key") {
        Some(k) if !k.is_empty() => k,
        _ => return r#"{"error":"missing key field"}"#.to_string(),
    };

    // Ensure session exists (create a stub if not)
    let now_ms = crate::utils::now_ms() as u128;
    store.get_or_create(&key, "api", now_ms);

    // Parse patch fields from body
    let patch = SessionPatch::from_json(body);

    // Apply and persist
    store.patch_session(&key, &patch)
}

/// Handle POST /sessions/reset  -  clear session transcript.
/// Body: {"key":"session_id", "reason":"new"|"reset"}
pub fn handle_sessions_reset(
    store: &mut crate::gateway::sessions::SessionStore,
    body: &str,
) -> String {
    let key = match extract_json_str(body, "key") {
        Some(k) if !k.is_empty() => k,
        _ => return r#"{"error":"missing key field"}"#.to_string(),
    };
    let now_ms = crate::utils::now_ms() as u128;
    let ok = store.reset_session(&key, now_ms);
    format!(r#"{{"ok":{},"key":"{}"}}"#, ok, esc(&key))
}

/// Handle DELETE /sessions/delete  -  delete session + transcript from disk.
/// Body: {"key":"session_id"}
pub fn handle_sessions_delete(
    store: &mut crate::gateway::sessions::SessionStore,
    bus: &mut crate::acp::AcpBus,
    body: &str,
) -> String {
    let key = match extract_json_str(body, "key") {
        Some(k) if !k.is_empty() => k,
        _ => return r#"{"error":"missing key field"}"#.to_string(),
    };
    store.delete_and_purge(&key);
    bus.purge(&key);
    format!(r#"{{"ok":true,"key":"{}"}}"#, esc(&key))
}

/// Handle POST /sessions/compact  -  trigger immediate compaction of a session.
/// Body: {"key":"session_id"}
/// The actual LLM summarisation happens inside compact_session_now().
pub fn handle_sessions_compact(
    store: &mut crate::gateway::sessions::SessionStore,
    body: &str,
) -> String {
    let key = match extract_json_str(body, "key") {
        Some(k) if !k.is_empty() => k,
        _ => return r#"{"error":"missing key field"}"#.to_string(),
    };
    let dropped = store.compact_collect_dropped(&key);
    if dropped.is_empty() {
        return format!(r#"{{"ok":true,"key":"{}","dropped":0,"note":"nothing to compact"}}"#, esc(&key));
    }
    // Build a minimal summary from dropped turns (real LLM summary done async)
    let summary = build_inline_summary(&dropped);
    store.apply_compact_summary(&key, &summary);
    store.save_index();
    format!(r#"{{"ok":true,"key":"{}","dropped":{},"summary_chars":{}}}"#,
        esc(&key), dropped.len(), summary.len())
}

/// Handle POST /sessions/chat  -  push a user message into a session's run queue.
/// Body: {"key":"session_id", "content":"hello", "attachments":[]}
/// Mirrors OpenClaw: sessions.chat method (gateway/server-methods/chat.ts).
/// The actual run is done by the existing run_agent() infrastructure.
pub fn handle_sessions_chat(
    store: &mut crate::gateway::sessions::SessionStore,
    bus: &mut crate::acp::AcpBus,
    body: &str,
) -> String {
    let key = match extract_json_str(body, "key") {
        Some(k) if !k.is_empty() => k,
        _ => return r#"{"error":"missing key field"}"#.to_string(),
    };
    let content = extract_json_str(body, "content").unwrap_or_default();
    if content.is_empty() {
        return r#"{"error":"empty content"}"#.to_string();
    }

    let now_ms = crate::utils::now_ms() as u128;
    store.get_or_create(&key, "api", now_ms);
    store.add_turn(&key, "user", &content, now_ms);

    // Emit TextDelta event so Java/UI sees the input reflected back
    bus.emit(crate::acp::AcpEvent::TextDelta {
        session:   key.clone(),
        delta:     content.clone(),
        block_idx: 0,
    });

    // Queue the run request. The HTTP server loop picks this up
    // via /ai/run (existing endpoint) or the channel polling loop.
    // We store a pending run marker in the ACP bus as a Command event.
    bus.emit(crate::acp::AcpEvent::Error {
        session: key.clone(),
        code:    crate::acp::ErrorCode::Processing,
        message: format!("run_queued:{}", esc(&content)),
    });

    format!(r#"{{"ok":true,"key":"{}","queued":true}}"#, esc(&key))
}

// ── S3: Internal helpers ──────────────────────────────────────────────────────

/// Extract a URL query param value: key=value from a query string.
pub fn extract_param(params: &str, key: &str) -> Option<String> {
    let needle = format!("{}=", key);
    let start  = params.find(&needle)? + needle.len();
    let rest   = &params[start..];
    let end    = rest.find('&').unwrap_or(rest.len());
    let raw    = &rest[..end];
    // Basic URL-decode: replace %20 with space etc.
    Some(raw.replace("%20", " ").replace("%2F", "/").replace("%3A", ":"))
}

/// Extract a JSON string value from a flat JSON body (single-level).
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    if !rest.starts_with('"') { return None; }
    let inner = &rest[1..];
    let mut out   = String::new();
    let mut chars = inner.chars().peekable();
    loop {
        match chars.next()? {
            '\\' => match chars.next()? {
                '"'  => out.push('"'),
                'n'  => out.push('\n'),
                't'  => out.push('\t'),
                '\\' => out.push('\\'),
                c    => { out.push('\\'); out.push(c); }
            },
            '"'  => break,
            c    => out.push(c),
        }
    }
    Some(out)
}

/// Build a quick inline summary from dropped turns (no LLM call).
/// For a real LLM summary, the runner's compact_session() is called instead.
fn build_inline_summary(turns: &[(String, String)]) -> String {
    let mut out = String::from("[Compacted context  -  key points:]\n");
    for (role, content) in turns.iter().take(20) {
        let preview = if content.len() > 120 { &content[..120] } else { content };
        out.push_str(&format!("{}: {}\n", role, preview));
    }
    if turns.len() > 20 {
        out.push_str(&format!("... ({} more turns omitted)\n", turns.len() - 20));
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
