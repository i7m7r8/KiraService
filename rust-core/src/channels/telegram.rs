// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: channels :: telegram
//
// Telegram Bot API client — full parity with OpenClaw src/telegram/.
// Session 1: types + stubs.
// Session 7: implement long-polling, send, streaming delivery, formatting.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Configuration for the Telegram channel
#[derive(Clone, Debug, Default)]
pub struct TelegramConfig {
    pub bot_token:      String,
    pub allowed_user_id: i64,   // 0 = accept all (with dm_policy)
    pub dm_policy:      super::shared::DmPolicy,
    pub polling_timeout: u32,   // getUpdates timeout in seconds
}

impl TelegramConfig {
    pub fn is_configured(&self) -> bool {
        !self.bot_token.is_empty()
    }

    pub fn api_host() -> &'static str { "api.telegram.org" }
    pub fn api_port() -> u16 { 443 }
    pub fn api_path(token: &str, method: &str) -> String {
        format!("/bot{}/{}", token, method)
    }
}

/// A received Telegram update (minimal fields needed for routing)
#[derive(Clone, Debug)]
pub struct TgUpdate {
    pub update_id: i64,
    pub chat_id:   i64,
    pub user_id:   i64,
    pub username:  String,
    pub text:      String,
    pub message_id: i64,
    pub ts:        u128,
}

/// Escape text for Telegram MarkdownV2 format.
/// Mirrors OpenClaw: src/telegram/format.ts escapeMarkdownV2()
pub fn escape_md_v2(s: &str) -> String {
    // Characters that must be escaped in MarkdownV2
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

/// Parse a Telegram getUpdates JSON response into a list of updates.
/// Minimal parser — only extracts fields Kira needs.
pub fn parse_updates(json: &str) -> Vec<TgUpdate> {
    let mut updates = Vec::new();
    let mut pos = 0;
    let bytes = json.as_bytes();

    // Find each "update_id" occurrence
    while let Some(uid_pos) = json[pos..].find("\"update_id\":")
        .map(|p| p + pos)
    {
        let slice = &json[uid_pos..];
        let update_id = extract_i64(slice, "update_id").unwrap_or(0);
        let chat_id   = extract_i64(slice, "\"chat\"").unwrap_or_else(
            || extract_i64_nested(slice, "chat", "id").unwrap_or(0)
        );
        let user_id   = extract_i64_nested(slice, "from", "id").unwrap_or(0);
        let username  = extract_str_nested(slice, "from", "username")
            .or_else(|| extract_str_nested(slice, "from", "first_name"))
            .unwrap_or_default();
        let text      = extract_str_field(slice, "text").unwrap_or_default();
        let message_id = extract_i64(slice, "message_id").unwrap_or(0);

        if update_id > 0 {
            updates.push(TgUpdate {
                update_id,
                chat_id,
                user_id,
                username,
                text,
                message_id,
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
            });
        }
        pos = uid_pos + 10; // advance past this occurrence
    }
    updates
}

// ── Minimal JSON field extractors (no serde_json dependency) ─────────────────

fn extract_i64(json: &str, key: &str) -> Option<i64> {
    let search = format!("\"{}\":", key);
    let start = json.find(&search)? + search.len();
    let slice = json[start..].trim_start();
    let neg = slice.starts_with('-');
    let s = if neg { &slice[1..] } else { slice };
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let n: i64 = s[..end].parse().ok()?;
    Some(if neg { -n } else { n })
}

fn extract_i64_nested(json: &str, obj_key: &str, field: &str) -> Option<i64> {
    let obj_search = format!("\"{}\":", obj_key);
    let obj_pos = json.find(&obj_search)? + obj_search.len();
    let obj_slice = json[obj_pos..].trim_start();
    if !obj_slice.starts_with('{') { return None; }
    // Find matching closing brace
    let mut depth = 0i32;
    let mut end = 0;
    for (i, c) in obj_slice.chars().enumerate() {
        match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { end = i + 1; break; } } _ => {} }
    }
    extract_i64(&obj_slice[..end], field)
}

fn extract_str_nested(json: &str, obj_key: &str, field: &str) -> Option<String> {
    let obj_search = format!("\"{}\":", obj_key);
    let obj_pos = json.find(&obj_search)? + obj_search.len();
    let obj_slice = json[obj_pos..].trim_start();
    if !obj_slice.starts_with('{') { return None; }
    let mut depth = 0i32;
    let mut end = 0;
    for (i, c) in obj_slice.chars().enumerate() {
        match c { '{' => depth += 1, '}' => { depth -= 1; if depth == 0 { end = i + 1; break; } } _ => {} }
    }
    extract_str_field(&obj_slice[..end], field)
}

fn extract_str_field(json: &str, key: &str) -> Option<String> {
    let search = format!("\"{}\":\"", key);
    let start = json.find(&search)? + search.len();
    let mut end = start;
    let bytes = json.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    Some(json[start..end].replace("\\\"", "\"").replace("\\n", "\n"))
}
