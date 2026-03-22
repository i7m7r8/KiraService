// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: security
//
// DM pairing codes + channel allowlists.
// Mirrors OpenClaw: src/security/dm-policy-shared.ts,
//                   src/channels/allow-from.ts
//
// Session 1: types.  Session 18: full enforcement.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A pending pairing request from an unknown sender
#[derive(Clone, Debug)]
pub struct PairingRequest {
    pub code:       String,   // 6-char alphanumeric code sent to user
    pub channel:    String,
    pub sender:     String,
    pub chat_id:    String,
    pub created_ms: u128,
    pub expires_ms: u128,     // code expires after 10 minutes
}

impl PairingRequest {
    pub fn new(channel: &str, sender: &str, chat_id: &str, now_ms: u128) -> Self {
        PairingRequest {
            code:       gen_pairing_code(now_ms, sender),
            channel:    channel.to_string(),
            sender:     sender.to_string(),
            chat_id:    chat_id.to_string(),
            created_ms: now_ms,
            expires_ms: now_ms + 600_000, // 10 min
        }
    }

    pub fn is_expired(&self, now_ms: u128) -> bool {
        now_ms > self.expires_ms
    }

    pub fn pairing_message(&self) -> String {
        format!(
            "🔐 Pairing required. Your code: *{}*\n\nSend this code to the operator to get approved.",
            self.code
        )
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"code":"{}","channel":"{}","sender":"{}","expires_ms":{}}}"#,
            self.code, self.channel, self.sender, self.expires_ms
        )
    }
}

/// An approved sender entry in the allowlist
#[derive(Clone, Debug)]
pub struct AllowlistEntry {
    pub channel:     String,
    pub sender:      String,
    pub approved_ms: u128,
    pub note:        String,
}

impl AllowlistEntry {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"channel":"{}","sender":"{}","approved_ms":{}}}"#,
            self.channel, self.sender, self.approved_ms
        )
    }
}

/// Generate a 6-character alphanumeric pairing code
fn gen_pairing_code(now_ms: u128, sender: &str) -> String {
    // Mix time + sender into a deterministic-looking code
    let mut h: u64 = 5381;
    for b in now_ms.to_le_bytes().iter().chain(sender.bytes().collect::<Vec<_>>().iter()) {
        h = h.wrapping_mul(33).wrapping_add(*b as u64);
    }
    const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut code = String::with_capacity(6);
    let mut val = h;
    for _ in 0..6 {
        code.push(CHARSET[(val as usize) % CHARSET.len()] as char);
        val >>= 5;
    }
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn code_is_6_chars() {
        let req = PairingRequest::new("telegram", "user123", "chat456", 1_000_000);
        assert_eq!(req.code.len(), 6);
        assert!(req.code.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()));
    }
}
