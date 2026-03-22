# KiraService — Full Rust Migration + Performance Goals
**Document version:** v49 baseline → target architecture
**Written:** 2026-03-20

---

## Philosophy

Kira is not a typical Android app that happens to use Rust. The target is the inverse: **a Rust intelligence that happens to run on Android**. Every line of Java that exists should exist only because the Android OS requires it. Every line of business logic, state management, AI reasoning, tool dispatch, memory, and compression belongs in Rust.

This document defines every migration, optimization, and addition required to reach that target — in precise, implementable steps.

---

## Part 1: Current State

### File inventory (v49 baseline)

| File | Size | Category |
|---|---|---|
| `MainActivity.java` | 155KB | UI shell + too much logic |
| `KiraTools.java` | 68KB | Tool dispatch — should be 95% Rust |
| `SetupActivity.java` | 45KB | Setup wizard UI |
| `RustBridge.java` | 23KB | JNI declarations — keep, expand |
| `KiraOtaUpdater.java` | 22KB | OTA — mostly good, delta logic in Java |
| `FloatingWindowService.java` | 18KB | Floating window — keep (WindowManager) |
| `KiraAI.java` | 15KB | AI engine — move to Rust entirely |
| `CrashActivity.java` | 14KB | Crash display — keep (Activity) |
| `KiraAccessibilityService.java` | 13KB | Screen reading — keep (OS interface) |
| `KiraTelegram.java` | 12KB | Telegram bot — move HTTP to Rust |
| `KiraWatcher.java` | 9KB | Macro polling — move logic to Rust |
| `ShizukuShell.java` | 8KB | Shell exec — keep (Shizuku JNI) |
| `KiraAgent.java` | 8KB | ReAct agent — move to Rust entirely |
| `KiraChain.java` | 6KB | Chain runner — move to Rust entirely |
| `KiraHeartbeat.java` | 6KB | Heartbeat — move to Rust |
| `KiraSkillEngine.java` | 6KB | Skills — move to Rust |
| `KiraVoiceService.java` | 5KB | Voice — keep (AudioRecord) |
| `KiraVision.java` | 5KB | Vision — keep (Camera2 API) |
| `KiraMemory.java` | 4KB | Memory — already in Rust, Java is redundant |
| `KiraConfig.java` | 3KB | Config — keep thin loader only |
| **`rust-core/src/lib.rs`** | **385KB** | **Rust core — expand significantly** |

**Total Java:** 512KB across 34 files  
**Target Java:** ~120KB across ~12 files (mandatory OS interfaces only)  
**Target Rust:** ~800KB (lib.rs split into modules)

---

## Part 2: What Cannot Move to Rust (Android OS Boundaries)

These 11 Java boundaries are **non-negotiable** — the Android OS calls into Java by design:

1. **`Activity` lifecycle** — `onCreate`, `onResume`, `onStop`, `onDestroy`. Android's `ActivityManagerService` calls these on the JVM. Cannot bypass.
2. **`Service.onStartCommand()`** — OS starts services by instantiating Java classes.
3. **`BroadcastReceiver.onReceive()`** — `BOOT_COMPLETED`, `PACKAGE_REPLACED` delivered via JVM.
4. **`AccessibilityService`** — Must extend this Java class. OS manages lifecycle.
5. **`SensorEventListener`** — `SensorManager.registerListener()` is a Java API.
6. **`PackageInstaller` / `PackageManager`** — Java-only APIs, no NDK equivalent.
7. **`WindowManager.LayoutParams`** — Floating overlay must be created via Java `WindowManager`.
8. **`NotificationManager`** — Notification system is Java-only.
9. **`Intent` / `PendingIntent`** — IPC between Android components. Java API.
10. **`requestPermissions()`** — Only callable from an `Activity`.
11. **`ContentProvider`** — If ever needed for inter-app data sharing.

Everything else is movable.

---

