// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: acp :: bus
//
// Per-session ACP event bus.
// Java polls GET /acp/events?session=X every 100ms.
// Runner threads push events via AcpBus::emit().
//
// Session 1: full implementation.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use super::types::AcpEvent;

// ── Per-session queue ─────────────────────────────────────────────────────────

const MAX_EVENTS_PER_SESSION: usize = 256;

#[derive(Default)]
struct SessionQueue {
    events:   VecDeque<AcpEvent>,
    /// Total events ever pushed (monotonic counter, for client catch-up)
    seq:      u64,
}

impl SessionQueue {
    fn push(&mut self, event: AcpEvent) {
        if self.events.len() >= MAX_EVENTS_PER_SESSION {
            self.events.pop_front(); // drop oldest, keep buffer bounded
        }
        self.events.push_back(event);
        self.seq += 1;
    }

    /// Drain all events into a JSON array string and return it.
    fn drain_json(&mut self) -> String {
        if self.events.is_empty() {
            return "[]".to_string();
        }
        let items: Vec<String> = self.events.drain(..).map(|e| e.to_json()).collect();
        format!("[{}]", items.join(","))
    }

    /// Peek at events without consuming (for SSE streaming path if needed)
    fn peek_json(&self) -> String {
        if self.events.is_empty() {
            return "[]".to_string();
        }
        let items: Vec<String> = self.events.iter().map(|e| e.to_json()).collect();
        format!("[{}]", items.join(","))
    }

    fn is_empty(&self) -> bool { self.events.is_empty() }
    fn seq(&self) -> u64 { self.seq }
}

// ── AcpBus ────────────────────────────────────────────────────────────────────

/// Global ACP event bus shared between runner threads and the HTTP poll handler.
/// Wrapped in Arc<Mutex<>> so it can live in KiraState.
pub struct AcpBus {
    queues: HashMap<String, SessionQueue>,
}

impl AcpBus {
    pub fn new() -> Self {
        AcpBus { queues: HashMap::new() }
    }

    // ── Write side (called by runner threads) ──────────────────────────────

    /// Emit an event to a session queue. Creates the queue if it doesn't exist.
    pub fn emit(&mut self, event: AcpEvent) {
        let session = event.session_id().to_string();
        self.queues.entry(session).or_default().push(event);
    }

    /// Emit to multiple sessions (broadcast)
    pub fn emit_all(&mut self, events: Vec<AcpEvent>) {
        for e in events { self.emit(e); }
    }

    // ── Read side (called by HTTP poll handler) ────────────────────────────

    /// Drain all events for a session and return as JSON array.
    /// Clears the queue (poll-and-consume semantics).
    pub fn drain(&mut self, session_id: &str) -> String {
        match self.queues.get_mut(session_id) {
            Some(q) => q.drain_json(),
            None    => "[]".to_string(),
        }
    }

    /// Peek without consuming (for testing / introspection)
    pub fn peek(&self, session_id: &str) -> String {
        match self.queues.get(session_id) {
            Some(q) => q.peek_json(),
            None    => "[]".to_string(),
        }
    }

    /// Return monotonic sequence number for a session (for client catch-up)
    pub fn seq(&self, session_id: &str) -> u64 {
        self.queues.get(session_id).map(|q| q.seq()).unwrap_or(0)
    }

    /// Check if there are pending events
    pub fn has_events(&self, session_id: &str) -> bool {
        self.queues.get(session_id).map(|q| !q.is_empty()).unwrap_or(false)
    }

    /// List all sessions that have pending events
    pub fn active_sessions(&self) -> Vec<&str> {
        self.queues.iter()
            .filter(|(_, q)| !q.is_empty())
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Purge all events for a session (on session delete)
    pub fn purge(&mut self, session_id: &str) {
        self.queues.remove(session_id);
    }

    /// Return JSON summary of all session queue sizes (for /acp/status)
    pub fn status_json(&self) -> String {
        if self.queues.is_empty() {
            return "{}".to_string();
        }
        let items: Vec<String> = self.queues.iter()
            .map(|(id, q)| format!(r#""{}":{{"pending":{},"seq":{}}}"#,
                esc(id), q.events.len(), q.seq))
            .collect();
        format!("{{{}}}", items.join(","))
    }
}

impl Default for AcpBus {
    fn default() -> Self { Self::new() }
}

// ── AcpBusHandle ──────────────────────────────────────────────────────────────
// Cheaply clonable handle for passing to runner threads.
// Avoids the runner holding a lock on all of KiraState.

pub type AcpBusHandle = Arc<Mutex<AcpBus>>;

pub fn new_bus_handle() -> AcpBusHandle {
    Arc::new(Mutex::new(AcpBus::new()))
}

// ── Helper ────────────────────────────────────────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
