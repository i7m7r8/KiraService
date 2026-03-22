// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: skills
//
// Skills platform — load, resolve, system-prompt injection.
// Mirrors OpenClaw: src/agents/skills.ts, src/agents/skills/frontmatter.ts
//
// Session 1: types + frontmatter parser.
// Session 6: full install/reload/filter pipeline.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// A loaded skill definition
#[derive(Clone, Debug)]
pub struct Skill {
    pub name:        String,
    pub description: String,
    pub trigger:     String,   // keyword that activates this skill
    pub content:     String,   // full markdown body (injected into system prompt)
    pub tools:       Vec<String>, // tools this skill enables
    pub enabled:     bool,
    pub usage_count: u32,
    pub source:      SkillSource,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SkillSource {
    Bundled,   // ships with Kira
    Installed, // user-installed
    Workspace, // from workspace directory
}

impl Skill {
    /// Parse a skill from a markdown file with YAML frontmatter.
    /// Frontmatter format:
    /// ```
    /// ---
    /// name: My Skill
    /// description: What it does
    /// trigger: keyword
    /// tools: [tool_a, tool_b]
    /// ---
    /// Skill content here...
    /// ```
    pub fn from_markdown(content: &str, source: SkillSource) -> Option<Self> {
        if !content.trim_start().starts_with("---") {
            return None;
        }
        let after_first = content.trim_start().trim_start_matches('-').trim_start();
        let end = after_first.find("\n---")?;
        let frontmatter = &after_first[..end];
        let body = after_first[end..].trim_start_matches('-').trim_start();

        let name        = fm_field(frontmatter, "name")?;
        let description = fm_field(frontmatter, "description").unwrap_or_default();
        let trigger     = fm_field(frontmatter, "trigger").unwrap_or_default();
        let tools       = fm_list(frontmatter, "tools");

        Some(Skill {
            name,
            description,
            trigger,
            content: body.to_string(),
            tools,
            enabled: true,
            usage_count: 0,
            source,
        })
    }

    pub fn to_json(&self) -> String {
        let tools_json: Vec<String> = self.tools.iter()
            .map(|t| format!("\"{}\"", t))
            .collect();
        format!(
            r#"{{"name":"{}","description":"{}","trigger":"{}","enabled":{},"usage_count":{},"tools":[{}]}}"#,
            self.name.replace('"', "\\\""),
            self.description.replace('"', "\\\""),
            self.trigger.replace('"', "\\\""),
            self.enabled,
            self.usage_count,
            tools_json.join(",")
        )
    }

    /// Build the system prompt fragment for this skill
    pub fn system_prompt_fragment(&self) -> String {
        format!("## Skill: {}\n{}\n", self.name, self.content)
    }
}

/// Extract a scalar field from YAML-like frontmatter
fn fm_field(fm: &str, key: &str) -> Option<String> {
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(&format!("{}:", key)) {
            return Some(rest.trim().trim_matches('"').to_string());
        }
    }
    None
}

/// Extract a list field: `tools: [a, b, c]` or multi-line `- item`
fn fm_list(fm: &str, key: &str) -> Vec<String> {
    // Inline: tools: [a, b, c]
    if let Some(line) = fm.lines().find(|l| l.trim().starts_with(&format!("{}:", key))) {
        if let Some(bracket_start) = line.find('[') {
            if let Some(bracket_end) = line.find(']') {
                return line[bracket_start+1..bracket_end]
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }
    // Multi-line: find key, then collect `- item` lines
    let mut in_key = false;
    let mut result = Vec::new();
    for line in fm.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("{}:", key)) {
            in_key = true;
            continue;
        }
        if in_key {
            if trimmed.starts_with("- ") {
                result.push(trimmed[2..].trim().to_string());
            } else if !trimmed.is_empty() {
                break; // new key, stop
            }
        }
    }
    result
}

/// Skill registry — loaded skills for the current session
pub struct SkillRegistry {
    skills: Vec<Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self { SkillRegistry { skills: Vec::new() } }

    pub fn register(&mut self, skill: Skill) {
        self.skills.retain(|s| s.name != skill.name);
        self.skills.push(skill);
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    pub fn enabled(&self) -> Vec<&Skill> {
        self.skills.iter().filter(|s| s.enabled).collect()
    }

    pub fn enable(&mut self, name: &str, on: bool) -> bool {
        if let Some(s) = self.skills.iter_mut().find(|s| s.name == name) {
            s.enabled = on; true
        } else { false }
    }

    /// Build the skills section of the system prompt
    pub fn build_system_prompt_section(&self) -> String {
        let fragments: Vec<String> = self.enabled().iter()
            .map(|s| s.system_prompt_fragment())
            .collect();
        if fragments.is_empty() { return String::new(); }
        format!("# Skills\n\n{}", fragments.join("\n"))
    }

    /// Find skills triggered by a user message
    pub fn triggered_by(&self, message: &str) -> Vec<&Skill> {
        let msg_lower = message.to_lowercase();
        self.enabled().into_iter()
            .filter(|s| !s.trigger.is_empty() && msg_lower.contains(&s.trigger.to_lowercase()))
            .collect()
    }

    pub fn list_json(&self) -> String {
        let items: Vec<String> = self.skills.iter().map(|s| s.to_json()).collect();
        format!("[{}]", items.join(","))
    }
}

impl Default for SkillRegistry { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_skill_frontmatter() {
        let md = "---\nname: Test Skill\ndescription: A test\ntrigger: test\ntools: [tool_a]\n---\nSome content\n";
        let skill = Skill::from_markdown(md, SkillSource::Bundled).unwrap();
        assert_eq!(skill.name, "Test Skill");
        assert_eq!(skill.trigger, "test");
        assert_eq!(skill.tools, vec!["tool_a"]);
        assert!(skill.content.contains("Some content"));
    }
}
