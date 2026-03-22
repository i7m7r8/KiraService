// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: channels
//
// Messaging channel adapters — Telegram, WhatsApp, WebChat, etc.
// Mirrors OpenClaw: src/channels/, src/telegram/, src/web/
//
// Session 1: module skeleton + shared types.
// Session 7: Telegram (full parity).
// Session 8: WhatsApp.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod telegram;
pub mod shared;

pub use shared::{InboundMessage, OutboundMessage, ChannelId, SendResult};
