// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: acp :: types
//
// ACP (Agent Control Protocol) message types.
// Mirrors OpenClaw: src/acp/types.ts, src/gateway/protocol/schema/
//
// Session 1: full typed wire protocol.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;

// ── Content blocks (mirrors OpenClaw ContentBlock union) ──────────────────────

#[derive(Clone, Debug)]
pub enum ContentBlock {
    Text    { text: String },
    Image   { url: String, media_type: String },
    File    { name: String, mime: String, data_b64: String },
    ToolUse { id: String, name: String, input: String },   // input = JSON string
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

impl ContentBlock {
    pub fn text(s: &str) -> Self { ContentBlock::Text { text: s.to_string() } }

    pub fn to_json(&self) -> String {
        match self {
            ContentBlock::Text { text } =>
                format!(r#"{{"type":"text","text":"{}"}}"#, esc(text)),
            ContentBlock::Image { url, media_type } =>
                format!(r#"{{"type":"image","url":"{}","media_type":"{}"}}"#, esc(url), esc(media_type)),
            ContentBlock::File { name, mime, data_b64 } =>
                format!(r#"{{"type":"file","name":"{}","mime":"{}","data":"{}"}}"#,
                    esc(name), esc(mime), esc(data_b64)),
            ContentBlock::ToolUse { id, name, input } =>
                format!(r#"{{"type":"tool_use","id":"{}","name":"{}","input":{}}}"#,
                    esc(id), esc(name), input),
            ContentBlock::ToolResult { tool_use_id, content, is_error } =>
                format!(r#"{{"type":"tool_result","tool_use_id":"{}","content":"{}","is_error":{}}}"#,
                    esc(tool_use_id), esc(content), is_error),
        }
    }
}

// ── Stop reason ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    Aborted,
    Error,
    Compacted,
}

impl StopReason {
    pub fn as_str(&self) -> &str {
        match self {
            StopReason::EndTurn   => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::ToolUse   => "tool_use",
            StopReason::Aborted   => "aborted",
            StopReason::Error     => "error",
            StopReason::Compacted => "compacted",
        }
    }
}

// ── Error codes ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum ErrorCode {
    NoApiKey,
    RateLimit,
    Overloaded,
    AuthFailed,
    NetworkError,
    ContextFull,
    InternalError,
    Aborted,
    ToolFailed,
}

impl ErrorCode {
    pub fn as_str(&self) -> &str {
        match self {
            ErrorCode::NoApiKey      => "no_api_key",
            ErrorCode::RateLimit     => "rate_limit",
            ErrorCode::Overloaded    => "overloaded",
            ErrorCode::AuthFailed    => "auth_failed",
            ErrorCode::NetworkError  => "network_error",
            ErrorCode::ContextFull   => "context_full",
            ErrorCode::InternalError => "internal_error",
            ErrorCode::Aborted       => "aborted",
            ErrorCode::ToolFailed    => "tool_failed",
        }
    }
    pub fn from_http_status(status: u16) -> Self {
        match status {
            401 | 403 => ErrorCode::AuthFailed,
            429       => ErrorCode::RateLimit,
            503       => ErrorCode::Overloaded,
            _         => ErrorCode::InternalError,
        }
    }
}

// ── Token usage ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct Usage {
    pub input_tokens:  u32,
    pub output_tokens: u32,
    pub cache_read:    u32,
    pub cache_write:   u32,
}

impl Usage {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"input":{},"output":{},"cache_read":{},"cache_write":{}}}"#,
            self.input_tokens, self.output_tokens, self.cache_read, self.cache_write
        )
    }
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

// ── ACP outbound messages (core → client) ─────────────────────────────────────
// Mirrors: src/acp/types.ts outbound union

#[derive(Clone, Debug)]
pub enum AcpEvent {
    /// Streaming text delta
    TextDelta {
        session:   String,
        delta:     String,
        block_idx: u32,
    },
    /// Thinking/reasoning delta (for extended thinking models)
    ThinkingDelta {
        session: String,
        delta:   String,
    },
    /// A tool call is starting
    ToolStart {
        session: String,
        id:      String,
        tool:    String,
        input:   String,   // JSON
    },
    /// A tool call completed
    ToolResult {
        session:  String,
        id:       String,
        tool:     String,
        output:   String,
        is_error: bool,
    },
    /// Agent turn completed
    Done {
        session:     String,
        stop_reason: StopReason,
        usage:       Usage,
    },
    /// An error occurred
    Error {
        session: String,
        code:    ErrorCode,
        message: String,
    },
    /// Context was compacted
    Compacted {
        session:       String,
        turns_removed: usize,
        summary_chars: usize,
    },
    /// Sub-agent status update
    AgentStatus {
        session:  String,
        agent_id: String,
        status:   String,   // "running" | "done" | "failed"
        result:   Option<String>,
    },
    /// System notification (e.g. tool approval needed)
    Notification {
        session: String,
        kind:    String,   // "approval_needed" | "info" | "warning"
        payload: String,   // JSON
    },
}

