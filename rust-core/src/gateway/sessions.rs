// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: sessions
//
// Full session model with LZ4-compressed disk persistence.
// Mirrors OpenClaw: src/config/sessions/store.ts,
//                   src/config/sessions/transcript.ts,
//                   src/agents/session-write-lock.ts
//
// Session 1: types.
// Session 4: persistence (LZ4 disk save/load), compaction integration,
//            GET /sessions/:key, DELETE /sessions/:key,
//            POST /sessions/:key/compact (force compact).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::{HashMap, VecDeque};
use lz4_flex::{compress_prepend_size, decompress_size_prepended};

// ── Session metadata ──────────────────────────────────────────────────────────

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
    /// Token estimate of current transcript (rough: len/4)
    pub token_estimate: u64,
    /// Whether this session has been compacted at least once
    pub compacted:   bool,
    /// Summary of compacted turns, prepended as context
    pub compact_summary: String,

    // ── OpenClaw SessionsPatch fields (S1) ───────────────────────────────
    /// Human-readable label for this session (e.g. "work", "research")
    pub label:               Option<String>,
    /// Override model for this session (None = use global default)
    pub model_override:      Option<String>,
    /// Thinking level: off | minimal | low | medium | high | xhigh
    pub thinking_level:      Option<String>,
    /// Fast mode: skip tool calls, reply immediately
    pub fast_mode:           Option<bool>,
    /// Verbose level: off | on | full
    pub verbose_level:       Option<String>,
    /// Reasoning level: off | on | stream
    pub reasoning_level:     Option<String>,
    /// How to report token usage: off | tokens | full
    pub response_usage:      Option<String>,
    /// Elevated mode (owner-only capabilities)
    pub elevated_level:      Option<String>,
    /// Exec host: which node runs shell commands
    pub exec_host:           Option<String>,
    /// Exec security: strict | default | relaxed
    pub exec_security:       Option<String>,
    /// Exec ask: always | auto | never
    pub exec_ask:            Option<String>,
    /// Exec node: which Android node to use for device tools
    pub exec_node:           Option<String>,
    /// Session that spawned this one (sub-agent parent)
    pub spawned_by:          Option<String>,
    /// Working directory for spawned sub-agents
    pub spawned_workspace:   Option<String>,
    /// Spawn depth (0 = top-level, max 5)
    pub spawn_depth:         u8,
    /// Sub-agent role: orchestrator | leaf
    pub subagent_role:       Option<String>,
    /// Sub-agent control scope: children | none
    pub subagent_ctrl_scope: Option<String>,
    /// Send policy: allow | deny
    pub send_policy:         Option<String>,
    /// Group activation: mention | always
    pub group_activation:    Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionMode {
    Chat,
    Agent,
    Cron,
}

impl SessionMode {
    pub fn as_str(&self) -> &str {
        match self {
            SessionMode::Chat  => "chat",
            SessionMode::Agent => "agent",
            SessionMode::Cron  => "cron",
        }
    }
    fn from_str(s: &str) -> Self {
        match s { "agent" => SessionMode::Agent, "cron" => SessionMode::Cron, _ => SessionMode::Chat }
    }
}

impl Session {
    pub fn new(id: &str, channel: &str, now_ms: u128) -> Self {
        Session {
            id:              id.to_string(),
            channel:         channel.to_string(),
            turns:           0,
            tokens:          0,
            created_ms:      now_ms,
            last_msg_ms:     now_ms,
            agent_id:        "default".to_string(),
            mode:            SessionMode::Chat,
            token_estimate:  0,
            compacted:       false,
            compact_summary: String::new(),
            // OpenClaw SessionsPatch defaults — all None/false/0
            label:               None,
            model_override:      None,
            thinking_level:      None,
            fast_mode:           None,
            verbose_level:       None,
            reasoning_level:     None,
            response_usage:      None,
            elevated_level:      None,
            exec_host:           None,
            exec_security:       None,
            exec_ask:            None,
            exec_node:           None,
            spawned_by:          None,
            spawned_workspace:   None,
            spawn_depth:         0,
            subagent_role:       None,
            subagent_ctrl_scope: None,
            send_policy:         None,
            group_activation:    None,
        }
    }

