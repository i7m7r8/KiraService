// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: ai :: subagents  (Session 3)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref SUBAGENT_REGISTRY: Arc<Mutex<SubAgentRegistry>> =
        Arc::new(Mutex::new(SubAgentRegistry::new(5)));
}

#[derive(Clone, Debug, PartialEq)]
pub enum SubAgentPhase { Spawning, Running, Done, Failed, Killed }
impl SubAgentPhase {
    pub fn as_str(&self) -> &str {
        match self {
            SubAgentPhase::Spawning=>"spawning", SubAgentPhase::Running=>"running",
            SubAgentPhase::Done=>"done", SubAgentPhase::Failed=>"failed",
            SubAgentPhase::Killed=>"killed",
        }
    }
    pub fn is_terminal(&self) -> bool {
        matches!(self, SubAgentPhase::Done|SubAgentPhase::Failed|SubAgentPhase::Killed)
    }
}

#[derive(Clone, Debug)]
pub struct SubAgentState {
    pub id: String, pub goal: String, pub session_id: String,
    pub parent_id: Option<String>, pub depth: u32,
    pub phase: SubAgentPhase, pub result: Option<String>, pub error: Option<String>,
    pub steps: u32, pub tools_used: Vec<String>,
    pub started_at: u128, pub ended_at: Option<u128>, pub model: String,
}
impl SubAgentState {
    pub fn to_json(&self) -> String {
        let tools: Vec<String> = self.tools_used.iter().map(|t| format!("\"{}\"",t)).collect();
        format!(r#"{{"id":"{}","goal":"{}","session":"{}","depth":{},"phase":"{}","steps":{},"tools_used":[{}],"result":{},"error":{},"started_at":{},"ended_at":{}}}"#,
            self.id, self.goal.replace('"',"\\\""), self.session_id, self.depth,
            self.phase.as_str(), self.steps, tools.join(","),
            self.result.as_deref().map(|r|format!("\"{}\"",r.replace('"',"\\\"").replace('\n',"\\n"))).unwrap_or("null".to_string()),
            self.error.as_deref().map(|e|format!("\"{}\"",e.replace('"',"\\\""))).unwrap_or("null".to_string()),
            self.started_at,
            self.ended_at.map(|t|t.to_string()).unwrap_or("null".to_string()))
    }
}

#[derive(Clone, Debug)]
pub struct SpawnRequest {
    pub goal: String, pub parent_id: Option<String>,
    pub session_id: Option<String>, pub model: Option<String>, pub max_steps: u32,
}
impl SpawnRequest {
    pub fn from_json(json: &str) -> Self {
        SpawnRequest {
            goal:       sf(json,"goal").unwrap_or_default(),
            parent_id:  sf(json,"parent_id"),
            session_id: sf(json,"session"),
            model:      sf(json,"model"),
            max_steps:  uf(json,"max_steps").unwrap_or(20),
        }
    }
}

pub struct SubAgentRegistry {
    pub agents: HashMap<String, SubAgentState>,
    pub max_depth: u32, counter: u64,
}
impl SubAgentRegistry {
    pub fn new(max_depth: u32) -> Self {
        SubAgentRegistry { agents: HashMap::new(), max_depth, counter: 0 }
    }
    pub fn next_id(&mut self) -> String {
        self.counter += 1;
        format!("sa_{}_{}", now_ms(), self.counter)
    }
    pub fn can_spawn(&self, parent_depth: u32) -> bool { parent_depth + 1 <= self.max_depth }
    pub fn register(&mut self, a: SubAgentState) { self.agents.insert(a.id.clone(), a); }
    pub fn get(&self, id: &str) -> Option<&SubAgentState> { self.agents.get(id) }
    pub fn kill(&mut self, id: &str) -> bool {
        if let Some(a) = self.agents.get_mut(id) {
            if !a.phase.is_terminal() { a.phase=SubAgentPhase::Killed; a.ended_at=Some(now_ms()); return true; }
        }
        false
    }
    pub fn finish(&mut self, id: &str, phase: SubAgentPhase, result: Option<String>,
                  error: Option<String>, steps: u32, tools: Vec<String>) {
        if let Some(a) = self.agents.get_mut(id) {
            if a.phase != SubAgentPhase::Killed {
                let term = phase.is_terminal();
                a.phase = phase; a.steps = steps; a.tools_used = tools;
                if result.is_some() { a.result = result; }
                if error.is_some()  { a.error  = error; }
                if term { a.ended_at = Some(now_ms()); }
            }
        }
    }
    pub fn list_json(&self) -> String {
        let items: Vec<String> = self.agents.values().map(|a|a.to_json()).collect();
        format!("[{}]",items.join(","))
    }
    pub fn running_count(&self) -> usize {
        self.agents.values().filter(|a|!a.phase.is_terminal()).count()
    }
    pub fn prune(&mut self, now: u128, ttl: u128) {
        self.agents.retain(|_,a| !a.phase.is_terminal() ||
            a.ended_at.map_or(true,|e| now.saturating_sub(e)<ttl));
    }
}
impl Default for SubAgentRegistry { fn default() -> Self { Self::new(5) } }