## Part 3: Migration Plan (ordered by impact)

### Phase 1 — Core Intelligence to Rust (Highest Impact)

#### 1.1 KiraAI → `POST /ai/chat` Rust endpoint

**Current:** `KiraAI.java` (15KB) — builds messages, calls OpenAI-compatible API via `HttpURLConnection`, parses JSON, manages history, dispatches tool calls, calls back to Java.

**Target:** Rust handles the entire chat loop. Java calls one JNI function and gets a result.

```rust
// New Rust endpoint — handles complete AI turn
("POST", "/ai/chat") => {
    // body: {"message":"..","session":"default","max_tools":5}
    // 1. Load context from STATE (history, memory, persona)
    // 2. Build OpenAI messages array
    // 3. HTTP call to configured provider (std::net::TcpStream)
    // 4. Parse streaming SSE response
    // 5. If tool calls: dispatch via tool_registry, loop
    // 6. Return final assistant message + tool results
    // Result: {"role":"assistant","content":"..","tools_used":["x"],"tokens":123}
}
```

**New JNI function:**
```java
// RustBridge.java — single call replaces entire KiraAI.java
public static native String chatSync(String message, String sessionId, int maxToolSteps);
// Returns JSON: {"content":"..","tools":[".."],"done":true}
```

**Java `MainActivity` becomes:**
```java
private void sendMessage() {
    String text = inputField.getText().toString().trim();
    new Thread(() -> {
        String result = RustBridge.chatSync(text, "default", 10);
        // parse result, update UI
        uiHandler.post(() -> addKiraBubble(result));
    }).start();
}
```

`KiraAI.java` is deleted. 15KB of Java → 0KB. Rust gains the AI loop.

**Implementation requires in Rust:**
- HTTP client via `std::net::TcpStream` (already have the pattern)
- SSE streaming parser for OpenAI format
- Tool dispatch integrated into the chat loop
- Session-scoped conversation history per `session_id`

---

#### 1.2 KiraAgent + KiraChain → `POST /ai/agent` + `POST /ai/chain`

**Current:** `KiraAgent.java` (8KB) runs a ReAct loop calling `KiraAI` step-by-step. `KiraChain.java` (6KB) runs a thought→action→observe chain.

**Target:** Both are Rust loops that call the same `/ai/chat` logic internally.

```rust
("POST", "/ai/agent") => {
    // body: {"goal":"..","max_steps":25,"session":"agent_default"}
    // ReAct loop: THINK → ACT → OBSERVE → repeat until done or max_steps
    // Each step calls internal chat() with accumulated context
    // Tool results feed back as observations
    // Returns: {"final":"..","steps":8,"tools_used":["x","y"],"success":true}
}

("POST", "/ai/chain") => {
    // body: {"goal":"..","depth":5}
    // Chain-of-thought: builds reasoning chain, not tool-calling loop
    // Returns: {"reasoning":["step1","step2"],"conclusion":".."}
}
```

`KiraAgent.java` deleted. `KiraChain.java` deleted. 14KB → 0KB.

---

#### 1.3 KiraTools → Tool Registry fully in Rust

**Current:** `KiraTools.java` (68KB) — 90+ tool implementations. Most are `ShizukuShell.exec()` calls. Some are Android intents.

**Split:**

**Tools that stay Java** (require Android intents — ~8 tools):
```java
// These 8 tools stay Java, everything else moves to Rust
openApp(pkg)           // startActivity(Intent)
callNumber(num)        // ACTION_CALL Intent
sendSms(to, body)      // SmsManager.sendTextMessage
openBrowser(url)       // ACTION_VIEW Intent
shareText(text)        // ACTION_SEND Intent
pickContact()          // ACTION_PICK Intent
takePicture()          // Camera2 / ACTION_IMAGE_CAPTURE
recordAudio()          // AudioRecord / MediaRecorder
```

