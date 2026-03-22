// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: tools
//
// Tool definitions, call/result types, and the registry that the AI loop
// dispatches into. Individual tool implementations live in ../tools/.
//
// Session 1: types + registry skeleton.
// Session 2: dispatch integration into AI loop.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use std::collections::HashMap;

/// A tool call extracted from LLM output
#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id:     String,
    pub name:   String,
    pub params: HashMap<String, String>,
}

impl ToolCall {
    pub fn new(id: &str, name: &str, params: HashMap<String, String>) -> Self {
        ToolCall { id: id.into(), name: name.into(), params }
    }

    pub fn to_json(&self) -> String {
        let params_json: Vec<String> = self.params.iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", k, v.replace('"', "\\\"")))
            .collect();
        format!(
            r#"{{"id":"{}","name":"{}","params":{{{}}}}}"#,
            self.id, self.name, params_json.join(",")
        )
    }
}

/// Result of executing a tool
#[derive(Clone, Debug)]
pub struct ToolResult {
    pub call_id: String,
    pub name:    String,
    pub output:  String,
    pub success: bool,
    pub blocked: bool,
}

impl ToolResult {
    pub fn ok(call_id: &str, name: &str, output: &str) -> Self {
        ToolResult { call_id: call_id.into(), name: name.into(),
            output: output.into(), success: true, blocked: false }
    }
    pub fn err(call_id: &str, name: &str, msg: &str) -> Self {
        ToolResult { call_id: call_id.into(), name: name.into(),
            output: msg.into(), success: false, blocked: false }
    }
    pub fn blocked_tool(call_id: &str, name: &str) -> Self {
        ToolResult { call_id: call_id.into(), name: name.into(),
            output: "Tool blocked by policy".into(), success: false, blocked: true }
    }
}

/// A tool definition exposed to the LLM (OpenAI function-calling schema)
#[derive(Clone, Debug)]
pub struct ToolDef {
    pub name:             String,
    pub description:      String,
    pub params:           Vec<ToolParam>,
    pub requires_approval: bool,
}

#[derive(Clone, Debug)]
pub struct ToolParam {
    pub name:        String,
    pub description: String,
    pub required:    bool,
    pub type_hint:   String,
}

impl ToolDef {
    pub fn to_openai_schema_json(&self) -> String {
        let props: Vec<String> = self.params.iter().map(|p| {
            format!(r#""{}":{{"type":"{}","description":"{}"}}"#,
                p.name, p.type_hint, p.description.replace('"', "\\\""))
        }).collect();
        let required: Vec<String> = self.params.iter()
            .filter(|p| p.required)
            .map(|p| format!("\"{}\"", p.name))
            .collect();
        format!(
            r#"{{"type":"function","function":{{"name":"{}","description":"{}","parameters":{{"type":"object","properties":{{{}}},"required":[{}]}}}}}}"#,
            self.name,
            self.description.replace('"', "\\\""),
            props.join(","),
            required.join(",")
        )
    }
}

pub type ToolHandler = Box<dyn Fn(&HashMap<String, String>) -> String + Send + Sync>;

pub struct ToolRegistry {
    defs:     Vec<ToolDef>,
    handlers: HashMap<String, ToolHandler>,
    blocked:  Vec<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        ToolRegistry { defs: Vec::new(), handlers: HashMap::new(), blocked: Vec::new() }
    }

    pub fn register(&mut self, def: ToolDef, handler: ToolHandler) {
        self.handlers.insert(def.name.clone(), handler);
        self.defs.push(def);
    }

    pub fn block(&mut self, tool_name: &str) {
        if !self.blocked.contains(&tool_name.to_string()) {
            self.blocked.push(tool_name.to_string());
        }
    }

    pub fn dispatch(&self, call: &ToolCall) -> ToolResult {
        if self.blocked.contains(&call.name) {
            return ToolResult::blocked_tool(&call.id, &call.name);
        }
        match self.handlers.get(&call.name) {
            Some(handler) => ToolResult::ok(&call.id, &call.name, &handler(&call.params)),
            None => ToolResult::err(&call.id, &call.name, &format!("Unknown tool: {}", call.name)),
        }
    }

    pub fn schema_json(&self) -> String {
        let schemas: Vec<String> = self.defs.iter()
            .filter(|d| !self.blocked.contains(&d.name))
            .map(|d| d.to_openai_schema_json())
            .collect();
        format!("[{}]", schemas.join(","))
    }

    pub fn names(&self) -> Vec<String> {
        self.defs.iter().map(|d| d.name.clone()).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}
