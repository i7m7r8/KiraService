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
pub mod persistence;

pub use sessions::{Session, SessionStore};
pub use routing::{RouteKey, AgentConfig};
pub use security::{PairingRequest, AllowlistEntry};
pub use persistence::{
    save_session_transcript, load_session_transcript, delete_session, list_session_ids,
    save_memory_index, load_memory_index,
    load_skill_files, save_skill_file, delete_skill_file,
    save_cron_jobs, load_cron_jobs, save_webhooks, load_webhooks,
    append_cron_run_log,
};