**Tools that move to Rust** (~82 tools):
Everything that currently does `ShizukuShell.exec("adb shell ...")`:

```rust
// In Rust tool registry — each tool is a match arm in handle_tool()
fn handle_tool(name: &str, params: &HashMap<String,String>) -> String {
    match name {
        "get_battery"       => /* read battery state from KiraState */,
        "get_wifi"          => /* read wifi state from KiraState */,
        "set_brightness"    => /* cmd via queued ShizukuAction */,
        "set_volume"        => /* cmd via queued ShizukuAction */,
        "get_notifications" => /* read STATE.notifications */,
        "read_screen"       => /* read STATE.screen_nodes */,
        "run_shell"         => /* queue in STATE.pending_shell_cmds */,
        "read_file"         => /* file I/O directly */,
        "write_file"        => /* file I/O directly */,
        "list_files"        => /* std::fs::read_dir */,
        "http_get"          => /* std::net::TcpStream */,
        "http_post"         => /* std::net::TcpStream */,
        "add_memory"        => /* STATE.memory_index.push */,
        "search_memory"     => /* search STATE.memory_index */,
        "run_macro"         => /* STATE.macros trigger */,
        "get_apps"          => /* read STATE.installed_apps */,
        "set_variable"      => /* STATE.variables insert */,
        "get_variable"      => /* STATE.variables get */,
        // ... 65 more
    }
}

// For tools needing Shizuku, queue a command:
// Java polls GET /shell/next_command every 200ms and executes
("GET", "/shell/next_command") => {
    // Returns next queued shell command for Java to execute via ShizukuShell
    // Java executes, POSTs result to /shell/result
    // This way Rust controls the shell without knowing about Shizuku
}
```

**New JNI surface:**
```java
// All tool execution becomes:
public static native String executeTool(String name, String paramsJson);
// Java only intercepts ~8 intent-based tools, routes rest to Rust
```

`KiraTools.java` shrinks from 68KB to ~5KB (just 8 intent dispatchers).

---

#### 1.4 KiraMemory → Delete entirely

**Current:** `KiraMemory.java` (4KB) wraps SharedPreferences for storing conversation history and facts. Rust already has a full `memory_index` with relevance scoring, pinning, and search.

**Target:** Delete `KiraMemory.java`. All reads go through `GET /memory/full` or `GET /memory/search`. All writes through `POST /memory`.

Java that called `KiraMemory` gets replaced with `RustBridge.getMemory()` and `RustBridge.addMemory()`.

---

#### 1.5 KiraSkillEngine → Rust skill registry

**Current:** `KiraSkillEngine.java` (6KB) loads custom skills from memory and dispatches them.

**Target:** Move entirely to Rust. Skills are stored in `STATE.skills` already. The dispatch loop becomes part of the tool registry.

---

### Phase 2 — Background Services to Rust

#### 2.1 KiraWatcher → Rust macro loop

**Current:** `KiraWatcher.java` (9KB) polls `RustBridge.nextMacroAction()` every 5s, calls `KiraAI` when a macro triggers.

**Target:** Remove polling entirely. Rust owns the trigger evaluation loop running on its own thread (already started in `startServer`). When a trigger fires, Rust calls `/ai/chat` internally, then adds the result to a queue. Java polls `GET /macro/pending_results` and dispatches the Android-side actions (intents, notifications).

```rust
// Rust macro execution thread (spawned once in startServer)
fn macro_loop() {
    loop {
        thread::sleep(Duration::from_secs(5));
        let triggered = evaluate_triggers(&STATE);
        for trigger in triggered {
            let result = run_ai_for_macro(&trigger);  // calls chat() internally
            STATE.lock().unwrap().macro_results.push_back(result);
        }
    }
}

("GET", "/macro/pending_results") => {
    // Java polls this, gets list of completed macro actions to execute
    // Returns: [{"action":"open_app","pkg":"com.spotify.music"},...]
}
```

