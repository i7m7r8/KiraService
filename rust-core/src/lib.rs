// Kira Rust Core v6 - World's most powerful Android AI agent
// Pure ASCII - no unicode box chars
// Features: HTTP server, command queue, event engine, task scheduler,
//           screen state cache, notification store, proactive triggers

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// =============================================================================
// Shared state
// =============================================================================

#[derive(Default)]
struct KiraState {
    // Screen
    screen_nodes:    String,
    screen_pkg:      String,
    // Notifications  
    notifications:   VecDeque<Notification>,
    // Command queue (Rust HTTP -> Java accessibility)
    pending_cmds:    VecDeque<(String, String)>,
    results:         HashMap<String, String>,
    // Proactive triggers
    triggers:        Vec<Trigger>,
    fired_triggers:  VecDeque<String>,
    // Battery/device state
    battery_pct:     i32,
    battery_charging:bool,
    // Task log (like NanoBot step recorder)
    task_log:        VecDeque<TaskStep>,
    // Agent memory cache
    agent_context:   String,
    // Server stats
    uptime_start:    u128,
    request_count:   u64,
}

#[derive(Clone, Default)]
struct Notification {
    pkg:   String,
    title: String,
    text:  String,
    time:  u128,
}

// Proactive trigger (NanoBot-style event watching)
#[derive(Clone)]
struct Trigger {
    id:          String,
    trigger_type:String, // "battery_low", "app_open", "time", "keyword_notif"
    value:       String, // threshold or app pkg or keyword
    action:      String, // what to do when fired
    fired:       bool,
    repeat:      bool,
}

// Task step recorder (ZeroClaw-style execution trace)
#[derive(Clone)]
struct TaskStep {
    task_id: String,
    step:    u32,
    action:  String,
    result:  String,
    time:    u128,
    success: bool,
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<KiraState>> = Arc::new(Mutex::new(KiraState {
        battery_pct: 100,
        ..Default::default()
    }));
}

// =============================================================================
// JNI Bridge
// =============================================================================

