// Kira Rust Core v7 -- OpenClaw + ZeroClaw + AndyClaw + NanoClaw
//
// What we learned from real research:
//
// OpenClaw (310k stars, real architecture):
//   - Gateway = WebSocket control plane on port 18789 (we use 7070 HTTP + 7071 WS)
//   - Agent Runtime = assembles context, invokes model, runs tools, persists state
//   - Skills = SKILL.md files with YAML frontmatter, selectively injected per turn
//   - Memory = plain files: MEMORY.md (long-term) + daily logs
//   - ClawHub = skill registry with vector-indexed search
//   - Channels = WhatsApp/Telegram/Slack/Discord/Signal/etc
//   - Canvas = agent-driven HTML workspace on separate port
//   - Nodes = device capability hosts (camera, screen, location, voice)
//   - Sessions = persistent conversation state across restarts
//   - SOUL.md = agent personality/identity file
//
// ZeroClaw / phoneclaw:
//   - ClawScript = JS scripting engine for runtime automation
//   - Vision-assisted UI targeting (screenshot + description -> tap)
//   - Cron-like scheduling for recurring automations
//   - sendAgentEmail for handoffs and notifications
//
// AndyClaw / openclaw-assistant:
//   - AES256-GCM encrypted credential storage
//   - mDNS/Bonjour gateway auto-discovery
//   - Ed25519 cryptographic device identity
//   - Wake word detection + voice input
//   - Wear OS node support
//   - Offline-capable with local model fallback
//   - Streaming AI responses via WebSocket
//
// NanoClaw / openclaw-pm:
//   - HEARTBEAT.md checklist pattern (health checks)
//   - Session isolation per task
//   - Checkpoint files for task state persistence
//   - Tool policy enforcement (allowlist/denylist)
//   - Audit trail for every tool call

#![allow(non_snake_case)]

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ??? Core State ???????????????????????????????????????????????????????????????

#[derive(Default)]
struct KiraState {
    // Device
    screen_nodes:      String,
    screen_pkg:        String,
    battery_pct:       i32,
    battery_charging:  bool,

    // Notifications
    notifications:     VecDeque<Notif>,

    // Command queue (Gateway -> Java accessibility)
    pending_cmds:      VecDeque<(String, String)>,
    results:           HashMap<String, String>,

    // ?? OpenClaw: Gateway session management
    sessions:          HashMap<String, Session>,
    active_session:    String,

    // ?? OpenClaw: MEMORY.md + daily log pattern
    memory_md:         String,           // long-term facts (MEMORY.md equivalent)
    daily_log:         VecDeque<String>, // today's log entries
    soul_md:           String,           // SOUL.md: agent personality

    // ?? OpenClaw: Skill registry (SKILL.md pattern)
    skills:            HashMap<String, Skill>,
    skill_turn_inject: Vec<String>,      // skills to inject for current turn

    // ?? OpenClaw: Context engine (turn history + compaction)
    context_turns:     VecDeque<ContextTurn>,
    context_compact:   String,
    agent_context:     String,
    total_tokens:      u64,

    // ?? OpenClaw: WebSocket node capabilities
    node_caps:         HashMap<String, NodeCapability>,

    // ?? OpenClaw: Trigger / webhook surface
    triggers:          Vec<Trigger>,
    fired_triggers:    VecDeque<String>,
    webhook_events:    VecDeque<String>,

    // ?? OpenClaw: Heartbeat checklist (openclaw-pm pattern)
    heartbeat_items:   Vec<HeartbeatItem>,
    heartbeat_log:     VecDeque<String>,

    // ?? ZeroClaw: Provider registry (17+ providers)
    providers:         Vec<Provider>,
    active_provider:   String,

    // ?? ZeroClaw: Encrypted credential store (simulated AES with XOR+hash)
    credentials:       HashMap<String, Vec<u8>>,

    // ?? ZeroClaw: Cron / scheduled jobs
    cron_jobs:         Vec<CronJob>,

    // ?? AndyClaw: Semantic memory FTS index with relevance scoring
    memory_index:      Vec<MemoryEntry>,

    // ?? AndyClaw: Tool policy enforcement
    tool_allowlist:    Vec<String>,
    tool_denylist:     Vec<String>,

    // ?? NanoClaw: Tool iteration counter (max 20 per session)
    tool_iterations:   HashMap<String, u32>,
    max_tool_iters:    u32,

    // ?? NanoClaw: Full audit log
    audit_log:         VecDeque<AuditEntry>,

    // ?? NanoClaw: Task log + checkpoints
    task_log:          VecDeque<TaskStep>,
    checkpoints:       HashMap<String, String>,

    // Rou Bao: streaming
    stream_chunks:     VecDeque<StreamChunk>,
    stream_sessions:   HashMap<String, StreamSession>,
    // ZeroClaw: response cache
    response_cache:    HashMap<String, CacheEntry>,
    // OpenClaw: knowledge base
    knowledge_base:    Vec<KbEntry>,
    // OpenClaw: event feed
    event_feed:        VecDeque<EventFeedEntry>,
    // Stats
    uptime_start:      u128,
    request_count:     u64,
    tool_call_count:   u64,
}

// OpenClaw session
#[derive(Clone, Default)]
struct Session {
    id:       String,
    channel:  String,  // telegram | webchat | cli | floating
    turns:    u32,
    tokens:   u64,
    created:  u128,
    last_msg: u128,
}

// OpenClaw skill (SKILL.md pattern)
#[derive(Clone, Default)]
struct Skill {
    name:        String,
    description: String,
    trigger:     String,    // keyword/pattern that auto-injects this skill
    content:     String,    // the SKILL.md instructions
    enabled:     bool,
    usage_count: u32,
}

// OpenClaw context turn
#[derive(Clone)]
struct ContextTurn {
    role:     String,
    content:  String,
    ts:       u128,
    tokens:   u32,
    session:  String,
}

