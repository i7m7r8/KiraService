// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: streaming
//
// Unified SSE / streaming response parser for all provider formats.
// Mirrors OpenClaw: src/agents/stream-handler.ts
//                   src/agents/providers/*/streaming.ts
//
// S5: Handles Anthropic, OpenAI, Google (Gemini), and generic SSE formats.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use super::models::ModelProvider;

// ── Streaming output ───────────────────────────────────────────────────────────

/// One parsed chunk from an SSE stream.
#[derive(Clone, Debug)]
pub enum StreamChunk {
    /// Text delta (content to display/accumulate)
    TextDelta  { text: String },
    /// Tool call started
    ToolStart  { id: String, name: String },
    /// Tool argument fragment (JSON fragment, must be accumulated)
    ToolDelta  { id: String, args_fragment: String },
    /// Tool call complete (all args accumulated)
    ToolDone   { id: String, name: String, args_json: String },
    /// Stream finished
    Done       { stop_reason: String, usage: Option<StreamUsage> },
    /// Recoverable parse error on one line (skip and continue)
    ParseError { line: String },
}

#[derive(Clone, Debug, Default)]
pub struct StreamUsage {
    pub input_tokens:  u32,
    pub output_tokens: u32,
}

// ── Parser state (for incremental tool-call accumulation) ─────────────────────

pub struct StreamParser {
    provider:       ModelProvider,
    /// Accumulated text content
    pub text:       String,
    /// Tool calls being assembled: id → (name, args_so_far)
    tool_calls:     std::collections::HashMap<String, (String, String)>,
    /// Order tool call ids appeared (for deterministic output)
    tool_order:     Vec<String>,
    pub stop_reason: Option<String>,
    pub usage:      Option<StreamUsage>,
}

impl Default for StreamParser {
    fn default() -> Self {
        StreamParser {
            provider:    ModelProvider::OpenAICompat,
            text:        String::new(),
            tool_calls:  std::collections::HashMap::new(),
            tool_order:  Vec::new(),
            stop_reason: None,
            usage:       None,
        }
    }
}

impl StreamParser {
    pub fn new(provider: ModelProvider) -> Self {
        StreamParser { provider, ..Default::default() }
    }

    /// Feed one SSE line (without the "data: " prefix) and return parsed chunks.
    pub fn feed_line(&mut self, line: &str) -> Vec<StreamChunk> {
        let line = line.trim();
        if line.is_empty() || line == "[DONE]" {
            if line == "[DONE]" {
                return vec![StreamChunk::Done {
                    stop_reason: self.stop_reason.clone().unwrap_or_else(|| "stop".to_string()),
                    usage: self.usage.clone(),
                }];
            }
            return vec![];
        }

        match &self.provider {
            p if p.is_anthropic_native() => self.parse_anthropic(line),
            p if p.is_google_native()    => self.parse_google(line),
            _                            => self.parse_openai(line),
        }
    }

    /// Parse a complete non-streaming response body.
    pub fn parse_complete_response(&mut self, body: &str) -> Vec<StreamChunk> {
        match &self.provider {
            p if p.is_anthropic_native() => self.parse_anthropic_complete(body),
            p if p.is_google_native()    => self.parse_google_complete(body),
            _                            => self.parse_openai_complete(body),
        }
    }

    // ── OpenAI / Groq / Together / Mistral / DeepSeek / xAI / etc. ──────────
    // Format: {"choices":[{"delta":{"content":"...","tool_calls":[...]}}]}

    fn parse_openai(&mut self, line: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();

        // Content delta
        if let Some(content) = self.extract_delta_content(line) {
            if !content.is_empty() {
                self.text.push_str(&content);
                chunks.push(StreamChunk::TextDelta { text: content });
            }
        }

        // Tool call deltas
        chunks.extend(self.extract_openai_tool_deltas(line));

        // Finish reason
        if let Some(reason) = str_field(line, "finish_reason") {
            if reason != "null" {
                self.stop_reason = Some(reason);
            }
        }

        // Usage (sometimes in last chunk)
        if let Some(usage) = self.extract_openai_usage(line) {
            self.usage = Some(usage);
        }

        chunks
    }