impl AcpEvent {
    pub fn to_json(&self) -> String {
        match self {
            AcpEvent::TextDelta { session, delta, block_idx } =>
                format!(r#"{{"type":"text_delta","session":"{}","delta":"{}","block_idx":{}}}"#,
                    esc(session), esc(delta), block_idx),

            AcpEvent::ThinkingDelta { session, delta } =>
                format!(r#"{{"type":"thinking_delta","session":"{}","delta":"{}"}}"#,
                    esc(session), esc(delta)),

            AcpEvent::ToolStart { session, id, tool, input } =>
                format!(r#"{{"type":"tool_start","session":"{}","id":"{}","tool":"{}","input":{}}}"#,
                    esc(session), esc(id), esc(tool), input),

            AcpEvent::ToolResult { session, id, tool, output, is_error } =>
                format!(r#"{{"type":"tool_result","session":"{}","id":"{}","tool":"{}","output":"{}","is_error":{}}}"#,
                    esc(session), esc(id), esc(tool), esc(output), is_error),

            AcpEvent::Done { session, stop_reason, usage } =>
                format!(r#"{{"type":"done","session":"{}","stop_reason":"{}","usage":{}}}"#,
                    esc(session), stop_reason.as_str(), usage.to_json()),

            AcpEvent::Error { session, code, message } =>
                format!(r#"{{"type":"error","session":"{}","code":"{}","message":"{}"}}"#,
                    esc(session), code.as_str(), esc(message)),

            AcpEvent::Compacted { session, turns_removed, summary_chars } =>
                format!(r#"{{"type":"compacted","session":"{}","turns_removed":{},"summary_chars":{}}}"#,
                    esc(session), turns_removed, summary_chars),

            AcpEvent::AgentStatus { session, agent_id, status, result } =>
                format!(r#"{{"type":"agent_status","session":"{}","agent_id":"{}","status":"{}","result":{}}}"#,
                    esc(session), esc(agent_id), esc(status),
                    result.as_deref().map(|r| format!("\"{}\"", esc(r))).unwrap_or_else(|| "null".to_string())),

            AcpEvent::Notification { session, kind, payload } =>
                format!(r#"{{"type":"notification","session":"{}","kind":"{}","payload":{}}}"#,
                    esc(session), esc(kind), payload),
        }
    }

    pub fn session_id(&self) -> &str {
        match self {
            AcpEvent::TextDelta     { session, .. } => session,
            AcpEvent::ThinkingDelta { session, .. } => session,
            AcpEvent::ToolStart     { session, .. } => session,
            AcpEvent::ToolResult    { session, .. } => session,
            AcpEvent::Done          { session, .. } => session,
            AcpEvent::Error         { session, .. } => session,
            AcpEvent::Compacted     { session, .. } => session,
            AcpEvent::AgentStatus   { session, .. } => session,
            AcpEvent::Notification  { session, .. } => session,
        }
    }
}

// ── ACP inbound commands (client → core) ──────────────────────────────────────
// Mirrors: src/acp/types.ts inbound union

#[derive(Clone, Debug)]
pub enum AcpCommand {
    /// Send a chat message to a session
    Chat {
        session:  String,
        content:  Vec<ContentBlock>,
        metadata: HashMap<String, String>,
    },
    /// Abort an in-progress session
    Abort { session: String },
    /// Update session configuration
    Configure {
        session: String,
        patch:   SessionPatch,
    },
    /// Spawn a new sub-agent
    SpawnAgent {
        parent_session: String,
        config:         SpawnConfig,
    },
}

// ── Session patch (mirrors OpenClaw SessionsPatch) ────────────────────────────
// Full field set from src/gateway/protocol/schema/sessions.ts

#[derive(Clone, Debug, Default)]
pub struct SessionPatch {
    // Identity
    pub label:           Option<String>,
    pub agent_id:        Option<String>,

    // Model selection
    pub model:           Option<String>,
    pub provider:        Option<String>,

    // Thinking / fast mode
    pub thinking_level:  Option<ThinkingLevel>,   // "none" | "low" | "medium" | "high"
    pub fast_mode:       Option<bool>,

    // Exec security (mirrors execSecurity from OpenClaw)
    pub exec_security:   Option<ExecSecurity>,
    pub exec_ask:        Option<bool>,