    pub fn to_json(&self) -> String {
        let os = |v: &Option<String>| -> String {
            match v { Some(s) => format!("\"{}\"", esc(s)), None => "null".into() }
        };
        let ob = |v: &Option<bool>| -> String {
            match v { Some(b) => b.to_string(), None => "null".into() }
        };
        format!(
            r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"created_ms":{},"last_msg_ms":{},"agent":"{}","mode":"{}","token_estimate":{},"compacted":{},"has_summary":{},"label":{},"model":{},"thinking_level":{},"fast_mode":{},"verbose_level":{},"reasoning_level":{},"response_usage":{},"elevated_level":{},"exec_host":{},"exec_security":{},"exec_ask":{},"exec_node":{},"spawned_by":{},"spawned_workspace":{},"spawn_depth":{},"subagent_role":{},"subagent_ctrl_scope":{},"send_policy":{},"group_activation":{}}}"#,
            esc(&self.id), esc(&self.channel), self.turns, self.tokens,
            self.created_ms, self.last_msg_ms, esc(&self.agent_id),
            self.mode.as_str(), self.token_estimate,
            self.compacted, !self.compact_summary.is_empty(),
            os(&self.label),            os(&self.model_override),
            os(&self.thinking_level),   ob(&self.fast_mode),
            os(&self.verbose_level),    os(&self.reasoning_level),
            os(&self.response_usage),   os(&self.elevated_level),
            os(&self.exec_host),        os(&self.exec_security),
            os(&self.exec_ask),         os(&self.exec_node),
            os(&self.spawned_by),       os(&self.spawned_workspace),
            self.spawn_depth,
            os(&self.subagent_role),    os(&self.subagent_ctrl_scope),
            os(&self.send_policy),      os(&self.group_activation),
        )
    }

    /// Serialise to a JSON line for the session index file (S2: persists patch fields)
    fn to_index_json(&self) -> String {
        let os = |v: &Option<String>| -> String {
            match v { Some(s) => format!("\"{}\"", esc(s)), None => "null".into() }
        };
        let ob = |v: &Option<bool>| -> String {
            match v { Some(b) => b.to_string(), None => "null".into() }
        };
        format!(
            r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"created_ms":{},"last_msg_ms":{},"agent":"{}","mode":"{}","compacted":{},"compact_summary":"{}","label":{},"model":{},"thinking_level":{},"fast_mode":{},"verbose_level":{},"reasoning_level":{},"response_usage":{},"elevated_level":{},"exec_host":{},"exec_security":{},"exec_ask":{},"exec_node":{},"spawned_by":{},"spawned_workspace":{},"spawn_depth":{},"subagent_role":{},"subagent_ctrl_scope":{},"send_policy":{},"group_activation":{}}}"#,
            esc(&self.id), esc(&self.channel), self.turns, self.tokens,
            self.created_ms, self.last_msg_ms, esc(&self.agent_id),
            self.mode.as_str(), self.compacted, esc(&self.compact_summary),
            os(&self.label),            os(&self.model_override),
            os(&self.thinking_level),   ob(&self.fast_mode),
            os(&self.verbose_level),    os(&self.reasoning_level),
            os(&self.response_usage),   os(&self.elevated_level),
            os(&self.exec_host),        os(&self.exec_security),
            os(&self.exec_ask),         os(&self.exec_node),
            os(&self.spawned_by),       os(&self.spawned_workspace),
            self.spawn_depth,
            os(&self.subagent_role),    os(&self.subagent_ctrl_scope),
            os(&self.send_policy),      os(&self.group_activation),
        )
    }

    fn from_index_json(json: &str) -> Option<Self> {
        let id        = extract_str(json, "id")?;
        let channel   = extract_str(json, "channel").unwrap_or_default();
        let turns     = extract_u64(json, "turns") as u32;
        let tokens    = extract_u64(json, "tokens");
        let created   = extract_u64(json, "created_ms") as u128;
        let last      = extract_u64(json, "last_msg_ms") as u128;
        let agent     = extract_str(json, "agent").unwrap_or_else(|| "default".to_string());
        let mode      = SessionMode::from_str(&extract_str(json, "mode").unwrap_or_default());
        let compacted = json.contains(r#""compacted":true"#);
        let summary   = extract_str(json, "compact_summary").unwrap_or_default();
        Some(Session {
            id, channel, turns, tokens,
            created_ms:       created,
            last_msg_ms:      last,
            agent_id:         agent,
            mode,
            token_estimate:   tokens,
            compacted,
            compact_summary:  summary,
            label:               extract_opt_str(json, "label"),
            model_override:      extract_opt_str(json, "model"),
            thinking_level:      extract_opt_str(json, "thinking_level"),
            fast_mode:           extract_opt_bool(json, "fast_mode"),
            verbose_level:       extract_opt_str(json, "verbose_level"),
            reasoning_level:     extract_opt_str(json, "reasoning_level"),
            response_usage:      extract_opt_str(json, "response_usage"),
            elevated_level:      extract_opt_str(json, "elevated_level"),
            exec_host:           extract_opt_str(json, "exec_host"),
            exec_security:       extract_opt_str(json, "exec_security"),
            exec_ask:            extract_opt_str(json, "exec_ask"),
            exec_node:           extract_opt_str(json, "exec_node"),
            spawned_by:          extract_opt_str(json, "spawned_by"),
            spawned_workspace:   extract_opt_str(json, "spawned_workspace"),
            spawn_depth:         extract_u64(json, "spawn_depth") as u8,
            subagent_role:       extract_opt_str(json, "subagent_role"),
            subagent_ctrl_scope: extract_opt_str(json, "subagent_ctrl_scope"),
            send_policy:         extract_opt_str(json, "send_policy"),
            group_activation:    extract_opt_str(json, "group_activation"),
        })
    }

    // ── S2: Apply a SessionPatch (from ACP wire protocol) ──────────────────
    /// Apply an OpenClaw-compatible SessionPatch to this session.
    /// Some(value) = set field. None in patch = leave unchanged (true optional).
    /// The patch uses "" or "null" string to mean "clear the field".
    pub fn apply_patch(&mut self, patch: &crate::acp::SessionPatch) {
        fn set(field: &mut Option<String>, v: &Option<String>) {
            if let Some(s) = v {
                if s.is_empty() { *field = None; }
                else { *field = Some(s.clone()); }
            }
        }
        set(&mut self.label,               &patch.label);
        set(&mut self.model_override,      &patch.model);
        set(&mut self.thinking_level,      &patch.thinking_level);
        set(&mut self.verbose_level,       &patch.verbose_level);
        set(&mut self.reasoning_level,     &patch.reasoning_level);
        set(&mut self.response_usage,      &patch.response_usage);
        set(&mut self.elevated_level,      &patch.elevated_level);
        set(&mut self.exec_host,           &patch.exec_host);
        set(&mut self.exec_security,       &patch.exec_security);
        set(&mut self.exec_ask,            &patch.exec_ask);
        set(&mut self.exec_node,           &patch.exec_node);
        set(&mut self.spawned_by,          &patch.spawned_by);
        set(&mut self.spawned_workspace,   &patch.spawned_workspace);
        set(&mut self.subagent_role,       &patch.subagent_role);
        set(&mut self.subagent_ctrl_scope, &patch.subagent_control_scope);
        set(&mut self.send_policy,         &patch.send_policy);
        set(&mut self.group_activation,    &patch.group_activation);
        if let Some(fm) = patch.fast_mode   { self.fast_mode   = Some(fm); }
        if let Some(d)  = patch.spawn_depth { self.spawn_depth = (d as u8).min(5); }
    }
}