mod jni_bridge {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use super::*;

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, port: i32,
    ) {
        let p = port as u16;
        {
            let mut s = STATE.lock().unwrap();
            s.uptime_start = now_ms();
        }
        thread::spawn(move || run_server(p));
        // Also start proactive trigger watcher
        thread::spawn(watch_triggers);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        pkg: *const c_char, title: *const c_char, text: *const c_char,
    ) {
        let pkg   = cstr(pkg);
        let title = cstr(title);
        let text  = cstr(text);
        let mut s = STATE.lock().unwrap();

        // Check keyword triggers
        let keyword_triggers: Vec<Trigger> = s.triggers.iter()
            .filter(|t| t.trigger_type == "keyword_notif" && !t.fired)
            .cloned().collect();
        for t in keyword_triggers {
            if title.to_lowercase().contains(&t.value.to_lowercase())
            || text.to_lowercase().contains(&t.value.to_lowercase()) {
                s.fired_triggers.push_back(format!("{{\"trigger\":\"{}\",\"action\":\"{}\",\"notif\":\"{}:{}\"}}", t.id, esc(&t.action), esc(&title), esc(&text)));
                // Mark fired if not repeating
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) {
                    tr.fired = !tr.repeat;
                }
            }
        }

        // Check app_open triggers based on pkg
        let app_triggers: Vec<Trigger> = s.triggers.iter()
            .filter(|t| t.trigger_type == "app_notif" && !t.fired && t.value == pkg)
            .cloned().collect();
        for t in app_triggers {
            s.fired_triggers.push_back(format!("{{\"trigger\":\"{}\",\"action\":\"{}\"}}", t.id, esc(&t.action)));
            if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) {
                tr.fired = !tr.repeat;
            }
        }

        s.notifications.push_back(Notification { pkg, title, text, time: now_ms() });
        if s.notifications.len() > 500 { s.notifications.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, json: *const c_char,
    ) {
        let json = cstr(json);
        let mut s = STATE.lock().unwrap();
        s.screen_nodes = json;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenPackage(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pkg: *const c_char,
    ) {
        STATE.lock().unwrap().screen_pkg = cstr(pkg);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        pct: i32, charging: bool,
    ) {
        let mut s = STATE.lock().unwrap();
        let prev = s.battery_pct;
        s.battery_pct = pct;
        s.battery_charging = charging;

        // Fire battery triggers
        let battery_triggers: Vec<Trigger> = s.triggers.iter()
            .filter(|t| t.trigger_type == "battery_low" && !t.fired)
            .cloned().collect();
        for t in battery_triggers {
            let threshold = t.value.parse::<i32>().unwrap_or(20);
            if pct <= threshold && prev > threshold {
                s.fired_triggers.push_back(format!("{{\"trigger\":\"{}\",\"action\":\"{}\",\"battery\":{}}}", t.id, esc(&t.action), pct));
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) {
                    tr.fired = !tr.repeat;
                }
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateAgentContext(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, ctx: *const c_char,
    ) {
        STATE.lock().unwrap().agent_context = cstr(ctx);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logTaskStep(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        task_id: *const c_char, step: i32, action: *const c_char,
        result: *const c_char, success: bool,
    ) {
        let mut s = STATE.lock().unwrap();
        s.task_log.push_back(TaskStep {
            task_id: cstr(task_id),
            step:    step as u32,
            action:  cstr(action),
            result:  cstr(result),
            time:    now_ms(),
            success,
        });
        if s.task_log.len() > 1000 { s.task_log.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().pending_cmds.pop_front() {
            Some((id, body)) => {
                CString::new(format!("{{\"id\":\"{}\",\"body\":{}}}", id, body))
                    .unwrap().into_raw()
            }
            None => std::ptr::null_mut(),
        }
    }

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
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id: *const c_char, result: *const c_char,
    ) {
        let id = cstr(id); let result = cstr(result);
        STATE.lock().unwrap().results.insert(id, result);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id: *const c_char, ttype: *const c_char, value: *const c_char,
        action: *const c_char, repeat: bool,
    ) {
        STATE.lock().unwrap().triggers.push(Trigger {
            id: cstr(id), trigger_type: cstr(ttype), value: cstr(value),
            action: cstr(action), fired: false, repeat,
        });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char,
    ) {
        let id = cstr(id);
        STATE.lock().unwrap().triggers.retain(|t| t.id != id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, s: *mut c_char,
    ) {
        if !s.is_null() { unsafe { drop(CString::from_raw(s)); } }
    }

    fn cstr(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }
}

// =============================================================================
// HTTP Server
// =============================================================================

fn run_server(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l)  => l,
        Err(e) => { eprintln!("Kira bind error: {}", e); return; }
    };
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle(stream));
    }
}

fn handle(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = req.lines().collect();
    if lines.is_empty() { return; }
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path   = parts[1];
    let body   = req.find("\r\n\r\n")
        .map(|i| req[i+4..].trim().to_string())
        .unwrap_or_default();

    let resp = route(method, path, &body);
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Engine: kira-rust-v6\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());

    STATE.lock().unwrap().request_count += 1;
}

