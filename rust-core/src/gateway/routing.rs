// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: gateway :: routing
//
// Route inbound messages to the correct agent + session.
// Mirrors OpenClaw: src/routing/resolve-route.ts
//
// Session 1: types.  Session 16: full multi-agent routing.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// The resolved key for a session (channel + peer identifier)
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct RouteKey(pub String);

impl RouteKey {
    pub fn from_parts(channel: &str, chat_id: &str) -> Self {
        RouteKey(format!("{}:{}", channel, chat_id))
    }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Agent configuration  -  which model/persona/skills to use for a session
#[derive(Clone, Debug)]
pub struct AgentConfig {
    pub id:          String,
    pub name:        String,
    pub persona:     String,
    pub model:       Option<String>,   // None = use global default
    pub skill_ids:   Vec<String>,
    pub memory_scope: String,          // "global" | session_id
    pub channels:    Vec<String>,      // which channels this agent handles
    pub enabled:     bool,
}

impl AgentConfig {
    pub fn default_agent() -> Self {
        AgentConfig {
            id:           "default".to_string(),
            name:         "Kira".to_string(),
            persona:      "You are Kira, a powerful Android AI agent.".to_string(),
            model:        None,
            skill_ids:    vec![],
            memory_scope: "global".to_string(),
            channels:     vec!["*".to_string()],
            enabled:      true,
        }
    }

    pub fn to_json(&self) -> String {
        let _skills_json: Vec<String> = self.skill_ids.iter()
            .map(|s| format!("\"{}\"", s))
            .collect();
        format!(
            r#"{{"id":"{}","name":"{}","model":{},"memory_scope":"{}","enabled":{}}}"#,
            self.id, self.name,
            self.model.as_deref()
                .map(|m| format!("\"{}\"", m))
                .unwrap_or_else(|| "null".to_string()),
            self.memory_scope, self.enabled
        )
    }
}

/// Route a message to the correct agent config.
/// Returns the agent_id to use.
pub fn resolve_agent<'a>(
    agents: &'a [AgentConfig],
    channel: &str,
    _sender: &str,
) -> &'a AgentConfig {
    // Find first enabled agent that handles this channel
    agents.iter().find(|a| {
        a.enabled && (
            a.channels.contains(&"*".to_string()) ||
            a.channels.iter().any(|c| c == channel)
        )
    }).unwrap_or_else(|| {
        // Fallback: return first enabled agent
        agents.iter().find(|a| a.enabled)
            .expect("No enabled agents configured")
    })
}
