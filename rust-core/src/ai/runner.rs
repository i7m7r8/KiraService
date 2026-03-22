// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: runner
//
// ReAct loop — THINK → ACT → OBSERVE → repeat.
// Mirrors OpenClaw: src/agents/pi-embedded-runner/run.ts
//
// Session 1: types only.
// Session 2: implement run_agent(), abort, status.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;

/// Current state of an AI run — exposed via GET /ai/status
#[derive(Clone, Debug, PartialEq)]
pub enum AiRunStatus {
    Idle,
    Running { session_id: String, step: u32 },
    Aborting { session_id: String },
    Done { session_id: String, steps: u32 },
    Error { message: String },
}

impl AiRunStatus {
    pub fn to_json(&self) -> String {
        match self {
            AiRunStatus::Idle =>
                r#"{"status":"idle"}"#.to_string(),
            AiRunStatus::Running { session_id, step } =>
                format!(r#"{{"status":"running","session":"{}","step":{}}}"#, session_id, step),
            AiRunStatus::Aborting { session_id } =>
                format!(r#"{{"status":"aborting","session":"{}"}}"#, session_id),
            AiRunStatus::Done { session_id, steps } =>
                format!(r#"{{"status":"done","session":"{}","steps":{}}}"#, session_id, steps),
            AiRunStatus::Error { message } =>
                format!(r#"{{"status":"error","message":"{}"}}"#,
                    message.replace('"', "\\\"")),
        }
    }
}

/// Inbound request for POST /ai/run
#[derive(Clone, Debug)]
pub struct AiRunRequest {
    pub message:    String,
    pub session_id: String,
    pub max_steps:  u32,     // default 25
    pub model:      Option<String>,
    pub thinking:   bool,    // extended reasoning budget
}

impl AiRunRequest {
    pub fn from_json(json: &str) -> Self {
        AiRunRequest {
            message:    extract_str(json, "message").unwrap_or_default(),
            session_id: extract_str(json, "session").unwrap_or_else(|| "default".to_string()),
            max_steps:  extract_u32(json, "max_steps").unwrap_or(25),
            model:      extract_str(json, "model"),
            thinking:   extract_bool(json, "thinking").unwrap_or(false),
        }
    }
}

/// Result returned from a completed AI run
#[derive(Clone, Debug)]
pub struct AiRunResult {
    pub content:    String,
    pub tools_used: Vec<String>,
    pub steps:      u32,
    pub tokens_in:  u32,
    pub tokens_out: u32,
    pub done:       bool,
    pub error:      Option<String>,
}

impl AiRunResult {
    pub fn to_json(&self) -> String {
        let tools = self.tools_used.iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
            .collect::<Vec<_>>()
            .join(",");
        let err = self.error.as_deref()
            .map(|e| format!(",\"error\":\"{}\"", e.replace('"', "\\\"")))
            .unwrap_or_default();
        format!(
            r#"{{"content":{},"tools_used":[{}],"steps":{},"tokens_in":{},"tokens_out":{},"done":{}{}}}"#,
            json_escape_str(&self.content),
            tools,
            self.steps, self.tokens_in, self.tokens_out,
            self.done, err
        )
    }

    pub fn error(msg: &str) -> Self {
        AiRunResult {
            content: String::new(),
            tools_used: vec![],
            steps: 0, tokens_in: 0, tokens_out: 0,
            done: false,
            error: Some(msg.to_string()),
        }
    }
}

// ── Conversation turn (for building LLM message arrays) ─────────────────────

#[derive(Clone, Debug)]
pub struct Turn {
    pub role:    String, // "system" | "user" | "assistant" | "tool"
    pub content: String,
    pub tool_call_id: Option<String>,
    pub tool_name:    Option<String>,
}

impl Turn {
    pub fn user(content: &str)     -> Self { Turn { role:"user".into(), content:content.into(), tool_call_id:None, tool_name:None } }
    pub fn assistant(content: &str) -> Self { Turn { role:"assistant".into(), content:content.into(), tool_call_id:None, tool_name:None } }
    pub fn system(content: &str)   -> Self { Turn { role:"system".into(), content:content.into(), tool_call_id:None, tool_name:None } }
    pub fn tool_result(id: &str, name: &str, content: &str) -> Self {
        Turn { role:"tool".into(), content:content.into(), tool_call_id:Some(id.into()), tool_name:Some(name.into()) }
    }

    /// Serialize as OpenAI-format message JSON object
    pub fn to_openai_json(&self) -> String {
        match self.role.as_str() {
            "tool" => format!(
                r#"{{"role":"tool","tool_call_id":{},"content":{}}}"#,
                json_escape_str(self.tool_call_id.as_deref().unwrap_or("")),
                json_escape_str(&self.content)
            ),
            _ => format!(
                r#"{{"role":"{}","content":{}}}"#,
                self.role,
                json_escape_str(&self.content)
            ),
        }
    }
}

// ── Loop detection ───────────────────────────────────────────────────────────
// Mirrors OpenClaw: src/agents/tool-loop-detection.ts

pub struct LoopDetector {
    recent: std::collections::VecDeque<String>,
    window: usize,
}

impl LoopDetector {
    pub fn new(window: usize) -> Self {
        LoopDetector { recent: std::collections::VecDeque::new(), window }
    }

    /// Returns true if this (tool, params) combo appeared in the last `window` calls
    pub fn is_loop(&mut self, tool: &str, params_hash: u64) -> bool {
        let key = format!("{}:{}", tool, params_hash);
        let found = self.recent.contains(&key);
        self.recent.push_back(key);
        if self.recent.len() > self.window { self.recent.pop_front(); }
        found
    }

    pub fn reset(&mut self) { self.recent.clear(); }
}

/// Simple djb2 hash for params HashMap — used by LoopDetector
pub fn hash_params(params: &HashMap<String, String>) -> u64 {
    let mut h: u64 = 5381;
    let mut keys: Vec<&String> = params.keys().collect();
    keys.sort();
    for k in keys {
        let v = &params[k];
        for b in format!("{}={}", k, v).bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
    }
    h
}

// ── Helpers (inline — avoids cross-module dep on utils.rs for now) ───────────

fn extract_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let mut end = start;
    let bytes = json.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    Some(json[start..end].to_string())
}

fn extract_u32(json: &str, key: &str) -> Option<u32> {
    let search = format!("\"{}\":", key);
    let start = json.find(&search)? + search.len();
    let slice = json[start..].trim_start();
    let end = slice.find(|c: char| !c.is_ascii_digit()).unwrap_or(slice.len());
    slice[..end].parse().ok()
}

fn extract_bool(json: &str, key: &str) -> Option<bool> {
    let search = format!("\"{}\":", key);
    let start = json.find(&search)? + search.len();
    let slice = json[start..].trim_start();
    if slice.starts_with("true")  { Some(true)  }
    else if slice.starts_with("false") { Some(false) }
    else { None }
}

pub fn json_escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"'  => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c    => out.push(c),
        }
    }
    out.push('"');
    out
}
