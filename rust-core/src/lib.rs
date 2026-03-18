// Kira Rust Core v8 — v38 edition
//
// All v38 changes live here in Rust:
//   - Setup wizard state (pages, collected values, provider selection)
//   - Custom AI provider persistence + full 17-provider registry
//   - Shizuku status tracking (set from Java, read from HTTP + JNI)
//   - UI theme config (accent colour, star field settings)
//   - Star field physics state (star positions, tilt offset, twinkle phases)
//   - Sensor data ingestion (accelerometer → star parallax)
//   - App-level config (KiraConfig equivalent, persisted in Rust state)
//   - All localhost:7070 endpoints now backed by real Rust state
//   + Everything from v7 (sessions, memory, audit, triggers, cron, skills…)

#![allow(non_snake_case, dead_code)]

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ─── v38: Setup Wizard State ─────────────────────────────────────────────────

#[derive(Default, Clone)]
struct SetupState {
    current_page:    u8,           // 0-5
    done:            bool,
    // Page 1 — API key + provider
    api_key:         String,
    base_url:        String,
    selected_provider_id: String,
    custom_url:      String,       // populated when provider == "custom"
    // Page 2 — user name
    user_name:       String,
    // Page 3 — model
    model:           String,
    // Page 4 — Telegram
    tg_token:        String,
    tg_allowed_id:   i64,
    // Quote cycling index
    quote_index:     usize,
}

// ─── v38: UI Theme Config ─────────────────────────────────────────────────────

#[derive(Clone)]
struct ThemeConfig {
    accent_color:    u32,   // ARGB, default 0xFFDC143C (crimson)
    bg_color:        u32,   // 0xFF050508
    card_color:      u32,   // 0xFF0e0e18
    muted_color:     u32,   // 0xFF666680
    // Star field
    star_count:      u32,   // 110
    star_speed:      f32,   // parallax multiplier
    star_tilt_x:     f32,   // current sensor tilt X
    star_tilt_y:     f32,   // current sensor tilt Y
    star_parallax_x: f32,   // smoothed parallax offset X
    star_parallax_y: f32,   // smoothed parallax offset Y
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            accent_color:    0xFFDC143C,
            bg_color:        0xFF050508,
            card_color:      0xFF0e0e18,
            muted_color:     0xFF666680,
            star_count:      110,
            star_speed:      0.013,
            star_tilt_x:     0.0,
            star_tilt_y:     0.0,
            star_parallax_x: 0.0,
            star_parallax_y: 0.0,
        }
    }
}

// ─── v38: KiraConfig (replaces Java SharedPreferences as source of truth) ─────

#[derive(Clone)]
struct KiraConfig {
    user_name:          String,
    api_key:            String,
    base_url:           String,
    model:              String,
    vision_model:       String,
    persona:            String,
    tg_token:           String,
    tg_allowed:         i64,
    agent_max_steps:    u32,
    agent_auto_approve: bool,
    heartbeat_interval: u32,   // minutes
    setup_done:         bool,
}

impl Default for KiraConfig {
    fn default() -> Self {
        KiraConfig {
            user_name:          "User".to_string(),
            api_key:            String::new(),
            base_url:           "https://api.groq.com/openai/v1".to_string(),
            model:              "llama-3.1-8b-instant".to_string(),
            vision_model:       String::new(),
            persona:            String::new(),
            tg_token:           String::new(),
            tg_allowed:         0,
            agent_max_steps:    25,
            agent_auto_approve: true,
            heartbeat_interval: 30,
            setup_done:         false,
        }
    }
}

// ─── v38: Shizuku Status ─────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct ShizukuStatus {
    installed:         bool,   // binder alive
    permission_granted:bool,   // checkSelfPermission == GRANTED
    last_checked_ms:   u128,
    error_msg:         String,
}

// ─── Core State (v7 + v38 additions) ─────────────────────────────────────────

#[derive(Default)]
struct KiraState {
    // Device
    screen_nodes:      String,
    screen_pkg:        String,
    battery_pct:       i32,
    battery_charging:  bool,

    // Notifications
    notifications:     VecDeque<Notif>,

    // Command queue
    pending_cmds:      VecDeque<(String, String)>,
    results:           HashMap<String, String>,

    // Sessions
    sessions:          HashMap<String, Session>,
    active_session:    String,

    // Memory (MEMORY.md pattern)
    memory_md:         String,
    daily_log:         VecDeque<String>,
    soul_md:           String,

    // Skills (SKILL.md pattern)
    skills:            HashMap<String, Skill>,
    skill_turn_inject: Vec<String>,

    // Context engine
    context_turns:     VecDeque<ContextTurn>,
    context_compact:   String,
    agent_context:     String,
    total_tokens:      u64,

    // Node capabilities
    node_caps:         HashMap<String, NodeCapability>,

    // Triggers
    triggers:          Vec<Trigger>,
    fired_triggers:    VecDeque<String>,
    webhook_events:    VecDeque<String>,

    // Heartbeat
    heartbeat_items:   Vec<HeartbeatItem>,
    heartbeat_log:     VecDeque<String>,

    // Providers (ZeroClaw: 17+)
    providers:         Vec<Provider>,
    active_provider:   String,

    // Credentials
    credentials:       HashMap<String, Vec<u8>>,

    // Cron
    cron_jobs:         Vec<CronJob>,

    // Memory index
    memory_index:      Vec<MemoryEntry>,

    // Tool policy
    tool_allowlist:    Vec<String>,
    tool_denylist:     Vec<String>,

    // Tool iteration counter
    tool_iterations:   HashMap<String, u32>,
    max_tool_iters:    u32,

    // Audit
    audit_log:         VecDeque<AuditEntry>,

    // Task log
    task_log:          VecDeque<TaskStep>,
    checkpoints:       HashMap<String, String>,

    // Streaming
    stream_chunks:     VecDeque<StreamChunk>,
    stream_sessions:   HashMap<String, StreamSession>,

    // Cache
    response_cache:    HashMap<String, CacheEntry>,

    // Knowledge base
    knowledge_base:    Vec<KbEntry>,

