// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: subagents
//
// Sub-agent registry — spawn, track, kill child agents.
// Mirrors OpenClaw: src/agents/subagent-registry.ts
//
// Session 1: types.  Session 3: implement.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum SubAgentPhase {
    Spawning,
    Running,
    Done,
    Failed,
    Killed,
}

impl SubAgentPhase {
    pub fn as_str(&self) -> &str {
        match self {
            SubAgentPhase::Spawning => "spawning",
            SubAgentPhase::Running  => "running",
            SubAgentPhase::Done     => "done",
            SubAgentPhase::Failed   => "failed",
            SubAgentPhase::Killed   => "killed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct SubAgentState {
    pub id:         String,
    pub goal:       String,
    pub session_id: String,
    pub parent_id:  Option<String>,
    pub depth:      u32,
    pub phase:      SubAgentPhase,
    pub result:     Option<String>,
    pub steps:      u32,
    pub started_at: u128,
    pub ended_at:   Option<u128>,
}

impl SubAgentState {
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"id":"{}","goal":"{}","session":"{}","depth":{},"phase":"{}","steps":{},"result":{}}}"#,
            self.id,
            self.goal.replace('"', "\\\""),
            self.session_id,
            self.depth,
            self.phase.as_str(),
            self.steps,
            self.result.as_deref()
                .map(|r| format!("\"{}\"", r.replace('"', "\\\"")))
                .unwrap_or_else(|| "null".to_string()),
        )
    }
}

pub struct SubAgentRegistry {
    agents:    HashMap<String, SubAgentState>,
    max_depth: u32,
}

impl SubAgentRegistry {
    pub fn new(max_depth: u32) -> Self {
        SubAgentRegistry { agents: HashMap::new(), max_depth }
    }

    pub fn can_spawn(&self, parent_depth: u32) -> bool {
        parent_depth < self.max_depth
    }

    pub fn register(&mut self, agent: SubAgentState) {
        self.agents.insert(agent.id.clone(), agent);
    }

    pub fn get(&self, id: &str) -> Option<&SubAgentState> {
        self.agents.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut SubAgentState> {
        self.agents.get_mut(id)
    }

    pub fn kill(&mut self, id: &str) -> bool {
        if let Some(a) = self.agents.get_mut(id) {
            a.phase = SubAgentPhase::Killed;
            true
        } else { false }
    }

    pub fn list_json(&self) -> String {
        let items: Vec<String> = self.agents.values()
            .map(|a| a.to_json())
            .collect();
        format!("[{}]", items.join(","))
    }

    /// Prune completed/failed/killed agents older than ttl_ms
    pub fn prune(&mut self, now_ms: u128, ttl_ms: u128) {
        self.agents.retain(|_, a| {
            match a.phase {
                SubAgentPhase::Done | SubAgentPhase::Failed | SubAgentPhase::Killed => {
                    a.ended_at.map_or(true, |end| now_ms - end < ttl_ms)
                }
                _ => true,
            }
        });
    }
}

impl Default for SubAgentRegistry {
    fn default() -> Self { Self::new(5) }
}
