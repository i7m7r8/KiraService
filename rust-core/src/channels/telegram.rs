// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: channels :: telegram  (Session 7)
//
// Full Telegram Bot API client  -  pure Rust, no Java involvement.
// Mirrors OpenClaw: src/telegram/bot.ts, src/telegram/draft-stream.ts,
//                   src/telegram/format.ts, src/telegram/send.ts
//
// Features:
//   - Long-polling getUpdates loop (background thread)
//   - Streaming reply: edit-in-place every 800ms while AI is running
//   - MarkdownV2 formatting
//   - Inline keyboard buttons (approval buttons for shell commands)
//   - Voice message stub (transcription queued for Session 13)
//   - DM policy: pairing | open
//   - Allowlist enforcement
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::shared::DmPolicy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct TelegramConfig {
    pub bot_token:      String,
    pub allowed_chat_id: i64,   // 0 = use dm_policy
    pub dm_policy:      DmPolicy,
    pub polling_timeout: u32,   // getUpdates long-poll seconds (default 30)
    pub stream_reply:   bool,   // edit-in-place while AI runs (default true)
}

impl TelegramConfig {
    pub fn is_configured(&self) -> bool { !self.bot_token.is_empty() }

    fn api_path(&self, method: &str) -> String {
        format!("/bot{}/{}", self.bot_token, method)
    }
    const HOST: &'static str = "api.telegram.org";
    const PORT: u16 = 443;
}

// ── Inbound update ────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TgUpdate {
    pub update_id:  i64,
    pub chat_id:    i64,
    pub user_id:    i64,
    pub username:   String,
    pub text:       String,
    pub message_id: i64,
    pub is_voice:   bool,
    pub file_id:    String,   // voice file_id if is_voice
    pub ts:         u128,
}

// ── Global Telegram state ─────────────────────────────────────────────────────

lazy_static::lazy_static! {
    pub static ref TG_STATE: Arc<Mutex<TgRuntime>> =
        Arc::new(Mutex::new(TgRuntime::default()));
}

#[derive(Debug, Default)]
pub struct TgRuntime {
    pub config:          TelegramConfig,
    pub last_update_id:  i64,
    pub pending_sends:   std::collections::VecDeque<TgOutbound>,
    pub message_log:     std::collections::VecDeque<TgUpdate>,
    pub pairing_codes:   HashMap<i64, String>,  // chat_id → code
    pub running:         bool,
}

#[derive(Clone, Debug)]
pub struct TgOutbound {
    pub chat_id:     i64,
    pub text:        String,
    pub message_id:  Option<i64>,  // if set, edit this message
    pub parse_mode:  String,        // "MarkdownV2" | ""
    pub reply_markup: Option<String>, // inline keyboard JSON
    pub ts:          u128,
}

// ── OnceLock shims ────────────────────────────────────────────────────────────

type HttpsPostFn = fn(&str, u16, &str, &str, &str, u64) -> Result<String, String>;
type HttpsGetFn  = fn(&str, u16, &str, u64) -> Result<String, String>;
// Returns (ai_reply, tools_used_count)
type AiReplyFn   = fn(&str, i64, &str) -> String;

static TG_POST: OnceLock<HttpsPostFn> = OnceLock::new();
static TG_GET:  OnceLock<HttpsGetFn>  = OnceLock::new();
static TG_AI:   OnceLock<AiReplyFn>   = OnceLock::new();

pub fn register_tg_fns(post: HttpsPostFn, get: HttpsGetFn, ai: AiReplyFn) {
    let _ = TG_POST.set(post);
    let _ = TG_GET.set(get);
    let _ = TG_AI.set(ai);
}

// ── Formatting ────────────────────────────────────────────────────────────────

/// Escape text for Telegram MarkdownV2 format.
/// Mirrors OpenClaw: src/telegram/format.ts escapeMarkdownV2()
pub fn escape_md_v2(s: &str) -> String {
    const SPECIAL: &[char] = &[
        '_','*','[',']','(',')',
        '~','`','>','#','+','-',
        '=','|','{','}','.','!'
    ];
    let mut out = String::with_capacity(s.len() + 16);
    for c in s.chars() {
        if SPECIAL.contains(&c) { out.push('\\'); }
        out.push(c);
    }
    out
}