    // Event feed
    event_feed:        VecDeque<EventFeedEntry>,

    // Stats
    uptime_start:      u128,
    request_count:     u64,
    tool_call_count:   u64,

    // ── v38 additions ──────────────────────────────────────────────────────
    setup:             SetupState,
    theme:             ThemeConfig,
    config:            KiraConfig,
    shizuku:           ShizukuStatus,
}

// ─── Sub-structs ──────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Session {
    id: String, channel: String, turns: u32, tokens: u64, created: u128, last_msg: u128,
}
#[derive(Clone, Default)]
struct Skill {
    name: String, description: String, trigger: String, content: String, enabled: bool, usage_count: u32,
}
#[derive(Clone)]
struct ContextTurn {
    role: String, content: String, ts: u128, tokens: u32, session: String,
}
#[derive(Clone, Default)]
struct NodeCapability {
    node_id: String, caps: Vec<String>, platform: String, online: bool, last_seen: u128,
}
#[derive(Clone)]
struct Trigger {
    id: String, trigger_type: String, value: String, action: String, fired: bool, repeat: bool,
}
#[derive(Clone)]
struct HeartbeatItem {
    id: String, check: String, action: String, enabled: bool, last_run: u128, interval_ms: u128,
}
#[derive(Clone, Default)]
struct Provider {
    id: String, name: String, base_url: String, model: String,
}
#[derive(Clone)]
struct CronJob {
    id: String, expression: String, action: String, last_run: u128, interval_ms: u128, enabled: bool,
}
#[derive(Clone)]
struct MemoryEntry {
    key: String, value: String, tags: Vec<String>, ts: u128, relevance: f32, access_count: u32,
}
#[derive(Clone)]
struct AuditEntry {
    session: String, tool: String, input: String, output: String, ts: u128, success: bool, blocked: bool,
}
#[derive(Clone)]
struct TaskStep {
    task_id: String, step: u32, action: String, result: String, time: u128, success: bool,
}
#[derive(Clone, Default)]
struct StreamChunk { session_id: String, text: String, done: bool, ts: u128 }
#[derive(Clone, Default)]
struct StreamSession { id: String, active: bool, started: u128, chunks: u32 }
#[derive(Clone)]
struct CacheEntry { value: String, expires_at: u128 }
#[derive(Clone)]
struct KbEntry { id: String, title: String, content: String, tags: Vec<String>, ts: u128 }
#[derive(Clone)]
struct EventFeedEntry { event: String, data: String, ts: u128 }
struct Notif { pkg: String, title: String, text: String, time: u128 }

// ─── Global State ─────────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<KiraState>> = Arc::new(Mutex::new(KiraState {
        battery_pct:     100,
        max_tool_iters:  20,
        active_session:  "default".to_string(),
        active_provider: "groq".to_string(),
        soul_md: "You are Kira, a powerful Android AI agent. You are helpful, proactive, and autonomous.".to_string(),
        theme: ThemeConfig::default(),
        config: KiraConfig::default(),
        setup: SetupState::default(),
        shizuku: ShizukuStatus::default(),
        ..Default::default()
    }));
}

// ─── JNI Bridge ───────────────────────────────────────────────────────────────

mod jni_bridge {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    fn cs(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }

