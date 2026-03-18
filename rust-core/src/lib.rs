use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

// ── Shared state ──────────────────────────────────────────────────────────────

struct State {
    notifications: VecDeque<String>,
    screen_nodes:  String,
    pending_cmds:  VecDeque<(String, String)>,
    results:       HashMap<String, String>,
}

impl State {
    fn new() -> Self {
        State {
            notifications: VecDeque::new(),
            screen_nodes:  "[]".to_string(),
            pending_cmds:  VecDeque::new(),
            results:       HashMap::new(),
        }
    }
}

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<State>> = Arc::new(Mutex::new(State::new()));
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
        let json = format!(
            r#"{{"package":"{}","title":"{}","text":"{}","timestamp":{}}}"#,
            esc(&pkg), esc(&title), esc(&text), ts
        );
        let mut s = STATE.lock().unwrap();
        s.notifications.push_back(json);
        if s.notifications.len() > 100 {
            s.notifications.pop_front();
        }
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
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        _env:  *mut std::ffi::c_void,
        _class: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let cmd = {
            let mut s = STATE.lock().unwrap();
            s.pending_cmds.pop_front()
        };
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
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
        .expect("bind failed");
    for stream in listener.incoming().flatten() {
        thread::spawn(|| handle(stream));
    }
}

fn handle(mut stream: TcpStream) {
    let mut buf = [0u8; 16384];
    let n = match stream.read(&mut buf) { Ok(n) => n, Err(_) => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let lines: Vec<&str> = req.lines().collect();
    if lines.is_empty() { return; }
    let parts: Vec<&str> = lines[0].split_whitespace().collect();
    if parts.len() < 2 { return; }
    let method = parts[0];
    let path   = parts[1];
    let body   = req.find("\r\n\r\n")
        .map(|i| req[i+4..].to_string())
        .unwrap_or_default();

    let resp = route(method, path, &body);
    let http = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n{}",
        resp.len(), resp
    );
    let _ = stream.write_all(http.as_bytes());
}

fn route(method: &str, path: &str, body: &str) -> String {
    match (method, path) {
        ("GET",  "/health")        => r#"{"status":"ok","engine":"rust","version":"2.0"}"#.to_string(),
        ("GET",  "/screenshot")    => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET",  "/notifications") => {
            let s = STATE.lock().unwrap();
            format!("[{}]", s.notifications.iter().cloned().collect::<Vec<_>>().join(","))
        },
        // All accessibility commands → queue to Java
        ("POST", _) | ("GET", _)   => {
            let endpoint = &path[1..];
            let id = gen_id();
            let cmd_body = if body.is_empty() { "{}".to_string() } else { body.to_string() };
            let payload = format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint, cmd_body);
            {
                STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
            }
            let timeout = if endpoint == "record_audio" { 30000 } else { 8000 };
            wait_result(&id, timeout).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
        }
    }
}

fn wait_result(id: &str, timeout_ms: u64) -> Option<String> {
    let start = std::time::Instant::now();
    loop {
        {
            let mut s = STATE.lock().unwrap();
            if let Some(r) = s.results.remove(id) {
                return Some(r);
            }
        }
        if start.elapsed().as_millis() as u64 >= timeout_ms { return None; }
        thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn gen_id() -> String {
    format!("{}{}", now_ms(), now_ms() % 9999)
}

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "")
}
