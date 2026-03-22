// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: channels :: shared
//
// Shared types for all channel adapters.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Which channel a message came from / goes to
#[derive(Clone, Debug, PartialEq)]
pub enum ChannelId {
    Telegram,
    WhatsApp,
    Discord,
    WebChat,
    Internal,  // system-generated, e.g. cron jobs
}

impl ChannelId {
    pub fn as_str(&self) -> &str {
        match self {
            ChannelId::Telegram  => "telegram",
            ChannelId::WhatsApp  => "whatsapp",
            ChannelId::Discord   => "discord",
            ChannelId::WebChat   => "webchat",
            ChannelId::Internal  => "internal",
        }
    }
}

/// Normalized inbound message from any channel
#[derive(Clone, Debug)]
pub struct InboundMessage {
    pub id:          String,
    pub channel:     ChannelId,
    pub sender:      String,   // user id / phone / username
    pub sender_name: String,
    pub text:        String,
    pub media_url:   Option<String>,
    pub reply_to_id: Option<String>,
    pub chat_id:     String,   // group or DM identifier
    pub ts:          u128,
    pub session_key: String,   // derived routing key
}

impl InboundMessage {
    pub fn derive_session_key(channel: &ChannelId, chat_id: &str) -> String {
        format!("{}:{}", channel.as_str(), chat_id)
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","channel":"{}","sender":"{}","text":"{}","chat_id":"{}","ts":{}}}"#,
            self.id, self.channel.as_str(),
            self.sender,
            self.text.replace('"', "\\\"").replace('\n', "\\n"),
            self.chat_id, self.ts
        )
    }
}

/// Outbound message to send via a channel
#[derive(Clone, Debug)]
pub struct OutboundMessage {
    pub channel:     ChannelId,
    pub chat_id:     String,
    pub text:        String,
    pub reply_to_id: Option<String>,
    pub parse_mode:  ParseMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseMode {
    Plain,
    Markdown,
    MarkdownV2,
    Html,
}

/// Result of a send attempt
#[derive(Clone, Debug)]
pub enum SendResult {
    Ok { message_id: String },
    Err { message: String, retryable: bool },
}

impl SendResult {
    pub fn to_json(&self) -> String {
        match self {
            SendResult::Ok { message_id } =>
                format!(r#"{{"ok":true,"message_id":"{}"}}"#, message_id),
            SendResult::Err { message, retryable } =>
                format!(r#"{{"ok":false,"error":"{}","retryable":{}}}"#,
                    message.replace('"', "\\\""), retryable),
        }
    }
}

/// DM policy — mirrors OpenClaw security defaults
#[derive(Clone, Debug, PartialEq)]
pub enum DmPolicy {
    /// Require pairing code from unknown senders
    Pairing,
    /// Accept messages from anyone
    Open,
    /// Reject all DMs
    Deny,
}

impl Default for DmPolicy {
    fn default() -> Self { DmPolicy::Pairing }
}

impl DmPolicy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "open"   => DmPolicy::Open,
            "deny"   => DmPolicy::Deny,
            _        => DmPolicy::Pairing,
        }
    }
}
