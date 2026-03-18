//! Kira Core v5 — OpenClaw-inspired Rust engine
//! Features: HTTP server, WebSocket gateway (port 18789), heartbeat, command queue,
//! notification store, screen state, multi-agent routing, event bus

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH, Duration};

// ── Shared global state ───────────────────────────────────────────────────────

#[derive(Default)]
struct State {
    notifications:  VecDeque<String>,
    screen_nodes:   String,
    pending_cmds:   VecDeque<(String, String)>,
    results:        HashMap<String, String>,
    battery_cache:  String,
    events:         VecDeque<String>,   // event bus for WebSocket clients
    heartbeat_count: u64,
    uptime_ms:      u128,
    agent_mode:     String,             // "manager" | "executor" | "reflector"
    active_task:    String,
    task_steps:     Vec<String>,
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
}

// ── JNI exports ───────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub mod jni {
    use std::ffi::{CStr, CString};
    use super::*;

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, port: i32,
    ) {
        let start = now_ms();
        STATE.lock().unwrap().uptime_ms = start;
        STATE.lock().unwrap().agent_mode = "executor".to_string();
        let http_port  = port as u16;
        let ws_port    = 18789u16;
        thread::spawn(move || run_http(http_port));
        thread::spawn(move || run_ws(ws_port));
        thread::spawn(move || run_heartbeat());
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void,
        pkg: *const i8, title: *const i8, text: *const i8,
    ) {
        let pkg   = unsafe { CStr::from_ptr(pkg).to_string_lossy().into_owned() };
        let title = unsafe { CStr::from_ptr(title).to_string_lossy().into_owned() };
        let text  = unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() };
        let entry = format!(
            r#"{{"type":"notification","package":"{}","title":"{}","text":"{}","ts":{}}}"#,
            esc(&pkg), esc(&title), esc(&text), now_ms()
        );
        let mut s = STATE.lock().unwrap();
        s.notifications.push_back(entry.clone());
        if s.notifications.len() > 200 { s.notifications.pop_front(); }
        s.events.push_back(entry);
        if s.events.len() > 500 { s.events.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, json: *const i8,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        STATE.lock().unwrap().screen_nodes = json;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, json: *const i8,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        STATE.lock().unwrap().battery_cache = json;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void,
    ) -> *mut i8 {
        let cmd = STATE.lock().unwrap().pending_cmds.pop_front();
        match cmd {
            Some((id, body)) => {
                let json = format!(r#"{{"id":"{}","body":{}}}"#, id, body);
                CString::new(json).unwrap().into_raw()
            }
            None => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void,
        id: *const i8, result: *const i8,
    ) {
        let id     = unsafe { CStr::from_ptr(id).to_string_lossy().into_owned() };
        let result = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        let event  = format!(r#"{{"type":"result","id":"{}","result":{}}}"#, id,
            if result.starts_with('{') || result.starts_with('[') { result.clone() }
            else { format!("\"{}\"", esc(&result)) });
        let mut s  = STATE.lock().unwrap();
        s.results.insert(id, result);
        s.events.push_back(event);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, s: *mut i8,
    ) {
        if !s.is_null() { unsafe { drop(CString::from_raw(s)); } }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setAgentMode(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, mode: *const i8,
    ) {
        let mode = unsafe { CStr::from_ptr(mode).to_string_lossy().into_owned() };
        STATE.lock().unwrap().agent_mode = mode;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushEvent(
        _: *mut std::ffi::c_void, _: *mut std::ffi::c_void, json: *const i8,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        let mut s = STATE.lock().unwrap();
        s.events.push_back(json);
        if s.events.len() > 500 { s.events.pop_front(); }
    }
}

// ── HTTP server (port 7070) ───────────────────────────────────────────────────

fn run_http(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l, Err(_) => return,
    };
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle_http(stream));
    }
}

