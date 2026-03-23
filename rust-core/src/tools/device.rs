// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: tools :: device
//
// Android device tools  -  all execute via Java JNI poll pattern.
// Session 1: stubs registered.  Session 15: full implementations.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::ai::tools::{ToolRegistry, ToolDef, ToolParam};
use std::collections::HashMap;

pub fn register_device_tools(registry: &mut ToolRegistry) {
    // ── get_notifications ────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "get_notifications".to_string(),
            description: "Get recent Android notifications".to_string(),
            params: vec![
                ToolParam { name: "limit".to_string(), description: "Max notifications (default 10)".to_string(), required: false, type_hint: "number".to_string() },
                ToolParam { name: "pkg".to_string(), description: "Filter by app package name".to_string(), required: false, type_hint: "string".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(java_poll_tool("get_notifications")),
    );

    // ── get_location ─────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "get_location".to_string(),
            description: "Get the device's current GPS location (lat, lon, accuracy)".to_string(),
            params: vec![],
            requires_approval: false,
        },
        Box::new(java_poll_tool("get_location")),
    );

    // ── send_sms ─────────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "send_sms".to_string(),
            description: "Send an SMS message".to_string(),
            params: vec![
                ToolParam { name: "to".to_string(), description: "Phone number".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "body".to_string(), description: "Message text".to_string(), required: true, type_hint: "string".to_string() },
            ],
            requires_approval: true,
        },
        Box::new(java_poll_tool("send_sms")),
    );

    // ── list_contacts ────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "list_contacts".to_string(),
            description: "Search contacts by name or phone number".to_string(),
            params: vec![
                ToolParam { name: "query".to_string(), description: "Name or number to search".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "limit".to_string(), description: "Max results (default 5)".to_string(), required: false, type_hint: "number".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(java_poll_tool("list_contacts")),
    );

    // ── list_calendar ────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "list_calendar".to_string(),
            description: "List calendar events within a date range".to_string(),
            params: vec![
                ToolParam { name: "from_ms".to_string(), description: "Start time (unix ms)".to_string(), required: false, type_hint: "number".to_string() },
                ToolParam { name: "to_ms".to_string(), description: "End time (unix ms)".to_string(), required: false, type_hint: "number".to_string() },
                ToolParam { name: "limit".to_string(), description: "Max events (default 10)".to_string(), required: false, type_hint: "number".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(java_poll_tool("list_calendar")),
    );

    // ── take_photo ───────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "take_photo".to_string(),
            description: "Capture a photo from the device camera".to_string(),
            params: vec![
                ToolParam { name: "camera".to_string(), description: "\"front\" or \"back\" (default back)".to_string(), required: false, type_hint: "string".to_string() },
            ],
            requires_approval: true,
        },
        Box::new(java_poll_tool("take_photo")),
    );
}

/// Helper: create a tool handler that queues a Java poll action
fn java_poll_tool(action: &'static str) -> impl Fn(&HashMap<String, String>) -> String + Send + Sync {
    move |params: &HashMap<String, String>| {
        // Serialize params to JSON for the Java poll endpoint
        let params_json: Vec<String> = params.iter()
            .map(|(k, v)| format!("\"{}\":\"{}\"", k, v.replace('"', "\\\"")))
            .collect();
        // Returns a command object  -  the route handler intercepts this
        // and queues it via STATE.pending_java_actions for Java to poll
        format!(r#"{{"action":"{}","params":{{{}}}}}"#,
            action, params_json.join(","))
    }
}