// OpenClaw WebSocket node capability
#[derive(Clone, Default)]
struct NodeCapability {
    node_id:  String,
    caps:     Vec<String>,  // camera, screen, location, voice, canvas
    platform: String,       // android | ios | macos
    online:   bool,
    last_seen:u128,
}

// Trigger (NanoBot/OpenClaw webhook surface)
#[derive(Clone)]
struct Trigger {
    id:           String,
    trigger_type: String,
    value:        String,
    action:       String,
    fired:        bool,
    repeat:       bool,
}

// OpenClaw-pm heartbeat item
#[derive(Clone)]
struct HeartbeatItem {
    id:          String,
    check:       String,
    action:      String,
    enabled:     bool,
    last_run:    u128,
    interval_ms: u128,
}

// ZeroClaw provider (17 supported)
#[derive(Clone, Default)]
struct Provider {
    id:       String,
    name:     String,
    base_url: String,
    model:    String,
}

// ZeroClaw cron job
#[derive(Clone)]
struct CronJob {
    id:         String,
    expression: String,   // "*/30 * * * *" style or simple interval in ms
    action:     String,
    last_run:   u128,
    interval_ms:u128,
    enabled:    bool,
}

// AndyClaw semantic memory entry
#[derive(Clone)]
struct MemoryEntry {
    key:       String,
    value:     String,
    tags:      Vec<String>,
    ts:        u128,
    relevance: f32,
    access_count: u32,
}

// NanoClaw audit entry
#[derive(Clone)]
struct AuditEntry {
    session: String,
    tool:    String,
    input:   String,
    output:  String,
    ts:      u128,
    success: bool,
    blocked: bool,     // true if tool was blocked by policy
}

// NanoClaw task step
#[derive(Clone)]
struct TaskStep {
    task_id: String,
    step:    u32,
    action:  String,
    result:  String,
    time:    u128,
    success: bool,
}

// ??? Global State ?????????????????????????????????????????????????????????????


// Rou Bao streaming structs
#[derive(Clone, Default)]
struct StreamChunk { session_id: String, text: String, done: bool, ts: u128 }
#[derive(Clone, Default)]
struct StreamSession { id: String, active: bool, started: u128, chunks: u32 }
// ZeroClaw cache
#[derive(Clone)]
struct CacheEntry { value: String, expires_at: u128 }
// OpenClaw knowledge base
#[derive(Clone)]
struct KbEntry { id: String, title: String, content: String, tags: Vec<String>, ts: u128 }
// OpenClaw event feed
#[derive(Clone)]
struct EventFeedEntry { event: String, data: String, ts: u128 }

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<KiraState>> = Arc::new(Mutex::new(KiraState {
        battery_pct:    100,
        max_tool_iters: 20,
        active_session: "default".to_string(),
        active_provider:"groq".to_string(),
        soul_md: "You are Kira, a powerful Android AI agent. You are helpful, proactive, and autonomous.".to_string(),
        ..Default::default()
    }));
}

// ??? JNI Bridge ???????????????????????????????????????????????????????????????

mod jni_bridge {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    fn cs(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, port: i32,
    ) {
        let p = port as u16;
        {
            let mut s = STATE.lock().unwrap();
            s.uptime_start = now_ms();
            s.providers     = make_providers();
            // Create default session
            let sess = Session { id: "default".into(), channel: "kira".into(), created: now_ms(), last_msg: now_ms(), ..Default::default() };
            s.sessions.insert("default".into(), sess);
        }
        thread::spawn(move || run_http(p));
        thread::spawn(run_trigger_watcher);
        thread::spawn(run_cron_scheduler);
    }