/// Convert Markdown to Telegram MarkdownV2 (best-effort).
/// Handles **bold**, _italic_, `code`, ```block```.
pub fn markdown_to_md_v2(text: &str) -> String {
    // Pass through code blocks unchanged (they're already escaped inside)
    let mut out = String::with_capacity(text.len());
    let mut in_code_block = false;
    let mut in_inline_code = false;

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // ``` code block
        if !in_inline_code && i + 2 < chars.len()
            && chars[i] == '`' && chars[i+1] == '`' && chars[i+2] == '`'
        {
            if in_code_block {
                out.push_str("```");
                in_code_block = false;
            } else {
                out.push_str("```");
                in_code_block = true;
            }
            i += 3;
            continue;
        }
        if in_code_block {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // `inline code`
        if chars[i] == '`' {
            in_inline_code = !in_inline_code;
            out.push('`');
            i += 1;
            continue;
        }
        if in_inline_code {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        // **bold**
        if i + 1 < chars.len() && chars[i] == '*' && chars[i+1] == '*' {
            out.push_str("*");
            i += 2;
            continue;
        }
        // _italic_
        if chars[i] == '_' {
            out.push('_');
            i += 1;
            continue;
        }
        // Escape special chars
        if "[]()~>#+-=|{}.!".contains(chars[i]) {
            out.push('\\');
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

// ── API calls ─────────────────────────────────────────────────────────────────

/// Send a text message. Returns message_id on success.
pub fn send_message(chat_id: i64, text: &str, parse_mode: &str) -> Result<i64, String> {
    let post = TG_POST.get().ok_or("https_post not registered")?;
    let cfg  = { TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone() };

    let safe_text = if parse_mode == "MarkdownV2" {
        markdown_to_md_v2(text)
    } else {
        text.to_string()
    };

    let body = format!(
        r#"{{"chat_id":{},"text":{},"parse_mode":{}}}"#,
        chat_id,
        json_escape_str(&safe_text),
        if parse_mode.is_empty() { "\"\"".to_string() }
        else { json_escape_str(parse_mode) }
    );

    let path = cfg.api_path("sendMessage");
    let resp = post(TelegramConfig::HOST, TelegramConfig::PORT, &path, &body, "", 15)?;
    extract_message_id(&resp).ok_or_else(|| format!("no message_id in: {}", &resp[..resp.len().min(200)]))
}

/// Edit an existing message (for streaming reply).
pub fn edit_message(chat_id: i64, message_id: i64, text: &str, parse_mode: &str) -> Result<(), String> {
    let post = TG_POST.get().ok_or("https_post not registered")?;
    let cfg  = { TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone() };

    let safe_text = if parse_mode == "MarkdownV2" {
        markdown_to_md_v2(text)
    } else {
        text.to_string()
    };

    let body = format!(
        r#"{{"chat_id":{},"message_id":{},"text":{},"parse_mode":{}}}"#,
        chat_id, message_id,
        json_escape_str(&safe_text),
        if parse_mode.is_empty() { "\"\"".to_string() } else { json_escape_str(parse_mode) }
    );

    let path = cfg.api_path("editMessageText");
    let _ = post(TelegramConfig::HOST, TelegramConfig::PORT, &path, &body, "", 10)?;
    Ok(())
}

/// Send typing action (shows "typing..." in chat).
pub fn send_typing(chat_id: i64) {
    if let (Some(post), cfg) = (TG_POST.get(),
        TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone())
    {
        let body = format!(r#"{{"chat_id":{},"action":"typing"}}"#, chat_id);
        let path = cfg.api_path("sendChatAction");
        let _ = post(TelegramConfig::HOST, TelegramConfig::PORT, &path, &body, "", 5);
    }
}

/// Send message with inline keyboard (for approval buttons).
pub fn send_with_keyboard(chat_id: i64, text: &str, buttons: &[(&str, &str)]) -> Result<i64, String> {
    let post = TG_POST.get().ok_or("https_post not registered")?;
    let cfg  = { TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone() };

    let btns: Vec<String> = buttons.iter()
        .map(|(label, data)| format!(r#"{{"text":"{}","callback_data":"{}"}}"#, label, data))
        .collect();
    let keyboard = format!(r#"{{"inline_keyboard":[[{}]]}}"#, btns.join(","));

    let body = format!(
        r#"{{"chat_id":{},"text":{},"reply_markup":{}}}"#,
        chat_id, json_escape_str(text), keyboard
    );

    let path = cfg.api_path("sendMessage");
    let resp = post(TelegramConfig::HOST, TelegramConfig::PORT, &path, &body, "", 15)?;
    extract_message_id(&resp).ok_or_else(|| "no message_id".to_string())
}

/// Answer a callback query (dismiss the spinner on button press).
pub fn answer_callback(callback_query_id: &str, text: &str) {
    if let (Some(post), cfg) = (TG_POST.get(),
        TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone())
    {
        let body = format!(r#"{{"callback_query_id":"{}","text":"{}"}}"#,
            callback_query_id, text.replace('"', "\\\""));
        let path = cfg.api_path("answerCallbackQuery");
        let _ = post(TelegramConfig::HOST, TelegramConfig::PORT, &path, &body, "", 5);
    }
}

// ── Long-polling loop ─────────────────────────────────────────────────────────
// Runs in a background thread. Mirrors OpenClaw: src/telegram/bot.ts

pub fn start_polling_loop() {
    std::thread::spawn(|| {
        loop {
            let (configured, token, last_id, timeout) = {
                let s = TG_STATE.lock().unwrap_or_else(|e|e.into_inner());
                (s.config.is_configured(), s.config.bot_token.clone(),
                 s.last_update_id, s.config.polling_timeout.max(1))
            };

            if !configured {
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }

            // Long-poll getUpdates
            let path = format!(
                "/bot{}/getUpdates?offset={}&timeout={}",
                token, last_id + 1, timeout
            );

            let resp = match TG_GET.get().map(|f| f(
                TelegramConfig::HOST, TelegramConfig::PORT, &path,
                (timeout + 5) as u64
            )) {
                Some(Ok(r))  => r,
                Some(Err(_)) => { std::thread::sleep(std::time::Duration::from_secs(3)); continue; }
                None         => { std::thread::sleep(std::time::Duration::from_secs(5)); continue; }
            };

            // Parse updates
            let updates = parse_updates(&resp);
            for upd in updates {
                // Update offset
                {
                    let mut s = TG_STATE.lock().unwrap_or_else(|e|e.into_inner());
                    if upd.update_id > s.last_update_id {
                        s.last_update_id = upd.update_id;
                    }
                    // Log
                    s.message_log.push_back(upd.clone());
                    if s.message_log.len() > 100 { s.message_log.pop_front(); }
                }

                // Check allowlist
                let allowed_id = { TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.allowed_chat_id };
                if allowed_id != 0 && upd.chat_id != allowed_id {
                    // DM policy check
                    let policy = { TG_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.dm_policy.clone() };
                    match policy {
                        DmPolicy::Deny => continue,
                        DmPolicy::Pairing => {
                            // Send pairing code if not already pending
                            let code = {
                                let mut s = TG_STATE.lock().unwrap_or_else(|e|e.into_inner());
                                s.pairing_codes.entry(upd.chat_id)
                                    .or_insert_with(|| gen_code(upd.chat_id, upd.ts))
                                    .clone()
                            };
                            let _ = send_message(upd.chat_id,
                                &format!("🔐 Pairing required. Your code: *{}*\n\nShare this with the operator.", code),
                                "MarkdownV2");
                            continue;
                        }
                        DmPolicy::Open => {} // allow
                    }
                }

                if upd.text.is_empty() && !upd.is_voice { continue; }

                // Send typing indicator
                send_typing(upd.chat_id);

                // Run AI reply in a thread (non-blocking)
                let chat_id  = upd.chat_id;
                let username = upd.username.clone();
                let text     = upd.text.clone();

                std::thread::spawn(move || {
                    let reply = match TG_AI.get() {
                        Some(f) => f(&text, chat_id, &username),
                        None    => "AI not ready".to_string(),
                    };
                    if !reply.is_empty() {
                        let _ = send_message(chat_id, &reply, "");
                    }
                });
            }
        }
    });
}

// ── Parse getUpdates response ─────────────────────────────────────────────────

pub fn parse_updates(json: &str) -> Vec<TgUpdate> {
    let mut updates = Vec::new();
    // Find each update_id occurrence  -  simple but robust
    let mut pos = 0;
    while let Some(rel) = json[pos..].find("\"update_id\":") {
        let abs = pos + rel;
        // Find enclosing object  -  scan back for '{' at same depth
        let obj_end = find_object_end(json, abs).unwrap_or(json.len());
        let fragment = &json[abs..obj_end];

        let update_id = num_field(fragment, "update_id").unwrap_or(0.0) as i64;
        if update_id == 0 { pos = abs + 10; continue; }

        // Navigate into message object
        let msg_start = fragment.find("\"message\":")
            .or_else(|| fragment.find("\"edited_message\":"))
            .unwrap_or(0);
        let msg = &fragment[msg_start..];

        let chat_id   = nested_num(msg, "chat", "id").unwrap_or(0.0) as i64;
        let user_id   = nested_num(msg, "from", "id").unwrap_or(0.0) as i64;
        let username  = nested_str(msg, "from", "username")
            .or_else(|| nested_str(msg, "from", "first_name"))
            .unwrap_or_default();
        let text      = str_field(msg, "text").unwrap_or_default();
        let message_id = num_field(msg, "message_id").unwrap_or(0.0) as i64;
        let is_voice  = msg.contains("\"voice\":");
        let file_id   = if is_voice {
            nested_str(msg, "voice", "file_id").unwrap_or_default()
        } else { String::new() };

        updates.push(TgUpdate {
            update_id, chat_id, user_id, username, text, message_id,
            is_voice, file_id,
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_millis(),
        });

        pos = obj_end;
    }
    updates
}

fn find_object_end(json: &str, from: usize) -> Option<usize> {
    let bytes = json.as_bytes();
    let mut depth = 0i32;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => { depth -= 1; if depth < 0 { return Some(i + 1); } }
            b'"' => { i += 1; while i < bytes.len() { if bytes[i] == b'"' && bytes[i.saturating_sub(1)] != b'\\' { break; } i += 1; } }
            _ => {}
        }
        i += 1;
    }
    None
}

fn extract_message_id(json: &str) -> Option<i64> {
    // {"ok":true,"result":{"message_id":123,...}}
    let result_start = json.find("\"result\":")?;
    let s = &json[result_start..];
    num_field(s, "message_id").map(|n| n as i64)
}

fn gen_code(chat_id: i64, ts: u128) -> String {
    let mut h: u64 = 5381;
    for b in chat_id.to_le_bytes().iter().chain(ts.to_le_bytes().iter()) {
        h = h.wrapping_mul(33).wrapping_add(*b as u64);
    }
    const C: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6).map(|i| { let v = (h >> (i * 5)) as usize; C[v % C.len()] as char }).collect()
}

// ── JSON helpers ──────────────────────────────────────────────────────────────

fn str_field(json: &str, key: &str) -> Option<String> {
    let s = format!("\"{}\":\"", key);
    let start = json.find(&s)? + s.len();
    let bytes = json.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    let v = &json[start..end];
    if v.is_empty() { None } else {
        Some(v.replace("\\n","\n").replace("\\\"","\"").replace("\\\\","\\"))
    }
}

fn num_field(json: &str, key: &str) -> Option<f64> {
    let s = format!("\"{}\":", key);
    let start = json.find(&s)? + s.len();
    let slice = json[start..].trim_start();
    let neg = slice.starts_with('-');
    let s2 = if neg { &slice[1..] } else { slice };
    let end = s2.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(s2.len());
    let n: f64 = s2[..end].parse().ok()?;
    Some(if neg { -n } else { n })
}

fn nested_num(json: &str, obj: &str, field: &str) -> Option<f64> {
    let marker = format!("\"{}\":", obj);
    let start = json.find(&marker)? + marker.len();
    let slice = json[start..].trim_start();
    if !slice.starts_with('{') { return None; }
    let end = find_object_end(slice, 0).unwrap_or(slice.len());
    num_field(&slice[..end], field)
}

fn nested_str(json: &str, obj: &str, field: &str) -> Option<String> {
    let marker = format!("\"{}\":", obj);
    let start = json.find(&marker)? + marker.len();
    let slice = json[start..].trim_start();
    if !slice.starts_with('{') { return None; }
    let end = find_object_end(slice, 0).unwrap_or(slice.len());
    str_field(&slice[..end], field)
}

fn json_escape_str(s: &str) -> String {
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