    fn parse_openai_complete(&mut self, body: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();

        // choices[0].message.content
        if let Some(msg_start) = body.find("\"message\":{") {
            let sub = &body[msg_start..];
            if let Some(content) = str_field(sub, "content") {
                if !content.is_empty() {
                    self.text = content.clone();
                    chunks.push(StreamChunk::TextDelta { text: content });
                }
            }
            // tool_calls in message
            chunks.extend(self.extract_complete_tool_calls(sub));
        }

        // finish_reason
        let reason = str_field(body, "finish_reason").unwrap_or_else(|| "stop".to_string());
        chunks.push(StreamChunk::Done {
            stop_reason: reason,
            usage: self.extract_openai_usage(body),
        });
        chunks
    }

    // ── Anthropic native format ────────────────────────────────────────────────
    // Events: content_block_start, content_block_delta, content_block_stop,
    //         message_delta (stop_reason, usage), message_stop

    fn parse_anthropic(&mut self, line: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();

        let event_type = str_field(line, "type").unwrap_or_default();
        match event_type.as_str() {
            "content_block_delta" => {
                // delta.type = "text_delta" | "input_json_delta"
                let delta_type = {
                    // find "delta":{ and extract type within it
                    if let Some(start) = line.find("\"delta\":{") {
                        str_field(&line[start..], "type").unwrap_or_default()
                    } else { String::new() }
                };
                match delta_type.as_str() {
                    "text_delta" => {
                        if let Some(text) = self.extract_anthropic_text_delta(line) {
                            self.text.push_str(&text);
                            chunks.push(StreamChunk::TextDelta { text });
                        }
                    }
                    "input_json_delta" => {
                        // Tool argument fragment
                        let index = u32_field(line, "index").unwrap_or(0);
                        let partial = str_field_in_delta(line, "partial_json").unwrap_or_default();
                        let id = format!("tool_{}", index);
                        let entry = self.tool_calls.entry(id.clone()).or_insert_with(|| (String::new(), String::new()));
                        entry.1.push_str(&partial);
                        chunks.push(StreamChunk::ToolDelta { id, args_fragment: partial });
                    }
                    _ => {}
                }
            }
            "content_block_start" => {
                // type=tool_use → record tool start
                let block_type = {
                    if let Some(start) = line.find("\"content_block\":{") {
                        str_field(&line[start..], "type").unwrap_or_default()
                    } else { String::new() }
                };
                if block_type == "tool_use" {
                    let id   = str_field_in_content_block(line, "id").unwrap_or_else(|| uuid_short());
                    let name = str_field_in_content_block(line, "name").unwrap_or_default();
                    if !self.tool_calls.contains_key(&id) {
                        self.tool_calls.insert(id.clone(), (name.clone(), String::new()));
                        self.tool_order.push(id.clone());
                    }
                    chunks.push(StreamChunk::ToolStart { id, name });
                }
            }
            "content_block_stop" => {
                // Finalise the tool call at this index
                let index = u32_field(line, "index").unwrap_or(0);
                let id    = format!("tool_{}", index);
                if let Some((name, args)) = self.tool_calls.get(&id) {
                    chunks.push(StreamChunk::ToolDone {
                        id:       id.clone(),
                        name:     name.clone(),
                        args_json: args.clone(),
                    });
                }
            }
            "message_delta" => {
                if let Some(reason) = str_field(line, "stop_reason") {
                    self.stop_reason = Some(reason);
                }
                if let Some(usage) = self.extract_anthropic_usage(line) {
                    self.usage = Some(usage);
                }
            }
            "message_stop" => {
                chunks.push(StreamChunk::Done {
                    stop_reason: self.stop_reason.clone().unwrap_or_else(|| "end_turn".to_string()),
                    usage: self.usage.clone(),
                });
            }
            _ => {}
        }
        chunks
    }

    fn parse_anthropic_complete(&mut self, body: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        // content array: [{type:"text",text:"..."},{type:"tool_use",...}]
        if let Some(text) = extract_anthropic_text_content(body) {
            self.text = text.clone();
            chunks.push(StreamChunk::TextDelta { text });
        }
        chunks.extend(self.extract_anthropic_tool_uses(body));
        let reason = str_field(body, "stop_reason").unwrap_or_else(|| "end_turn".to_string());
        chunks.push(StreamChunk::Done {
            stop_reason: reason,
            usage: self.extract_anthropic_usage(body),
        });
        chunks
    }

    // ── Google Gemini ─────────────────────────────────────────────────────────
    // Format: candidates[0].content.parts[{text:"..."}]

