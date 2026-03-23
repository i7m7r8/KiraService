// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: channels :: whatsapp  (Session 8)
//
// WhatsApp channel adapter  -  two modes:
//   A) Cloud API (Meta WhatsApp Business Cloud API  -  official, no phone needed)
//   B) Webhook bridge (Java runs a minimal Baileys forwarder, POSTs to Rust)
//
// Mode is selected by config:
//   cloud_api_token set  → Mode A (Cloud API)
//   cloud_api_token empty → Mode B (webhook bridge, same as existing Telegram pattern)
//
// Mirrors OpenClaw: src/channels/plugins/outbound/whatsapp.ts
//                   src/channels/plugins/normalize/whatsapp.ts
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::shared::{DmPolicy, SendResult};
use std::sync::{Arc, Mutex, OnceLock};

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct WhatsAppConfig {
    /// Mode A: Meta Cloud API bearer token
    pub cloud_api_token: String,
    /// Mode A: WhatsApp Business phone number ID
    pub phone_number_id: String,
    /// Mode A: Webhook verify token (for Cloud API webhook verification)
    pub webhook_verify_token: String,
    /// Mode B: bridge token (shared secret between Java Baileys bridge and Rust)
    pub bridge_token: String,
    pub dm_policy: DmPolicy,
    /// Comma-separated allowed JIDs/numbers (empty = allow all matching dm_policy)
    pub allowlist: Vec<String>,
}

impl WhatsAppConfig {
    pub fn mode_a(&self) -> bool { !self.cloud_api_token.is_empty() }
    pub fn is_configured(&self) -> bool {
        !self.cloud_api_token.is_empty() || !self.bridge_token.is_empty()
    }

    const CLOUD_HOST: &'static str = "graph.facebook.com";
    const CLOUD_PORT: u16 = 443;

    fn cloud_send_path(&self) -> String {
        format!("/v18.0/{}/messages", self.phone_number_id)
    }
}

// ── Inbound message ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct WaInbound {
    pub from:       String,   // phone number or JID
    pub name:       String,
    pub text:       String,
    pub msg_id:     String,
    pub chat_id:    String,   // group JID or same as from for DM
    pub is_group:   bool,
    pub ts:         u128,
    pub media_type: Option<String>,
    pub media_id:   Option<String>,
}

// ── Global state ──────────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    pub static ref WA_STATE: Arc<Mutex<WaRuntime>> =
        Arc::new(Mutex::new(WaRuntime::default()));
}

