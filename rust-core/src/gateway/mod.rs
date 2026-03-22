// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway
//
// WebSocket control plane types + session management.
// Mirrors OpenClaw: src/gateway/server.ts, src/config/sessions/
//
// Session 1: session types.
// Session 4: persistent session store.
// Session 16: multi-agent routing.
// Session 18: security / pairing.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod sessions;
pub mod routing;
pub mod security;

pub use sessions::{Session, SessionStore};
pub use routing::{RouteKey, AgentConfig};
pub use security::{PairingRequest, AllowlistEntry};