    fn parse_google(&mut self, line: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        if let Some(text) = str_field(line, "text") {
            if !text.is_empty() {
                self.text.push_str(&text);
                chunks.push(StreamChunk::TextDelta { text });
            }
        }
        if line.contains("\"finishReason\"") {
            let reason = str_field(line, "finishReason").unwrap_or_else(|| "STOP".to_string());
            chunks.push(StreamChunk::Done {
                stop_reason: reason.to_lowercase(),
                usage: None,
            });
        }
        chunks
    }

    fn parse_google_complete(&mut self, body: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        if let Some(text) = str_field(body, "text") {
            self.text = text.clone();
            chunks.push(StreamChunk::TextDelta { text });
        }
        chunks.push(StreamChunk::Done { stop_reason: "stop".to_string(), usage: None });
        chunks
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn extract_delta_content(&self, line: &str) -> Option<String> {
        // Find "delta":{ ... "content":"..." }
        let delta_start = line.find("\"delta\":{")?;
        let sub = &line[delta_start..];
        let v = str_field(sub, "content")?;
        if v == "null" { None } else { Some(v) }
    }

    fn extract_openai_tool_deltas(&mut self, line: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        // "tool_calls":[{"index":0,"id":"...","type":"function","function":{"name":"...","arguments":"..."}}]
        let tc_start = match line.find("\"tool_calls\":[") {
            Some(i) => i,
            None    => return chunks,
        };
        let sub = &line[tc_start + "\"tool_calls\":[".len()..];

        // Walk each tool_call object in the array
        let mut depth = 0i32;
        let mut obj_start = None;
        let bytes = sub.as_bytes();
        for (pos, &b) in bytes.iter().enumerate() {
            match b {
                b'{' => { depth += 1; if depth == 1 { obj_start = Some(pos); } }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            let fragment = &sub[start..=pos];
                            let index = u32_field(fragment, "index").unwrap_or(0);
                            let id    = str_field(fragment, "id")
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| format!("call_{}", index));
                            // Function name delta
                            if let Some(fn_start) = fragment.find("\"function\":{") {
                                let fn_frag = &fragment[fn_start..];
                                let name = str_field(fn_frag, "name").unwrap_or_default();
                                let args = str_field(fn_frag, "arguments").unwrap_or_default();
                                let entry = self.tool_calls.entry(id.clone())
                                    .or_insert_with(|| (String::new(), String::new()));
                                if !name.is_empty() && entry.0.is_empty() {
                                    entry.0 = name.clone();
                                    if !self.tool_order.contains(&id) {
                                        self.tool_order.push(id.clone());
                                    }
                                    chunks.push(StreamChunk::ToolStart {
                                        id: id.clone(), name,
                                    });
                                }
                                if !args.is_empty() {
                                    entry.1.push_str(&args);
                                    chunks.push(StreamChunk::ToolDelta {
                                        id: id.clone(), args_fragment: args,
                                    });
                                }
                            }
                        }
                        obj_start = None;
                        if bytes.get(pos + 1) == Some(&b']') { break; }
                    }
                }
                _ => {}
            }
        }
        chunks
    }

    fn extract_complete_tool_calls(&mut self, body: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        let tc_start = match body.find("\"tool_calls\":[") {
            Some(i) => i, None => return chunks,
        };
        let sub = &body[tc_start..];
        let mut depth = 0i32;
        let mut obj_start = None;
        let bytes = sub.as_bytes();
        for (pos, &b) in bytes.iter().enumerate() {
            match b {
                b'{' => { depth += 1; if depth == 1 { obj_start = Some(pos); } }
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(start) = obj_start {
                            let frag = &sub[start..=pos];
                            let id   = str_field(frag, "id").unwrap_or_else(|| uuid_short());
                            if let Some(fn_start) = frag.find("\"function\":{") {
                                let fn_frag = &frag[fn_start..];
                                let name = str_field(fn_frag, "name").unwrap_or_default();
                                let args = str_field(fn_frag, "arguments").unwrap_or_else(|| "{}".to_string());
                                chunks.push(StreamChunk::ToolDone { id, name, args_json: args });
                            }
                        }
                        obj_start = None;
                        if bytes.get(pos + 1) == Some(&b']') { break; }
                    }
                }
                _ => {}
            }
        }
        chunks
    }

    fn extract_anthropic_text_delta(&self, line: &str) -> Option<String> {
        // "delta":{"type":"text_delta","text":"..."}
        let delta_start = line.find("\"delta\":{")?;
        str_field(&line[delta_start..], "text")
    }

    fn extract_anthropic_tool_uses(&mut self, body: &str) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        // Look for {"type":"tool_use","id":"...","name":"...","input":{...}}
        let mut pos = 0;
        let bytes = body.as_bytes();
        while pos < bytes.len() {
            if let Some(tool_start) = body[pos..].find("\"type\":\"tool_use\"") {
                let abs = pos + tool_start;
                // Back up to find the opening { of this object
                let obj_start = body[..abs].rfind('{').unwrap_or(abs);
                // Find closing } (bracket-aware)
                let mut depth = 0i32;
                let mut obj_end = obj_start;
                for (i, &b) in bytes[obj_start..].iter().enumerate() {
                    match b {
                        b'{' => depth += 1,
                        b'}' => { depth -= 1; if depth == 0 { obj_end = obj_start + i; break; } }
                        _ => {}
                    }
                }
                let frag = &body[obj_start..=obj_end];
                let id   = str_field(frag, "id").unwrap_or_else(uuid_short);
                let name = str_field(frag, "name").unwrap_or_default();
                // input is an object  -  serialise it back
                let args = extract_object_field(frag, "input").unwrap_or_else(|| "{}".to_string());
                chunks.push(StreamChunk::ToolDone { id, name, args_json: args });
                pos = obj_end + 1;
            } else {
                break;
            }
        }
        chunks
    }

    fn extract_openai_usage(&self, line: &str) -> Option<StreamUsage> {
        let usage_start = line.find("\"usage\":{")?;
        let sub = &line[usage_start..];
        Some(StreamUsage {
            input_tokens:  u32_field(sub, "prompt_tokens").unwrap_or(0),
            output_tokens: u32_field(sub, "completion_tokens").unwrap_or(0),
        })
    }

    fn extract_anthropic_usage(&self, line: &str) -> Option<StreamUsage> {
        let usage_start = line.find("\"usage\":{")?;
        let sub = &line[usage_start..];
        Some(StreamUsage {
            input_tokens:  u32_field(sub, "input_tokens").unwrap_or(0),
            output_tokens: u32_field(sub, "output_tokens").unwrap_or(0),
        })
    }

    /// Return all completed tool calls as (id, name, args_json) triples.
    pub fn completed_tool_calls(&self) -> Vec<(String, String, String)> {
        self.tool_order.iter().filter_map(|id| {
            self.tool_calls.get(id).map(|(name, args)| {
                (id.clone(), name.clone(), args.clone())
            })
        }).collect()
    }
}

