//! Kira Core v4 — Ultra-fast Rust HTTP server + state management
//! Handles: HTTP server, command queue, notification store, screen state
//! All actual Android calls go through JNI to Java (which has the real APIs)

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Default)]
struct State {
    notifications:  VecDeque<String>,
    screen_nodes:   String,
    pending_cmds:   VecDeque<(String, String)>,
    results:        HashMap<String, String>,
    battery_cache:  String,
    device_info:    String,
    uptime_ms:      u128,
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<State>> = Arc::new(Mutex::new(State::default()));
}

// ── JNI bridge ────────────────────────────────────────────────────────────────

#[allow(non_snake_case)]
pub mod jni {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;
    use super::*;

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        port: i32,
    ) {
        let port = port as u16;
        let start = now_ms();
        {
            let mut s = STATE.lock().unwrap();
            s.uptime_ms = start;
        }
        thread::spawn(move || run_server(port));
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        pkg:   *const c_char,
        title: *const c_char,
        text:  *const c_char,
    ) {
        let pkg   = unsafe { CStr::from_ptr(pkg).to_string_lossy().into_owned() };
        let title = unsafe { CStr::from_ptr(title).to_string_lossy().into_owned() };
        let text  = unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() };
        let ts = now_ms();
        let entry = format!(
            r#"{{"package":"{}","title":"{}","text":"{}","timestamp":{}}}"#,
            esc(&pkg), esc(&title), esc(&text), ts
        );
        let mut s = STATE.lock().unwrap();
        s.notifications.push_back(entry);
        if s.notifications.len() > 200 { s.notifications.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _env:  *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        json:  *const c_char,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        STATE.lock().unwrap().screen_nodes = json;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _env:  *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        json:  *const c_char,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        STATE.lock().unwrap().battery_cache = json;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _env:  *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
    ) -> *mut c_char {
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
        _env:   *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        id:     *const c_char,
        result: *const c_char,
    ) {
        let id     = unsafe { CStr::from_ptr(id).to_string_lossy().into_owned() };
        let result = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        STATE.lock().unwrap().results.insert(id, result);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _env:   *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        s:      *mut c_char,
    ) {
        if !s.is_null() {
            unsafe { drop(CString::from_raw(s)); }
        }
    }
}

// ── HTTP server ───────────────────────────────────────────────────────────────

fn run_server(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l,
        Err(e) => { eprintln!("Kira server bind failed: {}", e); return; }
    };
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle(stream));
    }
}

fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 32768];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = req.lines().collect();
    if lines.is_empty() { return; }
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path   = parts[1];
    let body   = req.find("\r\n\r\n").map(|i| req[i+4..].trim().to_string()).unwrap_or_default();

    let resp = route(method, path, &body);
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Engine: rust\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());
}

fn route(method: &str, path: &str, body: &str) -> String {
    // Fast path — read-only state queries served directly from Rust
    match (method, path) {
        ("GET", "/health") => {
            let s = STATE.lock().unwrap();
            let uptime = now_ms() - s.uptime_ms;
            format!(r#"{{"status":"ok","engine":"rust","version":"4.0","uptime_ms":{}}}"#, uptime)
        }
        ("GET", "/screenshot")    => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET", "/notifications") => {
            let s = STATE.lock().unwrap();
            format!("[{}]", s.notifications.iter().cloned().collect::<Vec<_>>().join(","))
        }
        ("GET", "/battery_cache") => {
            STATE.lock().unwrap().battery_cache.clone()
        }
        ("GET", "/stats") => {
            let s = STATE.lock().unwrap();
            format!(
                r#"{{"notifications":{},"pending_cmds":{},"uptime_ms":{}}}"#,
                s.notifications.len(),
                s.pending_cmds.len(),
                now_ms() - s.uptime_ms
            )
        }
        // All other commands → queue to Java accessibility service
        (_, _) => {
            let endpoint = path.trim_start_matches('/');
            let id = gen_id();
            let cmd_body = if body.is_empty() { "{}".to_string() } else { body.to_string() };
            let payload = format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint, cmd_body);
            {
                STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
            }
            let timeout = match endpoint {
                "record_audio" => 30000,
                "install_apk"  => 60000,
                _              => 8000,
            };
            wait_result(&id, timeout).unwrap_or_else(|| r#"{"error":"timeout — accessibility service may not be running"}"#.to_string())
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
        thread::sleep(std::time::Duration::from_millis(8));
    }
}

fn gen_id() -> String {
    format!("{}{:04}", now_ms(), now_ms() % 9999)
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "")
     .replace('\t', " ")
}