    // Response verbosity
    pub response_usage:  Option<ResponseUsage>,

    // Sub-agent metadata
    pub spawned_by:      Option<String>,
    pub spawn_depth:     Option<u8>,
    pub subagent_role:   Option<String>,

    // Skills
    pub skill_ids:       Option<Vec<String>>,
}

impl SessionPatch {
    /// Parse a JSON object into a SessionPatch (lightweight, no serde)
    pub fn from_json(json: &str) -> Self {
        let mut p = SessionPatch::default();
        if let Some(v) = extract_str(json, "label")     { p.label = Some(v); }
        if let Some(v) = extract_str(json, "agent_id")  { p.agent_id = Some(v); }
        if let Some(v) = extract_str(json, "model")     { p.model = Some(v); }
        if let Some(v) = extract_str(json, "provider")  { p.provider = Some(v); }
        if let Some(v) = extract_str(json, "thinking_level") {
            p.thinking_level = Some(ThinkingLevel::from_str(&v));
        }
        if json.contains(r#""fast_mode":true"#)  { p.fast_mode = Some(true);  }
        if json.contains(r#""fast_mode":false"#) { p.fast_mode = Some(false); }
        if json.contains(r#""exec_ask":true"#)   { p.exec_ask = Some(true);   }
        if json.contains(r#""exec_ask":false"#)  { p.exec_ask = Some(false);  }
        if let Some(v) = extract_str(json, "exec_security") {
            p.exec_security = Some(ExecSecurity::from_str(&v));
        }
        if let Some(v) = extract_str(json, "response_usage") {
            p.response_usage = Some(ResponseUsage::from_str(&v));
        }
        if let Some(v) = extract_str(json, "spawned_by")   { p.spawned_by = Some(v); }
        if let Some(v) = extract_str(json, "subagent_role") { p.subagent_role = Some(v); }
        if let Some(d) = extract_u8(json, "spawn_depth")   { p.spawn_depth = Some(d); }
        p
    }
}

// ── Thinking level ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum ThinkingLevel {
    None,
    Low,     // budget_tokens ~1000
    Medium,  // budget_tokens ~8000
    High,    // budget_tokens ~32000
}

impl ThinkingLevel {
    pub fn as_str(&self) -> &str {
        match self { Self::None => "none", Self::Low => "low", Self::Medium => "medium", Self::High => "high" }
    }
    pub fn from_str(s: &str) -> Self {
        match s { "low" => Self::Low, "medium" => Self::Medium, "high" => Self::High, _ => Self::None }
    }
    pub fn budget_tokens(&self) -> u32 {
        match self { Self::None => 0, Self::Low => 1024, Self::Medium => 8192, Self::High => 32768 }
    }
}

// ── Exec security ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
pub enum ExecSecurity {
    AutoApprove,   // approve all shell commands
    AskOnce,       // ask once per command pattern
    AskAlways,     // ask before every execution
    Block,         // block all shell execution
}

impl ExecSecurity {
    pub fn as_str(&self) -> &str {
        match self {
            Self::AutoApprove => "auto_approve",
            Self::AskOnce     => "ask_once",
            Self::AskAlways   => "ask_always",
            Self::Block       => "block",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "ask_once"   => Self::AskOnce,
            "ask_always" => Self::AskAlways,
            "block"      => Self::Block,
            _            => Self::AutoApprove,
        }
    }
}

// ── Response usage verbosity ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum ResponseUsage {
    Off,    // don't include token counts
    Tokens, // include input/output counts
    Full,   // include cache stats too
}

impl ResponseUsage {
    pub fn from_str(s: &str) -> Self {
        match s { "tokens" => Self::Tokens, "full" => Self::Full, _ => Self::Off }
    }
    pub fn as_str(&self) -> &str {
        match self { Self::Off => "off", Self::Tokens => "tokens", Self::Full => "full" }
    }
}

// ── Sub-agent spawn config ────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct SpawnConfig {
    pub goal:       String,
    pub model:      Option<String>,
    pub max_steps:  u32,
    pub session_id: Option<String>,   // None = auto-generate
    pub depth:      u8,               // caller increments this; max = 5
}

// ── Attachment ───────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct Attachment {
    pub name:      String,
    pub mime:      String,
    pub data_b64:  String,
    pub size:      usize,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "")
}

fn extract_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start  = json.find(&needle)? + needle.len();
    let bytes  = json.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    Some(json[start..end].replace("\\\"", "\"").replace("\\n", "\n").replace("\\\\", "\\"))
}

fn extract_u8(json: &str, key: &str) -> Option<u8> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    let end    = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}