// ── Free-standing helpers ─────────────────────────────────────────────────────

/// Extract a string field from a flat JSON fragment (single-level, no serde).
pub fn str_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":\"", key);
    let start  = json.find(&needle)? + needle.len();
    let bytes  = json.as_bytes();
    let mut out = String::new();
    let mut i = start;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 1;
                if i < bytes.len() {
                    match bytes[i] {
                        b'"'  => out.push('"'),
                        b'n'  => out.push('\n'),
                        b't'  => out.push('\t'),
                        b'\\' => out.push('\\'),
                        b'r'  => out.push('\r'),
                        c     => { out.push('\\'); out.push(c as char); }
                    }
                }
            }
            b'"' => break,
            c    => out.push(c as char),
        }
        i += 1;
    }
    Some(out)
}

fn str_field_in_delta(json: &str, key: &str) -> Option<String> {
    let delta_start = json.find("\"delta\":{")?;
    str_field(&json[delta_start..], key)
}

fn str_field_in_content_block(json: &str, key: &str) -> Option<String> {
    let start = json.find("\"content_block\":{")?;
    str_field(&json[start..], key)
}

fn u32_field(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\":", key);
    let start  = json.find(&needle)? + needle.len();
    let rest   = json[start..].trim_start();
    let end    = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn extract_anthropic_text_content(body: &str) -> Option<String> {
    // Scan for {"type":"text","text":"..."} inside content array
    let type_needle = "\"type\":\"text\"";
    let start = body.find(type_needle)?;
    str_field(&body[start..], "text")
}

/// Extract a JSON object field value as a string (the raw JSON object).
fn extract_object_field(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":{{", key);
    let start  = json.find(&needle)? + needle.len() - 1;
    let bytes  = json.as_bytes();
    let mut depth = 0i32;
    let mut end = start;
    for i in start..bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => { depth -= 1; if depth == 0 { end = i; break; } }
            _ => {}
        }
    }
    Some(json[start..=end].to_string())
}

fn uuid_short() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos()).unwrap_or(0);
    format!("call_{:08x}", t)
}

// ModelProvider::default() provided by #[derive(Default)] or not needed here