#[derive(Debug, Default)]
pub struct WaRuntime {
    pub config:       WhatsAppConfig,
    pub pending_sends: std::collections::VecDeque<WaOutbound>,
    pub message_log:  std::collections::VecDeque<WaInbound>,
    pub pairing_codes: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct WaOutbound {
    pub to:   String,
    pub text: String,
    pub ts:   u128,
}

// ── OnceLock shims ────────────────────────────────────────────────────────────

type HttpsPostFn = fn(&str, u16, &str, &str, &str, u64) -> Result<String, String>;
type AiReplyFn   = fn(&str, &str, &str) -> String; // (text, chat_id, username) → reply

static WA_POST: OnceLock<HttpsPostFn> = OnceLock::new();
static WA_AI:   OnceLock<AiReplyFn>   = OnceLock::new();

pub fn register_wa_fns(post: HttpsPostFn, ai: AiReplyFn) {
    let _ = WA_POST.set(post);
    let _ = WA_AI.set(ai);
}

// ── Mode A: Cloud API send ────────────────────────────────────────────────────

/// Send a text message via WhatsApp Cloud API.
pub fn cloud_send_text(to: &str, text: &str) -> SendResult {
    let (post, cfg) = match (WA_POST.get(),
        WA_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone())
    {
        (Some(p), c) => (p, c),
        _ => return SendResult::Err { message: "not configured".into(), retryable: false },
    };

    // WhatsApp Cloud API: text message payload
    let body = format!(
        r#"{{"messaging_product":"whatsapp","recipient_type":"individual","to":"{}","type":"text","text":{{"preview_url":false,"body":"{}"}}}}"#,
        to.replace('"',"\\\""),
        text.replace('"',"\\\"").replace('\n',"\\n")
    );

    match post(
        WhatsAppConfig::CLOUD_HOST, WhatsAppConfig::CLOUD_PORT,
        &cfg.cloud_send_path(), &body, &cfg.cloud_api_token, 20
    ) {
        Ok(resp) => {
            // Parse message id from response
            let mid = extract_wa_message_id(&resp)
                .unwrap_or_else(|| "unknown".to_string());
            SendResult::Ok { message_id: mid }
        }
        Err(e) => SendResult::Err {
            message: e,
            retryable: true,
        }
    }
}

/// Mark a message as read (Cloud API).
pub fn cloud_mark_read(msg_id: &str) {
    let (post, cfg) = match (WA_POST.get(),
        WA_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone())
    {
        (Some(p), c) if c.mode_a() => (p, c),
        _ => return,
    };
    let body = format!(
        r#"{{"messaging_product":"whatsapp","status":"read","message_id":"{}"}}"#,
        msg_id.replace('"',"\\\"")
    );
    let _ = post(
        WhatsAppConfig::CLOUD_HOST, WhatsAppConfig::CLOUD_PORT,
        &cfg.cloud_send_path(), &body, &cfg.cloud_api_token, 10
    );
}

// ── Mode B: bridge send (queued for Java to deliver) ─────────────────────────

/// Queue a message for the Java Baileys bridge to send.
pub fn bridge_queue_send(to: &str, text: &str) {
    let mut s = WA_STATE.lock().unwrap_or_else(|e|e.into_inner());
    s.pending_sends.push_back(WaOutbound {
        to: to.to_string(), text: text.to_string(),
        ts: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default().as_millis(),
    });
}

// ── Inbound message processing ────────────────────────────────────────────────

/// Process an inbound message (from either mode).
/// Called from:
///   Mode A: POST /whatsapp/webhook (Cloud API webhook)
///   Mode B: POST /whatsapp/bridge/incoming
pub fn process_inbound(msg: WaInbound) {
    // Store in log
    {
        let mut s = WA_STATE.lock().unwrap_or_else(|e|e.into_inner());
        s.message_log.push_back(msg.clone());
        if s.message_log.len() > 100 { s.message_log.pop_front(); }
    }

    if msg.text.is_empty() { return; }

    // Allowlist check
    let (allowed, dm_policy, _allowlist) = {
        let s = WA_STATE.lock().unwrap_or_else(|e|e.into_inner());
        let allow = s.allowlist_check(&msg.from);
        (allow, s.config.dm_policy.clone(), s.config.allowlist.clone())
    };

    if !allowed {
        match dm_policy {
            DmPolicy::Deny => return,
            DmPolicy::Pairing => {
                let code = {
                    let mut s = WA_STATE.lock().unwrap_or_else(|e|e.into_inner());
                    s.pairing_codes.entry(msg.from.clone())
                        .or_insert_with(|| gen_wa_code(&msg.from))
                        .clone()
                };
                let pairing_msg = format!("🔐 Pairing required. Code: {}\n\nShare with operator.", code);
                // Send back through appropriate channel
                let cfg = WA_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.clone();
                if cfg.mode_a() {
                    let _ = cloud_send_text(&msg.from, &pairing_msg);
                } else {
                    bridge_queue_send(&msg.from, &pairing_msg);
                }
                return;
            }
            DmPolicy::Open => {}
        }
    }

    // Run AI in background thread
    let from   = msg.from.clone();
    let name   = msg.name.clone();
    let text   = msg.text.clone();
    let mode_a = WA_STATE.lock().unwrap_or_else(|e|e.into_inner()).config.mode_a();

    std::thread::spawn(move || {
        let reply = match WA_AI.get() {
            Some(f) => f(&text, &from, &name),
            None    => return,
        };
        if reply.is_empty() { return; }
        if mode_a {
            let _ = cloud_send_text(&from, &reply);
        } else {
            bridge_queue_send(&from, &reply);
        }
    });
}

impl WaRuntime {
    fn allowlist_check(&self, from: &str) -> bool {
        if self.config.allowlist.is_empty() { return true; }
        self.config.allowlist.iter().any(|a| a == from || a == "*")
    }
}

// ── Cloud API webhook parser ──────────────────────────────────────────────────

/// Parse a WhatsApp Cloud API webhook payload into WaInbound messages.
/// Handles the nested structure: entry[0].changes[0].value.messages[0]
pub fn parse_cloud_webhook(body: &str) -> Vec<WaInbound> {
    let mut msgs = Vec::new();

    // Find all "messages" arrays
    let mut pos = 0;
    while let Some(rel) = body[pos..].find("\"messages\":") {
        let abs = pos + rel + 11; // skip past "messages":
        let slice = body[abs..].trim_start();
        if !slice.starts_with('[') { pos = abs; continue; }

        // Walk message objects
        let mut ip = 1usize; // skip '['
        let bytes = slice.as_bytes();
        loop {
            while ip < bytes.len() && (bytes[ip] == b' ' || bytes[ip] == b',' || bytes[ip] == b'\n') { ip += 1; }
            if ip >= bytes.len() || bytes[ip] == b']' { break; }
            if bytes[ip] != b'{' { break; }

            let obj_end = find_wa_obj_end(slice, ip).unwrap_or(slice.len());
            let obj = &slice[ip..obj_end];

            let msg_id    = str_field(obj, "id").unwrap_or_default();
            let from      = str_field(obj, "from").unwrap_or_default();
            let ts_str    = str_field(obj, "timestamp").unwrap_or_default();
            let ts: u128  = ts_str.parse::<u128>().unwrap_or(0) * 1000;
            let msg_type  = str_field(obj, "type").unwrap_or_default();

            let (text, media_type, media_id) = if msg_type == "text" {
                (wa_nested_str(obj, "text", "body").unwrap_or_default(), None, None)
            } else if msg_type == "image" || msg_type == "audio" || msg_type == "video" {
                (format!("[{} message]", msg_type),
                 Some(msg_type.clone()),
                 wa_nested_str(obj, &msg_type, "id"))
            } else {
                (String::new(), None, None)
            };

            if !from.is_empty() {
                msgs.push(WaInbound {
                    from: from.clone(), name: from.clone(),
                    text, msg_id, chat_id: from,
                    is_group: false, ts, media_type, media_id,
                });
            }

            ip = obj_end;
        }
        pos = abs + ip;
    }
    msgs
}

fn find_wa_obj_end(json: &str, from: usize) -> Option<usize> {
    let bytes = json.as_bytes();
    let mut depth = 0i32;
    let mut i = from;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => { depth -= 1; if depth == 0 { return Some(i + 1); } }
            b'"' => { i += 1; while i < bytes.len() { if bytes[i] == b'"' && bytes[i.saturating_sub(1)] != b'\\' { break; } i += 1; } }
            _ => {}
        }
        i += 1;
    }
    None
}

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
    if v.is_empty() { None } else { Some(v.to_string()) }
}

fn wa_nested_str(json: &str, obj_key: &str, field: &str) -> Option<String> {
    let marker = format!("\"{}\":", obj_key);
    let start = json.find(&marker)? + marker.len();
    let slice = json[start..].trim_start();
    if !slice.starts_with('{') { return None; }
    let end = find_wa_obj_end(slice, 0).unwrap_or(slice.len());
    str_field(&slice[..end], field)
}

fn extract_wa_message_id(json: &str) -> Option<String> {
    // {"messages":[{"id":"wamid.xxx"}]}
    let start = json.find("\"messages\":")?;
    str_field(&json[start..], "id")
}

fn gen_wa_code(from: &str) -> String {
    let mut h: u64 = 5381;
    for b in from.bytes() { h = h.wrapping_mul(33).wrapping_add(b as u64); }
    const C: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    (0..6).map(|i| C[((h >> (i*5)) as usize) % C.len()] as char).collect()
}