// ── Transcript turn ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct TranscriptTurn {
    pub role:       String,
    pub content:    String,
    pub ts:         u128,
    pub tokens:     u32,
    pub session_id: String,
}

impl TranscriptTurn {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"role":"{}","content":"{}","ts":{},"tokens":{}}}"#,
            esc(&self.role),
            esc(&self.content),
            self.ts,
            self.tokens
        )
    }
}

// ── SessionStore ──────────────────────────────────────────────────────────────

/// Manages active sessions + their LZ4-compressed transcript storage.
/// Mirrors OpenClaw: src/config/sessions/store.ts
pub struct SessionStore {
    sessions:              HashMap<String, Session>,
    /// In-memory transcript buffer: session_id → VecDeque of compressed turns.
    /// Each entry is lz4(role\x00content\x00ts_ms\x00tokens).
    transcripts:           HashMap<String, VecDeque<Vec<u8>>>,
    pub max_turns:         usize,
    /// Compact when token estimate exceeds this fraction of context_limit
    pub compact_threshold: u64,   // token count, e.g. 100_000
    pub min_turns_keep:    usize, // always keep last N turns during compaction
}

impl SessionStore {
    pub fn new(max_turns: usize) -> Self {
        SessionStore {
            sessions:          HashMap::new(),
            transcripts:       HashMap::new(),
            max_turns,
            compact_threshold: 100_000,
            min_turns_keep:    12,
        }
    }

