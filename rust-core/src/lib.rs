// KiraService Rust Core
// Ultra-fast HTTP server + automation bridge
// Runs as native Android library, called from Java via JNI

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::collections::VecDeque;

// ── JNI imports ───────────────────────────────────────────────────────────────
#[allow(non_snake_case)]
mod jni_bridge {
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    // Called from Java to start the server
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        port: i32,
    ) {
        let port = port as u16;
        thread::spawn(move || {
            crate::run_server(port);
        });
    }

    // Called from Java to push a notification into Rust's store
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        pkg: *const c_char,
        title: *const c_char,
        text: *const c_char,
    ) {
        let pkg = unsafe { CStr::from_ptr(pkg).to_string_lossy().into_owned() };
        let title = unsafe { CStr::from_ptr(title).to_string_lossy().into_owned() };
        let text = unsafe { CStr::from_ptr(text).to_string_lossy().into_owned() };
        crate::push_notification(pkg, title, text);
    }

    // Called from Java to push screen nodes
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        json: *const c_char,
    ) {
        let json = unsafe { CStr::from_ptr(json).to_string_lossy().into_owned() };
        crate::update_screen_nodes(json);
    }

    // Called from Java to get next pending command (polling)
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match crate::next_command() {
            Some(cmd) => CString::new(cmd).unwrap().into_raw(),
            None => std::ptr::null_mut(),
        }
    }

    // Called from Java to push a command result back
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        id: *const c_char,
        result: *const c_char,
    ) {
        let id = unsafe { CStr::from_ptr(id).to_string_lossy().into_owned() };
        let result = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
        crate::push_result(id, result);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _env: *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
        s: *mut c_char,
    ) {
        if !s.is_null() {
            unsafe { drop(CString::from_raw(s)); }
        }
    }
}

// ── Shared state ──────────────────────────────────────────────────────────────

struct AppState {
    notifications: VecDeque<String>,   // JSON strings
    screen_nodes: String,              // latest screen dump JSON
    pending_cmds: VecDeque<(String, String)>, // (id, json_cmd)
    results: std::collections::HashMap<String, String>, // id -> result
}

impl AppState {
    fn new() -> Self {
        AppState {
            notifications: VecDeque::new(),
            screen_nodes: "[]".to_string(),
            pending_cmds: VecDeque::new(),
            results: std::collections::HashMap::new(),
        }
    }
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::new()));
}

fn push_notification(pkg: String, title: String, text: String) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let json = format!(
        r#"{{"package":"{}","title":"{}","text":"{}","timestamp":{}}}"#,
        escape(&pkg), escape(&title), escape(&text), ts
    );
    let mut state = STATE.lock().unwrap();
    state.notifications.push_back(json);
    if state.notifications.len() > 50 {
        state.notifications.pop_front();
    }
}

fn update_screen_nodes(json: String) {
    let mut state = STATE.lock().unwrap();
    state.screen_nodes = json;
}