// ── OnceLock shims ────────────────────────────────────────────────────────────
use std::sync::OnceLock;
type LlmFn    = fn(&str,&str,&str)->Result<String,String>;
type DispFn   = fn(&str,&HashMap<String,String>)->String;
// Returns (api_key, base_url, model, system_prompt, history)
type ConfigFn = fn()->(String,String,String,String,Vec<(String,String)>);
type ToolsFn  = fn()->String;  // returns tools JSON

static SA_LLM:    OnceLock<LlmFn>    = OnceLock::new();
static SA_DISP:   OnceLock<DispFn>   = OnceLock::new();
static SA_CONFIG: OnceLock<ConfigFn> = OnceLock::new();
static SA_TOOLS:  OnceLock<ToolsFn>  = OnceLock::new();

pub fn register_subagent_fns(llm: LlmFn, disp: DispFn, config: ConfigFn, tools: ToolsFn) {
    let _=SA_LLM.set(llm); let _=SA_DISP.set(disp);
    let _=SA_CONFIG.set(config); let _=SA_TOOLS.set(tools);
}

// ── spawn_subagent ────────────────────────────────────────────────────────────
pub fn spawn_subagent(req: SpawnRequest) -> Result<String, String> {
    if req.goal.is_empty() { return Err("goal is required".into()); }

    let parent_depth = req.parent_id.as_deref()
        .and_then(|pid| SUBAGENT_REGISTRY.lock().ok()
            .and_then(|r| r.get(pid).map(|a| a.depth)))
        .unwrap_or(0);

    {
        let reg = SUBAGENT_REGISTRY.lock().unwrap_or_else(|e|e.into_inner());
        if !reg.can_spawn(parent_depth) {
            return Err(format!("max depth {} reached", reg.max_depth));
        }
    }

    let agent_id   = { SUBAGENT_REGISTRY.lock().unwrap_or_else(|e|e.into_inner()).next_id() };
    let session_id = req.session_id.clone().unwrap_or_else(|| format!("sub_{}", &agent_id));

    let agent = SubAgentState {
        id: agent_id.clone(), goal: req.goal.clone(),
        session_id: session_id.clone(), parent_id: req.parent_id.clone(),
        depth: parent_depth+1, phase: SubAgentPhase::Spawning,
        result: None, error: None, steps: 0, tools_used: vec![],
        started_at: now_ms(), ended_at: None,
        model: req.model.clone().unwrap_or_default(),
    };
    SUBAGENT_REGISTRY.lock().unwrap_or_else(|e|e.into_inner()).register(agent);

    let aid = agent_id.clone();
    let goal = req.goal.clone();
    let model_override = req.model.clone();
    let max_steps = req.max_steps;
    let sid = session_id.clone();

    std::thread::spawn(move || {
        // Get LLM config
        let (api_key, base_url, model, sys_prompt, history) = match SA_CONFIG.get() {
            Some(f) => f(),
            None => {
                finish_agent(&aid, SubAgentPhase::Failed, None,
                    Some("config fn not registered".into()), 0, vec![]);
                return;
            }
        };
        let model = model_override.unwrap_or(model);
        let tools_json = SA_TOOLS.get().map(|f|f()).unwrap_or_else(||"[]".to_string());

        // Mark running
        if let Ok(mut reg) = SUBAGENT_REGISTRY.lock() {
            if let Some(a) = reg.agents.get_mut(&aid) {
                a.phase = SubAgentPhase::Running;
                if !model.is_empty() { a.model = model.clone(); }
            }
        }

        // Build prompt
        let system = format!(
            "{}\n\n## Sub-Agent Mission\nGoal: {}\nComplete this goal fully then stop.",
            sys_prompt, goal
        );

        // Run loop
        let result = subagent_loop(
            &aid, &api_key, &base_url, &model, &system,
            &history, &goal, max_steps, &tools_json
        );

        finish_agent(&aid,
            if result.error.is_some() { SubAgentPhase::Failed } else { SubAgentPhase::Done },
            if result.error.is_none() { Some(result.content) } else { None },
            result.error,
            result.steps, result.tools_used);
    });

    Ok(agent_id)
}

fn finish_agent(id: &str, phase: SubAgentPhase, result: Option<String>,
                error: Option<String>, steps: u32, tools: Vec<String>) {
    let mut reg = SUBAGENT_REGISTRY.lock().unwrap_or_else(|e|e.into_inner());
    reg.finish(id, phase, result, error, steps, tools);
}

struct LoopResult {
    content: String, tools_used: Vec<String>,
    steps: u32, error: Option<String>,
}

