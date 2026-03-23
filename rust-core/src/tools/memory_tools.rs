// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: tools :: memory_tools
//
// add_memory, search_memory, list_memories tools.
// These call into the shared STATE memory store.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::ai::tools::{ToolRegistry, ToolDef, ToolParam};
use std::collections::HashMap;

pub fn register_memory_tools(registry: &mut ToolRegistry) {
    // ── add_memory ───────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "add_memory".to_string(),
            description: "Store a piece of information in persistent memory for later retrieval".to_string(),
            params: vec![
                ToolParam { name: "content".to_string(), description: "The information to remember".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "tags".to_string(), description: "Comma-separated tags (e.g. 'work,important')".to_string(), required: false, type_hint: "string".to_string() },
                ToolParam { name: "key".to_string(), description: "Optional unique key (overwrites existing with same key)".to_string(), required: false, type_hint: "string".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let content = match params.get("content") {
                Some(c) if !c.is_empty() => c.clone(),
                _ => return r#"{"error":"content required"}"#.to_string(),
            };
            let tags: Vec<String> = params.get("tags")
                .map(|t| t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            // Actual storage is in STATE  -  this returns a command for the route
            // handler to execute after dispatch. The route layer calls
            // STATE.memory_index.push() with the result.
            format!(r#"{{"action":"add_memory","content":"{}","tags":[{}]}}"#,
                content.replace('"', "\\\""),
                tags.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(","))
        }),
    );

    // ── search_memory ────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "search_memory".to_string(),
            description: "Search stored memories by keyword or semantic query".to_string(),
            params: vec![
                ToolParam { name: "query".to_string(), description: "Search query".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "limit".to_string(), description: "Max results (default 5)".to_string(), required: false, type_hint: "number".to_string() },
                ToolParam { name: "tag".to_string(), description: "Filter by tag".to_string(), required: false, type_hint: "string".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let query = match params.get("query") {
                Some(q) if !q.is_empty() => q.clone(),
                _ => return r#"{"error":"query required"}"#.to_string(),
            };
            let limit: usize = params.get("limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(5)
                .min(20);
            // Returns a search command  -  route layer executes against STATE
            format!(r#"{{"action":"search_memory","query":"{}","limit":{}}}"#,
                query.replace('"', "\\\""), limit)
        }),
    );

    // ── list_memories ────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "list_memories".to_string(),
            description: "List all stored memories (optionally filtered by tag)".to_string(),
            params: vec![
                ToolParam { name: "tag".to_string(), description: "Filter by tag".to_string(), required: false, type_hint: "string".to_string() },
                ToolParam { name: "limit".to_string(), description: "Max entries (default 20)".to_string(), required: false, type_hint: "number".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let limit: usize = params.get("limit")
                .and_then(|v| v.parse().ok())
                .unwrap_or(20)
                .min(100);
            let tag = params.get("tag").cloned().unwrap_or_default();
            format!(r#"{{"action":"list_memories","tag":"{}","limit":{}}}"#,
                tag.replace('"', "\\\""), limit)
        }),
    );
}
