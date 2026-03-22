// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: sessions
// Session 1: types.  Session 4: persistence + compaction integration.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::VecDeque;

/// A conversation session
#[derive(Clone, Debug)]
pub struct Session {
    pub id:          String,
    pub channel:     String,
    pub turns:       u32,
    pub tokens:      u64,
    pub created_ms:  u128,
    pub last_msg_ms: u128,
    pub agent_id:    String,
    pub mode:        SessionMode,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionMode {
    Chat,      // normal back-and-forth
    Agent,     // running a multi-step agent task
    Cron,      // isolated cron session
}

impl SessionMode {
    pub fn as_str(&self) -> &str {
        match self {
            SessionMode::Chat  => "chat",
            SessionMode::Agent => "agent",
            SessionMode::Cron  => "cron",
        }
    }
}

impl Session {
    pub fn new(id: &str, channel: &str, now_ms: u128) -> Self {
        Session {
            id: id.to_string(),
            channel: channel.to_string(),
            turns: 0,
            tokens: 0,
            created_ms: now_ms,
            last_msg_ms: now_ms,
            agent_id: "default".to_string(),
            mode: SessionMode::Chat,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"created_ms":{},"last_msg_ms":{},"agent":"{}","mode":"{}"}}"#,
            self.id, self.channel, self.turns, self.tokens,
            self.created_ms, self.last_msg_ms, self.agent_id,
            self.mode.as_str()
        )
    }
}

/// Transcript turn for persistent storage
#[derive(Clone, Debug)]
pub struct TranscriptTurn {
    pub role:       String,
    pub content:    String,
    pub ts:         u128,
    pub tokens:     u32,
    pub session_id: String,
    /// LZ4-compressed bytes of (role + content) — set when persisting
    pub compressed: Option<Vec<u8>>,
}

/// Session store — manages active sessions and their transcripts
pub struct SessionStore {
    sessions:    std::collections::HashMap<String, Session>,
    transcripts: std::collections::HashMap<String, VecDeque<TranscriptTurn>>,
    max_turns_per_session: usize,
}

impl SessionStore {
    pub fn new(max_turns: usize) -> Self {
        SessionStore {
            sessions:   std::collections::HashMap::new(),
            transcripts: std::collections::HashMap::new(),
            max_turns_per_session: max_turns,
        }
    }

    pub fn get_or_create(&mut self, id: &str, channel: &str, now_ms: u128) -> &mut Session {
        if !self.sessions.contains_key(id) {
            let s = Session::new(id, channel, now_ms);
            self.sessions.insert(id.to_string(), s);
            self.transcripts.insert(id.to_string(), VecDeque::new());
        }
        self.sessions.get_mut(id).unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        self.transcripts.remove(id);
        self.sessions.remove(id).is_some()
    }

    pub fn add_turn(&mut self, session_id: &str, role: &str, content: &str, ts: u128) {
        let tokens = (content.len() as u32 / 4).max(1);
        let turns = self.transcripts.entry(session_id.to_string())
            .or_default();
        turns.push_back(TranscriptTurn {
            role:       role.to_string(),
            content:    content.to_string(),
            ts,
            tokens,
            session_id: session_id.to_string(),
            compressed: None,
        });
        // Trim to max turns
        while turns.len() > self.max_turns_per_session {
            turns.pop_front();
        }
        // Update session stats
        if let Some(s) = self.sessions.get_mut(session_id) {
            s.turns  += 1;
            s.tokens += tokens as u64;
            s.last_msg_ms = ts;
        }
    }

    pub fn get_turns(&self, session_id: &str) -> Vec<&TranscriptTurn> {
        self.transcripts.get(session_id)
            .map(|d| d.iter().collect())
            .unwrap_or_default()
    }

    pub fn list_sessions_json(&self) -> String {
        let items: Vec<String> = self.sessions.values()
            .map(|s| s.to_json())
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Prune sessions inactive for more than ttl_ms
    pub fn prune_inactive(&mut self, now_ms: u128, ttl_ms: u128) -> usize {
        let stale: Vec<String> = self.sessions.values()
            .filter(|s| now_ms - s.last_msg_ms > ttl_ms)
            .map(|s| s.id.clone())
            .collect();
        let count = stale.len();
        for id in stale { self.delete(&id); }
        count
    }
}

impl Default for SessionStore {
    fn default() -> Self { Self::new(200) }
}