    // ── CRUD ──────────────────────────────────────────────────────────────────

    pub fn get_or_create(&mut self, id: &str, channel: &str, now_ms: u128) -> &Session {
        if !self.sessions.contains_key(id) {
            self.sessions.insert(id.to_string(), Session::new(id, channel, now_ms));
            self.transcripts.insert(id.to_string(), VecDeque::new());
        }
        self.sessions.get(id).unwrap()
    }

    pub fn get(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// Delete session from memory. Caller should also call persist::delete_session().
    pub fn delete(&mut self, id: &str) -> bool {
        self.transcripts.remove(id);
        self.sessions.remove(id).is_some()
    }

    pub fn list(&self) -> Vec<&Session> {
        let mut v: Vec<&Session> = self.sessions.values().collect();
        v.sort_by(|a, b| b.last_msg_ms.cmp(&a.last_msg_ms));
        v
    }

    // ── Turn management ───────────────────────────────────────────────────────

    pub fn add_turn(&mut self, session_id: &str, role: &str, content: &str, ts: u128) {
        let tokens = ((content.len() as u32) / 4).max(1);
        let packed  = pack_turn(role, content, ts, tokens);

        let buf = self.transcripts.entry(session_id.to_string()).or_default();
        buf.push_back(packed);
        while buf.len() > self.max_turns {
            buf.pop_front();
        }

        if let Some(s) = self.sessions.get_mut(session_id) {
            s.turns          += 1;
            s.tokens         += tokens as u64;
            s.token_estimate += tokens as u64;
            s.last_msg_ms     = ts;
        }
    }

    /// Return turns as (role, content) pairs, oldest first.
    pub fn get_turns_raw(&self, session_id: &str) -> Vec<(String, String)> {
        self.transcripts.get(session_id)
            .map(|buf| buf.iter().filter_map(|b| {
                let (r, c, _, _) = unpack_turn(b)?;
                Some((r, c))
            }).collect())
            .unwrap_or_default()
    }

    /// Return turns as TranscriptTurn structs (for HTTP responses).
    pub fn get_turns(&self, session_id: &str) -> Vec<TranscriptTurn> {
        self.transcripts.get(session_id)
            .map(|buf| buf.iter().filter_map(|b| {
                let (role, content, ts, tokens) = unpack_turn(b)?;
                Some(TranscriptTurn { role, content, ts, tokens,
                    session_id: session_id.to_string() })
            }).collect())
            .unwrap_or_default()
    }

    pub fn turn_count(&self, session_id: &str) -> usize {
        self.transcripts.get(session_id).map(|b| b.len()).unwrap_or(0)
    }

    // ── Compaction (Session 4) ─────────────────────────────────────────────

    /// Check whether this session needs compaction (token estimate over threshold).
    pub fn needs_compact(&self, session_id: &str) -> bool {
        self.sessions.get(session_id)
            .map(|s| s.token_estimate > self.compact_threshold)
            .unwrap_or(false)
    }

    /// Trim-based compaction: drop oldest turns beyond min_turns_keep,
    /// return the dropped turns as (role, content) for LLM summarisation.
    /// After calling this, the caller should generate a summary and call
    /// apply_compact_summary().
    pub fn compact_collect_dropped(&mut self, session_id: &str) -> Vec<(String, String)> {
        let buf = match self.transcripts.get_mut(session_id) {
            Some(b) => b,
            None    => return vec![],
        };

        let total = buf.len();
        if total <= self.min_turns_keep {
            return vec![];
        }

        let drop_count = total - self.min_turns_keep;
        let mut dropped = Vec::with_capacity(drop_count);

        for _ in 0..drop_count {
            if let Some(packed) = buf.pop_front() {
                if let Some((r, c, _, _)) = unpack_turn(&packed) {
                    dropped.push((r, c));
                }
            }
        }

        // Recalculate token estimate from what remains
        if let Some(sess) = self.sessions.get_mut(session_id) {
            let remaining_tokens: u64 = buf.iter()
                .filter_map(|b| unpack_turn(b).map(|(_, c, _, _)| (c.len() as u64 / 4).max(1)))
                .sum();
            sess.token_estimate = remaining_tokens;
        }

        dropped
    }

    /// Apply the LLM-generated compact summary to a session.
    pub fn apply_compact_summary(&mut self, session_id: &str, summary: &str) {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.compact_summary = summary.to_string();
            sess.compacted       = true;
        }
    }