`KiraWatcher.java` shrinks from 9KB to ~30 lines (just the polling loop).

---

#### 2.2 KiraHeartbeat → Rust scheduled jobs

**Current:** `KiraHeartbeat.java` (6KB) runs periodic health checks and proactive suggestions.

**Target:** All scheduling moves to Rust's cron engine (`STATE.cron_jobs`). Java `KiraHeartbeat` becomes a single `handler.postDelayed` that calls `GET /heartbeat/tick` — Rust decides what to do.

---

#### 2.3 KiraTelegram → Rust HTTP polling

**Current:** `KiraTelegram.java` (12KB) implements Telegram bot polling via `HttpURLConnection`. This is pure HTTP — no Android API needed.

**Target:** Move entirely to Rust. Rust polls the Telegram API on its own thread (same pattern as the macro loop). When a message arrives, Rust routes it through the AI engine and queues the reply. Java is not involved at all.

```rust
// Rust Telegram thread
fn telegram_loop() {
    loop {
        if let Some(updates) = poll_telegram_api(&config.tg_token) {
            for update in updates {
                let reply = run_ai_for_message(&update.message);
                send_telegram_reply(&config.tg_token, update.chat_id, &reply);
            }
        }
        thread::sleep(Duration::from_secs(1));
    }
}
```

`KiraTelegram.java` (12KB) → deleted entirely.

---

### Phase 3 — UI Architecture Improvement

#### 3.1 MainActivity.java — shrink from 155KB to ~40KB

**Current:** MainActivity does everything. It builds bubble views, manages conversation state, polls Rust, handles permissions, manages OTA UI, builds settings rows, manages navigation, etc.

**Target architecture:**

```
MainActivity.java (~40KB — UI shell only)
├── onCreate() — sets up views, starts service, returns
├── initViews() — inflates layouts, wires listeners
├── showBubble(json) — renders a pre-built bubble from Rust JSON
├── handleSystemEvent(type) — permission results, onActivityResult
└── onRustEvent(type, data) — single callback from Rust-driven UI events

RustBridge.java (~25KB — expanded JNI surface)
├── chatSync(message) → String JSON
├── agentSync(goal) → String JSON  
├── executeTool(name, params) → String JSON
├── getUiState() → String JSON  ← new: Rust tells Java what to show
├── postUiEvent(type, data)    ← Java tells Rust what happened
└── [all existing methods]
```

**Key change:** Instead of Java building bubble content and deciding what to display, Rust returns a "UI descriptor" JSON that Java mechanically renders:

```json
// GET /ui/state — Java polls this, renders whatever Rust says
{
  "bubbles": [
    {"type":"kira","text":"Hello!","timestamp":1234,"animate":"spring_left"},
    {"type":"user","text":"hi","timestamp":1233,"animate":"spring_right"}
  ],
  "subtitle": "ready",
  "header_border_alpha": 0.12,
  "typing_visible": false,
  "send_enabled": true
}
```

Java becomes a dumb renderer of Rust's instructions. All conversation logic moves to Rust.

---

#### 3.2 SetupActivity.java — shrink from 45KB to ~20KB

**Current:** SetupActivity hardcodes all provider lists, model lists, validation logic, and wizard state in Java.

**Target:** Rust serves setup page data via `/setup/pages` endpoint. Java just renders what Rust provides. Provider lists, model lists, validation rules, and default values all come from Rust (and can be updated without an app update).

```rust
("GET", "/setup/pages") => {
    // Returns all 6 setup pages with their fields, validation rules, and options
    // Java renders generically — no hardcoded lists
}

("POST", "/setup/validate") => {
    // body: {"page":1,"api_key":"sk-..."}
    // Rust validates API key format, tests connection, returns errors
}
```

---

### Phase 4 — Performance Optimizations

#### 4.1 Cargo.toml additions (prioritized)

