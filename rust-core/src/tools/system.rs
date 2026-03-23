// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: tools :: system
//
// System tools: read_file, write_file, list_files, run_shell, http_get/post.
// Shell commands are queued for Java/Shizuku to execute  -  Rust never
// executes shell directly.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use crate::ai::tools::{ToolRegistry, ToolDef, ToolParam};
use std::collections::HashMap;

pub fn register_system_tools(registry: &mut ToolRegistry) {
    // ── read_file ────────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "read_file".to_string(),
            description: "Read the contents of a file at the given path".to_string(),
            params: vec![
                ToolParam { name: "path".to_string(), description: "Absolute file path".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "max_bytes".to_string(), description: "Max bytes to read (default 8192)".to_string(), required: false, type_hint: "number".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let path = match params.get("path") {
                Some(p) if !p.is_empty() => p.clone(),
                _ => return r#"{"error":"path required"}"#.to_string(),
            };
            let max_bytes: usize = params.get("max_bytes")
                .and_then(|v| v.parse().ok())
                .unwrap_or(8192);
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let truncated = &bytes[..bytes.len().min(max_bytes)];
                    let content = String::from_utf8_lossy(truncated);
                    format!(r#"{{"ok":true,"content":"{}","bytes":{}}}"#,
                        content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n"),
                        bytes.len())
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "\\\"")),
            }
        }),
    );

    // ── write_file ───────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "write_file".to_string(),
            description: "Write content to a file (creates or overwrites)".to_string(),
            params: vec![
                ToolParam { name: "path".to_string(), description: "Absolute file path".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "content".to_string(), description: "Text content to write".to_string(), required: true, type_hint: "string".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let path = match params.get("path") {
                Some(p) if !p.is_empty() => p.clone(),
                _ => return r#"{"error":"path required"}"#.to_string(),
            };
            let content = params.get("content").cloned().unwrap_or_default();
            // Refuse writes to system paths
            if path.starts_with("/proc") || path.starts_with("/sys") || path.starts_with("/dev") {
                return r#"{"error":"write to system path denied"}"#.to_string();
            }
            // Create parent dirs if needed
            if let Some(parent) = std::path::Path::new(&path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, content.as_bytes()) {
                Ok(_) => format!(r#"{{"ok":true,"bytes":{}}}"#, content.len()),
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "\\\"")),
            }
        }),
    );

    // ── list_files ───────────────────────────────────────────────────────────
    registry.register(
        ToolDef {
            name: "list_files".to_string(),
            description: "List files in a directory".to_string(),
            params: vec![
                ToolParam { name: "path".to_string(), description: "Directory path".to_string(), required: true, type_hint: "string".to_string() },
            ],
            requires_approval: false,
        },
        Box::new(|params: &HashMap<String, String>| {
            let path = params.get("path").cloned().unwrap_or_else(|| ".".to_string());
            match std::fs::read_dir(&path) {
                Ok(entries) => {
                    let names: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| {
                            let name = e.file_name().to_string_lossy().to_string();
                            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                            if is_dir { format!("{}/", name) } else { name }
                        })
                        .collect();
                    let list: Vec<String> = names.iter()
                        .map(|n| format!("\"{}\"", n.replace('"', "\\\"")))
                        .collect();
                    format!(r#"{{"ok":true,"files":[{}],"count":{}}}"#,
                        list.join(","), names.len())
                }
                Err(e) => format!(r#"{{"error":"{}"}}"#, e.to_string().replace('"', "\\\"")),
            }
        }),
    );

    // ── run_shell ────────────────────────────────────────────────────────────
    // Does NOT execute  -  queues for Java/Shizuku to run and poll result.
    registry.register(
        ToolDef {
            name: "run_shell".to_string(),
            description: "Run a shell command via Shizuku. Returns the command output.".to_string(),
            params: vec![
                ToolParam { name: "cmd".to_string(), description: "Shell command to execute".to_string(), required: true, type_hint: "string".to_string() },
                ToolParam { name: "timeout_ms".to_string(), description: "Timeout in ms (default 5000)".to_string(), required: false, type_hint: "number".to_string() },
            ],
            requires_approval: false, // approval logic handled in dispatch layer
        },
        Box::new(|params: &HashMap<String, String>| {
            let cmd = match params.get("cmd") {
                Some(c) if !c.is_empty() => c.clone(),
                _ => return r#"{"error":"cmd required"}"#.to_string(),
            };
            // Queue marker  -  the AI loop's dispatch layer handles the actual
            // Shizuku execution via the Java poll endpoint.
            format!(r#"{{"queued":true,"cmd":"{}","note":"result available via /shell/result"}}"#,
                cmd.replace('"', "\\\""))
        }),
    );
}
