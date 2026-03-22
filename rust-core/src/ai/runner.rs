// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: runner
//
// ReAct loop — THINK → ACT → OBSERVE → repeat.
// Mirrors OpenClaw: src/agents/pi-embedded-runner/run.ts
//
// Session 2: full implementation.
//   - run_agent(): multi-step tool loop, loop detection, compaction trigger
//   - parse_tool_calls_json(): OpenAI function-calling JSON format
//   - build_messages_json(): assembles turns into LLM payload
//   - Global RUN_STATE + abort flag for POST /ai/run/abort
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ── Global run state (shared between HTTP thread and worker thread) ───────────

lazy_static::lazy_static! {
    pub static ref RUN_STATE: Arc<Mutex<RunState>> =
        Arc::new(Mutex::new(RunState::default()));
}

#[derive(Clone, Debug, Default)]
pub struct RunState {
    pub status:     RunStatus,
    pub session_id: String,
    pub step:       u32,
    pub steps_done: u32,
    pub abort:      bool,
    pub last_result: Option<AiRunResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunStatus {
    Idle,
    Running,
    Aborting,
    Done,
    Error,
}

impl Default for RunStatus {
    fn default() -> Self { RunStatus::Idle }
}

impl RunState {
    pub fn to_json(&self) -> String {
        let status_str = match self.status {
            RunStatus::Idle     => "idle",
            RunStatus::Running  => "running",
            RunStatus::Aborting => "aborting",
            RunStatus::Done     => "done",
            RunStatus::Error    => "error",
        };
        match &self.last_result {
            Some(r) if self.status == RunStatus::Done || self.status == RunStatus::Error =>
                format!(
                    r#"{{"status":"{}","session":"{}","steps":{},"result":{}}}"#,
                    status_str, self.session_id, self.steps_done, r.to_json()
                ),
            _ =>
                format!(
                    r#"{{"status":"{}","session":"{}","step":{}}}"#,
                    status_str, self.session_id, self.step
                ),
        }
    }
}

// ── Types (unchanged from Session 1) ─────────────────────────────────────────

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

#[derive(Clone, Debug)]
pub struct AiRunRequest {
    pub message:    String,
    pub session_id: String,
    pub max_steps:  u32,
    pub model:      Option<String>,
    pub thinking:   bool,
    pub stream:     bool,
}

impl AiRunRequest {
    pub fn from_json(json: &str) -> Self {
        AiRunRequest {
            message:    extract_str(json, "message").unwrap_or_default(),
            session_id: extract_str(json, "session")
                .unwrap_or_else(|| "default".to_string()),
            max_steps:  extract_u32(json, "max_steps").unwrap_or(25),
            model:      extract_str(json, "model"),
            thinking:   extract_bool(json, "thinking").unwrap_or(false),
            stream:     extract_bool(json, "stream").unwrap_or(false),
        }
    }
}

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