fn handle_http(mut stream: TcpStream) {
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = req.lines().collect();
    if lines.is_empty() { return; }
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path   = parts[1].split('?').next().unwrap_or("/");
    let body   = req.find("\r\n\r\n").map(|i| req[i+4..].trim().to_string()).unwrap_or_default();
    let resp   = route_http(method, path, &body);
    let http   = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Engine: kira-rust/5.0\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    match (method, path) {
        ("GET", "/")         => r#"{"name":"kira","version":"5.0","engine":"rust"}"#.to_string(),
        ("GET", "/health")   => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"status":"ok","uptime_ms":{},"heartbeat":{},"agent_mode":"{}","active_task":"{}"}}"#,
                now_ms() - s.uptime_ms, s.heartbeat_count, s.agent_mode, esc(&s.active_task))
        }
        ("GET", "/notifications") => {
            let s = STATE.lock().unwrap();
            format!("[{}]", s.notifications.iter().cloned().collect::<Vec<_>>().join(","))
        }
        ("GET", "/screen")   => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET", "/battery")  => STATE.lock().unwrap().battery_cache.clone(),
        ("GET", "/events")   => {
            let s = STATE.lock().unwrap();
            format!("[{}]", s.events.iter().cloned().collect::<Vec<_>>().join(","))
        }
        ("GET", "/stats")    => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"notifications":{},"pending_cmds":{},"events":{},"task_steps":{},"uptime_ms":{}}}"#,
                s.notifications.len(), s.pending_cmds.len(), s.events.len(),
                s.task_steps.len(), now_ms() - s.uptime_ms)
        }
        ("POST", "/task")    => {
            let task = body.to_string();
            STATE.lock().unwrap().active_task = task.clone();
            STATE.lock().unwrap().task_steps.clear();
            format!(r#"{{"ok":true,"task":"{}"}}"#, esc(&task))
        }
        ("POST", "/task/step") => {
            STATE.lock().unwrap().task_steps.push(body.to_string());
            r#"{"ok":true}"#.to_string()
        }
        ("GET", "/task")     => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"active":"{}","steps":{}}}"#,
                esc(&s.active_task),
                format!("[{}]", s.task_steps.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",")))
        }
        // Everything else → queue to Java
        _ => {
            let endpoint = path.trim_start_matches('/');
            let id = gen_id();
            let payload = format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint,
                if body.is_empty() { "{}".to_string() } else { body.to_string() });
            STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
            wait_result(&id, 10000).unwrap_or_else(||
                r#"{"error":"timeout — check accessibility service"}"#.to_string())
        }
    }
}

// ── WebSocket gateway (port 18789 — OpenClaw compatible) ──────────────────────

fn run_ws(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l, Err(_) => return,
    };
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle_ws_upgrade(stream));
    }
}

fn handle_ws_upgrade(mut stream: TcpStream) {
    // Simple WS handshake
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) { Ok(n) => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    if !req.contains("Upgrade: websocket") { return; }

    // Extract Sec-WebSocket-Key
    let key = req.lines()
        .find(|l| l.starts_with("Sec-WebSocket-Key:"))
        .and_then(|l| l.split(':').nth(1))
        .map(|k| k.trim())
        .unwrap_or("");

    let accept = ws_accept_key(key);
    let resp = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept
    );
    let _ = stream.write_all(resp.as_bytes());

    // Send welcome
    let welcome = format!(r#"{{"type":"welcome","version":"5.0","uptime_ms":{}}}"#, now_ms());
    let _ = ws_send(&mut stream, &welcome);

    // Event pump — send new events to client
    let mut last_event_count = STATE.lock().unwrap().events.len();
    loop {
        thread::sleep(Duration::from_millis(100));
        let events: Vec<String> = {
            let s = STATE.lock().unwrap();
            if s.events.len() > last_event_count {
                let new = s.events.iter().skip(last_event_count).cloned().collect();
                last_event_count = s.events.len();
                new
            } else { vec![] }
        };
        for e in events {
            if ws_send(&mut stream, &e).is_err() { return; }
        }
    }
}

fn ws_accept_key(key: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    // Simplified - real impl needs SHA1+base64, this is a placeholder that lets connection upgrade
    let magic = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key, magic);
    let mut hasher = DefaultHasher::new();
    combined.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

fn ws_send(stream: &mut TcpStream, msg: &str) -> Result<(), std::io::Error> {
    let data = msg.as_bytes();
    let len = data.len();
    let mut frame = vec![0x81u8]; // text frame, fin
    if len < 126 {
        frame.push(len as u8);
    } else if len < 65536 {
        frame.push(126);
        frame.push((len >> 8) as u8);
        frame.push((len & 0xFF) as u8);
    } else {
        frame.push(127);
        for i in (0..8).rev() { frame.push((len >> (i * 8)) as u8 & 0xFF); }
    }
    frame.extend_from_slice(data);
    stream.write_all(&frame)
}

// ── Heartbeat (AndyClaw-inspired autonomous pulse) ────────────────────────────

fn run_heartbeat() {
    loop {
        thread::sleep(Duration::from_secs(30));
        let count = {
            let mut s = STATE.lock().unwrap();
            s.heartbeat_count += 1;
            s.heartbeat_count
        };
        let event = format!(
            r#"{{"type":"heartbeat","count":{},"ts":{},"uptime_ms":{}}}"#,
            count, now_ms(), {
                let s = STATE.lock().unwrap();
                now_ms() - s.uptime_ms
            }
        );
        let mut s = STATE.lock().unwrap();
        s.events.push_back(event);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn wait_result(id: &str, timeout_ms: u64) -> Option<String> {
    let start = std::time::Instant::now();
    loop {
        { let mut s = STATE.lock().unwrap(); if let Some(r) = s.results.remove(id) { return Some(r); } }
        if start.elapsed().as_millis() as u64 >= timeout_ms { return None; }
        thread::sleep(Duration::from_millis(8));
    }
}

fn gen_id() -> String { format!("{}{:04}", now_ms(), now_ms() % 9999) }

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "").replace('\t', " ")
}