**Add immediately:**
```toml
# Fast in-memory compression — reduces STATE memory footprint by 3-5x
lz4_flex = { version = "0.11", default-features = false }

# Fast serialization for internal state snapshots (not HTTP API)
# MessagePack is 3x smaller and 10x faster than JSON for binary data
rmp-serde = "1.3"

# Proper authenticated encryption for API keys and secrets
# Replaces the current XOR-based derive_key()
aes-gcm = { version = "0.10", default-features = false, features = ["aes"] }
```

**Add when AI moves to Rust:**
```toml
# For streaming SSE parsing (OpenAI API responses)
# Uses no_std-compatible parsing, no tokio required
httparse = "1.9"
```

**Do NOT add:**
```toml
# tokio — async runtime, 2MB+, not needed for thread-per-task model
# reqwest — HTTP client pulling tokio, use TcpStream directly  
# regex — 1MB+, use manual str methods (already doing this)
# diesel/sqlite — use Android's SQLite from Java for persistence
```

#### 4.2 Rust profile changes

```toml
[profile.release]
opt-level     = "z"
lto           = "fat"          # was: true — fat LTO eliminates more dead code across crates
codegen-units = 1
panic         = "abort"
strip         = "symbols"      # was: true — same effect, more explicit
overflow-checks = true

# Speed profile for hot paths (create a separate build target if needed)
[profile.release-speed]
inherits      = "release"
opt-level     = 3              # maximize speed over size for computation-heavy builds
```

#### 4.3 Memory compression for STATE

Apply LZ4 to the largest STATE fields before storing:

```rust
use lz4_flex::compress_prepend_size;
use lz4_flex::decompress_size_prepended;

// Compress conversation history when pushing to context_turns
// (most turns are similar text, compresses 4-6x)
impl KiraState {
    fn push_turn_compressed(&mut self, role: &str, content: &str) {
        let raw = format!("{}:{}", role, content);
        let compressed = compress_prepend_size(raw.as_bytes());
        self.context_turns_compressed.push_back(compressed);
        // Keep at most 40 turns, ~4KB average = ~160KB uncompressed → ~35KB compressed
        if self.context_turns_compressed.len() > 40 {
            self.context_turns_compressed.pop_front();
        }
    }
}
```

Estimated RAM saving: **40-80MB → 25-45MB** for conversation-heavy sessions.

#### 4.4 AES-256-GCM for secret storage

Replace the current XOR derive_key with proper authenticated encryption:

```rust
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, NewAead};

fn encrypt_secret(plaintext: &str, device_key: &[u8; 32]) -> Vec<u8> {
    let cipher = Aes256Gcm::new(Key::from_slice(device_key));
    let nonce  = Nonce::from_slice(b"kira_nonce_12"); // 12 bytes
    cipher.encrypt(nonce, plaintext.as_bytes()).unwrap()
}

fn decrypt_secret(ciphertext: &[u8], device_key: &[u8; 32]) -> Option<String> {
    let cipher = Aes256Gcm::new(Key::from_slice(device_key));
    let nonce  = Nonce::from_slice(b"kira_nonce_12");
    cipher.decrypt(nonce, ciphertext).ok()
        .and_then(|b| String::from_utf8(b).ok())
}
```

Device key derived from Android `ANDROID_ID` + app signature hash (done in Java once on first run, stored in `KiraState`).

#### 4.5 Rust module split (lib.rs is too large at 385KB)

Split `lib.rs` into focused modules:

```
rust-core/src/
├── lib.rs              — module declarations + startServer
├── state.rs            — KiraState, all structs
├── routes/
│   ├── mod.rs          — route_http dispatcher
│   ├── ai.rs           — /ai/chat, /ai/agent, /ai/chain
│   ├── theme.rs        — /theme, /layer0, /layer1, /layer2
│   ├── auto.rs         — /auto/*, /macro/*
│   ├── ota.rs          — /ota/*
│   ├── settings.rs     — /settings/*
│   ├── memory.rs       — /memory/*
│   └── system.rs       — /battery, /screen, /notifications, etc
├── tools.rs            — handle_tool(), all tool implementations
├── ai.rs               — chat(), agent(), chain() functions
├── memory.rs           — memory_index operations
├── crypto.rs           — AES-GCM, key derivation
├── compression.rs      — LZ4 wrappers for STATE fields
├── telegram.rs         — Telegram bot loop
├── jni_bridge.rs       — all #[no_mangle] JNI exports
└── app_packages.rs     — app_name_to_pkg() — 280+ entries
```

This enables incremental compilation — changing `ai.rs` doesn't recompile `jni_bridge.rs`. Build time drops significantly.

#### 4.6 Cold start optimization

**Current bottleneck in `MainActivity.onCreate()`:**
```java
ai = new KiraAI(this);        // loads history from disk — BLOCKING
agent = new KiraAgent(this);  // same
chain = new KiraChain(this);  // same
initViews();                  // inflates 4 fragments — slow
```

**Target:**
```java
@Override protected void onCreate(Bundle state) {
    super.onCreate(state);
    setContentView(R.layout.activity_main); // frame only — ~5ms
    
    // Everything else deferred
    uiHandler.post(() -> {
        initViews();           // inflate fragments on next frame
        showWelcome();         // show UI immediately
    });
    
    // Heavy init on background thread
    new Thread(() -> {
        KiraForegroundService.start(this); // starts Rust server
        // Rust loads all state — history, memory, config
        // Java doesn't need to load anything
    }).start();
}
```

Target cold start: **~150ms** (down from current ~400ms).

---

### Phase 5 — Rust HTTP Client for AI Calls

Currently Rust has no HTTP client — it uses Java's `HttpURLConnection` via JNI callbacks for AI API calls. This creates unnecessary round-trips.

**Target:** Pure Rust HTTP/HTTPS client using `std::net::TcpStream` + `rustls` (pure-Rust TLS):

```toml
rustls = { version = "0.23", default-features = false, features = ["ring"] }
```

```rust
fn http_post_json(host: &str, port: u16, path: &str, body: &str, token: &str) 
    -> Result<String, String> 
{
    use std::net::TcpStream;
    use std::io::{Write, BufRead, BufReader};
    
    let mut stream = TcpStream::connect((host, port))
        .map_err(|e| e.to_string())?;
    // TLS wrapping via rustls
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        path, host, token, body.len(), body
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
    // Read response, parse SSE stream
    // ...
}
```

This eliminates the JNI callback chain for every AI message, reduces latency by ~10-20ms per message, and keeps the entire AI flow in Rust memory space.

---

### Phase 6 — Shizuku Command Queue Pattern

Currently Rust cannot execute shell commands — it depends on Java calling `ShizukuShell.exec()`. This limits tool execution speed and requires Java involvement in every tool.

**Target pattern:** Rust queues shell commands, Java executes them and returns results.

```rust
// In KiraState:
pending_shell:    VecDeque<ShellJob>,
shell_results:    HashMap<String, ShellResult>,

struct ShellJob {
    id:      String,   // UUID
    cmd:     String,   // shell command to run
    timeout: u64,      // milliseconds
    created: u128,
}

("GET",  "/shell/next") => {
    // Java KiraWatcher polls this every 200ms
    // Returns next pending shell job
    // {"id":"abc","cmd":"pm list packages","timeout":5000}
}

("POST", "/shell/result") => {
    // body: {"id":"abc","stdout":"...","stderr":"","exit_code":0}
    // Rust stores result, unblocks any waiting AI tool call
}
```

```java
// KiraWatcher.java becomes tiny:
void pollShellQueue() {
    String job = RustBridge.getNextShellJob();
    if (job == null) return;
    String id  = parseId(job);
    String cmd = parseCmd(job);
    String out = ShizukuShell.exec(cmd, 5000);
    RustBridge.postShellResult(id, out);
}
```