    // ── Lifecycle ────────────────────────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, port: i32,
    ) {
        let p = port as u16;
        {
            let mut s = STATE.lock().unwrap();
            s.uptime_start   = now_ms();
            s.providers      = make_providers();
            let sess = Session { id: "default".into(), channel: "kira".into(), created: now_ms(), last_msg: now_ms(), ..Default::default() };
            s.sessions.insert("default".into(), sess);
        }
        thread::spawn(move || run_http(p));
        thread::spawn(run_trigger_watcher);
        thread::spawn(run_cron_scheduler);
    }

    // ── v38: KiraConfig sync (Java → Rust on every save) ────────────────────

    /// Java calls this whenever SharedPreferences are saved.
    /// Rust becomes the single source of truth for config.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_syncConfig(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        user_name:    *const c_char,
        api_key:      *const c_char,
        base_url:     *const c_char,
        model:        *const c_char,
        vision_model: *const c_char,
        persona:      *const c_char,
        tg_token:     *const c_char,
        tg_allowed:   i64,
        max_steps:    i32,
        auto_approve: bool,
        heartbeat:    i32,
        setup_done:   bool,
    ) {
        let mut s = STATE.lock().unwrap();
        s.config.user_name          = cs(user_name);
        s.config.api_key            = cs(api_key);
        s.config.base_url           = cs(base_url);
        s.config.model              = cs(model);
        s.config.vision_model       = cs(vision_model);
        s.config.persona            = cs(persona);
        s.config.tg_token           = cs(tg_token);
        s.config.tg_allowed         = tg_allowed;
        s.config.agent_max_steps    = max_steps as u32;
        s.config.agent_auto_approve = auto_approve;
        s.config.heartbeat_interval = heartbeat as u32;
        s.config.setup_done         = setup_done;
        // Mirror active provider from base_url
        let bu = s.config.base_url.clone();
        if let Some(p) = s.providers.iter().find(|p| p.base_url == bu) {
            s.active_provider = p.id.clone();
        }
    }

    /// Java reads config back from Rust (returns JSON).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getConfig(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = config_to_json(&s.config);
        CString::new(json).unwrap_or_default().into_raw()
    }

    // ── v38: Setup wizard state ──────────────────────────────────────────────

    /// Called by SetupActivity on each page advance with collected values.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateSetupPage(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        page:        i32,
        api_key:     *const c_char,
        base_url:    *const c_char,
        model:       *const c_char,
        user_name:   *const c_char,
        tg_token:    *const c_char,
        tg_id:       i64,
    ) {
        let mut s = STATE.lock().unwrap();
        s.setup.current_page = page as u8;
        let ak = cs(api_key);   if !ak.is_empty()  { s.setup.api_key   = ak.clone();  s.config.api_key  = ak; }
        let bu = cs(base_url);  if !bu.is_empty()  { s.setup.base_url  = bu.clone();  s.config.base_url = bu; }
        let mo = cs(model);     if !mo.is_empty()  { s.setup.model     = mo.clone();  s.config.model    = mo; }
        let un = cs(user_name); if !un.is_empty()  { s.setup.user_name = un.clone();  s.config.user_name= un; }
        let tt = cs(tg_token);  if !tt.is_empty()  { s.setup.tg_token  = tt.clone();  s.config.tg_token = tt; }
        if tg_id > 0 { s.setup.tg_allowed_id = tg_id; s.config.tg_allowed = tg_id; }
    }

    /// Called when user finishes setup. Rust marks setup as done.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_completeSetup(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) {
        let mut s = STATE.lock().unwrap();
        s.setup.done       = true;
        s.config.setup_done= true;
    }

    /// Java asks Rust: has setup been completed?
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_isSetupDone(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> bool {
        STATE.lock().unwrap().config.setup_done
    }

    // ── v38: Custom provider ─────────────────────────────────────────────────

    /// Register or update a custom provider URL from the setup wizard.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setCustomProvider(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        url:   *const c_char,
        model: *const c_char,
    ) {
        let url   = cs(url);
        let model = cs(model);
        let mut s = STATE.lock().unwrap();
        s.setup.custom_url               = url.clone();
        s.setup.selected_provider_id     = "custom".to_string();
        s.config.base_url                = url.clone();
        if !model.is_empty() { s.config.model = model.clone(); }
        // Update or insert the custom provider entry in the registry
        if let Some(p) = s.providers.iter_mut().find(|p| p.id == "custom") {
            p.base_url = url;
            if !model.is_empty() { p.model = model; }
        }
        s.active_provider = "custom".to_string();
    }

    /// Set active provider by ID (from settings picker).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setActiveProvider(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        provider_id: *const c_char,
    ) -> *mut c_char {
        let id = cs(provider_id);
        let mut s = STATE.lock().unwrap();
        let found = s.providers.iter().find(|p| p.id == id).cloned();
        let result = if let Some(p) = found {
            s.active_provider  = id;
            s.config.base_url  = p.base_url.clone();
            s.config.model     = p.model.clone();
            format!(r#"{{"ok":true,"id":"{}","base_url":"{}","model":"{}"}}"#,
                esc(&s.active_provider), esc(&p.base_url), esc(&p.model))
        } else {
            format!(r#"{{"error":"unknown provider {}"}}"#, esc(&id))
        };
        CString::new(result).unwrap_or_default().into_raw()
    }

    /// Get all providers as JSON (for the settings picker in Java).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProviders(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let json = {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.providers.iter().map(|p|
                format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#,
                    esc(&p.id), esc(&p.name), esc(&p.base_url), esc(&p.model), p.id == s.active_provider)
            ).collect();
            format!("[{}]", items.join(","))
        };
        CString::new(json).unwrap_or_default().into_raw()
    }

    // ── v38: Shizuku status (set from Java, read by Rust for stats) ──────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateShizukuStatus(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        installed:          bool,
        permission_granted: bool,
        error_msg:          *const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.shizuku.installed          = installed;
        s.shizuku.permission_granted = permission_granted;
        s.shizuku.error_msg          = cs(error_msg);
        s.shizuku.last_checked_ms    = now_ms();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getShizukuJson(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = shizuku_to_json(&s.shizuku);
        CString::new(json).unwrap_or_default().into_raw()
    }

    // ── v38: Sensor / star field ─────────────────────────────────────────────

    /// Called by SetupActivity's SensorEventListener on accelerometer change.
    /// Rust smooths the parallax and stores it; Java reads it back for drawing.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateTilt(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        ax: f32, ay: f32,
    ) {
        let mut s = STATE.lock().unwrap();
        s.theme.star_tilt_x = ax;
        s.theme.star_tilt_y = ay;
        // Smooth the parallax (EMA, α=0.08)
        let target_x = -ax * s.theme.star_speed;
        let target_y =  ay * s.theme.star_speed;
        s.theme.star_parallax_x += (target_x - s.theme.star_parallax_x) * 0.08;
        s.theme.star_parallax_y += (target_y - s.theme.star_parallax_y) * 0.08;
    }

    /// Java reads the smoothed parallax to position stars.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStarParallax(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = format!(r#"{{"px":{:.6},"py":{:.6},"ax":{:.4},"ay":{:.4}}}"#,
            s.theme.star_parallax_x, s.theme.star_parallax_y,
            s.theme.star_tilt_x, s.theme.star_tilt_y);
        CString::new(json).unwrap_or_default().into_raw()
    }

    /// Get current theme colours as JSON (for Java to apply at runtime).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getTheme(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = format!(
            r#"{{"accent":{},"bg":{},"card":{},"muted":{},"star_count":{}}}"#,
            s.theme.accent_color, s.theme.bg_color,
            s.theme.card_color, s.theme.muted_color, s.theme.star_count
        );
        CString::new(json).unwrap_or_default().into_raw()
    }

    // ── v38: App stats (replaces localhost:7070/health call in MainActivity) ──

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStatsJson(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = format!(
            r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{}}}"#,
            s.memory_index.len(),
            s.context_turns.len(),
            if s.shizuku.permission_granted { "active ✓" }
            else if s.shizuku.installed     { "no permission" }
            else                            { "not running" },
            // accessibility state tracked via updateAgentContext from KiraAccessibilityService
            if !s.agent_context.is_empty()  { "enabled ✓" } else { "disabled" },
            esc(&s.config.model),
            esc(&s.config.base_url),
            now_ms().saturating_sub(s.uptime_start)
        );
        CString::new(json).unwrap_or_default().into_raw()
    }

    // ── v7: Device state (unchanged) ─────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        pkg: *const c_char, title: *const c_char, text: *const c_char,
    ) {
        let (pkg, title, text) = (cs(pkg), cs(title), cs(text));
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        fire_notif_triggers(&mut s, &pkg, &title, &text);
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
        s.daily_log.push_back(format!("[{}] {}: {}", ts, role, &content[..content.len().min(80)]));
        s.context_turns.push_back(ContextTurn { role, content, ts, tokens, session: sess_id });
        if s.context_turns.len() > 60 { compact_context(&mut s); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_indexMemory(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        key: *const c_char, value: *const c_char, tags: *const c_char,
    ) {
        let (key, value, tags_raw) = (cs(key), cs(value), cs(tags));
        let tags: Vec<String> = tags_raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        let mut s = STATE.lock().unwrap();
        s.memory_index.retain(|e| e.key != key);
        let fact = format!("- {}: {}", key, value);
        if !s.memory_md.contains(&fact) { s.memory_md.push_str(&format!("\n{}", fact)); }
        s.memory_index.push(MemoryEntry { key, value, tags, ts: now_ms(), relevance: 1.0, access_count: 0 });
        if s.memory_index.len() > 10000 {
            s.memory_index.sort_by(|a, b| a.relevance.partial_cmp(&b.relevance).unwrap_or(std::cmp::Ordering::Equal));
            s.memory_index.remove(0);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_storeCredential(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name: *const c_char, value: *const c_char,
    ) {
        let name = cs(name); let value = cs(value);
        let enc = xor_crypt(value.as_bytes(), derive_key(&name).as_slice());
        STATE.lock().unwrap().credentials.insert(name, enc);
    }

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

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_incrementToolIter(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id: *const c_char,
    ) -> i32 {
        let id = cs(session_id);
        let mut s = STATE.lock().unwrap();
        let count = { let c = s.tool_iterations.entry(id).or_insert(0); *c += 1; *c };
        s.tool_call_count += 1;
        let max = s.max_tool_iters;
        if count > max { -1 } else { count as i32 }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resetToolIter(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id: *const c_char,
    ) { STATE.lock().unwrap().tool_iterations.remove(&cs(session_id)); }

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

// ─── HTTP Server ──────────────────────────────────────────────────────────────

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
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Kira-Engine: rust-v8\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());
    STATE.lock().unwrap().request_count += 1;
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    match (method, path_clean) {

        // ── Health & stats ──────────────────────────────────────────────────
        ("GET", "/health") | ("GET", "/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"status":"ok","version":"8.0","uptime_ms":{},"requests":{},"tool_calls":{},"battery":{},"charging":{},"notifications":{},"skills":{},"triggers":{},"memory_entries":{},"total_tokens":{},"sessions":{},"setup_done":{}}}"#,
                now_ms()-s.uptime_start, s.request_count, s.tool_call_count,
                s.battery_pct, s.battery_charging,
                s.notifications.len(), s.skills.len(),
                s.triggers.iter().filter(|t|!t.fired).count(),
                s.memory_index.len(), s.total_tokens, s.sessions.len(),
                s.config.setup_done)
        }
        ("GET", "/stats") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"notifications":{},"pending_cmds":{},"task_steps":{},"audit_entries":{},"context_turns":{},"daily_log_entries":{},"skills":{},"memory_entries":{},"cron_jobs":{},"tool_calls":{},"total_tokens":{},"uptime_ms":{}}}"#,
                s.notifications.len(), s.pending_cmds.len(), s.task_log.len(),
                s.audit_log.len(), s.context_turns.len(), s.daily_log.len(),
                s.skills.len(), s.memory_index.len(), s.cron_jobs.len(),
                s.tool_call_count, s.total_tokens, now_ms()-s.uptime_start)
        }

        // ── v38: Config endpoints ───────────────────────────────────────────
        ("GET",  "/config")        => { let s = STATE.lock().unwrap(); config_to_json(&s.config) }
        ("POST", "/config")        => update_config_from_http(body),
        ("GET",  "/setup")         => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"page":{},"done":{},"user_name":"{}","model":"{}","base_url":"{}","selected_provider":"{}","custom_url":"{}","quote_index":{}}}"#,
                s.setup.current_page, s.setup.done,
                esc(&s.setup.user_name), esc(&s.setup.model), esc(&s.setup.base_url),
                esc(&s.setup.selected_provider_id), esc(&s.setup.custom_url),
                s.setup.quote_index)
        }
        ("POST", "/setup/page")    => {
            if let Some(page) = extract_json_num(body, "page") {
                STATE.lock().unwrap().setup.current_page = page as u8;
            }
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/setup/complete") => {
            let mut s = STATE.lock().unwrap();
            s.setup.done = true; s.config.setup_done = true;
            r#"{"ok":true}"#.to_string()
        }

        // ── v38: Theme + star field ─────────────────────────────────────────
        ("GET",  "/theme")         => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"accent":{},"bg":{},"card":{},"muted":{},"star_count":{},"parallax_x":{:.6},"parallax_y":{:.6}}}"#,
                s.theme.accent_color, s.theme.bg_color, s.theme.card_color,
                s.theme.muted_color, s.theme.star_count,
                s.theme.star_parallax_x, s.theme.star_parallax_y)
        }
        ("POST", "/theme/tilt")    => {
            let ax = extract_json_f32(body, "ax").unwrap_or(0.0);
            let ay = extract_json_f32(body, "ay").unwrap_or(0.0);
            let mut s = STATE.lock().unwrap();
            s.theme.star_tilt_x = ax; s.theme.star_tilt_y = ay;
            let spd = s.theme.star_speed;
            let tx = -ax * spd; let ty = ay * spd;
            s.theme.star_parallax_x += (tx - s.theme.star_parallax_x) * 0.08;
            s.theme.star_parallax_y += (ty - s.theme.star_parallax_y) * 0.08;
            format!(r#"{{"px":{:.6},"py":{:.6}}}"#, s.theme.star_parallax_x, s.theme.star_parallax_y)
        }

        // ── v38: Shizuku status ─────────────────────────────────────────────
        ("GET",  "/shizuku")       => { let s = STATE.lock().unwrap(); shizuku_to_json(&s.shizuku) }
        ("POST", "/shizuku")       => {
            let installed   = body.contains(r#""installed":true"#);
            let granted     = body.contains(r#""permission_granted":true"#);
            let err         = extract_json_str(body, "error").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            s.shizuku.installed          = installed;
            s.shizuku.permission_granted = granted;
            s.shizuku.error_msg          = err;
            s.shizuku.last_checked_ms    = now_ms();
            r#"{"ok":true}"#.to_string()
        }

        // ── v38: App stats (replaces all localhost calls from MainActivity) ─
        ("GET", "/appstats")       => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{}}}"#,
                s.memory_index.len(),
                s.context_turns.len(),
                if s.shizuku.permission_granted { "active ✓" }
                else if s.shizuku.installed     { "no permission" }
                else                            { "not running" },
                if !s.agent_context.is_empty()  { "enabled ✓" } else { "disabled" },
                esc(&s.config.model),
                esc(&s.config.base_url),
                now_ms().saturating_sub(s.uptime_start))
        }

        // ── v38: Provider management ────────────────────────────────────────
        ("GET",  "/providers")     => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.providers.iter().map(|p|
                format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#,
                    esc(&p.id), esc(&p.name), esc(&p.base_url), esc(&p.model), p.id == s.active_provider)
            ).collect();
            format!("[{}]", items.join(","))
        }
        ("POST", "/providers/set") => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            if !id.is_empty() {
                let mut s = STATE.lock().unwrap();
                let found = s.providers.iter().find(|p| p.id == id).cloned();
                if let Some(p) = found {
                    s.active_provider = id.clone();
                    s.config.base_url = p.base_url;
                    s.config.model    = p.model;
                }
            }
            format!(r#"{{"ok":true,"active":"{}"}}"#, id)
        }
        ("POST", "/providers/custom") => {
            let url   = extract_json_str(body, "url").unwrap_or_default();
            let model = extract_json_str(body, "model").unwrap_or_default();
            if !url.is_empty() {
                let mut s = STATE.lock().unwrap();
                s.setup.custom_url           = url.clone();
                s.setup.selected_provider_id = "custom".to_string();
                s.config.base_url            = url.clone();
                if !model.is_empty() { s.config.model = model.clone(); }
                if let Some(p) = s.providers.iter_mut().find(|p| p.id == "custom") {
                    p.base_url = url; if !model.is_empty() { p.model = model; }
                }
                s.active_provider = "custom".to_string();
            }
            r#"{"ok":true}"#.to_string()
        }

        // ── v7: Device state ────────────────────────────────────────────────
        ("GET", "/screen")        => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET", "/screen_pkg")    => { let p = STATE.lock().unwrap().screen_pkg.clone(); format!(r#"{{"package":"{}"}}"#, esc(&p)) }
        ("GET", "/battery")       => { let s = STATE.lock().unwrap(); format!(r#"{{"percentage":{},"charging":{}}}"#, s.battery_pct, s.battery_charging) }
        ("GET", "/notifications") => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.notifications.iter().map(|n| format!(r#"{{"pkg":"{}","title":"{}","text":"{}","time":{}}}"#, esc(&n.pkg),esc(&n.title),esc(&n.text),n.time)).collect(); format!("[{}]", items.join(",")) }

        // ── v7: Memory ──────────────────────────────────────────────────────
        ("GET", "/memory")        => { let s = STATE.lock().unwrap(); format!(r#"{{"memory_md":{},"entries":{}}}"#, json_str(&s.memory_md), s.memory_index.len()) }
        ("GET", "/memory/search") => search_memory(path),
        ("GET", "/memory/full")   => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.memory_index.iter().map(|e| format!(r#"{{"key":"{}","value":"{}","tags":{},"relevance":{:.2},"access_count":{}}}"#, esc(&e.key),esc(&e.value),json_str_arr(&e.tags),e.relevance,e.access_count)).collect(); format!("[{}]", items.join(",")) }
        ("GET", "/daily_log")     => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.daily_log.iter().cloned().map(|l| format!("\"{}\"", esc(&l))).collect(); format!("[{}]", items.join(",")) }
        ("GET", "/context")       => get_context_json(),
        ("GET", "/soul")          => { let s = STATE.lock().unwrap(); format!(r#"{{"soul":{}}}"#, json_str(&s.soul_md)) }
        ("POST","/soul")          => { let val = extract_json_str(body, "content").unwrap_or_default(); if !val.is_empty() { STATE.lock().unwrap().soul_md = val; } r#"{"ok":true}"#.to_string() }

        // ── v7: Skills ──────────────────────────────────────────────────────
        ("GET", "/skills")             => get_skills_json(),
        ("POST","/skills/register")    => { register_skill(body); r#"{"ok":true}"#.to_string() }
        ("POST","/skills/enable")      => { let name = extract_json_str(body,"name").unwrap_or_default(); if let Some(sk) = STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=true; } r#"{"ok":true}"#.to_string() }
        ("POST","/skills/disable")     => { let name = extract_json_str(body,"name").unwrap_or_default(); if let Some(sk) = STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=false; } r#"{"ok":true}"#.to_string() }

        // ── v7: Sessions ────────────────────────────────────────────────────
        ("GET",  "/sessions")     => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.sessions.values().map(|sess| format!(r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"last_msg":{}}}"#, sess.id,sess.channel,sess.turns,sess.tokens,sess.last_msg)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/sessions/new") => new_session(body),

        // ── v7: Triggers ────────────────────────────────────────────────────
        ("GET",  "/triggers")          => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.triggers.iter().map(|t| format!(r#"{{"id":"{}","type":"{}","value":"{}","fired":{},"repeat":{}}}"#, t.id,t.trigger_type,esc(&t.value),t.fired,t.repeat)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/fired_triggers")    => { let mut s = STATE.lock().unwrap(); let items: Vec<String> = s.fired_triggers.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/webhook_events")    => { let mut s = STATE.lock().unwrap(); let items: Vec<String> = s.webhook_events.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/triggers/add")      => { add_trigger(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/webhook")           => { let ts = now_ms(); STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"body":{},"ts":{}}}"#, if body.is_empty(){"{}"} else {body}, ts)); r#"{"ok":true}"#.to_string() }

        // ── v7: Heartbeat ────────────────────────────────────────────────────
        ("GET", "/heartbeat_log") => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.heartbeat_log.iter().cloned().collect(); format!("[{}]", items.join(",")) }
        ("POST","/heartbeat/add") => { add_heartbeat(body); r#"{"ok":true}"#.to_string() }

        // ── v7: Cron ─────────────────────────────────────────────────────────
        ("GET",  "/cron")         => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.cron_jobs.iter().map(|j| format!(r#"{{"id":"{}","action":"{}","interval_ms":{},"enabled":{}}}"#, j.id,esc(&j.action),j.interval_ms,j.enabled)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/cron/add")     => { add_cron(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/cron/remove")  => { let id = extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().cron_jobs.retain(|j| j.id != id); r#"{"ok":true}"#.to_string() }

        // ── v7: Audit + task logs ─────────────────────────────────────────
        ("GET", "/task_log")      => get_task_log_json(),
        ("GET", "/audit_log")     => get_audit_log_json(),
        ("GET", "/checkpoints")   => { let s = STATE.lock().unwrap(); let items: Vec<String> = s.checkpoints.iter().map(|(k,v)| format!(r#"{{{}:{}}}"#, json_str(k), json_str(v))).collect(); format!("[{}]", items.join(",")) }
        ("POST","/checkpoint")    => { let k = extract_json_str(body,"id").unwrap_or_default(); let v = extract_json_str(body,"data").unwrap_or_default(); if !k.is_empty() { STATE.lock().unwrap().checkpoints.insert(k,v); } r#"{"ok":true}"#.to_string() }

        // ── v7: KB ───────────────────────────────────────────────────────────
        ("GET",  "/kb")           => get_kb_json(),
        ("GET",  "/kb/search")    => kb_search(path),
        ("POST", "/kb/add")       => { add_kb_entry(body); r#"{"ok":true}"#.to_string() }

        // ── v7: Events + metrics ─────────────────────────────────────────────
        ("GET", "/events")        => get_event_feed(),
        ("POST","/events")        => { let e = extract_json_str(body,"event").unwrap_or_default(); let d = extract_json_str(body,"data").unwrap_or_default(); push_event_feed(&e,&d); r#"{"ok":true}"#.to_string() }
        ("GET", "/metrics")       => get_metrics_text(),
        ("GET", "/budget")        => get_budget_json(),
        ("GET", "/stream")        => stream_poll(),
        ("POST","/stream/chunk")  => { let t = extract_json_str(body,"text").unwrap_or_default(); push_stream_chunk(&t); r#"{"ok":true}"#.to_string() }
        ("POST","/relay")         => relay_msg(body),
        ("GET", "/cache")         => cache_get(path),
        ("POST","/cache")         => cache_post(body),
        ("POST","/policy/allow")  => { let t = extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s = STATE.lock().unwrap(); s.tool_denylist.retain(|d| d != &t); if !s.tool_allowlist.contains(&t) { s.tool_allowlist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("POST","/policy/deny")   => { let t = extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s = STATE.lock().unwrap(); s.tool_allowlist.retain(|a| a != &t); if !s.tool_denylist.contains(&t) { s.tool_denylist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("GET", "/policy")        => get_policy_json(),
        ("POST","/nodes/register")=> { register_node(body); r#"{"ok":true}"#.to_string() }
        ("GET", "/nodes")         => get_nodes_json(),
        ("POST","/credentials/get")=> get_credential(body),

        // ── fallthrough → Java command queue ─────────────────────────────────
        _ => queue_to_java(path_clean.trim_start_matches('/'), body),
    }
}

// ─── v38: Config HTTP helpers ─────────────────────────────────────────────────

fn config_to_json(c: &KiraConfig) -> String {
    format!(
        r#"{{"user_name":"{}","api_key_set":{},"base_url":"{}","model":"{}","vision_model":"{}","tg_configured":{},"agent_max_steps":{},"agent_auto_approve":{},"heartbeat_interval":{},"setup_done":{}}}"#,
        esc(&c.user_name),
        !c.api_key.is_empty(),
        esc(&c.base_url),
        esc(&c.model),
        esc(&c.vision_model),
        !c.tg_token.is_empty(),
        c.agent_max_steps,
        c.agent_auto_approve,
        c.heartbeat_interval,
        c.setup_done
    )
}

fn shizuku_to_json(sz: &ShizukuStatus) -> String {
    format!(
        r#"{{"installed":{},"permission_granted":{},"last_checked_ms":{},"error":"{}","status":"{}"}}"#,
        sz.installed, sz.permission_granted, sz.last_checked_ms, esc(&sz.error_msg),
        if sz.permission_granted { "god_mode" }
        else if sz.installed     { "needs_permission" }
        else                     { "not_running" }
    )
}

fn update_config_from_http(body: &str) -> String {
    let mut s = STATE.lock().unwrap();
    if let Some(v) = extract_json_str(body, "user_name")  { s.config.user_name  = v; }
    if let Some(v) = extract_json_str(body, "api_key")    { s.config.api_key    = v; }
    if let Some(v) = extract_json_str(body, "base_url")   { s.config.base_url   = v; }
    if let Some(v) = extract_json_str(body, "model")      { s.config.model      = v; }
    if let Some(v) = extract_json_str(body, "tg_token")   { s.config.tg_token   = v; }
    if let Some(v) = extract_json_num(body, "tg_allowed") { s.config.tg_allowed = v as i64; }
    if let Some(v) = extract_json_num(body, "max_steps")  { s.config.agent_max_steps = v as u32; }
    r#"{"ok":true}"#.to_string()
}

// ─── Background threads ───────────────────────────────────────────────────────

fn run_trigger_watcher() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let tt: Vec<Trigger> = s.triggers.iter().filter(|t| t.trigger_type=="time" && !t.fired).cloned().collect();
        for t in tt {
            let fire_at = t.value.parse::<u128>().unwrap_or(0);
            if fire_at > 0 && now >= fire_at {
                s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","type":"time"}}"#, t.id, esc(&t.action)));
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) { tr.fired = !tr.repeat; }
            }
        }
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

// ─── Feature implementations ─────────────────────────────────────────────────

fn search_memory(path: &str) -> String {
    let query = path.find("q=").map(|i| &path[i+2..]).unwrap_or("").replace('+', " ");
    let ql = query.to_lowercase();
    let mut s = STATE.lock().unwrap();
    let mut results: Vec<(f32, String, String, u128)> = s.memory_index.iter_mut().filter_map(|e| {
        let mut score = 0.0f32;
        if e.key.to_lowercase() == ql { score += 10.0; }
        if e.key.to_lowercase().contains(&ql) { score += 5.0; }
        let vl = e.value.to_lowercase();
        for w in ql.split_whitespace() { if vl.contains(w) { score += 1.0; } }
        for tag in &e.tags { if tag.to_lowercase().contains(&ql) { score += 2.0; } }
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
        json_str(&s.context_compact), turns.join(","), s.total_tokens,
        json_str(&s.memory_md[..s.memory_md.len().min(1000)]))
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
    let interval = extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(3_600_000);
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
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","notif":"{}:{}" }}"#, t.id, esc(&t.action), esc(title), esc(text)));
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

// ─── Provider registry (17+, now includes "custom") ──────────────────────────

fn make_providers() -> Vec<Provider> {
    vec![
        Provider { id:"groq".into(),        name:"Groq".into(),          base_url:"https://api.groq.com/openai/v1".into(),                             model:"llama-3.1-8b-instant".into() },
        Provider { id:"openai".into(),       name:"OpenAI".into(),         base_url:"https://api.openai.com/v1".into(),                                  model:"gpt-4o-mini".into() },
        Provider { id:"anthropic".into(),    name:"Anthropic".into(),      base_url:"https://api.anthropic.com/v1".into(),                               model:"claude-3-haiku-20240307".into() },
        Provider { id:"gemini".into(),       name:"Gemini".into(),         base_url:"https://generativelanguage.googleapis.com/v1beta/openai".into(),    model:"gemini-2.0-flash".into() },
        Provider { id:"deepseek".into(),     name:"DeepSeek".into(),       base_url:"https://api.deepseek.com/v1".into(),                                model:"deepseek-chat".into() },
        Provider { id:"openrouter".into(),   name:"OpenRouter".into(),     base_url:"https://openrouter.ai/api/v1".into(),                               model:"openrouter/auto".into() },
        Provider { id:"ollama".into(),       name:"Ollama (local)".into(), base_url:"http://localhost:11434/v1".into(),                                  model:"llama3".into() },
        Provider { id:"together".into(),     name:"Together AI".into(),    base_url:"https://api.together.xyz/v1".into(),                                model:"meta-llama/Llama-3-8b-chat-hf".into() },
        Provider { id:"mistral".into(),      name:"Mistral".into(),        base_url:"https://api.mistral.ai/v1".into(),                                  model:"mistral-small-latest".into() },
        Provider { id:"cohere".into(),       name:"Cohere".into(),         base_url:"https://api.cohere.ai/v1".into(),                                   model:"command-r".into() },
        Provider { id:"perplexity".into(),   name:"Perplexity".into(),     base_url:"https://api.perplexity.ai".into(),                                  model:"llama-3.1-sonar-small-128k-online".into() },
        Provider { id:"xai".into(),          name:"xAI Grok".into(),       base_url:"https://api.x.ai/v1".into(),                                        model:"grok-2-latest".into() },
        Provider { id:"cerebras".into(),     name:"Cerebras".into(),       base_url:"https://api.cerebras.ai/v1".into(),                                 model:"llama3.1-8b".into() },
        Provider { id:"fireworks".into(),    name:"Fireworks".into(),      base_url:"https://api.fireworks.ai/inference/v1".into(),                      model:"accounts/fireworks/models/llama-v3p1-8b-instruct".into() },
        Provider { id:"sambanova".into(),    name:"SambaNova".into(),      base_url:"https://api.sambanova.ai/v1".into(),                                model:"Meta-Llama-3.1-8B-Instruct".into() },
        Provider { id:"novita".into(),       name:"Novita AI".into(),      base_url:"https://api.novita.ai/v3/openai".into(),                            model:"llama-3.1-8b-instruct".into() },
        // v38: custom provider — base_url set by user at runtime
        Provider { id:"custom".into(),       name:"Custom".into(),         base_url:String::new(),                                                       model:String::new() },
    ]
}

// ─── Misc feature helpers (unchanged from v7) ─────────────────────────────────

fn get_policy_json() -> String {
    let s = STATE.lock().unwrap();
    format!(r#"{{"allowlist":[{}],"denylist":[{}]}}"#,
        s.tool_allowlist.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","),
        s.tool_denylist.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","))
}
fn get_nodes_json() -> String {
    let s = STATE.lock().unwrap(); let now = now_ms();
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
        Some(enc) => { let key = derive_key(&name); let dec = xor_crypt(enc, &key); let val = String::from_utf8_lossy(&dec).to_string(); format!(r#"{{"name":"{}","value":"{}"}}"#, esc(&name), esc(&val)) }
        None      => format!(r#"{{"error":"credential '{}' not found"}}"#, esc(&name))
    }
}
fn stream_poll() -> String {
    let mut s = STATE.lock().unwrap();
    let chunks: Vec<String> = s.stream_chunks.drain(..).map(|c| format!(r#"{{"session_id":"{}","text":"{}","done":{},"ts":{}}}"#, esc(&c.session_id), esc(&c.text), c.done, c.ts)).collect();
    format!("[{}]", chunks.join(","))
}
fn push_stream_chunk(text: &str) {
    let mut s = STATE.lock().unwrap(); let sid = s.active_session.clone();
    s.stream_chunks.push_back(StreamChunk { session_id: sid, text: text.to_string(), done: false, ts: now_ms() });
    if s.stream_chunks.len() > 1000 { s.stream_chunks.pop_front(); }
}
fn relay_msg(body: &str) -> String {
    let ch = extract_json_str(body,"channel").unwrap_or_default(); let msg = extract_json_str(body,"message").unwrap_or_default(); let ts = now_ms();
    STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"type":"relay","channel":"{}","message":"{}","ts":{}}}"#, esc(&ch), esc(&msg), ts));
    r#"{"ok":true}"#.to_string()
}
fn cache_get(path: &str) -> String {
    let key = path.find("key=").map(|i| &path[i+4..]).unwrap_or("").split('&').next().unwrap_or(""); let s = STATE.lock().unwrap(); let now = now_ms();
    match s.response_cache.get(key) {
        Some(e) if e.expires_at > now => format!(r#"{{"key":"{}","value":"{}","ttl":{}}}"#, esc(key), esc(&e.value), e.expires_at - now),
        Some(_) => r#"{"error":"expired"}"#.to_string(), None => r#"{"error":"not_found"}"#.to_string(),
    }
}
fn cache_post(body: &str) -> String {
    let k = extract_json_str(body,"key").unwrap_or_default(); let v = extract_json_str(body,"value").unwrap_or_default();
    let ttl = extract_json_str(body,"ttl_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(300_000);
    STATE.lock().unwrap().response_cache.insert(k, CacheEntry { value: v, expires_at: now_ms() + ttl });
    r#"{"ok":true}"#.to_string()
}
fn get_budget_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.tool_iterations.iter().map(|(k,v)| format!(r#"{{"session":"{}","used":{},"remaining":{}}}"#, esc(k), v, s.max_tool_iters.saturating_sub(*v))).collect();
    format!(r#"{{"max":{},"sessions":[{}]}}"#, s.max_tool_iters, items.join(","))
}
fn get_kb_json() -> String {
    let s = STATE.lock().unwrap();
    let items: Vec<String> = s.knowledge_base.iter().map(|e| format!(r#"{{"id":"{}","title":"{}","snippet":"{}","tags":[{}],"ts":{}}}"#, esc(&e.id), esc(&e.title), esc(&e.content[..e.content.len().min(100)]), e.tags.iter().map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(","), e.ts)).collect();
    format!("[{}]", items.join(","))
}
fn add_kb_entry(body: &str) {
    let id = extract_json_str(body,"id").unwrap_or_else(gen_id); let title = extract_json_str(body,"title").unwrap_or_default(); let content = extract_json_str(body,"content").unwrap_or_default();
    let tags_s = extract_json_str(body,"tags").unwrap_or_default(); let tags: Vec<String> = tags_s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let mut s = STATE.lock().unwrap(); s.knowledge_base.retain(|e| e.id != id); s.knowledge_base.push(KbEntry { id, title, content, tags, ts: now_ms() });
    if s.knowledge_base.len() > 10000 { s.knowledge_base.remove(0); }
}
fn kb_search(path: &str) -> String {
    let query = path.find("q=").map(|i| &path[i+2..]).unwrap_or("").to_lowercase(); let s = STATE.lock().unwrap();
    let mut results: Vec<(u32, &KbEntry)> = s.knowledge_base.iter().filter_map(|e| { let mut sc = 0u32; if e.title.to_lowercase().contains(&query) { sc += 10; } if e.content.to_lowercase().contains(&query) { sc += 5; } for tag in &e.tags { if tag.to_lowercase().contains(&query) { sc += 3; } } if sc > 0 { Some((sc, e)) } else { None } }).collect();
    results.sort_by(|a,b| b.0.cmp(&a.0));
    let items: Vec<String> = results.iter().take(10).map(|(sc,e)| format!(r#"{{"id":"{}","title":"{}","content":"{}","score":{}}}"#, esc(&e.id), esc(&e.title), esc(&e.content[..e.content.len().min(300)]), sc)).collect();
    format!("[{}]", items.join(","))
}
fn get_metrics_text() -> String {
    let s = STATE.lock().unwrap();
    format!("kira_uptime_ms {}\nkira_requests_total {}\nkira_tool_calls {}\nkira_notifications {}\nkira_memory_entries {}\nkira_battery {}\nkira_skills {}\nkira_kb_entries {}\nkira_event_feed {}\n",
        now_ms()-s.uptime_start, s.request_count, s.tool_call_count, s.notifications.len(), s.memory_index.len(), s.battery_pct, s.skills.len(), s.knowledge_base.len(), s.event_feed.len())
}
fn get_event_feed() -> String {
    let s = STATE.lock().unwrap(); let skip = s.event_feed.len().saturating_sub(100);
    let items: Vec<String> = s.event_feed.iter().skip(skip).map(|e| format!(r#"{{"event":"{}","data":"{}","ts":{}}}"#, esc(&e.event), esc(&e.data), e.ts)).collect();
    format!("[{}]", items.join(","))
}
fn push_event_feed(event: &str, data: &str) {
    let mut s = STATE.lock().unwrap();
    s.event_feed.push_back(EventFeedEntry { event: event.to_string(), data: data.to_string(), ts: now_ms() });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

// ─── Crypto (ZeroClaw) ────────────────────────────────────────────────────────

fn derive_key(name: &str) -> Vec<u8> {
    let mut key = vec![0u8; 32];
    for (i, b) in name.bytes().enumerate() { key[i % 32] ^= b.wrapping_add((i as u8).wrapping_mul(7)); }
    key
}
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() { return data.to_vec(); }
    data.iter().enumerate().map(|(i, &b)| b ^ key[i % key.len()]).collect()
}

// ─── Utilities ────────────────────────────────────────────────────────────────

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
fn extract_json_num(json: &str, key: &str) -> Option<f64> {
    let search = format!("\"{}\":", key);
    let start  = json.find(&search)? + search.len();
    let slice  = json[start..].trim_start();
    let end    = slice.find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').unwrap_or(slice.len());
    slice[..end].parse::<f64>().ok()
}
fn extract_json_f32(json: &str, key: &str) -> Option<f32> {
    extract_json_num(json, key).map(|v| v as f32)
}