fn subagent_loop(
    agent_id: &str,
    api_key: &str, base_url: &str, model: &str,
    system: &str, history: &[(String,String)],
    goal: &str, max_steps: u32, tools_json: &str,
) -> LoopResult {
    use crate::ai::runner::{Turn, build_messages_json, parse_tool_calls_json,
        hash_params, json_escape_str};

    let mut turns = Vec::<Turn>::new();
    if !system.is_empty() { turns.push(Turn::system(system)); }
    for (r,c) in history {
        turns.push(if r=="assistant"{Turn::assistant(c)}else{Turn::user(c)});
    }
    turns.push(Turn::user(goal));

    let mut tools_used = Vec::new();
    let mut final_text = String::new();
    let mut seen: Vec<String> = Vec::new(); // loop detection

    for step in 0..max_steps {
        // Kill check
        if SUBAGENT_REGISTRY.lock().ok()
            .and_then(|r| r.agents.get(agent_id).map(|a| a.phase == SubAgentPhase::Killed))
            .unwrap_or(false)
        {
            return LoopResult { content: final_text, tools_used, steps: step,
                error: Some("killed".into()) };
        }

        let msgs = build_messages_json(&turns);
        let body = if tools_json.is_empty() || tools_json == "[]" {
            format!(r#"{{"model":{},"max_tokens":2048,"messages":{}}}"#,
                json_escape_str(model), msgs)
        } else {
            format!(r#"{{"model":{},"max_tokens":2048,"messages":{},"tools":{},"tool_choice":"auto"}}"#,
                json_escape_str(model), msgs, tools_json)
        };

        let resp = match SA_LLM.get().map(|f|f(api_key,base_url,&body)) {
            Some(Ok(r))  => r,
            Some(Err(e)) => return LoopResult{content:final_text,tools_used,steps:step,error:Some(e)},
            None         => return LoopResult{content:final_text,tools_used,steps:step,error:Some("llm not registered".into())},
        };

        let tcs   = parse_tool_calls_json(&resp);
        let text  = extract_content_simple(&resp).unwrap_or_default();

        if tcs.is_empty() {
            final_text = text;
            break;
        }

        // Build assistant turn with tool_calls JSON
        let tc_arr: Vec<String> = tcs.iter().map(|tc| {
            format!(r#"{{"id":"{}","type":"function","function":{{"name":"{}","arguments":"{}"}}}}"#,
                tc.id, tc.name, tc.args_json.replace('"',"\\\""))
        }).collect();
        let mut asst = Turn::assistant(&text);
        asst.tool_calls_json = Some(format!("[{}]", tc_arr.join(",")));
        turns.push(asst);
        final_text = text;

        for tc in &tcs {
            let key = format!("{}:{}", tc.name, hash_params(&tc.params));
            if seen.iter().filter(|&k|k==&key).count() >= 2 {
                turns.push(Turn::tool_result(&tc.id, &tc.name,
                    "Loop detected — stop calling this tool"));
                continue;
            }
            seen.push(key);

            let out = match SA_DISP.get() {
                Some(f) => f(&tc.name, &tc.params),
                None    => r#"{"error":"dispatch not registered"}"#.to_string(),
            };
            tools_used.push(tc.name.clone());
            turns.push(Turn::tool_result(&tc.id, &tc.name, &out));
        }
    }

    LoopResult { content: final_text, tools_used, steps: max_steps, error: None }
}

fn extract_content_simple(json: &str) -> Option<String> {
    fn fs(json:&str, key:&str)->Option<String>{
        let s=format!("\"{}\":\"",key); let start=json.find(&s)?+s.len();
        let bytes=json.as_bytes(); let mut end=start;
        while end<bytes.len(){if bytes[end]==b'"'&&(end==0||bytes[end-1]!=b'\\'){break;}end+=1;}
        let v=&json[start..end]; if v.is_empty(){None}else{
            Some(v.replace("\\n","\n").replace("\\\"","\"").replace("\\\\","\\"))
        }
    }
    if let Some(mi)=json.find("\"message\":{"){if let Some(c)=fs(&json[mi..],"content"){return Some(c);}}
    fs(json,"content").or_else(||fs(json,"text"))
}

fn now_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default().as_millis()
}
fn sf(json:&str, key:&str)->Option<String>{
    let s=format!("\"{}\":\"",key); let start=json.find(&s)?+s.len();
    let bytes=json.as_bytes(); let mut end=start;
    while end<bytes.len(){if bytes[end]==b'"'&&(end==0||bytes[end-1]!=b'\\'){break;}end+=1;}
    let v=&json[start..end]; if v.is_empty(){None}else{Some(v.to_string())}
}
fn uf(json:&str, key:&str)->Option<u32>{
    let s=format!("\"{}\":",key); let start=json.find(&s)?+s.len();
    let slice=json[start..].trim_start();
    let end=slice.find(|c:char|!c.is_ascii_digit()).unwrap_or(slice.len());
    slice[..end].parse().ok()
}