    // Device state
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        pkg: *const c_char, title: *const c_char, text: *const c_char,
    ) {
        let (pkg, title, text) = (cs(pkg), cs(title), cs(text));
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        fire_notif_triggers(&mut s, &pkg, &title, &text);
        // Daily log (OpenClaw daily log pattern)
        s.daily_log.push_back(format!("[{}] notif {}:{}", ts, pkg, &title[..title.len().min(40)]));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
        s.notifications.push_back(Notif { pkg, title, text, time: ts });
        if s.notifications.len() > 500 { s.notifications.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, json: *const c_char,
    ) { STATE.lock().unwrap().screen_nodes = cs(json); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenPackage(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pkg: *const c_char,
    ) { STATE.lock().unwrap().screen_pkg = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pct: i32, charging: bool,
    ) {
        let mut s = STATE.lock().unwrap();
        let prev = s.battery_pct;
        s.battery_pct = pct; s.battery_charging = charging;
        fire_battery_triggers(&mut s, pct, prev);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateAgentContext(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, ctx: *const c_char,
    ) { STATE.lock().unwrap().agent_context = cs(ctx); }

    // OpenClaw: context turn (builds MEMORY.md + daily log pattern)
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushContextTurn(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        role: *const c_char, content: *const c_char,
    ) {
        let role = cs(role); let content = cs(content);
        let tokens = estimate_tokens(&content);
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        let sess_id = s.active_session.clone();
        s.total_tokens += tokens as u64;
        // Daily log entry
        s.daily_log.push_back(format!("[{}] {}: {}", ts, role, &content[..content.len().min(80)]));
        // Context window
        s.context_turns.push_back(ContextTurn { role, content, ts, tokens, session: sess_id });
        // Compact if over 60 turns (OpenClaw compaction pattern)
        if s.context_turns.len() > 60 { compact_context(&mut s); }
    }

    // AndyClaw: semantic memory index
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_indexMemory(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        key: *const c_char, value: *const c_char, tags: *const c_char,
    ) {
        let (key, value, tags_raw) = (cs(key), cs(value), cs(tags));
        let tags: Vec<String> = tags_raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        let mut s = STATE.lock().unwrap();
        s.memory_index.retain(|e| e.key != key);
        // Also update MEMORY.md equivalent
        let fact = format!("- {}: {}", key, value);
        if !s.memory_md.contains(&fact) { s.memory_md.push_str(&format!("\n{}", fact)); }
        s.memory_index.push(MemoryEntry { key, value, tags, ts: now_ms(), relevance: 1.0, access_count: 0 });
        if s.memory_index.len() > 10000 {
            s.memory_index.sort_by(|a, b| a.relevance.partial_cmp(&b.relevance).unwrap_or(std::cmp::Ordering::Equal));
            s.memory_index.remove(0);
        }
    }

    // ZeroClaw: encrypted credential storage
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_storeCredential(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name: *const c_char, value: *const c_char,
    ) {
        let name = cs(name); let value = cs(value);
        let enc = xor_crypt(value.as_bytes(), derive_key(&name).as_slice());
        STATE.lock().unwrap().credentials.insert(name, enc);
    }

    // OpenClaw: skill registry
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_registerSkill(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name: *const c_char, desc: *const c_char, trigger: *const c_char, content: *const c_char,
    ) {
        let name = cs(name);
        STATE.lock().unwrap().skills.insert(name.clone(), Skill {
            name, description: cs(desc), trigger: cs(trigger), content: cs(content), enabled: true, usage_count: 0,
        });
    }

    // OpenClaw-pm: heartbeat checklist
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addHeartbeatItem(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id: *const c_char, check: *const c_char, action: *const c_char, interval_ms: i64,
    ) {
        let item = HeartbeatItem { id: cs(id), check: cs(check), action: cs(action), enabled: true, last_run: 0, interval_ms: interval_ms as u128 };
        let mut s = STATE.lock().unwrap();
        s.heartbeat_items.retain(|i| i.id != item.id);
        s.heartbeat_items.push(item);
    }

    // NanoClaw: tool iteration counter
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_incrementToolIter(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id: *const c_char,
    ) -> i32 {
        let id = cs(session_id);
        let mut s = STATE.lock().unwrap();
        let count = {
            let c = s.tool_iterations.entry(id).or_insert(0);
            *c += 1;
            *c
        };
        s.tool_call_count += 1;
        let max = s.max_tool_iters;
        if count > max { -1 } else { count as i32 }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resetToolIter(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id: *const c_char,
    ) { STATE.lock().unwrap().tool_iterations.remove(&cs(session_id)); }

    // NanoClaw: task log
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logTaskStep(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        task_id: *const c_char, step: i32, action: *const c_char, result: *const c_char, success: bool,
    ) {
        let (tid, act, res) = (cs(task_id), cs(action), cs(result));
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        do_audit(&mut s, &tid, &act, &act, &res, success, false);
        s.task_log.push_back(TaskStep { task_id: tid, step: step as u32, action: act, result: res, time: ts, success });
        if s.task_log.len() > 2000 { s.task_log.pop_front(); }
    }

    // Command queue
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().pending_cmds.pop_front() {
            Some((id, body)) => CString::new(format!(r#"{{"id":"{}","body":{}}}"#, id, body)).unwrap().into_raw(),
            None => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char, result: *const c_char,
    ) { STATE.lock().unwrap().results.insert(cs(id), cs(result)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextFiredTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().fired_triggers.pop_front() {
            Some(t) => CString::new(t).unwrap().into_raw(),
            None    => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id: *const c_char, ttype: *const c_char, value: *const c_char, action: *const c_char, repeat: bool,
    ) {
        STATE.lock().unwrap().triggers.push(Trigger { id: cs(id), trigger_type: cs(ttype), value: cs(value), action: cs(action), fired: false, repeat });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char,
    ) { let id = cs(id); STATE.lock().unwrap().triggers.retain(|t| t.id != id); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, s: *mut c_char,
    ) { if !s.is_null() { unsafe { drop(CString::from_raw(s)); } } }
}

struct Notif { pkg: String, title: String, text: String, time: u128 }

// ??? HTTP Server (port 7070) ???????????????????????????????????????????????????

fn run_http(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => { eprintln!("kira bind: {}", e); return; }
    };
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle_http(stream));
    }
}

fn handle_http(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let body = req.find("\r\n\r\n").map(|i| req[i+4..].trim().to_string()).unwrap_or_default();
    let resp = route_http(parts[0], parts[1], &body);
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Kira-Engine: rust-v7\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());
    STATE.lock().unwrap().request_count += 1;
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    // Strip query string for matching
    let path_clean = path.split('?').next().unwrap_or(path);
    match (method, path_clean) {
        // ?? Health & stats ???????????????????????????????????????????????????
        ("GET", "/health") | ("GET", "/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"status":"ok","version":"7.0","uptime_ms":{},"requests":{},"tool_calls":{},"battery":{},"charging":{},"notifications":{},"skills":{},"triggers":{},"memory_entries":{},"total_tokens":{},"sessions":{}}}"#,
                now_ms()-s.uptime_start, s.request_count, s.tool_call_count,
                s.battery_pct, s.battery_charging,
                s.notifications.len(), s.skills.len(),
                s.triggers.iter().filter(|t|!t.fired).count(),
                s.memory_index.len(), s.total_tokens, s.sessions.len())
        }
        ("GET", "/stats") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"notifications":{},"pending_cmds":{},"task_steps":{},"audit_entries":{},"context_turns":{},"daily_log_entries":{},"skills":{},"memory_entries":{},"cron_jobs":{},"tool_calls":{},"total_tokens":{},"uptime_ms":{}}}"#,
                s.notifications.len(), s.pending_cmds.len(), s.task_log.len(),
                s.audit_log.len(), s.context_turns.len(), s.daily_log.len(),
                s.skills.len(), s.memory_index.len(), s.cron_jobs.len(),
                s.tool_call_count, s.total_tokens, now_ms()-s.uptime_start)
        }

        // ?? Device state ?????????????????????????????????????????????????????
        ("GET", "/screen")         => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET", "/screen_pkg")     => { let p = STATE.lock().unwrap().screen_pkg.clone(); format!(r#"{{"package":"{}"}}"#, esc(&p)) }
        ("GET", "/battery")        => { let s = STATE.lock().unwrap(); format!(r#"{{"percentage":{},"charging":{}}}"#, s.battery_pct, s.battery_charging) }
        ("GET", "/notifications")  => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.notifications.iter().map(|n| format!(r#"{{"pkg":"{}","title":"{}","text":"{}","time":{}}}"#, esc(&n.pkg),esc(&n.title),esc(&n.text),n.time)).collect(); format!("[{}]", items.join(",")) }

        // ?? OpenClaw: Memory (MEMORY.md pattern) ?????????????????????????????
        ("GET", "/memory")         => { let s = STATE.lock().unwrap(); format!(r#"{{"memory_md":{},"entries":{}}}"#, json_str(&s.memory_md), s.memory_index.len()) }
        ("GET", "/memory/search")  => search_memory(path),
        ("GET", "/memory/full")    => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.memory_index.iter().map(|e| format!(r#"{{"key":"{}","value":"{}","tags":{},"relevance":{:.2},"access_count":{}}}"#, esc(&e.key),esc(&e.value),json_str_arr(&e.tags),e.relevance,e.access_count)).collect(); format!("[{}]", items.join(",")) }

        // ?? OpenClaw: Daily log ???????????????????????????????????????????????
        ("GET", "/daily_log")      => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.daily_log.iter().cloned().map(|l| format!("\"{}\"", esc(&l))).collect(); format!("[{}]", items.join(",")) }

        // ?? OpenClaw: Context + SOUL.md ???????????????????????????????????????
        ("GET", "/context")        => get_context_json(),
        ("GET", "/soul")           => { let s = STATE.lock().unwrap(); format!(r#"{{"soul":{}}}"#, json_str(&s.soul_md)) }
        ("POST", "/soul")          => { let val = extract_json_str(body, "content").unwrap_or_default(); if !val.is_empty() { STATE.lock().unwrap().soul_md = val; } r#"{"ok":true}"#.to_string() }

        // ?? OpenClaw: Skills ?????????????????????????????????????????????????
        ("GET", "/skills")         => get_skills_json(),
        ("POST", "/skills/register")=> { register_skill(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/enable") => { let name = extract_json_str(body,"name").unwrap_or_default(); if let Some(sk) = STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=true; } r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/disable")=> { let name = extract_json_str(body,"name").unwrap_or_default(); if let Some(sk) = STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=false; } r#"{"ok":true}"#.to_string() }

        // ?? OpenClaw: Sessions ???????????????????????????????????????????????
        ("GET", "/sessions")       => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.sessions.values().map(|sess| format!(r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"last_msg":{}}}"#, sess.id,sess.channel,sess.turns,sess.tokens,sess.last_msg)).collect(); format!("[{}]", items.join(",")) }

        // ?? Triggers / webhook surface ????????????????????????????????????????
        ("GET", "/triggers")       => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.triggers.iter().map(|t| format!(r#"{{"id":"{}","type":"{}","value":"{}","fired":{},"repeat":{}}}"#, t.id,t.trigger_type,esc(&t.value),t.fired,t.repeat)).collect(); format!("[{}]", items.join(",")) }
        ("GET", "/fired_triggers") => { let mut s = STATE.lock().unwrap(); let items: Vec<String> = s.fired_triggers.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("GET", "/webhook_events") => { let mut s = STATE.lock().unwrap(); let items: Vec<String> = s.webhook_events.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/triggers/add")  => { add_trigger(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/webhook")       => { let ts = now_ms(); STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"body":{},"ts":{}}}"#, if body.is_empty(){"{}"} else {body}, ts)); r#"{"ok":true}"#.to_string() }

        // ?? OpenClaw-pm: Heartbeat checklist ?????????????????????????????????
        ("GET", "/heartbeat_log")  => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.heartbeat_log.iter().cloned().collect(); format!("[{}]", items.join(",")) }
        ("POST", "/heartbeat/add") => { add_heartbeat(body); r#"{"ok":true}"#.to_string() }

        // ?? ZeroClaw: Providers ???????????????????????????????????????????????
        ("GET", "/providers")      => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.providers.iter().map(|p| format!(r#"{{"id":"{}","name":"{}","model":"{}","active":{}}}"#, p.id,esc(&p.name),esc(&p.model),p.id==s.active_provider)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/providers/set") => { let id = extract_json_str(body,"id").unwrap_or_default(); if !id.is_empty() { STATE.lock().unwrap().active_provider = id.clone(); } format!(r#"{{"ok":true,"active":"{}"}}"#, id) }

        // ?? ZeroClaw: Cron ????????????????????????????????????????????????????
        ("GET", "/cron")           => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.cron_jobs.iter().map(|j| format!(r#"{{"id":"{}","action":"{}","interval_ms":{},"enabled":{}}}"#, j.id,esc(&j.action),j.interval_ms,j.enabled)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/cron/add")      => { add_cron(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/cron/remove")   => { let id = extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().cron_jobs.retain(|j| j.id != id); r#"{"ok":true}"#.to_string() }

        // ?? NanoClaw: Audit + task logs ???????????????????????????????????????
        ("GET", "/task_log")       => get_task_log_json(),
        ("GET", "/audit_log")      => get_audit_log_json(),
        ("GET", "/checkpoints")    => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.checkpoints.iter().map(|(k,v)| format!(r#"{{{}:{}}}"#, json_str(k), json_str(v))).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/checkpoint")    => { let k = extract_json_str(body,"id").unwrap_or_default(); let v = extract_json_str(body,"data").unwrap_or_default(); if !k.is_empty() { STATE.lock().unwrap().checkpoints.insert(k,v); } r#"{"ok":true}"#.to_string() }

        // ?? Fallthrough: queue to Java ????????????????????????????????????????
        _ => queue_to_java(path_clean.trim_start_matches('/'), body),
    }
}

// ??? Background threads ???????????????????????????????????????????????????????

fn run_trigger_watcher() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        // Time triggers
        let tt: Vec<Trigger> = s.triggers.iter().filter(|t| t.trigger_type=="time" && !t.fired).cloned().collect();
        for t in tt {
            let fire_at = t.value.parse::<u128>().unwrap_or(0);
            if fire_at > 0 && now >= fire_at {
                s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","type":"time"}}"#, t.id, esc(&t.action)));
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) { tr.fired = !tr.repeat; }
            }
        }
        // Heartbeat items (OpenClaw-pm pattern)
        let hb: Vec<HeartbeatItem> = s.heartbeat_items.iter().filter(|i| i.enabled && (i.interval_ms == 0 || now - i.last_run >= i.interval_ms)).cloned().collect();
        for item in hb {
            s.heartbeat_log.push_back(format!(r#"{{"id":"{}","check":"{}","ts":{}}}"#, item.id, esc(&item.check), now));
            if s.heartbeat_log.len() > 500 { s.heartbeat_log.pop_front(); }
            s.fired_triggers.push_back(format!(r#"{{"trigger":"hb_{}","action":"{}","check":"{}"}}"#, item.id, esc(&item.action), esc(&item.check)));
            if let Some(i) = s.heartbeat_items.iter_mut().find(|x| x.id == item.id) { i.last_run = now; if i.interval_ms == 0 { i.enabled = false; } }
        }
    }
}

fn run_cron_scheduler() {
    loop {
        thread::sleep(Duration::from_secs(5));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let jobs: Vec<CronJob> = s.cron_jobs.iter().filter(|j| j.enabled && now - j.last_run >= j.interval_ms).cloned().collect();
        for job in jobs {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"cron_{}","action":"{}","type":"cron"}}"#, job.id, esc(&job.action)));
            if let Some(j) = s.cron_jobs.iter_mut().find(|x| x.id == job.id) { j.last_run = now; }
        }
    }
}

// ??? Feature implementations ??????????????????????????????????????????????????

fn search_memory(path: &str) -> String {
    let query = path.find("q=").map(|i| &path[i+2..]).unwrap_or("").replace('+', " ");
    let query_lower = query.to_lowercase();
    let mut s = STATE.lock().unwrap();
    let mut results: Vec<(f32, String, String, u128)> = s.memory_index.iter_mut()
        .filter_map(|e| {
            let mut score = 0.0f32;
            if e.key.to_lowercase() == query_lower { score += 10.0; }
            if e.key.to_lowercase().contains(&query_lower) { score += 5.0; }
            let vl = e.value.to_lowercase();
            for w in query_lower.split_whitespace() { if vl.contains(w) { score += 1.0; } }
            for tag in &e.tags { if tag.to_lowercase().contains(&query_lower) { score += 2.0; } }
            if score > 0.0 { e.relevance = (e.relevance + 0.1).min(5.0); e.access_count += 1; Some((score, e.key.clone(), e.value.clone(), e.ts)) } else { None }
        }).collect();
    results.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let items: Vec<String> = results.iter().take(10).map(|(sc,k,v,ts)|
        format!(r#"{{"key":"{}","value":"{}","score":{:.1},"ts":{}}}"#, esc(k),esc(v),sc,ts)
    ).collect();
    format!("[{}]", items.join(","))
}

fn compact_context(s: &mut KiraState) {
    let drain = s.context_turns.len() - 20;
    let old: Vec<ContextTurn> = s.context_turns.drain(..drain).collect();
    let summary: String = old.iter().map(|t| format!("[{}]{}", t.role, &t.content[..t.content.len().min(60)])).collect::<Vec<_>>().join(";");
    s.context_compact = if s.context_compact.is_empty() { summary } else { format!("{}; {}", s.context_compact, summary) };
}

fn get_context_json() -> String {
    let s = STATE.lock().unwrap();
    let turns: Vec<String> = s.context_turns.iter().map(|t|
        format!(r#"{{"role":"{}","content":"{}","tokens":{}}}"#, t.role, esc(&t.content[..t.content.len().min(300)]), t.tokens)
    ).collect();
    format!(r#"{{"compact":{},"turns":[{}],"total_tokens":{},"memory_md":{}}}"#,
        json_str(&s.context_compact), turns.join(","), s.total_tokens, json_str(&s.memory_md[..s.memory_md.len().min(1000)]))
}

fn get_skills_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.skills.values().map(|sk|
        format!(r#"{{"name":"{}","description":"{}","trigger":"{}","enabled":{},"usage_count":{}}}"#,
            esc(&sk.name), esc(&sk.description), esc(&sk.trigger), sk.enabled, sk.usage_count)
    ).collect();
    format!("[{}]", items.join(","))
}

fn get_task_log_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.task_log.iter().skip(s.task_log.len().saturating_sub(50)).map(|t|
        format!(r#"{{"task_id":"{}","step":{},"action":"{}","result":"{}","success":{},"time":{}}}"#,
            esc(&t.task_id),t.step,esc(&t.action),esc(&t.result),t.success,t.time)
    ).collect();
    format!("[{}]", items.join(","))
}

fn get_audit_log_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.audit_log.iter().skip(s.audit_log.len().saturating_sub(100)).map(|a|
        format!(r#"{{"session":"{}","tool":"{}","input":"{}","output":"{}","success":{},"blocked":{},"ts":{}}}"#,
            esc(&a.session),esc(&a.tool),esc(&a.input),esc(&a.output),a.success,a.blocked,a.ts)
    ).collect();
    format!("[{}]", items.join(","))
}

fn register_skill(body: &str) {
    let name    = extract_json_str(body,"name").unwrap_or_default();
    let desc    = extract_json_str(body,"description").unwrap_or_default();
    let trigger = extract_json_str(body,"trigger").unwrap_or_default();
    let content = extract_json_str(body,"content").unwrap_or_default();
    if !name.is_empty() {
        STATE.lock().unwrap().skills.insert(name.clone(), Skill { name, description: desc, trigger, content, enabled: true, usage_count: 0 });
    }
}

fn add_trigger(body: &str) {
    let id     = extract_json_str(body,"id").unwrap_or_else(|| gen_id());
    let ttype  = extract_json_str(body,"type").unwrap_or_default();
    let value  = extract_json_str(body,"value").unwrap_or_default();
    let action = extract_json_str(body,"action").unwrap_or_default();
    let repeat = body.contains("\"repeat\":true");
    STATE.lock().unwrap().triggers.push(Trigger { id, trigger_type: ttype, value, action, fired: false, repeat });
}

fn add_heartbeat(body: &str) {
    let id       = extract_json_str(body,"id").unwrap_or_else(|| gen_id());
    let check    = extract_json_str(body,"check").unwrap_or_default();
    let action   = extract_json_str(body,"action").unwrap_or_default();
    let interval = extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
    let mut s    = STATE.lock().unwrap();
    s.heartbeat_items.retain(|i| i.id != id);
    s.heartbeat_items.push(HeartbeatItem { id, check, action, enabled: true, last_run: 0, interval_ms: interval });
}

fn add_cron(body: &str) {
    let id       = extract_json_str(body,"id").unwrap_or_else(|| gen_id());
    let action   = extract_json_str(body,"action").unwrap_or_default();
    let interval = extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(3600000);
    let expr     = extract_json_str(body,"expression").unwrap_or_default();
    STATE.lock().unwrap().cron_jobs.push(CronJob { id, expression: expr, action, last_run: 0, interval_ms: interval, enabled: true });
}

fn fire_notif_triggers(s: &mut KiraState, pkg: &str, title: &str, text: &str) {
    let combined = format!("{} {} {}", pkg, title, text).to_lowercase();
    let tt: Vec<Trigger> = s.triggers.iter().filter(|t| (t.trigger_type=="keyword_notif"||t.trigger_type=="app_notif") && !t.fired).cloned().collect();
    for t in tt {
        let hit = match t.trigger_type.as_str() {
            "keyword_notif" => combined.contains(&t.value.to_lowercase()),
            "app_notif"     => pkg == t.value,
            _ => false,
        };
        if hit {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","notif":"{}:{}"  }}"#, t.id, esc(&t.action), esc(title), esc(text)));
            if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) { tr.fired = !tr.repeat; }
        }
    }
}

fn fire_battery_triggers(s: &mut KiraState, pct: i32, prev: i32) {
    let tt: Vec<Trigger> = s.triggers.iter().filter(|t| t.trigger_type=="battery_low" && !t.fired).cloned().collect();
    for t in tt {
        let threshold = t.value.parse::<i32>().unwrap_or(20);
        if pct <= threshold && prev > threshold {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","battery":{}}}"#, t.id, esc(&t.action), pct));
            if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) { tr.fired = !tr.repeat; }
        }
    }
}

fn do_audit(s: &mut KiraState, session: &str, tool: &str, input: &str, output: &str, success: bool, blocked: bool) {
    s.audit_log.push_back(AuditEntry {
        session: session.to_string(), tool: tool.to_string(),
        input:  input[..input.len().min(200)].to_string(),
        output: output[..output.len().min(200)].to_string(),
        ts: now_ms(), success, blocked,
    });
    if s.audit_log.len() > 5000 { s.audit_log.pop_front(); }
}

fn queue_to_java(endpoint: &str, body: &str) -> String {
    let id = gen_id();
    let payload = if body.is_empty() { format!(r#"{{"endpoint":"{}","data":{{}}}}"#, endpoint) }
                  else               { format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint, body) };
    STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
    let timeout = match endpoint { "install_apk"|"take_video" => 60000, _ => 10000 };
    wait_result(&id, timeout).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
}

fn wait_result(id: &str, ms: u64) -> Option<String> {
    let t = std::time::Instant::now();
    loop {
        { let mut s = STATE.lock().unwrap(); if let Some(r) = s.results.remove(id) { return Some(r); } }
        if t.elapsed().as_millis() as u64 >= ms { return None; }
        thread::sleep(Duration::from_millis(8));
    }
}

// ??? ZeroClaw: 17 providers ???????????????????????????????????????????????????

fn get_policy_json() -> String {
    let s = STATE.lock().unwrap();
    format!(r#"{{"allowlist":[{}],"denylist":[{}]}}"#,
        s.tool_allowlist.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","),
        s.tool_denylist.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","))
}

fn check_tool_policy(body: &str) -> String {
    let tool = extract_json_str(body, "tool").unwrap_or_default();
    let s = STATE.lock().unwrap();
    let blocked = s.tool_denylist.iter().any(|d| d == &tool || tool.starts_with(d.as_str()));
    let allowed = s.tool_allowlist.is_empty() || s.tool_allowlist.iter().any(|a| a == &tool);
    format!(r#"{{"tool":"{}","blocked":{},"allowed":{}}}"#, esc(&tool), blocked, allowed && !blocked)
}

fn get_nodes_json() -> String {
    let s = STATE.lock().unwrap();
    let now = now_ms();
    let items: Vec<String> = s.node_caps.values().map(|n|
        format!(r#"{{"id":"{}","platform":"{}","caps":[{}],"online":{},"last_seen":{}}}"#,
            esc(&n.node_id), esc(&n.platform),
            n.caps.iter().map(|c| format!("\"{}\"",esc(c))).collect::<Vec<_>>().join(","),
            n.online && now - n.last_seen < 30000, n.last_seen)
    ).collect();
    format!("[{}]", items.join(","))
}

fn register_node(body: &str) {
    let id       = extract_json_str(body, "node_id").unwrap_or_else(|| gen_id());
    let platform = extract_json_str(body, "platform").unwrap_or_else(|| "android".to_string());
    let caps_str = extract_json_str(body, "caps").unwrap_or_default();
    let caps: Vec<String> = caps_str.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
    STATE.lock().unwrap().node_caps.insert(id.clone(), NodeCapability { node_id: id, caps, platform, online: true, last_seen: now_ms() });
}

fn get_stream_json() -> String {
    let mut s = STATE.lock().unwrap();
    let chunks: Vec<String> = s.webhook_events.iter()
        .filter(|e| e.contains("stream_chunk"))
        .cloned()
        .collect();
    s.webhook_events.retain(|e| !e.contains("stream_chunk"));
    format!("[{}]", chunks.join(","))
}

fn new_session(body: &str) -> String {
    let id      = extract_json_str(body, "id").unwrap_or_else(|| gen_id());
    let channel = extract_json_str(body, "channel").unwrap_or_else(|| "kira".to_string());
    let ts = now_ms();
    let sess = Session { id: id.clone(), channel, turns: 0, tokens: 0, created: ts, last_msg: ts };
    STATE.lock().unwrap().sessions.insert(id.clone(), sess);
    format!(r#"{{"ok":true,"id":"{}"}}"#, id)
}

fn get_credential(body: &str) -> String {
    let name = extract_json_str(body, "name").unwrap_or_default();
    let s = STATE.lock().unwrap();
    match s.credentials.get(&name) {
        Some(enc) => {
            let key = derive_key(&name);
            let dec = xor_crypt(enc, &key);
            let val = String::from_utf8_lossy(&dec).to_string();
            format!(r#"{{"name":"{}","value":"{}"}}"#, esc(&name), esc(&val))
        }
        None => format!(r#"{{"error":"credential '{}' not found"}}"#, esc(&name))
    }
}


// --- Rou Bao streaming ---
fn stream_poll() -> String {
    let mut s = STATE.lock().unwrap();
    let chunks: Vec<String> = s.stream_chunks.drain(..)
        .map(|c| format!(r#"{{"session_id":"{}","text":"{}","done":{},"ts":{}}}"#, esc(&c.session_id), esc(&c.text), c.done, c.ts))
        .collect();
    format!("[{}]", chunks.join(","))
}
fn push_stream_chunk(text: &str) {
    let mut s = STATE.lock().unwrap();
    let sid = s.active_session.clone();
    s.stream_chunks.push_back(StreamChunk { session_id: sid, text: text.to_string(), done: false, ts: now_ms() });
    if s.stream_chunks.len() > 1000 { s.stream_chunks.pop_front(); }
}
fn begin_stream(sid: &str) {
    STATE.lock().unwrap().stream_sessions.insert(sid.to_string(), StreamSession { id: sid.to_string(), active: true, started: now_ms(), chunks: 0 });
}
fn end_stream(sid: &str) {
    let mut s = STATE.lock().unwrap();
    s.stream_chunks.push_back(StreamChunk { session_id: sid.to_string(), text: String::new(), done: true, ts: now_ms() });
    if let Some(sess) = s.stream_sessions.get_mut(sid) { sess.active = false; }
}
// --- OpenClaw relay ---
fn relay_msg(body: &str) -> String {
    let ch  = extract_json_str(body,"channel").unwrap_or_default();
    let msg = extract_json_str(body,"message").unwrap_or_default();
    let ts  = now_ms();
    STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"type":"relay","channel":"{}","message":"{}","ts":{}}}"#, esc(&ch), esc(&msg), ts));
    r#"{"ok":true}"#.to_string()
}
// --- ZeroClaw cache ---
fn cache_get(path: &str) -> String {
    let key = path.find("key=").map(|i| &path[i+4..]).unwrap_or("").split('&').next().unwrap_or("");
    let s = STATE.lock().unwrap();
    let now = now_ms();
    match s.response_cache.get(key) {
        Some(e) if e.expires_at > now => format!(r#"{{"key":"{}","value":"{}","ttl":{}}}"#, esc(key), esc(&e.value), e.expires_at - now),
        Some(_) => r#"{"error":"expired"}"#.to_string(),
        None    => r#"{"error":"not_found"}"#.to_string(),
    }
}
fn cache_post(body: &str) -> String {
    let k   = extract_json_str(body,"key").unwrap_or_default();
    let v   = extract_json_str(body,"value").unwrap_or_default();
    let ttl = extract_json_str(body,"ttl_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(300000);
    let exp = now_ms() + ttl;
    STATE.lock().unwrap().response_cache.insert(k, CacheEntry { value: v, expires_at: exp });
    r#"{"ok":true}"#.to_string()
}
// --- NanoClaw budget ---
fn get_budget_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.tool_iterations.iter()
        .map(|(k,v)| format!(r#"{{"session":"{}","used":{},"remaining":{}}}"#, esc(k), v, s.max_tool_iters.saturating_sub(*v)))
        .collect();
    format!(r#"{{"max":{},"sessions":[{}]}}"#, s.max_tool_iters, items.join(","))
}
// --- OpenClaw knowledge base ---
fn get_kb_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.knowledge_base.iter()
        .map(|e| format!(r#"{{"id":"{}","title":"{}","snippet":"{}","tags":[{}],"ts":{}}}"#,
            esc(&e.id), esc(&e.title),
            esc(&e.content[..e.content.len().min(100)]),
            e.tags.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","),
            e.ts))
        .collect();
    format!("[{}]", items.join(","))
}
fn add_kb_entry(body: &str) {
    let id      = extract_json_str(body,"id").unwrap_or_else(gen_id);
    let title   = extract_json_str(body,"title").unwrap_or_default();
    let content = extract_json_str(body,"content").unwrap_or_default();
    let tags_s  = extract_json_str(body,"tags").unwrap_or_default();
    let tags: Vec<String> = tags_s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let mut s   = STATE.lock().unwrap();
    s.knowledge_base.retain(|e| e.id != id);
    s.knowledge_base.push(KbEntry { id, title, content, tags, ts: now_ms() });
    if s.knowledge_base.len() > 10000 { s.knowledge_base.remove(0); }
}
fn kb_search(path: &str) -> String {
    let query = path.find("q=").map(|i| &path[i+2..]).unwrap_or("").to_lowercase();
    let s = STATE.lock().unwrap();
    let mut results: Vec<(u32, &KbEntry)> = s.knowledge_base.iter().filter_map(|e| {
        let mut score = 0u32;
        if e.title.to_lowercase().contains(&query) { score += 10; }
        if e.content.to_lowercase().contains(&query) { score += 5; }
        for tag in &e.tags { if tag.to_lowercase().contains(&query) { score += 3; } }
        if score > 0 { Some((score, e)) } else { None }
    }).collect();
    results.sort_by(|a,b| b.0.cmp(&a.0));
    let items: Vec<String> = results.iter().take(10).map(|(sc,e)|
        format!(r#"{{"id":"{}","title":"{}","content":"{}","score":{}}}"#,
            esc(&e.id), esc(&e.title), esc(&e.content[..e.content.len().min(300)]), sc))
        .collect();
    format!("[{}]", items.join(","))
}
// --- ZeroClaw Prometheus metrics ---
fn get_metrics_text() -> String {
    let s = STATE.lock().unwrap();
    format!(
        "kira_uptime_ms {}\nkira_requests_total {}\nkira_tool_calls {}\nkira_notifications {}\nkira_memory_entries {}\nkira_battery {}\nkira_skills {}\nkira_kb_entries {}\nkira_event_feed {}\n",
        now_ms()-s.uptime_start, s.request_count, s.tool_call_count,
        s.notifications.len(), s.memory_index.len(), s.battery_pct,
        s.skills.len(), s.knowledge_base.len(), s.event_feed.len()
    )
}
// --- OpenClaw event feed ---
fn get_event_feed() -> String {
    let s = STATE.lock().unwrap();
    let skip = s.event_feed.len().saturating_sub(100);
    let items: Vec<String> = s.event_feed.iter().skip(skip)
        .map(|e| format!(r#"{{"event":"{}","data":"{}","ts":{}}}"#, esc(&e.event), esc(&e.data), e.ts))
        .collect();
    format!("[{}]", items.join(","))
}
fn push_event_feed(event: &str, data: &str) {
    let mut s = STATE.lock().unwrap();
    s.event_feed.push_back(EventFeedEntry { event: event.to_string(), data: data.to_string(), ts: now_ms() });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

fn make_providers() -> Vec<Provider> {
    vec![
        Provider { id:"groq".into(),       name:"Groq".into(),          base_url:"https://api.groq.com/openai/v1".into(),                              model:"llama-3.1-8b-instant".into() },
        Provider { id:"openai".into(),     name:"OpenAI".into(),         base_url:"https://api.openai.com/v1".into(),                                   model:"gpt-4o-mini".into() },
        Provider { id:"anthropic".into(),  name:"Anthropic".into(),      base_url:"https://api.anthropic.com/v1".into(),                                model:"claude-3-haiku-20240307".into() },
        Provider { id:"gemini".into(),     name:"Gemini".into(),         base_url:"https://generativelanguage.googleapis.com/v1beta/openai".into(),     model:"gemini-2.0-flash".into() },
        Provider { id:"deepseek".into(),   name:"DeepSeek".into(),       base_url:"https://api.deepseek.com/v1".into(),                                 model:"deepseek-chat".into() },
        Provider { id:"openrouter".into(), name:"OpenRouter".into(),     base_url:"https://openrouter.ai/api/v1".into(),                                model:"openrouter/auto".into() },
        Provider { id:"ollama".into(),     name:"Ollama Local".into(),   base_url:"http://localhost:11434/v1".into(),                                   model:"llama3".into() },
        Provider { id:"together".into(),   name:"Together AI".into(),    base_url:"https://api.together.xyz/v1".into(),                                 model:"meta-llama/Llama-3-8b-chat-hf".into() },
        Provider { id:"mistral".into(),    name:"Mistral".into(),        base_url:"https://api.mistral.ai/v1".into(),                                   model:"mistral-small-latest".into() },
        Provider { id:"cohere".into(),     name:"Cohere".into(),         base_url:"https://api.cohere.ai/v1".into(),                                    model:"command-r".into() },
        Provider { id:"perplexity".into(), name:"Perplexity".into(),     base_url:"https://api.perplexity.ai".into(),                                   model:"llama-3.1-sonar-small-128k-online".into() },
        Provider { id:"xai".into(),        name:"xAI Grok".into(),       base_url:"https://api.x.ai/v1".into(),                                         model:"grok-2-latest".into() },
        Provider { id:"cerebras".into(),   name:"Cerebras".into(),       base_url:"https://api.cerebras.ai/v1".into(),                                  model:"llama3.1-8b".into() },
        Provider { id:"fireworks".into(),  name:"Fireworks".into(),      base_url:"https://api.fireworks.ai/inference/v1".into(),                       model:"accounts/fireworks/models/llama-v3p1-8b-instruct".into() },
        Provider { id:"sambanova".into(),  name:"SambaNova".into(),      base_url:"https://api.sambanova.ai/v1".into(),                                 model:"Meta-Llama-3.1-8B-Instruct".into() },
        Provider { id:"novita".into(),     name:"Novita AI".into(),      base_url:"https://api.novita.ai/v3/openai".into(),                             model:"llama-3.1-8b-instruct".into() },
        Provider { id:"custom".into(),     name:"Custom".into(),         base_url:"".into(),                                                             model:"".into() },
    ]
}

// ??? ZeroClaw: simple XOR+hash crypto (no C deps for Android) ????????????????
fn derive_key(name: &str) -> Vec<u8> {
    let mut key = vec![0u8; 32];
    for (i, b) in name.bytes().enumerate() { key[i % 32] ^= b.wrapping_add((i as u8).wrapping_mul(7)); }
    key
}
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() { return data.to_vec(); }
    data.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect()
}

// ??? Utilities ?????????????????????????????????????????????????????????????????
fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn gen_id()  -> String { format!("k{}", now_ms()) }
fn estimate_tokens(s: &str) -> u32 { (s.len() / 4).max(1) as u32 }
fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "") }
fn json_str(s: &str) -> String { format!("\"{}\"", esc(s)) }
fn json_str_arr(v: &[String]) -> String { format!("[{}]", v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",")) }
fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start  = json.find(&search)? + search.len();
    let end    = json[start..].find('"')? + start;
    Some(json[start..end].to_string())
}