fn next_command() -> Option<String> {
    let mut state = STATE.lock().unwrap();
    state.pending_cmds.pop_front().map(|(id, cmd)| {
        format!(r#"{{"id":"{}","cmd":{}}}"#, id, cmd)
    })
}

fn push_result(id: String, result: String) {
    let mut state = STATE.lock().unwrap();
    state.results.insert(id, result);
}

fn queue_command(id: String, cmd: String) {
    let mut state = STATE.lock().unwrap();
    state.pending_cmds.push_back((id, cmd));
}

fn wait_result(id: &str, timeout_ms: u64) -> Option<String> {
    let start = std::time::Instant::now();
    loop {
        {
            let mut state = STATE.lock().unwrap();
            if let Some(r) = state.results.remove(id) {
                return Some(r);
            }
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms {
            return None;
        }
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

// ── HTTP Server ───────────────────────────────────────────────────────────────

fn run_server(port: u16) {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).expect("failed to bind");
    
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            thread::spawn(|| handle_connection(stream));
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0u8; 8192];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return,
    };
    
    let request = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = request.lines().collect();
    if lines.is_empty() { return; }
    
    let first_line: Vec<&str> = lines[0].split_whitespace().collect();
    if first_line.len() < 2 { return; }
    
    let method = first_line[0];
    let path = first_line[1];
    
    // Parse body
    let body = if method == "POST" {
        if let Some(body_start) = request.find("\r\n\r\n") {
            request[body_start + 4..].to_string()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let response = route(method, path, &body);
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        response.len(), response
    );
    let _ = stream.write_all(http.as_bytes());
}

fn route(method: &str, path: &str, body: &str) -> String {
    let state = STATE.lock().unwrap();
    
    match (method, path) {
        ("GET", "/health") => r#"{"status":"ok","engine":"rust"}"#.to_string(),

        ("GET", "/screenshot") => state.screen_nodes.clone(),

        ("GET", "/notifications") => {
            let notifs: Vec<&String> = state.notifications.iter().collect();
            format!("[{}]", notifs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(","))
        },

        // All commands that need Java Accessibility Service go through command queue
        ("POST", "/tap") |
        ("POST", "/long_press") |
        ("POST", "/swipe") |
        ("POST", "/type") |
        ("POST", "/find_and_tap") |
        ("POST", "/scroll") |
        ("POST", "/open") |
        ("POST", "/volume") |
        ("POST", "/brightness") |
        ("POST", "/torch") |
        ("POST", "/clipboard_set") => {
            drop(state); // release lock before waiting
            let endpoint = &path[1..]; // strip leading /
            let id = gen_id();
            let cmd = format!(r#"{{"endpoint":"{}","body":{}}}"#, endpoint, if body.is_empty() { "{}" } else { body });
            queue_command(id.clone(), cmd);
            match wait_result(&id, 8000) {
                Some(r) => r,
                None => r#"{"error":"timeout"}"#.to_string(),
            }
        },

        ("GET", "/back") |
        ("GET", "/home") |
        ("GET", "/recents") |
        ("GET", "/wake_screen") |
        ("GET", "/lock") |
        ("GET", "/get_focused") |
        ("GET", "/clipboard_get") |
        ("GET", "/installed_apps") |
        ("GET", "/recent_apps") => {
            drop(state);
            let endpoint = &path[1..];
            let id = gen_id();
            let cmd = format!(r#"{{"endpoint":"{}","body":{{}}}}"#, endpoint);
            queue_command(id.clone(), cmd);
            match wait_result(&id, 8000) {
                Some(r) => r,
                None => r#"{"error":"timeout"}"#.to_string(),
            }
        },

        ("POST", "/shizuku") => {
            drop(state);
            let id = gen_id();
            let cmd = format!(r#"{{"endpoint":"shizuku","body":{}}}"#, if body.is_empty() { "{}" } else { body });
            queue_command(id.clone(), cmd);
            match wait_result(&id, 15000) {
                Some(r) => r,
                None => r#"{"error":"shizuku timeout"}"#.to_string(),
            }
        },

        // Sensor data — read directly
        ("GET", "/battery") => {
            drop(state);
            let id = gen_id();
            let cmd = r#"{"endpoint":"battery","body":{}}"#.to_string();
            queue_command(id.clone(), cmd);
            wait_result(&id, 3000).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
        },

        ("GET", "/sensors") => {
            drop(state);
            let id = gen_id();
            let cmd = r#"{"endpoint":"sensors","body":{}}"#.to_string();
            queue_command(id.clone(), cmd);
            wait_result(&id, 3000).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
        },

        ("POST", "/record_audio") => {
            drop(state);
            let id = gen_id();
            let cmd = format!(r#"{{"endpoint":"record_audio","body":{}}}"#, if body.is_empty() { "{}" } else { body });
            queue_command(id.clone(), cmd);
            wait_result(&id, 30000).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
        },

        _ => r#"{"error":"not found"}"#.to_string(),
    }
}

fn gen_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{}{}", t.as_millis(), t.subsec_nanos() % 10000)
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
}