This pattern lets Rust's tool handler call a shell command and wait for the result without Java being in the control path — Java is just the execution proxy.

---

## Part 7: Security Hardening

### 7.1 HTTP server authentication (enforce on ALL endpoints)

Current: `http_secret` exists but is not consistently checked. Fix: add `check_auth` to every non-public endpoint.

```rust
// In route_http, before dispatching ANY sensitive endpoint:
fn route_http(method: &str, path: &str, body: &str, auth_header: &str) -> String {
    // Public endpoints (no auth needed):
    let public = ["/health", "/layer0", "/layer1", "/layer2/bubbles", "/crash"];
    if !public.contains(&path) {
        if !check_auth(auth_header) {
            return r#"{"error":"unauthorized","code":401}"#.to_string();
        }
    }
    // ... rest of routing
}
```

### 7.2 Encrypted config storage

Replace plain SharedPreferences for API keys with AES-256-GCM encrypted storage:

```java
// KiraConfig.save() — encrypt sensitive fields before saving
String encryptedKey = RustBridge.encryptSecret(cfg.apiKey);
prefs.edit().putString("api_key_enc", encryptedKey).commit();

// KiraConfig.load() — decrypt on load
String encryptedKey = prefs.getString("api_key_enc", "");
cfg.apiKey = encryptedKey.isEmpty() ? "" : RustBridge.decryptSecret(encryptedKey);
```

### 7.3 JNI surface hardening

Every JNI function that takes a string should validate length and content:

```rust
fn cs_safe(p: *const c_char, max_len: usize) -> Result<String, ()> {
    if p.is_null() { return Ok(String::new()); }
    let s = unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() };
    if s.len() > max_len { return Err(()); }
    Ok(s)
}
// Use cs_safe(ptr, 4096) instead of cs(ptr) everywhere
```

---

## Part 8: Target Architecture (End State)

### What Java will contain (~120KB, ~12 files)

```
KiraApp.java           (4KB)  — Application: crash handler only
MainActivity.java      (40KB) — UI shell: views, sensors, permissions, JNI calls
SetupActivity.java     (20KB) — Setup wizard: render Rust page data
CrashActivity.java     (14KB) — Crash display: keep as-is
KiraForegroundService  (2KB)  — starts Rust server, stays alive
KiraAccessibilityService (13KB) — screen data → push to Rust via JNI
BootReceiver.java      (1KB)  — starts service on boot
FloatingWindowService  (18KB) — floating overlay (WindowManager required)
RustBridge.java        (25KB) — JNI declarations (expanded)
ShizukuShell.java      (8KB)  — Shizuku execution bridge
KiraVoiceService.java  (5KB)  — AudioRecord (Java required)
KiraVision.java        (5KB)  — Camera2 (Java required)
```

### What Rust will contain (~800KB split into modules)

```
lib.rs + modules:
  ai/          — chat, agent, chain, streaming SSE, tool loop
  tools/       — 90+ tool implementations, app_packages
  memory/      — memory_index, compression, search
  auto/        — macros, triggers, conditions, variables
  telegram/    — bot polling, message routing
  ota/         — version check, delta download planning, SHA verify
  crypto/      — AES-256-GCM, key derivation, secret storage
  routes/      — all HTTP endpoints
  state/       — KiraState, all structs, lz4 compression
  jni/         — all JNI exports (thin wrappers only)
```

### Performance targets

| Metric | v49 current | Target |
|---|---|---|
| Cold start to first frame | ~400ms | ~150ms |
| AI response dispatch latency | ~5ms | ~1ms |
| RAM at idle | ~60MB | ~35MB |
| RAM during conversation | ~80MB | ~45MB |
| APK arm64 size | ~8MB | ~5MB |
| Concurrent automations | 100s | 10,000s |
| Tool execution (shell) | ~50ms | ~15ms (queue pattern) |
| Secret storage | XOR obfuscation | AES-256-GCM |