    /// Build the full context for an LLM call:
    /// If the session has a compact summary, prepend it as a synthetic user turn.
    pub fn build_context(&self, session_id: &str) -> Vec<(String, String)> {
        let mut ctx = Vec::new();

        // Prepend summary as context if this session has been compacted
        if let Some(sess) = self.sessions.get(session_id) {
            if !sess.compact_summary.is_empty() {
                ctx.push((
                    "user".to_string(),
                    format!("[Earlier context summary]\n{}", sess.compact_summary),
                ));
                ctx.push((
                    "assistant".to_string(),
                    "Understood. I have context from the earlier conversation.".to_string(),
                ));
            }
        }

        ctx.extend(self.get_turns_raw(session_id));
        ctx
    }

    // ── Disk persistence (Session 4) ──────────────────────────────────────

    /// Save session index (all session metadata) to disk.
    pub fn save_index(&self) {
        let lines: Vec<String> = self.sessions.values()
            .map(|s| s.to_index_json())
            .collect();
        let json = format!("[{}]", lines.join(","));
        save_bytes("sessions/index.json", json.as_bytes());
    }

    /// Save a single session's transcript to disk (LZ4 compressed).
    pub fn save_transcript(&self, session_id: &str) {
        let buf = match self.transcripts.get(session_id) {
            Some(b) => b,
            None    => return,
        };
        // Serialise as JSON array of turn objects, then LZ4-compress
        let turns_json: Vec<String> = buf.iter()
            .filter_map(|b| unpack_turn(b).map(|(r, c, ts, tok)| {
                format!(r#"{{"role":"{}","content":"{}","ts":{},"tokens":{}}}"#,
                    esc(&r), esc(&c), ts, tok)
            }))
            .collect();
        let json = format!("[{}]", turns_json.join(","));
        let compressed = compress_prepend_size(json.as_bytes());
        let safe_id = sanitise_id(session_id);
        save_bytes(&format!("sessions/{}.lz4", safe_id), &compressed);
    }

    /// Load session index and all transcripts from disk at startup.
    pub fn load_from_disk(&mut self) {
        // 1. Load session index
        if let Some(json) = load_str("sessions/index.json") {
            // Parse JSON array manually (no serde dependency here)
            for entry in split_json_array(&json) {
                if let Some(sess) = Session::from_index_json(&entry) {
                    let id = sess.id.clone();
                    self.sessions.entry(id.clone()).or_insert(sess);
                    self.transcripts.entry(id).or_default();
                }
            }
        }

        // 2. Load transcripts for each known session
        let ids: Vec<String> = self.sessions.keys().cloned().collect();
        for id in ids {
            self.load_transcript(&id);
        }
    }