    pub fn error_result(msg: &str) -> Self {
        AiRunResult {
            content: String::new(), tools_used: vec![],
            steps: 0, tokens_in: 0, tokens_out: 0,
            done: false, error: Some(msg.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Turn {
    pub role:         String,
    pub content:      String,
    pub tool_call_id: Option<String>,
    pub tool_name:    Option<String>,
    // For assistant turns that include tool_calls array
    pub tool_calls_json: Option<String>,
}

impl Turn {
    pub fn user(c: &str)      -> Self { Turn { role:"user".into(), content:c.into(), tool_call_id:None, tool_name:None, tool_calls_json:None } }
    pub fn assistant(c: &str) -> Self { Turn { role:"assistant".into(), content:c.into(), tool_call_id:None, tool_name:None, tool_calls_json:None } }
    pub fn system(c: &str)    -> Self { Turn { role:"system".into(), content:c.into(), tool_call_id:None, tool_name:None, tool_calls_json:None } }
    pub fn tool_result(id: &str, name: &str, content: &str) -> Self {
        Turn { role:"tool".into(), content:content.into(),
               tool_call_id:Some(id.into()), tool_name:Some(name.into()),
               tool_calls_json:None }
    }

    pub fn to_openai_json(&self) -> String {
        match self.role.as_str() {
            "tool" => format!(
                r#"{{"role":"tool","tool_call_id":{},"name":{},"content":{}}}"#,
                json_escape_str(self.tool_call_id.as_deref().unwrap_or("")),
                json_escape_str(self.tool_name.as_deref().unwrap_or("")),
                json_escape_str(&self.content)
            ),
            "assistant" if self.tool_calls_json.is_some() => format!(
                r#"{{"role":"assistant","content":{},"tool_calls":{}}}"#,
                json_escape_str(&self.content),
                self.tool_calls_json.as_deref().unwrap_or("[]")
            ),
            _ => format!(
                r#"{{"role":"{}","content":{}}}"#,
                self.role, json_escape_str(&self.content)
            ),
        }
    }
}

// ── Loop detection ────────────────────────────────────────────────────────────

pub struct LoopDetector {
    recent: std::collections::VecDeque<String>,
    window: usize,
}

impl LoopDetector {
    pub fn new(window: usize) -> Self {
        LoopDetector { recent: std::collections::VecDeque::new(), window }
    }
    pub fn is_loop(&mut self, tool: &str, params_hash: u64) -> bool {
        let key = format!("{}:{}", tool, params_hash);
        let found = self.recent.contains(&key);
        self.recent.push_back(key);
        if self.recent.len() > self.window { self.recent.pop_front(); }
        found
    }
    pub fn reset(&mut self) { self.recent.clear(); }
}

pub fn hash_params(params: &HashMap<String, String>) -> u64 {
    let mut h: u64 = 5381;
    let mut keys: Vec<&String> = params.keys().collect();
    keys.sort();
    for k in keys {
        for b in format!("{}={}", k, &params[k]).bytes() {
            h = h.wrapping_mul(33).wrapping_add(b as u64);
        }
    }
    h
}

// ── Tool call from OpenAI function-calling JSON ───────────────────────────────

#[derive(Clone, Debug)]
pub struct JsonToolCall {
    pub id:     String,
    pub name:   String,
    pub params: HashMap<String, String>,
    /// Raw arguments JSON string (for re-injecting into assistant turn)
    pub args_json: String,
}

/// Parse OpenAI tool_calls array from LLM response JSON.
/// Format: {"tool_calls":[{"id":"call_x","type":"function",
///   "function":{"name":"foo","arguments":"{\"k\":\"v\"}"}}]}
pub fn parse_tool_calls_json(response_json: &str) -> Vec<JsonToolCall> {
    let mut calls = Vec::new();

    // Find tool_calls array
    let tc_marker = "\"tool_calls\":";
    let Some(tc_start) = response_json.find(tc_marker) else { return calls; };
    let after = &response_json[tc_start + tc_marker.len()..].trim_start();
    if !after.starts_with('[') { return calls; }

    // Walk the array — find each object
    let mut pos = 1usize; // skip '['
    let bytes = after.as_bytes();
    loop {
        // Skip whitespace and commas
        while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b',' || bytes[pos] == b'\n') {
            pos += 1;
        }
        if pos >= bytes.len() || bytes[pos] == b']' { break; }
        if bytes[pos] != b'{' { break; }

        // Extract one object
        let mut depth = 0i32;
        let obj_start = pos;
        loop {
            if pos >= bytes.len() { break; }
            match bytes[pos] {
                b'{' => depth += 1,
                b'}' => { depth -= 1; if depth == 0 { pos += 1; break; } }
                b'"' => {
                    // Skip string
                    pos += 1;
                    while pos < bytes.len() {
                        if bytes[pos] == b'"' && bytes[pos.saturating_sub(1)] != b'\\' { break; }
                        pos += 1;
                    }
                }
                _ => {}
            }
            pos += 1;
        }
        let obj = &after[obj_start..pos];

        // Extract id
        let id = str_field(obj, "id").unwrap_or_else(|| format!("call_{}", calls.len()));

        // Extract function.name and function.arguments
        let func_marker = "\"function\":";
        if let Some(fi) = obj.find(func_marker) {
            let func_slice = &obj[fi + func_marker.len()..].trim_start();
            let name = str_field(func_slice, "name").unwrap_or_default();
            let args_json = str_field(func_slice, "arguments").unwrap_or_else(|| "{}".to_string());

            // Parse arguments JSON string (it's a double-encoded string)
            let params = parse_flat_json_args(&args_json);

            calls.push(JsonToolCall { id, name, params, args_json });
        }
    }
    calls
}

/// Parse a flat JSON object like {"key":"val","key2":"val2"} into a HashMap.
/// Handles string and number values. Ignores nested objects.
fn parse_flat_json_args(json: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let json = json.trim().trim_start_matches('{').trim_end_matches('}');
    // Simple key:"value" scanner
    let mut rest = json;
    loop {
        // Find next "key":
        let Some(ks) = rest.find('"') else { break; };
        rest = &rest[ks+1..];
        let Some(ke) = rest.find('"') else { break; };
        let key = rest[..ke].to_string();
        rest = &rest[ke+1..];
        // Skip :
        let Some(colon) = rest.find(':') else { break; };
        rest = &rest[colon+1..].trim_start();
        // Read value — string or number/bool
        if rest.starts_with('"') {
            rest = &rest[1..];
            let mut val = String::new();
            let mut chars = rest.char_indices().peekable();
            let mut end_pos = 0;
            while let Some((i, c)) = chars.next() {
                if c == '\\' {
                    if let Some((_, nc)) = chars.next() {
                        match nc { 'n' => val.push('\n'), 't' => val.push('\t'), _ => val.push(nc) }
                    }
                } else if c == '"' {
                    end_pos = i + 1;
                    break;
                } else {
                    val.push(c);
                    end_pos = i + 1;
                }
            }
            map.insert(key, val);
            rest = &rest[end_pos..];
        } else {
            // Number or bool
            let end = rest.find(|c: char| c == ',' || c == '}' || c == '\n')
                .unwrap_or(rest.len());
            let val = rest[..end].trim().to_string();
            map.insert(key, val);
            rest = &rest[end..];
        }
        // Skip comma
        if let Some(ci) = rest.find(',') { rest = &rest[ci+1..]; } else { break; }
    }
    map
}

/// Build the OpenAI messages JSON array from a slice of Turns.
pub fn build_messages_json(turns: &[Turn]) -> String {
    let msgs: Vec<String> = turns.iter().map(|t| t.to_openai_json()).collect();
    format!("[{}]", msgs.join(","))
}

/// Build OpenAI-format tool definitions JSON array from (name, description, params) list.
/// params: Vec<(name, description, required, type)>
pub fn build_tools_json(
    tools: &[(&str, &str, Vec<(&str, &str, bool)>)]
) -> String {
    let defs: Vec<String> = tools.iter().map(|(name, desc, params)| {
        let props: Vec<String> = params.iter().map(|(pname, pdesc, _)| {
            format!(r#""{}":{{"type":"string","description":{}}}"#,
                pname, json_escape_str(pdesc))
        }).collect();
        let required: Vec<String> = params.iter()
            .filter(|(_, _, req)| *req)
            .map(|(n, _, _)| format!("\"{}\"", n))
            .collect();
        format!(
            r#"{{"type":"function","function":{{"name":{},"description":{},"parameters":{{"type":"object","properties":{{{}}},"required":[{}]}}}}}}"#,
            json_escape_str(name), json_escape_str(desc),
            props.join(","), required.join(",")
        )
    }).collect();
    format!("[{}]", defs.join(","))
}

// ── Core ReAct loop ───────────────────────────────────────────────────────────
//
// Called from a background thread (spawned by POST /ai/run).
// Updates RUN_STATE as it progresses so GET /ai/run/status is live.
//
// Parameters come from the caller via AgentRunConfig (avoids needing
// direct STATE access inside this module).

pub struct AgentRunConfig {
    pub api_key:       String,
    pub base_url:      String,
    pub model:         String,
    pub system_prompt: String,
    pub session_id:    String,
    pub user_message:  String,
    pub history:       Vec<(String, String)>,  // (role, content) pairs
    pub max_steps:     u32,
    pub tools_json:    String,   // pre-built tool schema JSON array
}

/// The full ReAct loop. Runs synchronously (call from a thread).
/// Returns the final AiRunResult.
///
/// Loop:
///   1. Build messages from history + user message
///   2. Call LLM with tools
///   3. If response has tool_calls → dispatch each, push results, loop
///   4. If no tool_calls → final answer, return
///   5. Loop detection: abort if same (tool, params) seen twice in window
///   6. Compaction: trim history if turns exceed budget
pub fn run_agent(cfg: AgentRunConfig) -> AiRunResult {
    // Set running state
    {
        let mut rs = RUN_STATE.lock().unwrap_or_else(|e| e.into_inner());
        rs.status     = RunStatus::Running;
        rs.session_id = cfg.session_id.clone();
        rs.step       = 0;
        rs.abort      = false;
        rs.last_result = None;
    }

    let mut loop_detector = LoopDetector::new(6);
    let mut tools_used: Vec<String> = Vec::new();
    let mut total_tokens_in:  u32 = 0;
    let mut total_tokens_out: u32 = 0;
    let mut final_content = String::new();

    // Build working turns list from history
    let mut turns: Vec<Turn> = Vec::new();
    if !cfg.system_prompt.is_empty() {
        turns.push(Turn::system(&cfg.system_prompt));
    }
    for (role, content) in &cfg.history {
        turns.push(match role.as_str() {
            "assistant" => Turn::assistant(content),
            _           => Turn::user(content),
        });
    }
    turns.push(Turn::user(&cfg.user_message));

    for step in 0..cfg.max_steps {
        // Abort check
        if RUN_STATE.lock().unwrap_or_else(|e| e.into_inner()).abort {
            let result = AiRunResult {
                content:    final_content,
                tools_used, steps: step,
                tokens_in:  total_tokens_in,
                tokens_out: total_tokens_out,
                done: false,
                error: Some("aborted".to_string()),
            };
            set_run_done(RunStatus::Error, result.clone());
            return result;
        }

        // Update step counter
        {
            let mut rs = RUN_STATE.lock().unwrap_or_else(|e| e.into_inner());
            rs.step = step;
        }

        // Compaction: trim if >100 non-system turns
        let non_system_count = turns.iter().filter(|t| t.role != "system").count();
        if non_system_count > 100 {
            compact_turns_inplace(&mut turns, 60);
        }

        // Build request body
        let messages_json = build_messages_json(&turns);
        let body = if cfg.tools_json.is_empty() || cfg.tools_json == "[]" {
            format!(
                r#"{{"model":{},"max_tokens":4096,"messages":{}}}"#,
                json_escape_str(&cfg.model), messages_json
            )
        } else {
            format!(
                r#"{{"model":{},"max_tokens":4096,"messages":{},"tools":{},"tool_choice":"auto"}}"#,
                json_escape_str(&cfg.model), messages_json, cfg.tools_json
            )
        };

        // Call LLM
        let response_body = match call_llm_raw(
            &cfg.api_key, &cfg.base_url, &body
        ) {
            Ok(r)  => r,
            Err(e) => {
                let result = AiRunResult::error_result(&e);
                set_run_done(RunStatus::Error, result.clone());
                return result;
            }
        };

        // Token accounting (rough)
        total_tokens_in  += estimate_tokens(&messages_json);
        total_tokens_out += estimate_tokens(&response_body);

        // Try OpenAI function-calling format first, then XML fallback
        let json_tool_calls = parse_tool_calls_json(&response_body);
        let text_content = extract_content_from_response(&response_body)
            .unwrap_or_default();

        if json_tool_calls.is_empty() {
            // No tool calls — this is the final answer
            final_content = text_content.clone();
            turns.push(Turn::assistant(&text_content));
            break;
        }

        // ── Tool execution ────────────────────────────────────────────────────

        // Build the assistant turn with tool_calls array
        let tc_json = build_tool_calls_array_json(&json_tool_calls);
        let mut assistant_turn = Turn::assistant(&text_content);
        assistant_turn.tool_calls_json = Some(tc_json);
        turns.push(assistant_turn);

        let mut loop_detected = false;
        for tc in &json_tool_calls {
            // Loop detection
            let ph = hash_params(&tc.params);
            if loop_detector.is_loop(&tc.name, ph) {
                loop_detected = true;
                // Push a tool result telling the AI to stop repeating
                turns.push(Turn::tool_result(
                    &tc.id, &tc.name,
                    "Error: loop detected — this exact tool call was already made. Stop repeating and give a final answer."
                ));
                continue;
            }

            // Dispatch
            let result = dispatch_one_tool(&tc.name, &tc.params);
            tools_used.push(tc.name.clone());
            turns.push(Turn::tool_result(&tc.id, &tc.name, &result));
        }

        if loop_detected && json_tool_calls.len() == 1 {
            // All calls were loops — force stop
            final_content = "I noticed I was repeating the same tool call. Here is what I know so far.".to_string();
            break;
        }

        final_content = text_content; // carry forward partial content
    }

    let result = AiRunResult {
        content:    final_content,
        tools_used,
        steps:      {
            let rs = RUN_STATE.lock().unwrap_or_else(|e| e.into_inner());
            rs.step + 1
        },
        tokens_in:  total_tokens_in,
        tokens_out: total_tokens_out,
        done:       true,
        error:      None,
    };
    set_run_done(RunStatus::Done, result.clone());
    result
}

fn set_run_done(status: RunStatus, result: AiRunResult) {
    let mut rs = RUN_STATE.lock().unwrap_or_else(|e| e.into_inner());
    rs.steps_done  = rs.step + 1;
    rs.status      = status;
    rs.last_result = Some(result);
}

// ── Tool dispatch shim ────────────────────────────────────────────────────────
// Calls back into lib.rs dispatch_tool via a function pointer set at startup.
// This avoids a circular dependency (runner.rs → lib.rs).

use std::sync::OnceLock;

type DispatchFn = fn(&str, &HashMap<String, String>) -> String;
static DISPATCH_FN: OnceLock<DispatchFn> = OnceLock::new();

pub fn register_dispatch(f: DispatchFn) {
    let _ = DISPATCH_FN.set(f);
}

fn dispatch_one_tool(name: &str, params: &HashMap<String, String>) -> String {
    match DISPATCH_FN.get() {
        Some(f) => f(name, params),
        None    => format!("{{\"error\":\"tool dispatch not registered: {}\"}}", name),
    }
}

// ── LLM call shim ─────────────────────────────────────────────────────────────

type LlmCallFn = fn(&str, &str, &str) -> Result<String, String>;
static LLM_CALL_FN: OnceLock<LlmCallFn> = OnceLock::new();

pub fn register_llm_call(f: LlmCallFn) {
    let _ = LLM_CALL_FN.set(f);
}

fn call_llm_raw(api_key: &str, base_url: &str, body: &str) -> Result<String, String> {
    match LLM_CALL_FN.get() {
        Some(f) => f(api_key, base_url, body),
        None    => Err("LLM call not registered".to_string()),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Compact turns in-place: keep system + last `keep` non-system turns
fn compact_turns_inplace(turns: &mut Vec<Turn>, keep: usize) {
    let (sys, non_sys): (Vec<Turn>, Vec<Turn>) =
        turns.drain(..).partition(|t| t.role == "system");
    let skip = non_sys.len().saturating_sub(keep);
    turns.extend(sys);
    turns.extend(non_sys.into_iter().skip(skip));
}

/// Build tool_calls JSON array from parsed JsonToolCalls (for re-injection)
/// Public alias used by subagents module
pub fn build_tool_calls_array_json_pub(calls: &[JsonToolCall]) -> String {
    build_tool_calls_array_json(calls)
}

fn build_tool_calls_array_json(calls: &[JsonToolCall]) -> String {
    let items: Vec<String> = calls.iter().map(|c| {
        format!(
            r#"{{"id":{},"type":"function","function":{{"name":{},"arguments":{}}}}}"#,
            json_escape_str(&c.id),
            json_escape_str(&c.name),
            json_escape_str(&c.args_json)
        )
    }).collect();
    format!("[{}]", items.join(","))
}

/// Extract text content from LLM response (OpenAI + Anthropic + Gemini)
fn extract_content_from_response(json: &str) -> Option<String> {
    fn unescape(s: &str) -> String {
        s.replace("\\n", "\n").replace("\\t", "\t")
         .replace("\\\"", "\"").replace("\\\\", "\\")
    }
    fn find_str(json: &str, key: &str) -> Option<String> {
        let s1 = format!("\"{}\":\"", key);
        let s2 = format!("\"{}\": \"", key);
        let start = json.find(&s1).map(|i| i + s1.len())
            .or_else(|| json.find(&s2).map(|i| i + s2.len()))?;
        let bytes = json.as_bytes();
        let mut end = start;
        while end < bytes.len() {
            if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
            end += 1;
        }
        let s = &json[start..end];
        if s.is_empty() { None } else { Some(unescape(s)) }
    }

    // OpenAI/Groq: choices[0].message.content
    if let Some(mi) = json.find("\"message\":{") {
        if let Some(c) = find_str(&json[mi..], "content") { return Some(c); }
    }
    // Anthropic: content[0].text
    if json.contains("\"type\":\"text\"") {
        if let Some(c) = find_str(json, "text") { return Some(c); }
    }
    // Gemini: candidates[0].content.parts[0].text
    if json.contains("\"candidates\":[") {
        if let Some(c) = find_str(json, "text") { return Some(c); }
    }
    // Fallback
    find_str(json, "content")
        .or_else(|| find_str(json, "text"))
        .or_else(|| find_str(json, "response"))
}

fn estimate_tokens(s: &str) -> u32 { (s.len() as u32 / 4).max(1) }

fn str_field(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let bytes = json.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    let s = &json[start..end];
    if s.is_empty() { None } else {
        Some(s.replace("\\\"", "\"").replace("\\\\", "\\")
              .replace("\\n", "\n").replace("\\t", "\t"))
    }
}

// ── JSON helpers (same as Session 1, kept local) ──────────────────────────────

fn extract_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let bytes = json.as_bytes();
    let mut end = start;
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
    if slice.starts_with("true") { Some(true) }
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