fn route(method: &str, path: &str, body: &str) -> String {
    match (method, path) {
        // Fast read-only endpoints served directly from Rust
        ("GET", "/health") => {
            let s = STATE.lock().unwrap();
            format!(
                "{{\"status\":\"ok\",\"engine\":\"rust\",\"version\":\"6.0\",\"uptime_ms\":{},\"requests\":{},\"battery\":{},\"charging\":{},\"notifications\":{},\"triggers\":{}}}",
                now_ms() - s.uptime_start, s.request_count, s.battery_pct, s.battery_charging,
                s.notifications.len(), s.triggers.iter().filter(|t| !t.fired).count()
            )
        }
        ("GET", "/screen") => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET", "/screen_pkg") => {
            let pkg = STATE.lock().unwrap().screen_pkg.clone();
            format!("{{\"package\":\"{}\"}}", esc(&pkg))
        }
        ("GET", "/notifications") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.notifications.iter().map(|n| {
                format!("{{\"pkg\":\"{}\",\"title\":\"{}\",\"text\":\"{}\",\"time\":{}}}", esc(&n.pkg), esc(&n.title), esc(&n.text), n.time)
            }).collect();
            format!("[{}]", items.join(","))
        }
        ("GET", "/battery") => {
            let s = STATE.lock().unwrap();
            format!("{{\"percentage\":{},\"charging\":{}}}", s.battery_pct, s.battery_charging)
        }
        ("GET", "/triggers") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.triggers.iter().map(|t| {
                format!("{{\"id\":\"{}\",\"type\":\"{}\",\"value\":\"{}\",\"fired\":{},\"repeat\":{}}}", t.id, t.trigger_type, esc(&t.value), t.fired, t.repeat)
            }).collect();
            format!("[{}]", items.join(","))
        }
        ("GET", "/fired_triggers") => {
            let mut s = STATE.lock().unwrap();
            let items: Vec<String> = s.fired_triggers.drain(..).collect();
            format!("[{}]", items.join(","))
        }
        ("GET", "/task_log") => {
            let s = STATE.lock().unwrap();
            let limit = 50usize;
            let start = if s.task_log.len() > limit { s.task_log.len() - limit } else { 0 };
            let items: Vec<String> = s.task_log.iter().skip(start).map(|t| {
                format!("{{\"task_id\":\"{}\",\"step\":{},\"action\":\"{}\",\"result\":\"{}\",\"success\":{},\"time\":{}}}",
                    esc(&t.task_id), t.step, esc(&t.action), esc(&t.result), t.success, t.time)
            }).collect();
            format!("[{}]", items.join(","))
        }
        ("GET", "/stats") => {
            let s = STATE.lock().unwrap();
            format!(
                "{{\"notifications\":{},\"pending_cmds\":{},\"task_steps\":{},\"triggers\":{},\"uptime_ms\":{}}}",
                s.notifications.len(), s.pending_cmds.len(), s.task_log.len(),
                s.triggers.len(), now_ms() - s.uptime_start
            )
        }
        // Add trigger via HTTP POST
        ("POST", "/add_trigger") => {
            parse_and_add_trigger(body);
            "{\"ok\":true}".to_string()
        }
        // Queue command to Java
        _ => {
            let endpoint = path.trim_start_matches('/');
            let id = gen_id();
            let payload = if body.is_empty() {
                format!("{{\"endpoint\":\"{}\",\"data\":{{}}}}", endpoint)
            } else {
                format!("{{\"endpoint\":\"{}\",\"data\":{}}}", endpoint, body)
            };
            {
                STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
            }
            let timeout = match endpoint {
                "install_apk" | "take_video" => 60000,
                "record_audio" => 30000,
                _ => 10000,
            };
            wait_result(&id, timeout)
                .unwrap_or_else(|| "{\"error\":\"timeout\"}".to_string())
        }
    }
}

fn parse_and_add_trigger(body: &str) {
    // Simple JSON field extraction (no serde needed)
    let id    = extract_json_str(body, "id").unwrap_or_else(|| gen_id());
    let ttype = extract_json_str(body, "type").unwrap_or_default();
    let value = extract_json_str(body, "value").unwrap_or_default();
    let action= extract_json_str(body, "action").unwrap_or_default();
    let repeat= body.contains("\"repeat\":true");
    STATE.lock().unwrap().triggers.push(Trigger {
        id, trigger_type: ttype, value, action, fired: false, repeat,
    });
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let end = json[start..].find('"')? + start;
    Some(json[start..end].to_string())
}

// Proactive trigger watcher thread (NanoBot-style)
fn watch_triggers() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let time_triggers: Vec<Trigger> = s.triggers.iter()
            .filter(|t| t.trigger_type == "time" && !t.fired)
            .cloned().collect();
        for t in time_triggers {
            let fire_at = t.value.parse::<u128>().unwrap_or(0);
            if fire_at > 0 && now >= fire_at {
                s.fired_triggers.push_back(
                    format!("{{\"trigger\":\"{}\",\"action\":\"{}\"}}", t.id, esc(&t.action))
                );
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id == t.id) {
                    tr.fired = !tr.repeat;
                }
            }
        }
    }
}

fn wait_result(id: &str, timeout_ms: u64) -> Option<String> {
    let start = std::time::Instant::now();
    loop {
        {
            let mut s = STATE.lock().unwrap();
            if let Some(r) = s.results.remove(id) { return Some(r); }
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms { return None; }
        thread::sleep(Duration::from_millis(8));
    }
}

fn gen_id() -> String { format!("{}", now_ms()) }

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "")
}