    fn load_transcript(&mut self, session_id: &str) {
        let safe_id  = sanitise_id(session_id);
        let bytes    = match load_bytes(&format!("sessions/{}.lz4", safe_id)) {
            Some(b) => b,
            None    => return,
        };
        let json_bytes = match decompress_size_prepended(&bytes) {
            Ok(b)  => b,
            Err(_) => return,
        };
        let json = match String::from_utf8(json_bytes) {
            Ok(s)  => s,
            Err(_) => return,
        };

        let buf = self.transcripts.entry(session_id.to_string()).or_default();
        buf.clear();

        for entry in split_json_array(&json) {
            let role    = extract_str(&entry, "role").unwrap_or_default();
            let content = extract_str(&entry, "content").unwrap_or_default();
            let ts      = extract_u64(&entry, "ts") as u128;
            let tokens  = extract_u64(&entry, "tokens") as u32;
            if !role.is_empty() {
                buf.push_back(pack_turn(&role, &content, ts, tokens));
            }
        }
    }

    /// Delete a session from both memory and disk.
    pub fn delete_and_purge(&mut self, session_id: &str) {
        self.delete(session_id);
        let safe_id = sanitise_id(session_id);
        let _ = std::fs::remove_file(data_path(&format!("sessions/{}.lz4", safe_id)));
        self.save_index();
    }

    /// Prune sessions inactive for more than ttl_ms, purge from disk too.
    pub fn prune_inactive(&mut self, now_ms: u128, ttl_ms: u128) -> usize {
        let stale: Vec<String> = self.sessions.values()
            .filter(|s| now_ms.saturating_sub(s.last_msg_ms) > ttl_ms)
            .map(|s| s.id.clone())
            .collect();
        let count = stale.len();
        for id in stale {
            self.delete_and_purge(&id);
        }
        if count > 0 { self.save_index(); }
        count
    }

    pub fn list_sessions_json(&self) -> String {
        let items: Vec<String> = self.list().iter().map(|s| s.to_json()).collect();
        format!("[{}]", items.join(","))
    }

    // ── S2: Patch + Persist ───────────────────────────────────────────────

    /// Apply a SessionPatch to a session and immediately persist the index.
    /// Returns the updated session JSON on success, error JSON on failure.
    pub fn patch_session(&mut self, session_id: &str, patch: &crate::acp::SessionPatch) -> String {
        match self.sessions.get_mut(session_id) {
            Some(sess) => {
                sess.apply_patch(patch);
                let json = sess.to_json();
                self.save_index();
                json
            }
            None => format!(r#"{{"error":"session_not_found","id":"{}"}}"#,
                esc(session_id)),
        }
    }

    /// Reset a session: clear transcript and compaction state.
    /// Mirrors OpenClaw: sessions.reset
    pub fn reset_session(&mut self, session_id: &str, now_ms: u128) -> bool {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.turns           = 0;
            sess.tokens          = 0;
            sess.token_estimate  = 0;
            sess.compacted       = false;
            sess.compact_summary = String::new();
            sess.last_msg_ms     = now_ms;
        }
        if let Some(buf) = self.transcripts.get_mut(session_id) {
            buf.clear();
        }
        self.save_index();
        self.sessions.contains_key(session_id)
    }

    /// Return session JSON by id or null-object if not found.
    pub fn get_json(&self, session_id: &str) -> String {
        match self.sessions.get(session_id) {
            Some(s) => s.to_json(),
            None    => "null".to_string(),
        }
    }
}

impl Default for SessionStore {
    fn default() -> Self { Self::new(200) }
}

// ── Turn packing helpers ──────────────────────────────────────────────────────

/// Pack a turn into: role \x00 content \x00 ts_ms(10 ascii digits) \x00 tokens
fn pack_turn(role: &str, content: &str, ts: u128, tokens: u32) -> Vec<u8> {
    format!("{}\x00{}\x00{}\x00{}", role, content, ts, tokens).into_bytes()
}