---

## Implementation Order (by session)

**Session A:** ✅ DONE (v0.0.6) — Split lib.rs into 5 modules via include!(): state.rs (1627L), jni_bridge.rs (1252L), http.rs (2768L), app_packages.rs (1762L), utils.rs (25L)  
**Session B:** ✅ DONE (v0.0.7) — lz4_flex added; context_turns_lz4 VecDeque<Vec<u8>> in KiraState; push_turn_compressed/decompress_context/lz4_pack_turn helpers; wired into pushContextTurn JNI; GET /memory/compression stats endpoint  
**Session C:** ✅ DONE (v0.0.8) — aes-gcm added; aes_encrypt/aes_decrypt/derive_aes_key/derive_nonce in utils.rs; encryptSecret/decryptSecret/deriveKeySeed JNI exports; KiraConfig.java rewrites apiKey+tgToken with AES-256-GCM; transparent migration from plaintext  
**Session D:** ✅ DONE (v0.0.8) — call_llm_sync/parse_tool_calls/clean_reply/dispatch_tool/build_system_prompt in state.rs; POST /ai/chat + GET /ai/history + DELETE /ai/history in http.rs; ShellJob queue pattern; chatSync/getNextShellJob/postShellResult JNI; KiraAI.java 377→116 lines  
**Session E:** ✅ DONE (v0.0.9) — /ai/agent (ReAct loop), /ai/chain (CoT), /ai/agent/status, /ai/agent/stop; AgentTask struct + agent_tasks VecDeque; agentSync/chainSync/stopAgent JNI; KiraAgent 212→89 lines, KiraChain 158→72 lines  
**Session F:** ✅ DONE (v0.0.9) — /telegram/incoming (AI processing), /telegram/next_send (reply queue), /telegram/log, /telegram/last_update_id; TgSend+TgMessage structs; KiraTelegram 333→177 lines (Java only polls Telegram API + dispatches to Rust)  
**Session G:** ✅ DONE (v0.0.9) — executeTool JNI (delegates 82 tools to Rust dispatch_tool()); appNameToPkg JNI; KiraTools 1073→154 lines (8 intent tools: open_app, call_number, send_sms, open_url, share_text, set_clipboard, press_home, press_back)  
**Session H:** ✅ DONE (v0.1.0) — /macro/tick evaluates all AutoMacro triggers in Rust; /macro/pending_results queue; foreground_pkg in KiraState; KiraWatcher 229→176 lines (Java only reads battery/pkg/screen, POSTs to Rust)  
**Session I:** ✅ DONE (v0.1.0) — AI object init deferred off main thread; requestAllPermissions delayed 2s; GET /ai/history polled on startup to restore compressed context  
**Session J:** ✅ DONE (v0.1.0) — GET /setup/providers (6 providers + models), POST /setup/validate (key format check per provider), GET /setup/status; SetupActivity loads providers from Rust with hardcoded fallback  
**Session K:** ✅ DONE (v0.1.1) — rustls 0.23 + ring + webpki-roots added; https_post/https_get in utils.rs; call_llm_sync upgraded: HTTPS via rustls for port 443, plain TcpStream fallback for localhost/LAN providers; zero Java round-trip for AI API calls on arm64  
**Session L:** ✅ DONE (v0.1.1) — requires_auth default-deny with public whitelist; cs_safe() bounded JNI input validation (32KB message cap, 256B seed cap, 64B domain cap); GET /security/audit (reports key encryption, auth, TLS, Shizuku); POST /security/rotate_secret; lto = 'fat' for cross-crate dead code elimination  

---

*This document is the authoritative goal specification. Each session should reference it, implement one lettered session, and verify against the metrics table.*