fn unpack_turn(raw: &[u8]) -> Option<(String, String, u128, u32)> {
    let s = std::str::from_utf8(raw).ok()?;
    let mut parts = s.splitn(4, '\x00');
    let role    = parts.next()?.to_string();
    let content = parts.next()?.to_string();
    let ts      = parts.next()?.parse::<u128>().unwrap_or(0);
    let tokens  = parts.next()?.parse::<u32>().unwrap_or(1);
    Some((role, content, ts, tokens))
}

// ── Disk I/O helpers ──────────────────────────────────────────────────────────

const DATA_DIR: &str = "/data/data/com.kira.service";

fn data_path(sub: &str) -> std::path::PathBuf {
    std::path::Path::new(DATA_DIR).join(sub)
}

fn ensure_dir(p: &std::path::Path) {
    if !p.exists() { let _ = std::fs::create_dir_all(p); }
}

fn sanitise_id(id: &str) -> String {
    id.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

fn save_bytes(rel: &str, data: &[u8]) {
    let path = data_path(rel);
    if let Some(parent) = path.parent() { ensure_dir(parent); }
    let _ = std::fs::write(&path, data);
}

fn load_bytes(rel: &str) -> Option<Vec<u8>> {
    std::fs::read(data_path(rel)).ok()
}

fn load_str(rel: &str) -> Option<String> {
    std::fs::read_to_string(data_path(rel)).ok()
}

// ── Minimal JSON helpers (no serde dependency) ────────────────────────────────

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"',  "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
}

/// Extract a string field value from a flat JSON object (single-level only).
fn extract_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    if rest.starts_with('"') {
        let inner = &rest[1..];
        let mut out = String::new();
        let mut chars = inner.chars().peekable();
        loop {
            match chars.next()? {
                '\\' => match chars.next()? {
                    '"'  => out.push('"'),
                    '\\' => out.push('\\'),
                    'n'  => out.push('\n'),
                    'r'  => out.push('\r'),
                    c    => { out.push('\\'); out.push(c); }
                },
                '"' => break,
                c   => out.push(c),
            }
        }
        Some(out)
    } else {
        None
    }
}

fn extract_u64(json: &str, key: &str) -> u64 {
    let needle = format!("\"{}\":", key);
    let start  = match json.find(&needle).map(|i| i + needle.len()) {
        Some(i) => i,
        None    => return 0,
    };
    let rest   = json[start..].trim_start();
    let end    = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().unwrap_or(0)
}
/// Extract an optional string field: returns None if key missing OR value is JSON null.
fn extract_opt_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    if rest.starts_with("null") { return None; }
    extract_str(json, key)
}

/// Extract an optional bool field: returns None if key missing OR value is JSON null.
fn extract_opt_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    if rest.starts_with("null")  { return None; }
    if rest.starts_with("true")  { return Some(true); }
    if rest.starts_with("false") { return Some(false); }
    None
}


/// Split a JSON array string into individual element strings (shallow, bracket-aware).
fn split_json_array(json: &str) -> Vec<String> {
    let trimmed = json.trim();
    if !trimmed.starts_with('[') { return vec![]; }
    let inner = &trimmed[1..trimmed.rfind(']').unwrap_or(trimmed.len())];
    let mut results = Vec::new();
    let mut depth   = 0i32;
    let mut start   = 0usize;
    let mut in_str  = false;
    let mut escape  = false;
    for (i, c) in inner.char_indices() {
        if escape            { escape = false; continue; }
        if c == '\\' && in_str { escape = true; continue; }
        if c == '"'          { in_str = !in_str; continue; }
        if in_str            { continue; }
        match c {
            '{' | '[' => { if depth == 0 { start = i; } depth += 1; }
            '}' | ']' => {
                depth -= 1;
                if depth == 0 {
                    results.push(inner[start..=i].to_string());
                }
            }
            _ => {}
        }
    }
    results
}
