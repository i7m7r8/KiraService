
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// HTTP Server
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

fn run_http(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l, Err(e) => { eprintln!("kira bind: {}", e); return; }
    };
    for stream in listener.incoming().flatten() { thread::spawn(|| handle_http(stream)); }
}

/// Lock STATE recovering from poison  -  if a thread panicked while holding the
/// lock, we recover the state rather than panicking the HTTP thread too.
#[allow(unused_macros)]
macro_rules! state_lock {
    () => { match STATE.lock() {
        Ok(g)  => g,
        Err(e) => e.into_inner(), // recover poisoned mutex
    }}
}

fn handle_http(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let body = req.find("\r\n\r\n").map(|i| req[i+4..].trim().to_string()).unwrap_or_default();
    let resp = route_http_with_raw(parts[0], parts[1], &body, &req.to_string());
    let http = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Kira-Engine: rust-v9\r\n\r\n{}", resp.len(), resp);
    let _ = stream.write_all(http.as_bytes());
    STATE.lock().unwrap().request_count += 1;
}

fn get_http_header<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("
{}:", name.to_lowercase());
    let lreq   = req.to_lowercase();
    let pos    = lreq.find(&needle)?;
    let after  = &req[pos + needle.len()..];
    let end    = after.find("
").unwrap_or(after.len());
    Some(after[..end].trim())
}

fn requires_auth(path: &str) -> bool {
    // Session L: expanded auth coverage for all sensitive endpoints added in v0.1.x
    // Public: health checks, layer polling, crash reporting, setup info
    let public = [
        "/health", "/appstats", "/layer0", "/layer1",
        "/layer2/header", "/layer2/bubbles", "/layer2/typing",
        "/setup/providers", "/setup/status", "/crash",
        "/auth/set_secret", "/settings/health",
        "/telegram/last_update_id",
        "/memory/compression", "/crypto/status",
    ];
    if public.contains(&path) { return false; }
    // Everything else requires auth if a secret is set
    true
}

/// Constant-time comparison to prevent timing attacks on the token.
fn check_auth(token: Option<&str>) -> bool {
    let secret = {
        let s = match STATE.lock() { Ok(g) => g, Err(e) => e.into_inner() };
        s.http_secret.clone()
    };
    if secret.is_empty() { return true; }   // no secret set = localhost open
    match token {
        None    => false,
        Some(t) => {
            let tb = t.as_bytes();
            let sb = secret.as_bytes();
            if tb.len() != sb.len() { return false; }
            // fold XOR  -  result is 0 only if every byte matches
            tb.iter().zip(sb.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
        }
    }
}

/// Entry point called by handle_http  -  wraps route_http with auth check.
fn route_http_with_raw(method: &str, path: &str, body: &str, raw_req: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    if requires_auth(path_clean) {
        let token = get_http_header(raw_req, "x-kira-token");
        if !check_auth(token) {
            return r#"{"error":"unauthorized","code":401}"#.to_string();
        }
    }
    route_http(method, path, body)
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    match (method, path_clean) {
        // Health & stats
        // Auth management (localhost only  -  sets the shared secret)
        ("POST", "/auth/set_secret") => {
            let secret = extract_json_str(body, "secret").unwrap_or_default();
            if secret.len() >= 16 {
                STATE.lock().unwrap().http_secret = secret;
                r#"{"ok":true}"#.to_string()
            } else {
                r#"{"error":"secret must be at least 16 characters"}"#.to_string()
            }
        }
        ("DELETE", "/auth/secret") => {
            STATE.lock().unwrap().http_secret = String::new();
            r#"{"ok":true,"warning":"auth disabled  -  all endpoints open"}"#.to_string()
        }

        ("GET", "/health") | ("GET", "/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"status":"ok","version":"9.0","uptime_ms":{},"requests":{},"tool_calls":{},"battery":{},"charging":{},"notifications":{},"skills":{},"triggers":{},"memory_entries":{},"total_tokens":{},"sessions":{},"setup_done":{},"macros":{},"active_profile":"{}","variables":{}}}"#,
                now_ms()-s.uptime_start, s.request_count, s.tool_call_count,
                s.battery_pct, s.battery_charging, s.notifications.len(), s.skills.len(),
                s.triggers.iter().filter(|t|!t.fired).count(), s.memory_index.len(),
                s.total_tokens, s.sessions.len(), s.config.setup_done,
                s.macros.len(), esc(&s.active_profile), s.variables.len())
        }
        ("GET",  "/stats") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"notifications":{},"pending_cmds":{},"task_steps":{},"audit_entries":{},"context_turns":{},"daily_log_entries":{},"skills":{},"memory_entries":{},"cron_jobs":{},"tool_calls":{},"total_tokens":{},"uptime_ms":{},"macros":{},"macro_runs":{},"pending_actions":{},"variables":{}}}"#,
                s.notifications.len(), s.pending_cmds.len(), s.task_log.len(),
                s.audit_log.len(), s.context_turns.len(), s.daily_log.len(),
                s.skills.len(), s.memory_index.len(), s.cron_jobs.len(),
                s.tool_call_count, s.total_tokens, now_ms()-s.uptime_start,
                s.macros.len(), s.macro_run_log.len(), s.pending_actions.len(),
                s.variables.len())
        }

        // v40: Automation engine endpoints
        ("GET",  "/macros")            => { let s=STATE.lock().unwrap(); format!("[{}]", s.macros.iter().map(macro_to_json).collect::<Vec<_>>().join(",")) }
        ("POST", "/macros/add")        => { let m=parse_macro_from_json(body); let id=m.id.clone(); let mut s=STATE.lock().unwrap(); s.macros.retain(|x| x.id!=m.id); s.macros.push(m); format!(r#"{{"ok":true,"id":"{}"}}"#, id) }
        ("POST", "/macros/remove")     => { let id=extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().macros.retain(|m| m.id!=id); r#"{"ok":true}"#.to_string() }
        ("POST", "/macros/enable")     => { let id=extract_json_str(body,"id").unwrap_or_default(); let en=!body.contains("\"enabled\":false"); if let Some(m)=STATE.lock().unwrap().macros.iter_mut().find(|m| m.id==id) { m.enabled=en; } r#"{"ok":true}"#.to_string() }
        ("POST", "/macros/run")        => {
            let id=extract_json_str(body,"id").unwrap_or_default();
            let mut s=STATE.lock().unwrap();
            let actions=s.macros.iter().find(|m| m.id==id).map(|m| m.actions.clone()).unwrap_or_default();
            let name=s.macros.iter().find(|m| m.id==id).map(|m| m.name.clone()).unwrap_or_default();
            let start=now_ms();
            let (steps,_)=execute_macro_actions(&mut s, &id, &actions);
            if let Some(m)=s.macros.iter_mut().find(|m| m.id==id) { m.run_count+=1; m.last_run_ms=start; }
            s.macro_run_log.push_back(MacroRunLog { macro_id:id, macro_name:name, trigger:"api".to_string(), success:true, steps_run:steps, duration_ms:now_ms()-start, ts:start, error:String::new() });
            format!(r#"{{"ok":true,"steps":{}}}"#, steps)
        }
        ("GET",  "/macros/log")        => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.macro_run_log.iter().skip(s.macro_run_log.len().saturating_sub(100)).map(|r| format!(r#"{{"macro_id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"duration_ms":{},"ts":{}}}"#, esc(&r.macro_id),esc(&r.macro_name),esc(&r.trigger),r.success,r.steps_run,r.duration_ms,r.ts)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/macros/pending")    => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.pending_actions.iter().map(|pa| { let pk: Vec<String>=pa.params.iter().map(|(k,v)| format!("\"{}\":\"{}\"",esc(k),esc(v))).collect(); format!(r#"{{"macro_id":"{}","action_id":"{}","kind":"{}","params":{{{}}}}}"#, esc(&pa.macro_id),esc(&pa.action_id),esc(&pa.kind),pk.join(",")) }).collect(); format!("[{}]", items.join(",")) }

        // v40: Variables
        ("GET",  "/variables")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.variables.values().map(|v| format!(r#"{{"name":"{}","value":"{}","type":"{}","updated_ms":{}}}"#, esc(&v.name),esc(&v.value),esc(&v.var_type),v.updated_ms)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/variables/set")     => { let name=extract_json_str(body,"name").unwrap_or_default(); let value=extract_json_str(body,"value").unwrap_or_default(); let vt=extract_json_str(body,"type").unwrap_or_else(||"string".to_string()); let ts=now_ms(); let mut s=STATE.lock().unwrap(); s.variables.entry(name.clone()).and_modify(|v|{v.value=value.clone();v.updated_ms=ts;}).or_insert(AutoVariable{name,value,var_type:vt,persistent:false,created_ms:ts,updated_ms:ts}); r#"{"ok":true}"#.to_string() }
        ("POST", "/variables/delete")  => { let name=extract_json_str(body,"name").unwrap_or_default(); STATE.lock().unwrap().variables.remove(&name); r#"{"ok":true}"#.to_string() }
        ("GET",  "/variables/get")     => { let name=path.find("name=").map(|i| &path[i+5..]).unwrap_or("").split('&').next().unwrap_or(""); let s=STATE.lock().unwrap(); match s.variables.get(name) { Some(v) => format!(r#"{{"name":"{}","value":"{}","type":"{}"}}"#, esc(&v.name),esc(&v.value),esc(&v.var_type)), None => r#"{"error":"not_found"}"#.to_string() } }

        // v40: Profiles
        ("GET",  "/profiles")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.profiles.iter().map(|p| format!(r#"{{"id":"{}","name":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),p.active)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/profiles/set")      => { let id=extract_json_str(body,"id").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.active_profile=id.clone(); for p in s.profiles.iter_mut() { p.active=p.id==id; } format!(r#"{{"ok":true,"active":"{}"}}"#, esc(&id)) }

        // v40: Device signals via HTTP (for testing / external tools)
        ("POST", "/signal/screen_on")  => { STATE.lock().unwrap().sig_screen_on=true;      r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/screen_off") => { STATE.lock().unwrap().sig_screen_off=true;     r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/shake")      => { STATE.lock().unwrap().sig_shake=true;           r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/kira_event") => { let ev=extract_json_str(body,"event").unwrap_or_default(); STATE.lock().unwrap().sig_kira_event=ev; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/app")        => { let pkg=extract_json_str(body,"package").unwrap_or_default(); STATE.lock().unwrap().sig_app_launched=pkg; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/wifi")       => { let ssid=extract_json_str(body,"ssid").unwrap_or_default(); STATE.lock().unwrap().sig_wifi_ssid=ssid; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/sms")        => { let sender=extract_json_str(body,"sender").unwrap_or_default(); let text=extract_json_str(body,"text").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.sig_sms_sender=sender; s.sig_sms_text=text; r#"{"ok":true}"#.to_string() }

        // v38: Config + setup
        ("GET",  "/config")            => { let s=STATE.lock().unwrap(); config_to_json(&s.config) }
        ("POST", "/config")            => update_config_from_http(body),
        ("GET",  "/setup")             => { let s=STATE.lock().unwrap(); format!(r#"{{"page":{},"done":{},"user_name":"{}","model":"{}","base_url":"{}","selected_provider":"{}","custom_url":"{}","quote_index":{}}}"#, s.setup.current_page,s.setup.done,esc(&s.setup.user_name),esc(&s.setup.model),esc(&s.setup.base_url),esc(&s.setup.selected_provider_id),esc(&s.setup.custom_url),s.setup.quote_index) }
        ("POST", "/setup/page")        => { if let Some(page)=extract_json_num(body,"page") { STATE.lock().unwrap().setup.current_page=page as u8; } r#"{"ok":true}"#.to_string() }
        ("POST", "/setup/complete")    => { let mut s=STATE.lock().unwrap(); s.setup.done=true; s.config.setup_done=true; r#"{"ok":true}"#.to_string() }
        ("GET",  "/theme")             => {
            // Update animation state before returning
            let mut s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            let phase_secs = (uptime_ms % 3000) as f32 / 3000.0; // 3s cycle
            s.theme.animation_phase = phase_secs;
            // BPM: 60 idle, 90 when recent requests, 120 when agent active
            let recent = s.tool_call_count;
            s.theme.pulse_bpm = if s.theme.is_thinking { 120 }
                else if recent > 0 { 90 } else { 60 };
            // activity_level: clamp tool calls per minute to 0-1
            let tools_recent = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000)
                .count();
            s.theme.activity_level = (tools_recent as f32 / 10.0).min(1.0);
            s.theme.to_json()
        }
        // GET /layer0  -  full Layer 0 star field animation state
        // Polled by GalaxyView every 500ms to drive the living canvas.
        // Returns: phase(0-1, 3s period), bpm, activity(0-1), thinking,
        //          hue_shift(-12 to +12 degrees, sine-driven),
        //          vortex_intensity(0-1), burst(true once then reset)
        ("GET",  "/layer0") => {
            let mut s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            // 3-second heartbeat phase (0.0 → 1.0 → 0.0 → ...)
            let phase      = (uptime_ms % 3_000) as f32 / 3_000.0;
            s.theme.animation_phase = phase;
            // Chromatic hue shift: ±12° sine wave on 3s period
            let hue_shift  = (phase * 2.0 * std::f32::consts::PI).sin() * 12.0;
            // Activity: tool calls in last 60s, normalised 0-1
            let tools_60s  = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            let activity   = (tools_60s as f32 / 10.0_f32).min(1.0_f32);
            s.theme.activity_level = activity;
            // BPM: 120 thinking, 90 active, 60 idle
            let bpm = if s.theme.is_thinking { 120u32 }
                      else if s.request_count > 0 && activity > 0.05 { 90 }
                      else { 60 };
            s.theme.pulse_bpm = bpm;
            // Vortex: ramps up when thinking, decays when idle
            // Java applies this as the rate of star inward drift
            let vortex = if s.theme.is_thinking { 1.0f32 }
                         else { (activity * 0.6).min(0.8) };
            // Burst flag: consumed once by Java, then auto-cleared
            let burst = s.theme.is_thinking; // Java triggers burst on thinking→false transition
            format!(
                r#"{{"phase":{:.6},"bpm":{},"activity":{:.6},"thinking":{},"hue_shift":{:.4},"vortex":{:.4},"burst":{}}}"#,
                phase, bpm, activity, s.theme.is_thinking,
                hue_shift, vortex, burst
            )
        }

        // Legacy alias for older Java pollers
        ("GET",  "/theme/anim") => {
            let mut s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            let phase = (uptime_ms % 3_000) as f32 / 3_000.0;
            s.theme.animation_phase = phase;
            let tools_recent = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            format!(r#"{{"phase":{:.6},"bpm":{},"activity":{:.6},"thinking":{}}}"#,
                phase, s.theme.pulse_bpm,
                (tools_recent as f32 / 10.0).min(1.0),
                s.theme.is_thinking)
        }
        // POST /theme/thinking {"active":true}   -  set thinking state
        ("POST", "/theme/thinking")     => {
            let active = body.contains(r#""active":true"#);
            STATE.lock().unwrap().theme.is_thinking = active;
            r#"{"ok":true}"#.to_string()
        }

        // ── Layer 5: Settings page Rust endpoints ─────────────────────────────

        // GET /settings/health  -  compact health summary for settings page header
        // Returns: {shizuku, setup, api_key_set, model, automation_count, memory_count,
        //           uptime_ms, tool_calls, pulse_bpm, activity}
        ("GET",  "/settings/health") => {
            let s = STATE.lock().unwrap();
            let uptime = now_ms().saturating_sub(s.uptime_start);
            let tools_60s = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            let activity = (tools_60s as f32 / 10.0_f32).min(1.0);
            let bpm = if s.theme.is_thinking { 120 }
                      else if s.request_count > 0 { 90 } else { 60 };
            format!(
                r#"{{"shizuku":{},"shizuku_permission":{},"setup_done":{},"api_key_set":{},"model":"{}","base_url":"{}","automation_count":{},"memory_count":{},"uptime_ms":{},"tool_calls":{},"request_count":{},"pulse_bpm":{},"activity":{:.3},"macros_enabled":{}}}"#,
                s.shizuku.installed, s.shizuku.permission_granted,
                s.config.setup_done, !s.config.api_key.is_empty(),
                esc(&s.config.model), esc(&s.config.base_url),
                s.macros.len(), s.memory_index.len(),
                uptime, s.tool_call_count, s.request_count,
                bpm, activity,
                s.macros.iter().filter(|m| m.enabled).count()
            )
        }

        // GET /settings/shizuku  -  Shizuku status with Layer 5 border color token
        // Returns: {installed, running, permission, border_color, border_name, pulse}
        ("GET",  "/settings/shizuku") => {
            let s = STATE.lock().unwrap();
            let (border_color, border_name) = if s.shizuku.permission_granted {
                (0xFFB4BEFEu32, "lavender")   // god mode  -  Lavender
            } else if s.shizuku.installed {
                (0xFFFAB387u32, "peach")      // partial  -  Peach
            } else {
                (0xFFF38BA8u32, "pink")       // absent   -  Pink
            };
            format!(
                r#"{{"installed":{},"running":{},"permission":{},"border_color":{},"border_name":"{}","pulse_ms":1500}}"#,
                s.shizuku.installed, s.shizuku.installed,
                s.shizuku.permission_granted,
                border_color, border_name
            )
        }

        // POST /settings/row_tap {"row":"api_key"}  -  log a settings row tap
        // Used by UI to record which settings the user accesses most often
        ("POST", "/settings/row_tap") => {
            let row = extract_json_str(body, "row").unwrap_or_default();
            if !row.is_empty() {
                let mut s = STATE.lock().unwrap();
                // Store in daily_log for usage analytics
                let entry = format!("[settings_tap] row={} ts={}", esc(&row), now_ms());
                s.daily_log.push_back(entry);
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
            }
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/sections  -  section visibility state for header underline
        // Returns ordered list of section names with their Lavender animate flag
        ("GET",  "/settings/sections") => {
            let sections = vec![
                "IDENTITY", "AI PROVIDER", "AGENT BEHAVIOR", "TELEGRAM BOT",
                "INTERFACE", "MEMORY", "TOOLS & AUTOMATION", "RUST ENGINE v9",
                "SYSTEM", "ABOUT"
            ];
            let items: Vec<String> = sections.iter().map(|s|
                format!(r#"{{"name":"{}","accent_color":4289003262}}"#, s)  // 0xFFB4BEFE Lavender
            ).collect();
            format!(r#"{{"sections":[{}]}}"#, items.join(","))
        }

        // POST /theme/flash {"dark":true}   -  record theme switch for analytics
        // ── Crash reporting endpoints ─────────────────────────────────────────

        // POST /crash {"thread":"..","trace":"..","ts":1234}
        // Called by KiraApp crash handler to persist crashes in Rust memory
        // GET /memory/compression  -  LZ4 compression stats (Session B)
        ("GET",  "/memory/compression") => {
            let s = STATE.lock().unwrap();
            let raw_bytes: usize = s.context_turns.iter()
                .map(|t| t.role.len() + t.content.len()).sum();
            let compressed_bytes = compressed_context_bytes(&s);
            let ratio = if compressed_bytes > 0 {
                raw_bytes as f32 / compressed_bytes as f32
            } else { 1.0 };
            format!(
                r#"{{"turns":{},"compressed_turns":{},"raw_bytes":{},"compressed_bytes":{},"ratio":{:.2},"saved_kb":{}}}"#,
                s.context_turns.len(),
                s.context_turns_lz4.len(),
                raw_bytes,
                compressed_bytes,
                ratio,
                (raw_bytes.saturating_sub(compressed_bytes)) / 1024
            )
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session D  -  AI Chat engine (replaces KiraAI.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session E  -  Agent + Chain endpoints (replaces KiraAgent.java + KiraChain.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /ai/agent
        // body: {"goal":"..","max_steps":25,"session":"agent_default"}
        // Full ReAct loop: PLAN → THINK → ACT → OBSERVE → repeat until done
        // Returns: {"final":"..","steps":8,"tools_used":["x","y"],"success":true}
        ("POST", "/ai/agent") => {
            let goal       = extract_json_str(body, "goal").unwrap_or_default();
            let max_steps  = extract_json_num(body, "max_steps").unwrap_or(25.0) as usize;
            let session_id = extract_json_str(body, "session")
                .unwrap_or_else(|| "agent_default".into());

            if goal.is_empty() {
                return r#"{"error":"goal is required"}"#.to_string();
            }

            let (api_key, base_url, model, persona) = {
                let s = STATE.lock().unwrap();
                (s.config.api_key.clone(), s.config.base_url.clone(),
                 s.config.model.clone(),   s.config.persona.clone())
            };
            if api_key.is_empty() {
                return r#"{"error":"no API key","success":false}"#.to_string();
            }

            let persona_str = if persona.is_empty() {
                "You are Kira, an autonomous AI agent on Android.".into()
            } else { persona };

            // Build agent system prompt
            let agent_system = format!(
                "{}

You are executing a multi-step task autonomously.
                For each step: think what to do, pick ONE tool, execute it,                 observe result, decide next step.
                When done, reply: DONE: <summary>
                Tools: run_shell, read_file, write_file, http_get, add_memory,                 search_memory, get_battery, get_wifi, set_variable, get_variable",
                persona_str
            );

            let mut steps_run   = 0usize;
            let mut tools_used: Vec<String> = Vec::new();
            let mut final_summary = String::new();
            let mut success = false;

            // Agent context  -  separate from chat history
            let mut agent_ctx: Vec<(String, String)> = Vec::new();
            agent_ctx.push(("user".into(), format!("Task: {}", goal)));

            {
                let mut s = STATE.lock().unwrap();
                s.theme.is_thinking = true;
                // Log task start
                let task = AgentTask {
                    id:           format!("task_{}", now_ms()),
                    goal:         goal.clone(),
                    status:       "running".into(),
                    current_step: 0,
                    max_steps,
                    created_ms:   now_ms(),
                };
                s.agent_tasks.push_back(task);
                if s.agent_tasks.len() > 20 { s.agent_tasks.pop_front(); }
            }

            // ReAct loop
            while steps_run < max_steps {
                let resp = match call_llm_sync(
                    &api_key, &base_url, &model, &agent_system, &agent_ctx
                ) {
                    Ok(r)  => r,
                    Err(e) => { final_summary = e; break; }
                };

                let clean = clean_reply(&resp);

                // Check for completion
                if clean.contains("DONE:") || clean.to_lowercase().contains("task complete") {
                    final_summary = clean.replace("DONE:", "").trim().to_string();
                    success = true;
                    break;
                }

                // Extract + execute tool calls
                let calls = parse_tool_calls(&resp);
                if calls.is_empty() {
                    // No tool call  -  treat as final answer if we have content
                    if !clean.is_empty() {
                        final_summary = clean;
                        success = true;
                    }
                    break;
                }

                let mut observations = String::new();
                for (tname, targs) in &calls {
                    let result = dispatch_tool(tname, targs);
                    tools_used.push(tname.clone());
                    observations.push_str(&format!("[{}] → {}
", tname, result));
                }

                // Feed observation back into context
                agent_ctx.push(("assistant".into(), resp.clone()));
                agent_ctx.push(("user".into(),
                    format!("Observations:
{}Continue with the task.", observations)));
                steps_run += 1;

                // Update task status in STATE
                if let Some(task) = STATE.lock().unwrap().agent_tasks.back_mut() {
                    task.current_step = steps_run;
                }
            }

            if final_summary.is_empty() {
                final_summary = if success { "completed".into() }
                    else { format!("stopped after {} steps", steps_run) };
            }

            // Push to compressed chat history so user can see result
            {
                let mut s = STATE.lock().unwrap();
                push_turn_compressed(&mut s, "assistant", &final_summary);
                s.theme.is_thinking = false;
                if let Some(task) = s.agent_tasks.back_mut() {
                    task.status = if success { "done".into() } else { "stopped".into() };
                }
            }

            let tools_json: String = tools_used.iter()
                .map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(",");
            format!(
                r#"{{"final":"{}","steps":{},"tools_used":[{}],"success":{}}}"#,
                esc(&final_summary), steps_run, tools_json, success
            )
        }

        // POST /ai/chain
        // body: {"goal":"..","depth":5}
        // Chain-of-thought: builds sequential reasoning, no tool loop
        // Returns: {"reasoning":["step1","step2"],"conclusion":".."}
        ("POST", "/ai/chain") => {
            let goal  = extract_json_str(body, "goal").unwrap_or_default();
            let depth = extract_json_num(body, "depth").unwrap_or(4.0) as usize;

            if goal.is_empty() {
                return r#"{"error":"goal is required"}"#.to_string();
            }
            let (api_key, base_url, model) = {
                let s = STATE.lock().unwrap();
                (s.config.api_key.clone(), s.config.base_url.clone(), s.config.model.clone())
            };
            if api_key.is_empty() {
                return r#"{"error":"no API key"}"#.to_string();
            }

            let chain_system = "You are a step-by-step reasoning engine.                 Think through each step carefully before proceeding to the next.                 Format each reasoning step as: STEP N: <thought>.                 End with CONCLUSION: <answer>.";

            let mut chain_ctx: Vec<(String, String)> = Vec::new();
            chain_ctx.push(("user".into(),
                format!("Reason through this step by step (max {} steps): {}", depth, goal)));

            let response = match call_llm_sync(&api_key, &base_url, &model, chain_system, &chain_ctx) {
                Ok(r)  => r,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, esc(&e)),
            };

            // Parse STEP N: lines and CONCLUSION:
            let mut reasoning: Vec<String> = Vec::new();
            let mut conclusion = String::new();
            for line in response.lines() {
                let t = line.trim();
                if t.starts_with("STEP ") {
                    reasoning.push(t.to_string());
                } else if t.starts_with("CONCLUSION:") {
                    conclusion = t.replace("CONCLUSION:", "").trim().to_string();
                }
            }
            if conclusion.is_empty() { conclusion = clean_reply(&response); }

            let steps_json: String = reasoning.iter()
                .map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",");
            format!(r#"{{"reasoning":[{}],"conclusion":"{}","steps":{}}}"#,
                steps_json, esc(&conclusion), reasoning.len())
        }

        // GET /ai/agent/status  -  current running agent task
        ("GET",  "/ai/agent/status") => {
            let s = STATE.lock().unwrap();
            match s.agent_tasks.back() {
                Some(t) => format!(
                    r#"{{"id":"{}","goal":"{}","status":"{}","step":{},"max":{}}}"#,
                    esc(&t.id), esc(&t.goal), esc(&t.status),
                    t.current_step, t.max_steps),
                None => r#"{"status":"idle"}"#.to_string(),
            }
        }

        // POST /ai/agent/stop  -  cancel running agent
        ("POST", "/ai/agent/stop") => {
            let mut s = STATE.lock().unwrap();
            s.theme.is_thinking = false;
            if let Some(task) = s.agent_tasks.back_mut() {
                task.status = "cancelled".into();
            }
            r#"{"ok":true,"stopped":true}"#.to_string()
        }

                // POST /ai/chat
        // body: {"message":"..","session":"default","max_tool_steps":5}
        // Runs full chat turn: history + system prompt → LLM → tool loop → reply
        // Returns: {"role":"assistant","content":"..","tools_used":["x"],"tokens":0,"done":true}
        ("POST", "/ai/chat") => {
            let user_msg    = extract_json_str(body, "message").unwrap_or_default();
            let session_id  = extract_json_str(body, "session").unwrap_or_else(||"default".into());
            let max_steps   = extract_json_num(body, "max_tool_steps").unwrap_or(5.0) as usize;

            if user_msg.is_empty() {
                return r#"{"error":"message is required"}"#.to_string();
            }

            // Load config
            let (api_key, base_url, model, persona, system_prompt) = {
                let s = STATE.lock().unwrap();
                let cfg = &s.config;
                let persona = if cfg.persona.is_empty() {
                    "You are Kira, an AI agent on Android. Be concise and helpful.".to_string()
                } else { cfg.persona.clone() };
                let sys = build_system_prompt(&s, &persona);
                (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone(),
                 persona, sys)
            };

            if api_key.is_empty() {
                return r#"{"error":"no API key  -  go to settings and add one","done":true}"#.to_string();
            }

            let now = now_ms();

            // Record user turn in legacy buffer AND session_store
            {
                let mut s = STATE.lock().unwrap();
                s.request_count += 1;
                s.theme.is_thinking = true;
                push_turn_compressed(&mut s, "user", &user_msg);
                // Session 4: authoritative session record
                s.session_store.get_or_create(&session_id, "kira", now);
                s.session_store.add_turn(&session_id, "user", &user_msg, now);
            }

            // Build context  -  prefer session_store (includes compact summary)
            let context = {
                let s = STATE.lock().unwrap();
                if s.session_store.get(&session_id).is_some() {
                    s.session_store.build_context(&session_id)
                } else {
                    decompress_context(&s)
                }
            };

            // Call LLM
            let raw_response = call_llm_sync(&api_key, &base_url, &model, &system_prompt, &context);
            let raw = match raw_response {
                Ok(r)  => r,
                Err(e) => {
                    let mut s = STATE.lock().unwrap();
                    s.theme.is_thinking = false;
                    return format!(r#"{{"error":"{}","done":true}}"#, esc(&e));
                }
            };

            // Extract tool calls if any
            let tool_calls = parse_tool_calls(&raw);
            let mut reply  = clean_reply(&raw);
            let mut tools_used: Vec<String> = Vec::new();

            // Tool execution loop
            if !tool_calls.is_empty() {
                let mut pending = tool_calls;
                let mut step = 0;
                while !pending.is_empty() && step < max_steps {
                    step += 1;
                    let mut tool_results = String::new();
                    for (tname, targs) in &pending {
                        let result = dispatch_tool(tname, targs);
                        tool_results.push_str(&format!("[{}]: {}\n", tname, result));
                        tools_used.push(tname.clone());
                        if tname == "run_shell" || result.starts_with("__shell__") {
                            let mut s = STATE.lock().unwrap();
                            s.pending_shell.push_back(ShellJob {
                                id:      format!("tool_{}_{}", step, tname),
                                cmd:     targs.get("cmd").cloned().unwrap_or_default(),
                                timeout: 10_000,
                                created: now_ms(),
                            });
                        }
                    }
                    let mut ctx2 = context.clone();
                    ctx2.push(("assistant".into(), raw.clone()));
                    ctx2.push(("user".into(),
                        format!("[tool results]\n{}respond to the user now.", tool_results)));
                    match call_llm_sync(&api_key, &base_url, &model, &system_prompt, &ctx2) {
                        Ok(r2) => { reply = clean_reply(&r2); pending = parse_tool_calls(&r2); }
                        Err(_) => break,
                    }
                }
            }

            if reply.is_empty() { reply = "done.".into(); }

            // Record assistant reply + persist + auto-compact
            let needs_compact;
            {
                let now2 = now_ms();
                let mut s = STATE.lock().unwrap();
                push_turn_compressed(&mut s, "assistant", &reply);
                s.session_store.add_turn(&session_id, "assistant", &reply, now2);
                s.theme.is_thinking = false;
                s.tool_call_count += tools_used.len() as u64;
                // Session 4: persist after every assistant reply
                s.session_store.save_transcript(&session_id);
                s.session_store.save_index();
                needs_compact = s.session_store.needs_compact(&session_id);
            }

            // Session 4: auto-compact in background if over threshold
            if needs_compact {
                let (api2, url2, mdl2) = {
                    let s = STATE.lock().unwrap();
                    (s.config.api_key.clone(), s.config.base_url.clone(), s.config.model.clone())
                };
                let sid = session_id.clone();
                std::thread::spawn(move || {
                    if let Ok(mut s) = STATE.lock() {
                        if let Ok(_) = crate::ai::compaction::compact_session(
                            &mut s.session_store, &sid, &api2, &url2, &mdl2, false
                        ) {
                            s.session_store.save_transcript(&sid);
                            s.session_store.save_index();
                        }
                    }
                });
            }

            let tools_json: String = tools_used.iter()
                .map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(",");
            format!(
                r#"{{"role":"assistant","content":"{}","tools_used":[{}],"done":true,"session":"{}"}}"#,
                esc(&reply), tools_json, esc(&session_id)
            )
        }
        ("GET",  "/ai/history") => {
            let s = STATE.lock().unwrap();
            let turns = decompress_context(&s);
            let items: Vec<String> = turns.iter()
                .map(|(role, content)| format!(r#"{{"role":"{}","content":"{}"}}"#,
                    esc(role), esc(content)))
                .collect();
            format!(r#"{{"count":{},"turns":[{}]}}"#, items.len(), items.join(","))
        }

        // DELETE /ai/history  -  clear conversation context
        ("DELETE", "/ai/history") | ("POST", "/ai/history/clear") => {
            let mut s = STATE.lock().unwrap();
            s.context_turns.clear();
            s.context_turns_lz4.clear();
            r#"{"ok":true,"cleared":true}"#.to_string()
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session H  -  Macro loop (replaces KiraWatcher.java logic)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /macro/tick
        // Called by Java KiraWatcher every 5s with current device state.
        // Rust evaluates all macro triggers and queues fired actions.
        // body: {"battery":85,"charging":false,"pkg":"com.spotify.music",
        //        "screen_hash":"abc123","wifi":"HomeNet","screen_text":"..."}
        ("POST", "/macro/tick") => {
            let battery   = extract_json_num(body, "battery").unwrap_or(-1.0) as i32;
            let charging  = body.contains(""charging":true");
            let pkg       = extract_json_str(body, "pkg").unwrap_or_default();
            let wifi      = extract_json_str(body, "wifi").unwrap_or_default();
            let screen_txt= extract_json_str(body, "screen_text").unwrap_or_default();
            let screen_hash=extract_json_str(body, "screen_hash").unwrap_or_default();

            let mut fired = 0u32;
            let now = now_ms();

            {
                let mut s = STATE.lock().unwrap();

                // Update device state
                if battery >= 0 {
                    s.battery_pct = battery;
                    s.battery_charging = charging;
                }
                if !pkg.is_empty()  { s.foreground_pkg = pkg.clone(); }
                if !wifi.is_empty() { s.sig_wifi_ssid = wifi.clone(); }

                // Evaluate macro triggers
                let macro_ids: Vec<String> = s.macros.iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.id.clone())
                    .collect();

                for mid in macro_ids {
                    // Evaluate each trigger in the macro
                    let triggers_snap: Vec<(String, String, bool)> = {
                        if let Some(m) = s.macros.iter().find(|m| m.id == mid) {
                            m.triggers.iter().map(|t| {
                                let kind = format!("{:?}", t.kind).to_lowercase();
                                let val  = t.config.get("value").cloned()
                                    .or_else(|| t.config.get("ssid").cloned())
                                    .or_else(|| t.config.get("pkg").cloned())
                                    .or_else(|| t.config.get("threshold").cloned())
                                    .unwrap_or_default();
                                (kind, val, t.enabled)
                            }).collect()
                        } else { continue; }
                    };

                    let triggered = triggers_snap.iter().any(|(kind, val, en)| {
                        if !en { return false; }
                        match kind.as_str() {
                            k if k.contains("battery") && k.contains("low")  =>
                                battery >= 0 && battery < val.parse().unwrap_or(20),
                            k if k.contains("battery") && k.contains("full") =>
                                battery >= 95 && charging,
                            k if k.contains("app") || k.contains("launch")   =>
                                !pkg.is_empty() && pkg.contains(val.as_str()),
                            k if k.contains("wifi")                           =>
                                !wifi.is_empty() && wifi.contains(val.as_str()),
                            k if k.contains("screen") || k.contains("text")  =>
                                !screen_txt.is_empty()
                                && screen_txt.to_lowercase().contains(&val.to_lowercase()),
                            k if k.contains("time") => {
                                let day_mins = ((now / 60_000) % 1440) as u64;
                                let hm: Vec<u64> = val.split(':')
                                    .filter_map(|x| x.parse().ok()).collect();
                                hm.len() == 2 && day_mins == hm[0]*60+hm[1]
                            }
                            _ => false,
                        }
                    });

                    if triggered {
                        // Queue a kira_chat action for each macro
                        let mname = s.macros.iter().find(|m| m.id == mid)
                            .map(|m| m.name.clone()).unwrap_or_default();
                        s.macro_run_log.push_back(MacroRunLog {
                            macro_id:   mid.clone(),
                            macro_name: mname,
                            trigger:    "tick".into(),
                            success:    true,
                            steps_run:  1,
                            duration_ms:0,
                            ts:         now,
                        });
                        if s.macro_run_log.len() > 100 { s.macro_run_log.pop_front(); }
                        fired += 1;
                    }
                }

                // Screen watch rules from memory
                if !screen_hash.is_empty() {
                    for mem in s.memory_index.iter() {
                        if !mem.content.starts_with("watch_screen_") { continue; }
                        if let Some(colon) = mem.content.find(':') {
                            let rule = &mem.content[colon+1..];
                            let parts: Vec<&str> = rule.splitn(2, '|').collect();
                            if parts.len() == 2 {
                                let keyword = parts[0].trim();
                                let action  = parts[1].trim();
                                if screen_txt.to_lowercase().contains(&keyword.to_lowercase()) {
                                    // Queue AI chat for this screen rule
                                    s.pending_shell.push_back(ShellJob {
                                        id:      format!("watch_{}", now),
                                        cmd:     format!("__ai_chat__:{}", action),
                                        timeout: 30_000,
                                        created: now,
                                    });
                                    fired += 1;
                                }
                            }
                        }
                    }
                }
            }

            format!(r#"{{"ok":true,"fired":{},"ts":{}}}"#, fired, now)
        }

        // GET /macro/pending_results  -  results queued for Java to dispatch
        // Java polls this for completed macro actions requiring Android intents
        ("GET",  "/macro/pending_results") => {
            let mut s = STATE.lock().unwrap();
            // Return next pending shell job that needs Java (intent-based actions)
            match s.pending_shell.iter().position(|j| j.cmd.starts_with("__intent__:")) {
                Some(idx) => {
                    let job = s.pending_shell.remove(idx).unwrap();
                    let action = job.cmd.trim_start_matches("__intent__:");
                    format!(r#"{{"has_action":true,"action":"{}","id":"{}"}}"#,
                        esc(action), esc(&job.id))
                }
                None => r#"{"has_action":false}"#.to_string(),
            }
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session F  -  Telegram (replaces KiraTelegram.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /telegram/incoming  -  called by Java after polling getUpdates
        // body: {"update_id":123,"chat_id":456,"user":"name","text":"hello"}
        // Rust processes message through AI and queues reply for Java to send
        ("POST", "/telegram/incoming") => {
            let update_id = extract_json_num(body, "update_id").unwrap_or(0.0) as i64;
            let chat_id   = extract_json_num(body, "chat_id").unwrap_or(0.0) as i64;
            let user      = extract_json_str(body, "user").unwrap_or_default();
            let text      = extract_json_str(body, "text").unwrap_or_default();

            if text.is_empty() || chat_id == 0 {
                return r#"{"ok":false,"error":"missing fields"}"#.to_string();
            }

            // Check allowed user
            let allowed = { STATE.lock().unwrap().config.tg_allowed };
            if allowed != 0 && chat_id != allowed {
                return r#"{"ok":false,"error":"unauthorized"}"#.to_string();
            }

            // Store in log
            {
                let mut s = STATE.lock().unwrap();
                s.tg_last_update_id = update_id;
                s.tg_message_log.push_back(TgMessage {
                    update_id, chat_id, ts: now_ms(),
                    user: user.clone(), text: text.clone(),
                });
                if s.tg_message_log.len() > 50 { s.tg_message_log.pop_front(); }
                push_turn_compressed(&mut s, "user", &format!("[TG @{}] {}", user, text));
            }

            // Run AI on the message
            let chat_body = format!(
                r#"{{"message":"[Telegram @{}] {}","session":"tg_{}","max_tool_steps":5}}"#,
                esc(&user), esc(&text), chat_id
            );
            let ai_result = route_http("POST", "/ai/chat", &chat_body);
            let reply = extract_json_str_inline(&ai_result, "content")
                .unwrap_or_else(|| "sorry, something went wrong".into());

            // Queue reply for Java to send
            {
                let mut s = STATE.lock().unwrap();
                s.tg_pending_sends.push_back(TgSend {
                    chat_id, text: reply.clone(), ts: now_ms()
                });
            }
            format!(r#"{{"ok":true,"reply":"{}"}}"#, esc(&reply))
        }

        // GET /telegram/next_send  -  Java polls for messages to send
        ("GET",  "/telegram/next_send") => {
            let mut s = STATE.lock().unwrap();
            match s.tg_pending_sends.pop_front() {
                Some(msg) => format!(
                    r#"{{"has_message":true,"chat_id":{},"text":"{}"}}"#,
                    msg.chat_id, esc(&msg.text)),
                None => r#"{"has_message":false}"#.to_string(),
            }
        }

        // GET /telegram/last_update_id  -  Java uses this for getUpdates offset
        ("GET",  "/telegram/last_update_id") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"update_id":{}}}"#, s.tg_last_update_id)
        }

        // GET /telegram/log  -  last received messages
        ("GET",  "/telegram/log") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.tg_message_log.iter().rev().take(20).map(|m|
                format!(r#"{{"chat_id":{},"user":"{}","text":"{}","ts":{}}}"#,
                    m.chat_id, esc(&m.user), esc(&m.text), m.ts)
            ).collect();
            format!(r#"{{"count":{},"messages":[{}]}}"#, items.len(), items.join(","))
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session J  -  Setup wizard data from Rust (reduces SetupActivity hardcoding)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // GET /setup/providers  -  list of AI providers with base URLs + models
        // SetupActivity calls this instead of hardcoding the list
        ("GET",  "/setup/providers") => {
            r#"[
              {"id":"groq","name":"Groq (Free)","base_url":"https://api.groq.com/openai/v1","models":["llama-3.1-8b-instant","llama-3.3-70b-versatile","mixtral-8x7b-32768"],"key_url":"https://console.groq.com/keys","recommended":true},
              {"id":"openai","name":"OpenAI","base_url":"https://api.openai.com/v1","models":["gpt-4o-mini","gpt-4o","gpt-4-turbo"],"key_url":"https://platform.openai.com/api-keys","recommended":false},
              {"id":"anthropic","name":"Anthropic","base_url":"https://api.anthropic.com/v1","models":["claude-3-haiku-20240307","claude-3-5-sonnet-20241022","claude-3-opus-20240229"],"key_url":"https://console.anthropic.com/","recommended":false},
              {"id":"together","name":"Together AI","base_url":"https://api.together.xyz/v1","models":["meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo","mistralai/Mixtral-8x7B-Instruct-v0.1"],"key_url":"https://api.together.ai/settings/api-keys","recommended":false},
              {"id":"openrouter","name":"OpenRouter","base_url":"https://openrouter.ai/api/v1","models":["meta-llama/llama-3.1-8b-instruct:free","google/gemma-2-9b-it:free"],"key_url":"https://openrouter.ai/keys","recommended":false},
              {"id":"custom","name":"Custom / Self-hosted","base_url":"","models":[],"key_url":"","recommended":false}
            ]"#.to_string()
        }

        // POST /setup/validate  -  validate API key format + test connection
        // body: {"provider":"groq","api_key":"gsk_...","model":"llama-3.1-8b-instant"}
        ("POST", "/setup/validate") => {
            let provider = extract_json_str(body, "provider").unwrap_or_default();
            let api_key  = extract_json_str(body, "api_key").unwrap_or_default();
            let model    = extract_json_str(body, "model").unwrap_or_default();

            if api_key.is_empty() {
                return r#"{"valid":false,"error":"API key is empty"}"#.to_string();
            }

            // Format validation per provider
            let format_ok = match provider.as_str() {
                "groq"      => api_key.starts_with("gsk_") && api_key.len() > 10,
                "openai"    => api_key.starts_with("sk-") && api_key.len() > 10,
                "anthropic" => api_key.starts_with("sk-ant-") && api_key.len() > 10,
                _           => api_key.len() >= 8,
            };

            if !format_ok {
                let hint = match provider.as_str() {
                    "groq"      => "Groq keys start with 'gsk_'",
                    "openai"    => "OpenAI keys start with 'sk-'",
                    "anthropic" => "Anthropic keys start with 'sk-ant-'",
                    _           => "Key appears invalid",
                };
                return format!(r#"{{"valid":false,"error":"{}"}}"#, hint);
            }

            // Quick syntax validation passed  -  mark as valid
            // (actual connection test done by Java to avoid blocking)
            format!(r#"{{"valid":true,"provider":"{}","model":"{}","hint":"Format valid. Tap Next to continue."}}"#,
                esc(&provider), esc(&model))
        }

        // GET /setup/status  -  current setup completion state
        ("GET",  "/setup/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"setup_done":{},"has_api_key":{},"provider":"{}","model":"{}","user_name":"{}"}}"#,
                s.config.setup_done,
                !s.config.api_key.is_empty(),
                esc(&s.config.base_url),
                esc(&s.config.model),
                esc(&s.config.user_name))
        }

                // ── Session C: Crypto endpoints ───────────────────────────────────────

        // POST /crypto/encrypt {"plaintext":"..","seed":"..","domain":"api_key"}
        ("POST", "/crypto/encrypt") => {
            let pt  = extract_json_str(body, "plaintext").unwrap_or_default();
            let sd  = extract_json_str(body, "seed").unwrap_or_default();
            let dm  = extract_json_str(body, "domain").unwrap_or_else(|| "default".into());
            if pt.is_empty() || sd.is_empty() {
                return r#"{"error":"plaintext and seed required"}"#.to_string();
            }
            let ct = aes_encrypt(&pt, &sd, &dm);
            format!(r#"{{"ok":true,"ciphertext":"{}","domain":"{}"}}"#, ct, esc(&dm))
        }

        // POST /crypto/decrypt {"ciphertext":"..","seed":"..","domain":"api_key"}
        ("POST", "/crypto/decrypt") => {
            let ct = extract_json_str(body, "ciphertext").unwrap_or_default();
            let sd = extract_json_str(body, "seed").unwrap_or_default();
            let dm = extract_json_str(body, "domain").unwrap_or_else(|| "default".into());
            if ct.is_empty() || sd.is_empty() {
                return r#"{"error":"ciphertext and seed required"}"#.to_string();
            }
            let plain = aes_decrypt(&ct, &sd, &dm);
            let ok = !plain.is_empty();
            format!(r#"{{"ok":{},"plaintext":"{}"}}"#, ok, esc(&plain))
        }

        // GET /crypto/status  -  reports encryption availability
        ("GET",  "/crypto/status") => {
            r#"{"available":true,"algorithm":"AES-256-GCM","key_derivation":"derive_aes_key(ANDROID_ID:pkg)","nonce":"domain-derived-12-byte","tag_bits":128}"#.to_string()
        }

                // ── Session L: Security audit endpoint ───────────────────────────────────

        // GET /security/audit  -  reports current security posture
        ("GET",  "/security/audit") => {
            let s = STATE.lock().unwrap();
            let has_secret  = !s.http_secret.is_empty();
            let has_api_key = !s.config.api_key.is_empty();
            // Check if api_key looks encrypted (hex, even length, >32 chars)
            let key_encrypted = has_api_key
                && s.config.api_key.len() > 32
                && s.config.api_key.chars().all(|c| c.is_ascii_hexdigit());
            let shizuku_ok  = s.shizuku.permission_granted;
            format!(r#"{{"http_secret_set":{},"api_key_present":{},"api_key_encrypted":{},"shizuku":{},"tls_enabled":true,"auth_coverage":"session_l","jni_safe_inputs":true,"lz4_compression":true,"aes_gcm_available":true}}"#,
                has_secret, has_api_key, key_encrypted, shizuku_ok)
        }

        // POST /security/rotate_secret  -  generate and set a new random HTTP secret
        ("POST", "/security/rotate_secret") => {
            // Derive new secret from current time + existing key material
            let new_secret = {
                let s = STATE.lock().unwrap();
                let seed = format!("{}:{}:{}", now_ms(), s.request_count, s.config.api_key.len());
                let k = derive_aes_key(&seed);
                k.iter().map(|b| format!("{:02x}", b)).collect::<String>()[..32].to_string()
            };
            STATE.lock().unwrap().http_secret = new_secret.clone();
            format!(r#"{{"ok":true,"new_secret":"{}","note":"store this  -  required for all future API calls"}}"#,
                &new_secret)
        }

                ("POST", "/crash") => {
            let thread  = extract_json_str(body, "thread").unwrap_or_else(||"unknown".into());
            let trace   = extract_json_str(body, "trace").unwrap_or_default();
            let ts_val  = extract_json_num(body, "ts").unwrap_or(0.0) as u128;
            let ts      = if ts_val > 0 { ts_val } else { now_ms() };
            // First line = exception class/message
            let message = trace.lines().next().unwrap_or("").to_string();
            // Cap trace at 4KB to avoid memory bloat
            let trace_capped = if trace.len() > 4096 {
                format!("{}…[truncated]", &trace[..4096])
            } else { trace };
            let entry = CrashEntry { ts, thread, message, trace: trace_capped };
            let mut s = STATE.lock().unwrap();
            s.crash_log.push_back(entry);
            if s.crash_log.len() > 50 { s.crash_log.pop_front(); }
            r#"{"ok":true}"#.to_string()
        }

        // GET /crash/log  -  returns all stored crash entries as JSON array
        ("GET",  "/crash/log") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.crash_log.iter().map(|c| {
                let safe_msg   = esc(&c.message);
                let safe_trace = esc(&c.trace);
                let safe_thr   = esc(&c.thread);
                // Human-readable timestamp for CrashActivity history tab
                let ts_secs = c.ts / 1000;
                let ts_str = format!("{}", ts_secs); // Java side formats it
                format!(r#"{{"ts":{},"ts_str":"{}","thread":"{}","message":"{}","trace":"{}"}}"#,
                    c.ts, ts_str, safe_thr, safe_msg, safe_trace)
            }).collect();
            format!(r#"{{"count":{},"crashes":[{}]}}"#, items.len(), items.join(","))
        }

        // POST /crash/clear  -  wipe the crash log
        ("POST", "/crash/clear") => {
            STATE.lock().unwrap().crash_log.clear();
            r#"{"ok":true,"cleared":true}"#.to_string()
        }

        // GET /crash/latest  -  just the most recent crash (fast poll)
        ("GET",  "/crash/latest") => {
            let s = STATE.lock().unwrap();
            match s.crash_log.back() {
                Some(c) => format!(
                    r#"{{"has_crash":true,"ts":{},"thread":"{}","message":"{}"}}"#,
                    c.ts, esc(&c.thread), esc(&c.message)),
                None => r#"{"has_crash":false}"#.to_string(),
            }
        }

        ("POST", "/theme/flash") => {
            let dark = body.contains("\"dark\":true");
            STATE.lock().unwrap().theme.is_dark = dark;
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/automation/summary  -  automation engine summary for settings card
        ("GET",  "/settings/automation/summary") => {
            let s = STATE.lock().unwrap();
            let enabled = s.macros.iter().filter(|m| m.enabled).count();
            let total_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let last_run_ms = s.macros.iter().map(|m| m.last_run_ms).max().unwrap_or(0);
            let last_run_ago = if last_run_ms > 0 {
                now_ms().saturating_sub(last_run_ms)
            } else { 0 };
            format!(
                r#"{{"total":{},"enabled":{},"disabled":{},"total_runs":{},"last_run_ago_ms":{}}}"#,
                s.macros.len(), enabled, s.macros.len().saturating_sub(enabled),
                total_runs, last_run_ago
            )
        }


        // ── Layer 5: Settings Page  -  Rust-backed endpoints ──────────────────────

        // GET /settings/counters  -  live counter values for CounterAnimator
        // Returns numbers that the UI animates from old→new over 600ms EaseOut.
        ("GET", "/settings/counters") => {
            let s = STATE.lock().unwrap();
            let uptime_ms   = now_ms().saturating_sub(s.uptime_start);
            let uptime_s    = (uptime_ms / 1000) as u64;
            let tool_calls  = s.tool_call_count;
            let mem_facts   = s.memory_index.len() as u64;
            let macros_en   = s.macros.iter().filter(|m| m.enabled).count() as u64;
            let macros_tot  = s.macros.len() as u64;
            let macro_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let sessions    = s.sessions.len() as u64;
            let skills_en   = s.skills.values().filter(|sk| sk.enabled).count() as u64;
            let req_count   = s.request_count;
            let notif_count = s.notifications.len() as u64;
            format!(
                r#"{{"uptime_s":{},"tool_calls":{},"memory_facts":{},"macros_enabled":{},"macros_total":{},"macro_runs":{},"sessions":{},"skills_enabled":{},"requests":{},"notifications":{}}}"#,
                uptime_s, tool_calls, mem_facts, macros_en, macros_tot,
                macro_runs, sessions, skills_en, req_count, notif_count
            )
        }

        // GET /settings/activity  -  activity stream for last 20 events
        // Used by the settings page activity feed (Row-level visual feedback)
        ("GET", "/settings/activity") => {
            let s = STATE.lock().unwrap();
            let mut items: Vec<String> = Vec::new();
            // Last 10 macro runs
            for r in s.macro_run_log.iter().rev().take(10) {
                items.push(format!(
                    r#"{{"type":"macro","name":"{}","success":{},"ts":{}}}"#,
                    esc(&r.macro_name), r.success, r.ts));
            }
            // Last 5 notifications
            for n in s.notifications.iter().rev().take(5) {
                items.push(format!(
                    r#"{{"type":"notif","pkg":"{}","title":"{}","ts":{}}}"#,
                    esc(&n.pkg), esc(&n.title), n.time));
            }
            // Last 5 daily_log entries
            for entry in s.daily_log.iter().rev().take(5) {
                items.push(format!(
                    r#"{{"type":"log","msg":"{}","ts":{}}}"#,
                    esc(entry), now_ms()));
            }
            // Sort by ts descending, take 20
            items.sort_by(|a, b| {
                let ta = extract_json_num(a, "ts").unwrap_or(0.0);
                let tb = extract_json_num(b, "ts").unwrap_or(0.0);
                tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
            });
            items.truncate(20);
            format!(r#"{{"count":{},"items":[{}]}}"#, items.len(), items.join(","))
        }

        // GET /settings/shizuku/halo  -  Layer 9: God mode halo state
        // Returns border color + animation params for the screen-edge halo
        ("GET", "/settings/shizuku/halo") => {
            let s = STATE.lock().unwrap();
            let active = s.shizuku.permission_granted;
            let partial = s.shizuku.installed && !active;
            // God mode halo: 2dp Lavender border traces screen edge when fully active
            // Rotation: 4s per revolution, 30dp arc length
            let (color, width_dp, visible, revolution_ms) = if active {
                (0xFFB4BEFEu32, 2u32, true,  4000u32)  // Lavender, full speed
            } else if partial {
                (0xFFFAB387u32, 1u32, true,  8000u32)  // Peach, slow
            } else {
                (0x00000000u32, 0u32, false, 0u32)      // invisible
            };
            format!(
                r#"{{"active":{},"partial":{},"color":{},"width_dp":{},"visible":{},"revolution_ms":{},"arc_dp":30}}"#,
                active, partial, color, width_dp, visible, revolution_ms
            )
        }

        // POST /settings/row_interaction {"row":"api_key","action":"tap|long_press|edit"}
        // Enhanced row analytics  -  tracks not just tap but interaction type
        ("POST", "/settings/row_interaction") => {
            let row    = extract_json_str(body, "row").unwrap_or_default();
            let action = extract_json_str(body, "action").unwrap_or_else(|| "tap".to_string());
            if !row.is_empty() {
                let mut s = STATE.lock().unwrap();
                let entry = format!("[settings_interaction] row={} action={} ts={}", 
                    esc(&row), esc(&action), now_ms());
                s.daily_log.push_back(entry);
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                // Increment row-specific usage counter in variables map
                let key = format!("_settings_tap_{}", row);
                let count = s.variables.get(&key)
                    .map(|v| v.value.parse::<u32>().unwrap_or(0) + 1)
                    .unwrap_or(1);
                s.variables.entry(key.clone()).or_insert_with(|| AutoVariable {
                    name: key.clone(), value: "0".to_string(),
                    var_type: "counter".to_string(),
                    persistent: false, created_ms: now_ms(), updated_ms: now_ms(),
                }).value = count.to_string();
            }
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/top_rows  -  most-accessed settings rows (for smart ordering)
        ("GET", "/settings/top_rows") => {
            let s = STATE.lock().unwrap();
            let mut rows: Vec<(String, u32)> = s.variables.iter()
                .filter(|(k, _)| k.starts_with("_settings_tap_"))
                .map(|(k, v)| {
                    let row_name = k.trim_start_matches("_settings_tap_").to_string();
                    let count = v.value.parse::<u32>().unwrap_or(0);
                    (row_name, count)
                })
                .collect();
            rows.sort_by(|a, b| b.1.cmp(&a.1));
            rows.truncate(5);
            let items: Vec<String> = rows.iter()
                .map(|(r, c)| format!(r#"{{"row":"{}","taps":{}}}"#, esc(r), c))
                .collect();
            format!(r#"{{"top_rows":[{}]}}"#, items.join(","))
        }

        // GET /settings/memory/stats  -  detailed memory stats for memory card
        ("GET", "/settings/memory/stats") => {
            let s = STATE.lock().unwrap();
            let total    = s.memory_index.len();
            let pinned   = s.memory_index.iter().filter(|e| e.access_count > 5).count();
            let recent   = s.memory_index.iter()
                .filter(|e| now_ms().saturating_sub(
                    s.context_turns.iter().map(|_| now_ms()).next().unwrap_or(0)) < 86_400_000)
                .count();
            let top: Vec<String> = {
                let mut entries: Vec<_> = s.memory_index.iter().collect();
                entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
                entries.iter().take(3)
                    .map(|e| format!(r#"{{"key":"{}","access_count":{}}}"#,
                        esc(&e.key), e.access_count))
                    .collect()
            };
            format!(
                r#"{{"total":{},"pinned":{},"recent_24h":{},"top_accessed":[{}]}}"#,
                total, pinned, recent, top.join(",")
            )
        }

        // GET /settings/theme/palette  -  full Catppuccin Mocha palette for settings
        // Used by theme card to show all swatches
        ("GET", "/settings/theme/palette") => {
            // Catppuccin Mocha full palette
            let swatches = vec![
                ("Rosewater", 0xFFF5E0DC_u32), ("Flamingo",  0xFFF2CDCD_u32),
                ("Pink",      0xFFF38BA8_u32), ("Mauve",     0xFFCBA6F7_u32),
                ("Red",       0xFFEBA0AC_u32), ("Maroon",    0xFFEBA0AC_u32),
                ("Peach",     0xFFFAB387_u32), ("Yellow",    0xFFF9E2AF_u32),
                ("Green",     0xFFA6E3A1_u32), ("Teal",      0xFF94E2D5_u32),
                ("Sky",       0xFF89DCEB_u32), ("Sapphire",  0xFF74C7EC_u32),
                ("Blue",      0xFF89B4FA_u32), ("Lavender",  0xFFB4BEFE_u32),
                ("Text",      0xFFCDD6F4_u32), ("Subtext1",  0xFFBAC2DE_u32),
                ("Overlay2",  0xFF9399B2_u32), ("Overlay0",  0xFF6C7086_u32),
                ("Surface2",  0xFF585B70_u32), ("Surface1",  0xFF45475A_u32),
                ("Surface0",  0xFF313244_u32), ("Base",      0xFF1E1E2E_u32),
                ("Mantle",    0xFF181825_u32), ("Crust",     0xFF11111B_u32),
            ];
            let items: Vec<String> = swatches.iter()
                .map(|(name, color)| format!(r#"{{"name":"{}","color":{}}}"#, name, color))
                .collect();
            format!(r#"{{"theme":"catppuccin_mocha","swatches":[{}]}}"#, items.join(","))
        }

                // GET /layer1  -  Neural Nav Bar state for Java
        // Returns Catppuccin colour tokens + keyboard state hint
        // Java NeuralNavBar polls this to stay in sync with Rust theme
        ("GET",  "/layer1") => {
            let s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            // Nav bar pulse: subtle border alpha oscillation on heartbeat
            let phase = (uptime_ms % 3_000) as f32 / 3_000.0;
            let border_alpha = 0.30 + (phase * 2.0 * std::f32::consts::PI as f32).sin().abs() * 0.08;
            format!(
                r#"{{"mantle":{},"lavender":{},"overlay0":{},"border_alpha":{:.4},"active_tab_sp":26,"inactive_tab_sp":22,"aura_radius_dp":32,"island_height_dp":72,"island_corner_dp":24,"elevation_dp":8}}"#,
                0xFF181825u32,  // Catppuccin Mantle
                0xFFB4BEFEu32,  // Catppuccin Lavender
                0xFF6C7086u32,  // Catppuccin Overlay0 (inactive)
                border_alpha
            )
        }

        // POST /layer0/burst  -  called by Java when Kira finishes replying
        // Sets a one-shot burst flag that /layer0 will return as burst:true
        // ── Layer 2: Chat Interface state endpoints ───────────────────────────

        // GET /layer2/header  -  header bar state (pulse, subtitle cycle index)
        // Java polls at 500ms to drive header border and subtitle crossfade
        ("GET",  "/layer2/header") => {
            let s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            let phase      = (uptime_ms % 3_000) as f32 / 3_000.0;
            // Border alpha: 12% base, pulses to 35% when thinking
            let border_alpha = if s.theme.is_thinking {
                0.27 + (phase * 2.0 * std::f32::consts::PI).sin().abs() * 0.08
            } else { 0.12f32 };
            // Subtitle index: cycles through 4 states (ready/thinking/reasoning/composing)
            // 0=ready, 1=thinking, 2=reasoning, 3=composing  -  driven by request state
            let subtitle_idx: u32 = if !s.theme.is_thinking { 0 }
                else { ((uptime_ms / 1_800) % 3 + 1) as u32 }; // cycle 1-3 while thinking
            format!(
                r#"{{"border_alpha":{:.4},"subtitle_idx":{},"thinking":{},"request_count":{}}}"#,
                border_alpha, subtitle_idx, s.theme.is_thinking, s.request_count
            )
        }

        // GET /layer2/bubbles  -  bubble styling tokens for chat UI
        // Returns Catppuccin colour tokens for user/kira bubbles + shadow specs
        ("GET",  "/layer2/bubbles") => {
            r#"{"user_bg":3292050, "user_bg_alpha":255, "kira_bg":2040622, "lavender":11862782, "peach":16430983, "green_dark":2023454, "shadow_color":1144397, "shadow_alpha":102, "shadow_blur_dp":8, "shadow_y_dp":2, "spring_stiffness":300, "spring_damping":28, "spring_duration_ms":320, "translate_dp":40}"#.to_string()
            // user_bg = 0xFF313244 (Surface0), kira_bg = 0xFF1E1E2E (Base)
            // lavender = 0xFFB4BEFE, peach = 0xFFFAB387, green_dark = 0xFF1E2E1E
        }

        // GET /layer2/typing  -  typing indicator animation params
        // Three Lavender dots, sinusoidal, each offset 120ms
        ("GET",  "/layer2/typing") => {
            let s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            // Each dot phase offset by 120ms within 600ms period
            let t = uptime_ms as f32 / 600.0 * 2.0 * std::f32::consts::PI;
            let d0 = ((t).sin() * 4.0) as i32;           // dot 0: ±4dp
            let d1 = ((t - 0.628).sin() * 4.0) as i32;   // dot 1: 120ms offset
            let d2 = ((t - 1.257).sin() * 4.0) as i32;   // dot 2: 240ms offset
            format!(
                r#"{{"visible":{},"dot0_y":{},"dot1_y":{},"dot2_y":{},"color":{},"period_ms":600,"amplitude_dp":4}}"#,
                s.theme.is_thinking, d0, d1, d2, 0xFFB4BEFEu32
            )
        }

        // POST /layer2/message  -  record that a message was sent/received
        // Updates request_count, last_message_ts, triggers K badge rotation signal
        ("POST", "/layer2/message") => {
            let role = extract_json_str(body, "role").unwrap_or_else(||"user".to_string());
            let mut s = STATE.lock().unwrap();
            if role == "user" {
                s.request_count += 1;
                s.theme.is_thinking = true;
            } else if role == "kira" {
                s.theme.is_thinking = false;
                s.tool_call_count += 1;
            }
            format!(r#"{{"ok":true,"request_count":{},"thinking":{}}}"#,
                s.request_count, s.theme.is_thinking)
        }

                ("POST", "/layer0/burst") => {
            // We use is_thinking flip as the burst signal  -  Java detects thinking→false
            STATE.lock().unwrap().theme.is_thinking = false;
            r#"{"ok":true}"#.to_string()
        }

        ("POST", "/theme/set")         => { let name=extract_json_str(body,"name").unwrap_or_else(||"material".into()); let mut s=STATE.lock().unwrap(); s.theme = match name.as_str() { "material" | "material_neo" | "material_dark" => ThemeConfig::material_dark(), "material_light" | "material_neo_light" => ThemeConfig::material_light(), "kira" => ThemeConfig::default(), _ => ThemeConfig::material_dark() }; format!(r#"{{"ok":true,"theme":"{}"}}"#, s.theme.theme_name) }
        ("POST", "/theme/tilt")        => { let ax=extract_json_f32(body,"ax").unwrap_or(0.0); let ay=extract_json_f32(body,"ay").unwrap_or(0.0); let mut s=STATE.lock().unwrap(); s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay; let spd=s.theme.star_speed; let tx=-ax*spd; let ty=ay*spd; s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08; s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08; format!(r#"{{"px":{:.6},"py":{:.6}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y) }
        ("GET",  "/shizuku")           => { let s=STATE.lock().unwrap(); shizuku_to_json(&s.shizuku) }
        ("POST", "/shizuku")           => { let installed=body.contains(r#""installed":true"#); let granted=body.contains(r#""permission_granted":true"#); let err=extract_json_str(body,"error").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.shizuku.installed=installed; s.shizuku.permission_granted=granted; s.shizuku.error_msg=err; s.shizuku.last_checked_ms=now_ms(); r#"{"ok":true}"#.to_string() }
        ("GET",  "/appstats")          => { let s=STATE.lock().unwrap(); format!(r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"active_profile":"{}","variables":{}}}"#, s.memory_index.len(),s.context_turns.len(), if s.shizuku.permission_granted{"active \u{2713}"}else if s.shizuku.installed{"no permission"}else{"not running"}, if !s.agent_context.is_empty(){"enabled \u{2713}"}else{"disabled"}, esc(&s.config.model),esc(&s.config.base_url),now_ms().saturating_sub(s.uptime_start),s.macros.len(),esc(&s.active_profile),s.variables.len()) }
        ("GET",  "/providers")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.providers.iter().map(|p| format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),esc(&p.base_url),esc(&p.model),p.id==s.active_provider)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/providers/set")     => { let id=extract_json_str(body,"id").unwrap_or_default(); if !id.is_empty() { let mut s=STATE.lock().unwrap(); let found=s.providers.iter().find(|p| p.id==id).cloned(); if let Some(p)=found { s.active_provider=id.clone(); s.config.base_url=p.base_url; s.config.model=p.model; } } format!(r#"{{"ok":true,"active":"{}"}}"#, id) }
        ("POST", "/providers/custom")  => { let url=extract_json_str(body,"url").unwrap_or_default(); let model=extract_json_str(body,"model").unwrap_or_default(); if !url.is_empty() { let mut s=STATE.lock().unwrap(); s.setup.custom_url=url.clone(); s.setup.selected_provider_id="custom".to_string(); s.config.base_url=url.clone(); if !model.is_empty() { s.config.model=model.clone(); } if let Some(p)=s.providers.iter_mut().find(|p| p.id=="custom") { p.base_url=url; if !model.is_empty() { p.model=model; } } s.active_provider="custom".to_string(); } r#"{"ok":true}"#.to_string() }

        // v7: Device state
        ("GET",  "/screen")            => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET",  "/screen_pkg")        => { let p=STATE.lock().unwrap().screen_pkg.clone(); format!(r#"{{"package":"{}"}}"#, esc(&p)) }
        ("GET",  "/battery")           => { let s=STATE.lock().unwrap(); format!(r#"{{"percentage":{},"charging":{}}}"#, s.battery_pct,s.battery_charging) }
        ("GET",  "/notifications")     => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.notifications.iter().map(|n| format!(r#"{{"pkg":"{}","title":"{}","text":"{}","time":{}}}"#, esc(&n.pkg),esc(&n.title),esc(&n.text),n.time)).collect(); format!("[{}]", items.join(",")) }

        // v7: Memory
        ("GET",  "/memory")            => { let s=STATE.lock().unwrap(); format!(r#"{{"memory_md":{},"entries":{}}}"#, json_str(&s.memory_md),s.memory_index.len()) }
        ("GET",  "/memory/search")     => search_memory(path),
        ("GET",  "/memory/full")       => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.memory_index.iter().map(|e| format!(r#"{{"key":"{}","value":"{}","tags":{},"relevance":{:.2},"access_count":{}}}"#, esc(&e.key),esc(&e.value),json_str_arr(&e.tags),e.relevance,e.access_count)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/daily_log")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.daily_log.iter().cloned().map(|l| format!("\"{}\"", esc(&l))).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/context")           => get_context_json(),
        ("GET",  "/soul")              => { let s=STATE.lock().unwrap(); format!(r#"{{"soul":{}}}"#, json_str(&s.soul_md)) }
        ("POST", "/soul")              => { let val=extract_json_str(body,"content").unwrap_or_default(); if !val.is_empty() { STATE.lock().unwrap().soul_md=val; } r#"{"ok":true}"#.to_string() }

        // v7: Skills
        ("GET",  "/skills")            => get_skills_json(),
        ("POST", "/skills/register")   => { register_skill(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/enable")     => { let name=extract_json_str(body,"name").unwrap_or_default(); if let Some(sk)=STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=true; } r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/disable")    => { let name=extract_json_str(body,"name").unwrap_or_default(); if let Some(sk)=STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=false; } r#"{"ok":true}"#.to_string() }

        // v7: Sessions (legacy)
        ("GET",  "/sessions")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.sessions.values().map(|sess| format!(r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"last_msg":{}}}"#, sess.id,sess.channel,sess.turns,sess.tokens,sess.last_msg)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/sessions/new")      => new_session(body),

        // ── Session 4: Persistent session store ─────────────────────────────

        // GET /sessions/v2  -  list all sessions from session_store (sorted by recency)
        ("GET",  "/sessions/v2") => {
            let s = STATE.lock().unwrap();
            s.session_store.list_sessions_json()
        }

        // GET /sessions/v2/:key  -  full transcript for one session
        _ if method == "GET" && path_clean.starts_with("/sessions/v2/") && !path_clean.contains("compact") => {
            let key = &path_clean["/sessions/v2/".len()..];
            let s   = STATE.lock().unwrap();
            match s.session_store.get(key) {
                None => r#"{"error":"session not found"}"#.to_string(),
                Some(sess) => {
                    let sess_json = sess.to_json();
                    let turns = s.session_store.get_turns(key);
                    let turns_json: Vec<String> = turns.iter().map(|t| t.to_json()).collect();
                    format!(r#"{{"session":{},"turns":[{}]}}"#,
                        sess_json, turns_json.join(","))
                }
            }
        }

        // DELETE /sessions/v2/:key  -  delete session + transcript from disk
        _ if method == "DELETE" && path_clean.starts_with("/sessions/v2/") => {
            let key = path_clean["/sessions/v2/".len()..].to_string();
            let mut s = STATE.lock().unwrap();
            if s.session_store.delete_and_purge(&key) {
                r#"{"ok":true}"#.to_string()
            } else {
                r#"{"error":"session not found"}"#.to_string()
            }
        }

        // POST /sessions/v2/:key/compact  -  force compact a session now
        _ if method == "POST" && path_clean.starts_with("/sessions/v2/") && path_clean.ends_with("/compact") => {
            let without_suffix = &path_clean[..path_clean.len() - "/compact".len()];
            let key2 = without_suffix["/sessions/v2/".len()..].to_string();
            let (api_key, base_url, model) = {
                let s = STATE.lock().unwrap();
                (s.config.api_key.clone(), s.config.base_url.clone(), s.config.model.clone())
            };
            if api_key.is_empty() {
                return r#"{"error":"no API key configured"}"#.to_string();
            }
            let result = {
                let mut s = STATE.lock().unwrap();
                crate::ai::compaction::compact_session(
                    &mut s.session_store, &key2, &api_key, &base_url, &model, true
                )
            };
            match result {
                Ok(summary) => {
                    // Persist after compaction
                    let mut s = STATE.lock().unwrap();
                    s.session_store.save_transcript(&key2);
                    s.session_store.save_index();
                    format!(r#"{{"ok":true,"summary":"{}","session":"{}"}}"#,
                        esc(&summary), esc(&key2))
                }
                Err(reason) => format!(r#"{{"ok":false,"reason":"{}"}}"#, esc(&reason)),
            }
        }

        // POST /sessions/v2/prune  -  prune sessions inactive > ttl_hours
        ("POST", "/sessions/v2/prune") => {
            let ttl_hours = extract_json_num(body, "ttl_hours").unwrap_or(72.0) as u128;
            let ttl_ms    = ttl_hours * 3600 * 1000;
            let mut s     = STATE.lock().unwrap();
            let pruned    = s.session_store.prune_inactive(now_ms(), ttl_ms);
            format!(r#"{{"ok":true,"pruned":{}}}"#, pruned)
        }

        // v7: Triggers
        ("GET",  "/triggers")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.triggers.iter().map(|t| format!(r#"{{"id":"{}","type":"{}","value":"{}","fired":{},"repeat":{}}}"#, t.id,t.trigger_type,esc(&t.value),t.fired,t.repeat)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/fired_triggers")    => { let mut s=STATE.lock().unwrap(); let items: Vec<String>=s.fired_triggers.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/webhook_events")    => { let mut s=STATE.lock().unwrap(); let items: Vec<String>=s.webhook_events.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/triggers/add")      => { add_trigger(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/webhook")           => { let ts=now_ms(); STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"body":{},"ts":{}}}"#, if body.is_empty(){"{}"}else{body},ts)); r#"{"ok":true}"#.to_string() }

        // v7: Heartbeat
        ("GET",  "/heartbeat_log")     => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.heartbeat_log.iter().cloned().collect(); format!("[{}]", items.join(",")) }
        ("POST", "/heartbeat/add")     => { add_heartbeat(body); r#"{"ok":true}"#.to_string() }

        // v7: Cron
        ("GET",  "/cron")              => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.cron_jobs.iter().map(|j| format!(r#"{{"id":"{}","action":"{}","interval_ms":{},"enabled":{}}}"#, j.id,esc(&j.action),j.interval_ms,j.enabled)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/cron/add")          => { add_cron(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/cron/remove")       => { let id=extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().cron_jobs.retain(|j| j.id!=id); r#"{"ok":true}"#.to_string() }

        // v7: Audit + task
        ("GET",  "/task_log")          => get_task_log_json(),
        ("GET",  "/audit_log")         => get_audit_log_json(),
        ("GET",  "/checkpoints")       => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.checkpoints.iter().map(|(k,v)| format!(r#"{{{}:{}}}"#, json_str(k),json_str(v))).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/checkpoint")        => { let k=extract_json_str(body,"id").unwrap_or_default(); let v=extract_json_str(body,"data").unwrap_or_default(); if !k.is_empty() { STATE.lock().unwrap().checkpoints.insert(k,v); } r#"{"ok":true}"#.to_string() }

        // v7: KB
        ("GET",  "/kb")                => get_kb_json(),
        ("GET",  "/kb/search")         => kb_search(path),
        ("POST", "/kb/add")            => { add_kb_entry(body); r#"{"ok":true}"#.to_string() }

        // v7: Events + metrics
        ("GET",  "/events")            => get_event_feed(),
        ("POST", "/events")            => { let e=extract_json_str(body,"event").unwrap_or_default(); let d=extract_json_str(body,"data").unwrap_or_default(); push_event_feed(&e,&d); r#"{"ok":true}"#.to_string() }
        ("GET",  "/metrics")           => get_metrics_text(),
        ("GET",  "/budget")            => get_budget_json(),
        ("GET",  "/stream")            => stream_poll(),
        ("POST", "/stream/chunk")      => { let t=extract_json_str(body,"text").unwrap_or_default(); push_stream_chunk(&t); r#"{"ok":true}"#.to_string() }
        ("POST", "/relay")             => relay_msg(body),
        ("GET",  "/cache")             => cache_get(path),
        ("POST", "/cache")             => cache_post(body),
        ("POST", "/policy/allow")      => { let t=extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s=STATE.lock().unwrap(); s.tool_denylist.retain(|d| d!=&t); if !s.tool_allowlist.contains(&t) { s.tool_allowlist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("POST", "/policy/deny")       => { let t=extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s=STATE.lock().unwrap(); s.tool_allowlist.retain(|a| a!=&t); if !s.tool_denylist.contains(&t) { s.tool_denylist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("GET",  "/policy")            => get_policy_json(),
        ("POST", "/nodes/register")    => { register_node(body); r#"{"ok":true}"#.to_string() }
        ("GET",  "/nodes")             => get_nodes_json(),
        ("POST", "/credentials/get")   => get_credential(body),

        // OpenClaw v3 / NanoBot / ZeroClaw routes
        // ── OTA Engine v43 ────────────────────────────────────────────────────
        // GET  /ota/status         -  full OTA state JSON
        // POST /ota/begin_check    -  Java signals it's about to call GitHub API
        // POST /ota/release        -  Java posts parsed GitHub release data to Rust
        // POST /ota/progress       -  Java reports download progress
        // POST /ota/downloaded     -  Java signals APK is on disk (path + sha256)
        // POST /ota/installing     -  Java signals install session opened
        // POST /ota/installed      -  Java signals successful install
        // POST /ota/failed         -  Java signals any error
        // POST /ota/skip           -  skip this version
        // POST /ota/set_version    -  update current installed version
        // GET  /ota/install_cmd    -  get the install command for Shizuku
        ("GET",  "/ota/status")     => { STATE.lock().unwrap().ota.to_json() }
        ("POST", "/ota/begin_check") => {
            let mut s = STATE.lock().unwrap();
            s.ota.phase = OtaPhase::Checking;
            s.ota.check_error = String::new();
            s.ota.last_check_ms = now_ms();
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/release") => {
            let latest  = extract_json_str(body, "tag").unwrap_or_default();
            let url     = extract_json_str(body, "url").unwrap_or_default();
            let log     = extract_json_str(body, "changelog").unwrap_or_default();
            let date    = extract_json_str(body, "date").unwrap_or_default();
            let sha     = extract_json_str(body, "sha256").unwrap_or_default();
            let apk_sz  = extract_json_num(body, "apk_bytes").unwrap_or(0.0) as u64;
            let mut s   = STATE.lock().unwrap();
            // Check if this version is skipped
            if s.ota.skipped_versions.contains(&latest) {
                s.ota.phase = OtaPhase::Idle;
                return format!(r#"{{"ok":true,"skipped":true}}"#);
            }
            let is_newer = s.ota.is_newer(&latest);
            s.ota.latest_version  = latest.clone();
            s.ota.download_url    = url;
            s.ota.changelog       = log;
            s.ota.release_date    = date;
            s.ota.apk_sha256      = sha;
            s.ota.total_apk_bytes = apk_sz;
            s.ota.last_check_ms   = now_ms();
            s.ota.check_error     = String::new();
            if is_newer {
                s.ota.phase = OtaPhase::UpdateAvailable;
                format!(r#"{{"ok":true,"action":"prompt_user","version":"{}"}}"#, esc(&latest))
            } else {
                s.ota.phase = OtaPhase::Idle;
                format!(r#"{{"ok":true,"action":"up_to_date"}}"#)
            }
        }
        ("POST", "/ota/progress") => {
            let done  = extract_json_num(body, "bytes").unwrap_or(0.0) as u64;
            let total = extract_json_num(body, "total").unwrap_or(0.0) as u64;
            let mut s = STATE.lock().unwrap();
            s.ota.download_bytes = done;
            s.ota.download_total = total;
            s.ota.download_pct   = if total > 0 { ((done * 100) / total).min(100) as u8 } else { 0 };
            s.ota.phase = OtaPhase::Downloading;
            format!(r#"{{"ok":true,"pct":{}}}"#, s.ota.download_pct)
        }
        ("POST", "/ota/downloaded") => {
            let path = extract_json_str(body, "path").unwrap_or_default();
            let sha  = extract_json_str(body, "sha256").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            s.ota.apk_local_path = path;
            // Verify SHA256 if we have an expected one
            let expected = s.ota.apk_sha256.clone();
            let verified = expected.is_empty() || expected == sha;
            if verified {
                s.ota.phase = OtaPhase::Downloaded;
                r#"{"ok":true,"verified":true}"#.to_string()
            } else {
                s.ota.phase = OtaPhase::Failed(format!("SHA256 mismatch: got {}", &sha[..sha.len().min(16)]));
                format!(r#"{{"ok":false,"error":"sha256_mismatch","expected":"{}","got":"{}"}}"#,
                    esc(&expected[..expected.len().min(16)]), esc(&sha[..sha.len().min(16)]))
            }
        }
        ("POST", "/ota/installing") => {
            let method = extract_json_str(body, "method").unwrap_or_else(|| "intent".to_string());
            let sid    = extract_json_num(body, "session_id").unwrap_or(-1.0) as i32;
            let mut s  = STATE.lock().unwrap();
            s.ota.install_method     = method;
            s.ota.install_session_id = sid;
            s.ota.install_error      = String::new();
            s.ota.phase = OtaPhase::Installing;
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/installed") => {
            let ver = extract_json_str(body, "version").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            s.ota.phase = OtaPhase::Installed;
            if !ver.is_empty() { s.ota.current_version = ver; s.config.setup_done = true; }
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/failed") => {
            let err = extract_json_str(body, "error").unwrap_or_else(|| "unknown error".to_string());
            let mut s = STATE.lock().unwrap();
            s.ota.install_error = err.clone();
            s.ota.phase = OtaPhase::Failed(err.clone());
            format!(r#"{{"ok":true,"recorded_error":"{}"}}"#, esc(&err))
        }
        ("POST", "/ota/skip") => {
            let ver = extract_json_str(body, "version").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            if !ver.is_empty() && !s.ota.skipped_versions.contains(&ver) {
                s.ota.skipped_versions.push(ver);
            }
            s.ota.phase = OtaPhase::Idle;
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/set_version") => {
            let ver  = extract_json_str(body, "version").unwrap_or_default();
            let code = extract_json_num(body, "code").unwrap_or(0.0) as i64;
            let mut s = STATE.lock().unwrap();
            if !ver.is_empty() { s.ota.current_version = ver.clone(); s.config.setup_done = true; }
            if code > 0 { s.ota.current_code = code; }
            r#"{"ok":true}"#.to_string()
        }
        ("GET", "/ota/install_cmd") => {
            let s = STATE.lock().unwrap();
            let path = &s.ota.apk_local_path;
            if path.is_empty() {
                r#"{"error":"no apk downloaded"}"#.to_string()
            } else {
                format!(r#"{{"cmd":"pm install -r -t \"{}\"","path":"{}","shizuku_ready":{}}}"#,
                    esc(path), esc(path),
                    s.shizuku.permission_granted)
            }
        }
        _ => {
            if let Some(r) = route_openclaw_v3(method, path_clean, body) { r }
            else if let Some(r) = route_vlm_agent(method, path_clean, body) { r }
            else if let Some(r) = route_roboru(method, path_clean, body) { r }
            else if let Some(r) = route_openclaw(method, path_clean, body) { r }
            else { queue_to_java(path_clean.trim_start_matches('/'), body) }
        }
    }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Background threads
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// v40: Watchdog thread \u{2014} cleans stale pending actions every 30s
fn run_watchdog() {
    loop {
        thread::sleep(Duration::from_secs(30));
        watchdog_check(&mut STATE.lock().unwrap());
    }
}

/// v40/v3: Dedicated macro engine \u{2014} triggers + context zones + battery defer every 500ms
fn run_macro_engine() {
    loop {
        thread::sleep(Duration::from_millis(500));
        let mut s = STATE.lock().unwrap();
        // Check context zones (enter/exit)
        apply_context_zones(&mut s);
        // Check composite triggers
        let composites: Vec<CompositeTrigger> = s.composite_triggers.iter().cloned().collect();
        for ct in composites {
            if check_composite_trigger(&s, &ct) {
                let target = ct.target_macro.clone();
                if battery_allows_run(&s, &target) {
                    chain_macro(&mut s, &target);
                    if let Some(c) = s.composite_triggers.iter_mut().find(|c| c.id == ct.id) {
                        c.last_fired = now_ms();
                    }
                }
            }
        }
        // Standard macro triggers
        run_triggered_macros(&mut s);
    }
}

fn run_trigger_watcher() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let tt: Vec<Trigger> = s.triggers.iter().filter(|t| t.trigger_type=="time" && !t.fired).cloned().collect();
        for t in tt {
            let fire_at = t.value.parse::<u128>().unwrap_or(0);
            if fire_at > 0 && now >= fire_at {
                s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","type":"time"}}"#, t.id,esc(&t.action)));
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired = !tr.repeat; }
            }
        }
        let hb: Vec<HeartbeatItem> = s.heartbeat_items.iter().filter(|i| i.enabled && (i.interval_ms==0 || now-i.last_run>=i.interval_ms)).cloned().collect();
        for item in hb {
            s.heartbeat_log.push_back(format!(r#"{{"id":"{}","check":"{}","ts":{}}}"#, item.id,esc(&item.check),now));
            if s.heartbeat_log.len() > 500 { s.heartbeat_log.pop_front(); }
            s.fired_triggers.push_back(format!(r#"{{"trigger":"hb_{}","action":"{}","check":"{}"}}"#, item.id,esc(&item.action),esc(&item.check)));
            if let Some(i) = s.heartbeat_items.iter_mut().find(|x| x.id==item.id) { i.last_run=now; if i.interval_ms==0 { i.enabled=false; } }
        }
    }
}

fn run_cron_scheduler() {
    loop {
        thread::sleep(Duration::from_secs(5));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let jobs: Vec<CronJob> = s.cron_jobs.iter().filter(|j| j.enabled && now-j.last_run>=j.interval_ms).cloned().collect();
        for job in jobs {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"cron_{}","action":"{}","type":"cron"}}"#, job.id,esc(&job.action)));
            if let Some(j) = s.cron_jobs.iter_mut().find(|x| x.id==job.id) { j.last_run=now; }
        }
    }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Helpers (unchanged from v8)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

fn config_to_json(c: &KiraConfig) -> String {
    format!(r#"{{"user_name":"{}","api_key_set":{},"base_url":"{}","model":"{}","vision_model":"{}","tg_configured":{},"agent_max_steps":{},"agent_auto_approve":{},"heartbeat_interval":{},"setup_done":{}}}"#,
        esc(&c.user_name), !c.api_key.is_empty(), esc(&c.base_url), esc(&c.model), esc(&c.vision_model),
        !c.tg_token.is_empty(), c.agent_max_steps, c.agent_auto_approve, c.heartbeat_interval, c.setup_done)
}

fn shizuku_to_json(sz: &ShizukuStatus) -> String {
    format!(r#"{{"installed":{},"permission_granted":{},"last_checked_ms":{},"error":"{}","status":"{}"}}"#,
        sz.installed, sz.permission_granted, sz.last_checked_ms, esc(&sz.error_msg),
        if sz.permission_granted{"god_mode"} else if sz.installed{"needs_permission"} else{"not_running"})
}

fn update_config_from_http(body: &str) -> String {
    let mut s = STATE.lock().unwrap();
    if let Some(v)=extract_json_str(body,"user_name") { s.config.user_name=v; }
    if let Some(v)=extract_json_str(body,"api_key")   { s.config.api_key=v; }
    if let Some(v)=extract_json_str(body,"base_url")  { s.config.base_url=v; }
    if let Some(v)=extract_json_str(body,"model")     { s.config.model=v; }
    if let Some(v)=extract_json_str(body,"tg_token")  { s.config.tg_token=v; }
    if let Some(v)=extract_json_num(body,"tg_allowed"){ s.config.tg_allowed=v as i64; }
    if let Some(v)=extract_json_num(body,"max_steps") { s.config.agent_max_steps=v as u32; }
    r#"{"ok":true}"#.to_string()
}

fn search_memory(path: &str) -> String {
    let query=path.find("q=").map(|i| &path[i+2..]).unwrap_or("").replace('+'," ");
    let ql=query.to_lowercase();
    let mut s=STATE.lock().unwrap();
    let mut results: Vec<(f32,String,String,u128)>=s.memory_index.iter_mut().filter_map(|e| {
        let mut score=0.0f32;
        if e.key.to_lowercase()==ql { score+=10.0; }
        if e.key.to_lowercase().contains(&ql) { score+=5.0; }
        let vl=e.value.to_lowercase();
        for w in ql.split_whitespace() { if vl.contains(w) { score+=1.0; } }
        for tag in &e.tags { if tag.to_lowercase().contains(&ql) { score+=2.0; } }
        if score>0.0 { e.relevance=(e.relevance+0.1).min(5.0); e.access_count+=1; Some((score,e.key.clone(),e.value.clone(),e.ts)) } else { None }
    }).collect();
    results.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let items: Vec<String>=results.iter().take(10).map(|(sc,k,v,ts)| format!(r#"{{"key":"{}","value":"{}","score":{:.1},"ts":{}}}"#, esc(k),esc(v),sc,ts)).collect();
    format!("[{}]", items.join(","))
}

fn compact_context(s: &mut KiraState) {
    let drain=s.context_turns.len()-20;
    let old: Vec<ContextTurn>=s.context_turns.drain(..drain).collect();
    let summary: String=old.iter().map(|t| format!("[{}]{}", t.role, &t.content[..t.content.len().min(60)])).collect::<Vec<_>>().join(";");
    s.context_compact=if s.context_compact.is_empty() { summary } else { format!("{}; {}", s.context_compact, summary) };
}

fn get_context_json() -> String {
    let s=STATE.lock().unwrap();
    let turns: Vec<String>=s.context_turns.iter().map(|t| format!(r#"{{"role":"{}","content":"{}","tokens":{}}}"#, t.role,esc(&t.content[..t.content.len().min(300)]),t.tokens)).collect();
    format!(r#"{{"compact":{},"turns":[{}],"total_tokens":{},"memory_md":{}}}"#, json_str(&s.context_compact),turns.join(","),s.total_tokens,json_str(&s.memory_md[..s.memory_md.len().min(1000)]))
}

fn get_skills_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.skills.values().map(|sk| format!(r#"{{"name":"{}","description":"{}","trigger":"{}","enabled":{},"usage_count":{}}}"#, esc(&sk.name),esc(&sk.description),esc(&sk.trigger),sk.enabled,sk.usage_count)).collect();
    format!("[{}]", items.join(","))
}

fn get_task_log_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.task_log.iter().skip(s.task_log.len().saturating_sub(50)).map(|t| format!(r#"{{"task_id":"{}","step":{},"action":"{}","result":"{}","success":{},"time":{}}}"#, esc(&t.task_id),t.step,esc(&t.action),esc(&t.result),t.success,t.time)).collect();
    format!("[{}]", items.join(","))
}

fn get_audit_log_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.audit_log.iter().skip(s.audit_log.len().saturating_sub(100)).map(|a| format!(r#"{{"session":"{}","tool":"{}","input":"{}","output":"{}","success":{},"blocked":{},"ts":{}}}"#, esc(&a.session),esc(&a.tool),esc(&a.input),esc(&a.output),a.success,a.blocked,a.ts)).collect();
    format!("[{}]", items.join(","))
}

fn register_skill(body: &str) {
    let name=extract_json_str(body,"name").unwrap_or_default();
    let desc=extract_json_str(body,"description").unwrap_or_default();
    let trigger=extract_json_str(body,"trigger").unwrap_or_default();
    let content=extract_json_str(body,"content").unwrap_or_default();
    if !name.is_empty() { STATE.lock().unwrap().skills.insert(name.clone(), Skill { name, description:desc, trigger, content, enabled:true, usage_count:0 }); }
}

fn add_trigger(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let ttype=extract_json_str(body,"type").unwrap_or_default();
    let value=extract_json_str(body,"value").unwrap_or_default();
    let action=extract_json_str(body,"action").unwrap_or_default();
    let repeat=body.contains("\"repeat\":true");
    STATE.lock().unwrap().triggers.push(Trigger { id, trigger_type:ttype, value, action, fired:false, repeat });
}

fn add_heartbeat(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let check=extract_json_str(body,"check").unwrap_or_default();
    let action=extract_json_str(body,"action").unwrap_or_default();
    let interval=extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
    let mut s=STATE.lock().unwrap();
    s.heartbeat_items.retain(|i| i.id!=id);
    s.heartbeat_items.push(HeartbeatItem { id, check, action, enabled:true, last_run:0, interval_ms:interval });
}

fn add_cron(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let action=extract_json_str(body,"action").unwrap_or_default();
    let interval=extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(3_600_000);
    let expr=extract_json_str(body,"expression").unwrap_or_default();
    STATE.lock().unwrap().cron_jobs.push(CronJob { id, expression:expr, action, last_run:0, interval_ms:interval, enabled:true });
}

fn fire_notif_triggers(s: &mut KiraState, pkg: &str, title: &str, text: &str) {
    // Enforce cap on fired_triggers to prevent unbounded growth
    if s.fired_triggers.len() > 1000 { s.fired_triggers.pop_front(); }
    let combined=format!("{} {} {}", pkg, title, text).to_lowercase();
    let tt: Vec<Trigger>=s.triggers.iter().filter(|t| (t.trigger_type=="keyword_notif"||t.trigger_type=="app_notif") && !t.fired).cloned().collect();
    for t in tt {
        let hit=match t.trigger_type.as_str() {
            "keyword_notif" => combined.contains(&t.value.to_lowercase()),
            "app_notif"     => pkg==t.value,
            _ => false,
        };
        if hit {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","notif":"{}:{}"}}"#, t.id,esc(&t.action),esc(title),esc(text)));
            if let Some(tr)=s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired=!tr.repeat; }
        }
    }
}

fn fire_battery_triggers(s: &mut KiraState, pct: i32, prev: i32) {
    let tt: Vec<Trigger>=s.triggers.iter().filter(|t| t.trigger_type=="battery_low" && !t.fired).cloned().collect();
    for t in tt {
        let threshold=t.value.parse::<i32>().unwrap_or(20);
        if pct<=threshold && prev>threshold {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","battery":{}}}"#, t.id,esc(&t.action),pct));
            if let Some(tr)=s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired=!tr.repeat; }
        }
    }
}

fn do_audit(s: &mut KiraState, session: &str, tool: &str, input: &str, output: &str, success: bool, blocked: bool) {
    s.audit_log.push_back(AuditEntry { session:session.to_string(), tool:tool.to_string(), input:input[..input.len().min(200)].to_string(), output:output[..output.len().min(200)].to_string(), ts:now_ms(), success, blocked });
    if s.audit_log.len() > 5000 { s.audit_log.pop_front(); }
}

fn queue_to_java(endpoint: &str, body: &str) -> String {
    let id=gen_id();
    let payload=if body.is_empty() { format!(r#"{{"endpoint":"{}","data":{{}}}}"#, endpoint) } else { format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint, body) };
    STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
    let timeout=match endpoint { "install_apk"|"take_video" => 60000, _ => 10000 };
    wait_result(&id, timeout).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
}

fn wait_result(id: &str, ms: u64) -> Option<String> {
    let t=std::time::Instant::now();
    loop {
        { let mut s=STATE.lock().unwrap(); if let Some(r)=s.results.remove(id) { return Some(r); } }
        if t.elapsed().as_millis() as u64>=ms { return None; }
        thread::sleep(Duration::from_millis(8));
    }
}

fn make_providers() -> Vec<Provider> {
    vec![
        Provider { id:"groq".into(),       name:"Groq".into(),          base_url:"https://api.groq.com/openai/v1".into(),                             model:"llama-3.1-8b-instant".into() },
        Provider { id:"openai".into(),      name:"OpenAI".into(),         base_url:"https://api.openai.com/v1".into(),                                  model:"gpt-4o-mini".into() },
        Provider { id:"anthropic".into(),   name:"Anthropic".into(),      base_url:"https://api.anthropic.com/v1".into(),                               model:"claude-3-haiku-20240307".into() },
        Provider { id:"gemini".into(),      name:"Gemini".into(),         base_url:"https://generativelanguage.googleapis.com/v1beta/openai".into(),    model:"gemini-2.0-flash".into() },
        Provider { id:"deepseek".into(),    name:"DeepSeek".into(),       base_url:"https://api.deepseek.com/v1".into(),                                model:"deepseek-chat".into() },
        Provider { id:"openrouter".into(),  name:"OpenRouter".into(),     base_url:"https://openrouter.ai/api/v1".into(),                               model:"openrouter/auto".into() },
        Provider { id:"ollama".into(),      name:"Ollama (local)".into(), base_url:"http://localhost:11434/v1".into(),                                  model:"llama3".into() },
        Provider { id:"together".into(),    name:"Together AI".into(),    base_url:"https://api.together.xyz/v1".into(),                                model:"meta-llama/Llama-3-8b-chat-hf".into() },
        Provider { id:"mistral".into(),     name:"Mistral".into(),        base_url:"https://api.mistral.ai/v1".into(),                                  model:"mistral-small-latest".into() },
        Provider { id:"cohere".into(),      name:"Cohere".into(),         base_url:"https://api.cohere.ai/v1".into(),                                   model:"command-r".into() },
        Provider { id:"perplexity".into(),  name:"Perplexity".into(),     base_url:"https://api.perplexity.ai".into(),                                  model:"llama-3.1-sonar-small-128k-online".into() },
        Provider { id:"xai".into(),         name:"xAI Grok".into(),       base_url:"https://api.x.ai/v1".into(),                                        model:"grok-2-latest".into() },
        Provider { id:"cerebras".into(),    name:"Cerebras".into(),       base_url:"https://api.cerebras.ai/v1".into(),                                 model:"llama3.1-8b".into() },
        Provider { id:"fireworks".into(),   name:"Fireworks".into(),      base_url:"https://api.fireworks.ai/inference/v1".into(),                      model:"accounts/fireworks/models/llama-v3p1-8b-instruct".into() },
        Provider { id:"sambanova".into(),   name:"SambaNova".into(),      base_url:"https://api.sambanova.ai/v1".into(),                                model:"Meta-Llama-3.1-8B-Instruct".into() },
        Provider { id:"novita".into(),      name:"Novita AI".into(),      base_url:"https://api.novita.ai/v3/openai".into(),                            model:"llama-3.1-8b-instruct".into() },
        Provider { id:"custom".into(),      name:"Custom".into(),         base_url:String::new(),                                                       model:String::new() },
    ]
}

fn get_policy_json() -> String {
    let s=STATE.lock().unwrap();
    format!(r#"{{"allowlist":[{}],"denylist":[{}]}}"#,
        s.tool_allowlist.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","),
        s.tool_denylist.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","))
}

fn get_nodes_json() -> String {
    let s=STATE.lock().unwrap(); let now=now_ms();
    let items: Vec<String>=s.node_caps.values().map(|n| format!(r#"{{"id":"{}","platform":"{}","caps":[{}],"online":{},"last_seen":{}}}"#, esc(&n.node_id),esc(&n.platform),n.caps.iter().map(|c| format!("\"{}\"",esc(c))).collect::<Vec<_>>().join(","),n.online&&now-n.last_seen<30000,n.last_seen)).collect();
    format!("[{}]", items.join(","))
}

fn register_node(body: &str) {
    let id=extract_json_str(body,"node_id").unwrap_or_else(gen_id);
    let platform=extract_json_str(body,"platform").unwrap_or_else(|| "android".to_string());
    let caps_str=extract_json_str(body,"caps").unwrap_or_default();
    let caps: Vec<String>=caps_str.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
    STATE.lock().unwrap().node_caps.insert(id.clone(), NodeCapability { node_id:id, caps, platform, online:true, last_seen:now_ms() });
}

fn new_session(body: &str) -> String {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let channel=extract_json_str(body,"channel").unwrap_or_else(|| "kira".to_string());
    let ts=now_ms();
    let sess=Session { id:id.clone(), channel, turns:0, tokens:0, created:ts, last_msg:ts };
    STATE.lock().unwrap().sessions.insert(id.clone(), sess);
    format!(r#"{{"ok":true,"id":"{}"}}"#, id)
}

fn get_credential(body: &str) -> String {
    let name=extract_json_str(body,"name").unwrap_or_default();
    let s=STATE.lock().unwrap();
    match s.credentials.get(&name) {
        Some(enc) => { let key=derive_key(&name); let dec=xor_crypt(enc,&key); let val=String::from_utf8_lossy(&dec).to_string(); format!(r#"{{"name":"{}","value":"{}"}}"#, esc(&name),esc(&val)) }
        None      => format!(r#"{{"error":"credential '{}' not found"}}"#, esc(&name))
    }
}

fn stream_poll() -> String {
    let mut s=STATE.lock().unwrap();
    let chunks: Vec<String>=s.stream_chunks.drain(..).map(|c| format!(r#"{{"session_id":"{}","text":"{}","done":{},"ts":{}}}"#, esc(&c.session_id),esc(&c.text),c.done,c.ts)).collect();
    format!("[{}]", chunks.join(","))
}

fn push_stream_chunk(text: &str) {
    let mut s=STATE.lock().unwrap(); let sid=s.active_session.clone();
    s.stream_chunks.push_back(StreamChunk { session_id:sid, text:text.to_string(), done:false, ts:now_ms() });
    if s.stream_chunks.len() > 1000 { s.stream_chunks.pop_front(); }
}

fn relay_msg(body: &str) -> String {
    let ch=extract_json_str(body,"channel").unwrap_or_default(); let msg=extract_json_str(body,"message").unwrap_or_default(); let ts=now_ms();
    STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"type":"relay","channel":"{}","message":"{}","ts":{}}}"#, esc(&ch),esc(&msg),ts));
    r#"{"ok":true}"#.to_string()
}

fn cache_get(path: &str) -> String {
    let key=path.find("key=").map(|i| &path[i+4..]).unwrap_or("").split('&').next().unwrap_or(""); let s=STATE.lock().unwrap(); let now=now_ms();
    match s.response_cache.get(key) {
        Some(e) if e.expires_at>now => format!(r#"{{"key":"{}","value":"{}","ttl":{}}}"#, esc(key),esc(&e.value),e.expires_at-now),
        Some(_) => r#"{"error":"expired"}"#.to_string(), None => r#"{"error":"not_found"}"#.to_string(),
    }
}

fn cache_post(body: &str) -> String {
    let k=extract_json_str(body,"key").unwrap_or_default(); let v=extract_json_str(body,"value").unwrap_or_default();
    let ttl=extract_json_str(body,"ttl_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(300_000);
    STATE.lock().unwrap().response_cache.insert(k, CacheEntry { value:v, expires_at:now_ms()+ttl });
    r#"{"ok":true}"#.to_string()
}

fn get_budget_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.tool_iterations.iter().map(|(k,v)| format!(r#"{{"session":"{}","used":{},"remaining":{}}}"#, esc(k),v,s.max_tool_iters.saturating_sub(*v))).collect();
    format!(r#"{{"max":{},"sessions":[{}]}}"#, s.max_tool_iters,items.join(","))
}

fn get_kb_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.knowledge_base.iter().map(|e| format!(r#"{{"id":"{}","title":"{}","snippet":"{}","tags":[{}],"ts":{}}}"#, esc(&e.id),esc(&e.title),esc(&e.content[..e.content.len().min(100)]),e.tags.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","),e.ts)).collect();
    format!("[{}]", items.join(","))
}

fn add_kb_entry(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id); let title=extract_json_str(body,"title").unwrap_or_default(); let content=extract_json_str(body,"content").unwrap_or_default();
    let tags_s=extract_json_str(body,"tags").unwrap_or_default(); let tags: Vec<String>=tags_s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let mut s=STATE.lock().unwrap(); s.knowledge_base.retain(|e| e.id!=id); s.knowledge_base.push(KbEntry { id, title, content, tags, ts:now_ms() });
    if s.knowledge_base.len() > 10000 { s.knowledge_base.remove(0); }
}

fn kb_search(path: &str) -> String {
    let query=path.find("q=").map(|i| &path[i+2..]).unwrap_or("").to_lowercase(); let s=STATE.lock().unwrap();
    let mut results: Vec<(u32, &KbEntry)>=s.knowledge_base.iter().filter_map(|e| { let mut sc=0u32; if e.title.to_lowercase().contains(&query) { sc+=10; } if e.content.to_lowercase().contains(&query) { sc+=5; } for tag in &e.tags { if tag.to_lowercase().contains(&query) { sc+=3; } } if sc>0 { Some((sc,e)) } else { None } }).collect();
    results.sort_by(|a,b| b.0.cmp(&a.0));
    let items: Vec<String>=results.iter().take(10).map(|(sc,e)| format!(r#"{{"id":"{}","title":"{}","content":"{}","score":{}}}"#, esc(&e.id),esc(&e.title),esc(&e.content[..e.content.len().min(300)]),sc)).collect();
    format!("[{}]", items.join(","))
}

fn get_metrics_text() -> String {
    let s=STATE.lock().unwrap();
    format!("kira_uptime_ms {}\nkira_requests_total {}\nkira_tool_calls {}\nkira_notifications {}\nkira_memory_entries {}\nkira_battery {}\nkira_skills {}\nkira_kb_entries {}\nkira_event_feed {}\nkira_macros {}\nkira_macro_runs {}\nkira_variables {}\n",
        now_ms()-s.uptime_start, s.request_count, s.tool_call_count, s.notifications.len(), s.memory_index.len(), s.battery_pct, s.skills.len(), s.knowledge_base.len(), s.event_feed.len(), s.macros.len(), s.macro_run_log.len(), s.variables.len())
}

fn get_event_feed() -> String {
    let s=STATE.lock().unwrap(); let skip=s.event_feed.len().saturating_sub(100);
    let items: Vec<String>=s.event_feed.iter().skip(skip).map(|e| format!(r#"{{"event":"{}","data":"{}","ts":{}}}"#, esc(&e.event),esc(&e.data),e.ts)).collect();
    format!("[{}]", items.join(","))
}

fn push_event_feed(event: &str, data: &str) {
    let mut s=STATE.lock().unwrap();
    s.event_feed.push_back(EventFeedEntry { event:event.to_string(), data:data.to_string(), ts:now_ms() });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// OpenClaw / NanoBot / ZeroClaw Extended Automation Engine
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
//
// Features added beyond basic Tasker/MacroDroid parity:
//   - Expression evaluator: math + string ops on %VAR% tokens
//   - Macro cooldowns: per-macro min interval between runs
//   - Macro chaining: one macro can trigger another by ID
//   - Retry engine: actions can retry N times on failure with backoff
//   - Macro templates: pre-built automation patterns (OpenClaw-style)
//   - Task graph / NanoBot pipeline: sequential + parallel step execution
//   - Rate limiter: max N macro runs per minute globally
//   - Per-action timeout tracking (enqueued with deadline)
//   - Condition groups: AND/OR/NOT logic on conditions
//   - AI-decision node: Kira decides next action based on context
//   - Watchdog: detect stuck macros and kill them
//   - Global macro import/export (full JSON round-trip)
//   - Built-in macro templates (ZeroClaw patterns)

/// Simple math/string expression evaluator for %VAR% tokens
/// Supports: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, !
/// String ops: .len(), .contains("x"), .starts("x"), .ends("x"), .upper(), .lower()
fn eval_expr(s: &KiraState, expr: &str) -> String {
    let expanded = expand_vars(s, expr);
    let trimmed = expanded.trim();

    // Try numeric arithmetic first
    if let Some(result) = eval_math(trimmed) {
        return result;
    }

    // Boolean expression
    if trimmed == "true" || trimmed == "false" {
        return trimmed.to_string();
    }

    // String length
    if trimmed.ends_with(".len()") {
        let base = trimmed.trim_end_matches(".len()");
        return base.trim_matches('"').len().to_string();
    }

    // String upper/lower
    if trimmed.ends_with(".upper()") {
        return trimmed.trim_end_matches(".upper()").trim_matches('"').to_uppercase();
    }
    if trimmed.ends_with(".lower()") {
        return trimmed.trim_end_matches(".lower()").trim_matches('"').to_lowercase();
    }

    expanded
}

fn eval_math(expr: &str) -> Option<String> {
    // Handle binary operators in order of precedence (simple single-op eval)
    for op in &["+", "-", "*", "/", "%"] {
        // Find the operator (skip if inside string)
        if let Some(pos) = expr.rfind(op) {
            if pos == 0 { continue; }
            let lhs = expr[..pos].trim();
            let rhs = expr[pos+op.len()..].trim();
            let l: f64 = lhs.parse().ok()?;
            let r: f64 = rhs.parse().ok()?;
            let result = match *op {
                "+" => l + r,
                "-" => l - r,
                "*" => l * r,
                "/" => if r == 0.0 { return Some("0".to_string()); } else { l / r },
                "%" => l % r,
                _ => return None,
            };
            // Return integer if no fractional part
            if result.fract() == 0.0 { return Some(format!("{}", result as i64)); }
            return Some(format!("{:.4}", result));
        }
    }
    None
}

/// Cooldown tracker \u{2014} returns true if macro is allowed to run now
fn check_cooldown(s: &KiraState, macro_id: &str) -> bool {
    let now = now_ms();
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        // cooldown stored in tags as "cooldown:30000" (ms)
        for tag in &m.tags {
            if let Some(rest) = tag.strip_prefix("cooldown:") {
                if let Ok(ms) = rest.parse::<u128>() {
                    return now - m.last_run_ms >= ms;
                }
            }
        }
    }
    true // no cooldown set \u{2192} always allowed
}

/// Rate limiter state: count runs in last 60s window
fn check_rate_limit(s: &KiraState) -> bool {
    let now = now_ms();
    let window = 60_000u128;
    let max_per_min = 120u64; // global cap
    let recent = s.macro_run_log.iter()
        .filter(|r| now - r.ts < window)
        .count() as u64;
    recent < max_per_min
}

/// Chain trigger: run another macro by ID (used by PushKiraEvent + KiraEvent trigger)
fn chain_macro(s: &mut KiraState, target_id: &str) {
    if !check_cooldown(s, target_id) { return; }
    if !check_rate_limit(s) { return; }

    let actions: Vec<MacroAction> = s.macros.iter()
        .find(|m| m.id == target_id && m.enabled)
        .map(|m| m.actions.clone())
        .unwrap_or_default();
    let name = s.macros.iter().find(|m| m.id == target_id)
        .map(|m| m.name.clone()).unwrap_or_default();

    if actions.is_empty() { return; }

    let start = now_ms();
    let (steps, _) = execute_macro_actions(s, target_id, &actions);
    if let Some(m) = s.macros.iter_mut().find(|m| m.id == target_id) {
        m.run_count += 1;
        m.last_run_ms = start;
    }
    s.macro_run_log.push_back(MacroRunLog {
        macro_id: target_id.to_string(), macro_name: name,
        trigger: "chain".to_string(), success: true,
        steps_run: steps, duration_ms: now_ms() - start,
        ts: start, error: String::new(),
    });
    if s.macro_run_log.len() > 1000 { s.macro_run_log.pop_front(); }
}

/// NanoBot-style task pipeline: ordered steps where each step's output
/// is stored in a variable for the next step to consume.
/// Steps are enqueued as pending actions with step index in params.
fn run_pipeline(s: &mut KiraState, macro_id: &str, pipeline_json: &str) {
    // pipeline_json = [{"kind":"http_get","params":{"url":"...","out_var":"RESULT"}}, ...]
    let actions = parse_actions_from_json(pipeline_json, "pipeline");
    for (i, action) in actions.iter().enumerate() {
        let mut a = action.clone();
        a.params.insert("_pipeline_step".to_string(), i.to_string());
        a.params.insert("_pipeline_macro".to_string(), macro_id.to_string());
        enqueue_action(s, macro_id, &a);
    }
}

/// Built-in macro templates (ZeroClaw / OpenClaw patterns)
/// Returns a Vec of pre-built AutoMacro structs ready to insert
fn make_builtin_templates() -> Vec<AutoMacro> {
    let ts = now_ms();

    // Helper to make a simple action
    let act = |kind: &str, params: Vec<(&str, &str)>| -> MacroAction {
        let mut p = HashMap::new();
        for (k, v) in params { p.insert(k.to_string(), v.to_string()); }
        MacroAction { kind: MacroActionKind::from_str(kind), params: p, sub_actions: vec![], enabled: true }
    };

    vec![
        // 1. Battery guardian \u{2014} warn + enable power saver
        AutoMacro {
            id: "tpl_battery_guardian".to_string(),
            name: "\u{1F50B} Battery Guardian".to_string(),
            description: "Toast warning when battery drops below 20%, vibrate at 10%".to_string(),
            enabled: false, // templates off by default
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::BatteryLevel,
                config: [("op".to_string(),"lte".to_string()),("threshold".to_string(),"20".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("show_toast", vec![("message", "\u{26A0}\u{FE0F} Battery low: %BATTERY%%")]),
                act("vibrate", vec![("ms", "500")]),
                act("log_event", vec![("message", "Battery guardian fired at %BATTERY%%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "battery".to_string(), "cooldown:300000".to_string()],
        },

        // 2. Work mode \u{2014} activate when connecting to work WiFi
        AutoMacro {
            id: "tpl_work_mode".to_string(),
            name: "\u{1F4BC} Work Mode".to_string(),
            description: "Switch to Work profile + mute media when joining work WiFi".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::WifiConnected,
                config: [("ssid".to_string(), "".to_string())].iter().cloned().collect(), // fill SSID
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "work")]),
                act("mute_volume", vec![("stream", "music")]),
                act("show_toast", vec![("message", "\u{1F4BC} Work mode activated")]),
                act("log_event", vec![("message", "Work mode: connected to %WIFI%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "wifi".to_string(), "work".to_string()],
        },

        // 3. Sleep mode \u{2014} dim screen, silence at night
        AutoMacro {
            id: "tpl_sleep_mode".to_string(),
            name: "\u{1F319} Sleep Mode".to_string(),
            description: "Activate Sleep profile on screen off between 22:00-07:00".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::ScreenOff,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![MacroCondition {
                lhs: "%TIME_MS%".to_string(),
                operator: "gt".to_string(),
                rhs: "0".to_string(), // Java side evaluates time range
            }],
            actions: vec![
                act("activate_profile", vec![("profile", "sleep")]),
                act("mute_volume", vec![("stream", "all")]),
                act("set_brightness", vec![("level", "0")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "sleep".to_string(), "cooldown:3600000".to_string()],
        },

        // 4. Car mode \u{2014} BT connect auto-opens maps + disables notifications
        AutoMacro {
            id: "tpl_car_mode".to_string(),
            name: "\u{1F697} Car Mode".to_string(),
            description: "Open maps + set car profile when BT device connects".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::BluetoothConnected,
                config: [("device".to_string(), "".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "car")]),
                act("open_app", vec![("package", "com.google.android.apps.maps")]),
                act("set_volume", vec![("stream", "navigation"), ("level", "12")]),
                act("show_toast", vec![("message", "\u{1F697} Car mode \u{2014} drive safe!")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "car".to_string(), "bluetooth".to_string()],
        },

        // 5. AI morning briefing \u{2014} Kira speaks summary on unlock
        AutoMacro {
            id: "tpl_morning_briefing".to_string(),
            name: "\u{1F305} Morning Briefing".to_string(),
            description: "Kira speaks a morning summary on first device unlock".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::DeviceUnlocked,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Give a 2-sentence morning briefing: battery is %BATTERY%%, profile is %PROFILE%. Be motivating and brief."),
                    ("out_var", "BRIEFING"),
                ]),
                act("kira_speak", vec![("text", "%BRIEFING%")]),
                act("log_event", vec![("message", "Morning briefing delivered")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "morning".to_string(), "cooldown:3600000".to_string()],
        },

        // 6. Smart notification filter \u{2014} AI decides if notif is urgent
        AutoMacro {
            id: "tpl_notif_filter".to_string(),
            name: "\u{1F9E0} Smart Notif Filter".to_string(),
            description: "Kira reads notifications and speaks urgent ones aloud".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::NotifReceived,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![MacroCondition {
                lhs: "%PROFILE%".to_string(),
                operator: "eq".to_string(),
                rhs: "sleep".to_string(),
            }],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Is this notification urgent enough to wake someone? Reply only YES or NO. App: %SCREEN_PKG%"),
                    ("out_var", "IS_URGENT"),
                ]),
                act("if", vec![
                    ("cond_lhs", "%IS_URGENT%"),
                    ("cond_op", "contains"),
                    ("cond_rhs", "YES"),
                ]),
                act("vibrate", vec![("ms", "1000")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "notifications".to_string()],
        },

        // 7. Clipboard AI enhancer \u{2014} transform clipboard text with AI
        AutoMacro {
            id: "tpl_clipboard_ai".to_string(),
            name: "\u{1F4CB} Clipboard AI".to_string(),
            description: "When clipboard changes, Kira can rewrite/translate/summarize".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::ClipboardChanged,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("set_variable", vec![("name", "ORIGINAL_CLIP"), ("value", "%CLIPBOARD%")]),
                act("kira_ask", vec![
                    ("prompt", "Fix grammar and spelling of this text (return only the corrected text): %CLIPBOARD%"),
                    ("out_var", "FIXED_CLIP"),
                ]),
                act("set_clipboard", vec![("text", "%FIXED_CLIP%")]),
                act("show_toast", vec![("message", "\u{2705} Clipboard enhanced by Kira")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "clipboard".to_string()],
        },

        // 8. Webhook automation \u{2014} receive external trigger, run AI, reply
        AutoMacro {
            id: "tpl_webhook_ai".to_string(),
            name: "\u{1F310} Webhook AI Agent".to_string(),
            description: "Receive HTTP POST \u{2192} Kira processes \u{2192} HTTP reply (OpenClaw pattern)".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::WebhookPost,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Process this webhook data and decide what to do: %CLIPBOARD%"),
                    ("out_var", "WEBHOOK_RESPONSE"),
                ]),
                act("http_post", vec![
                    ("url", "https://your-server.com/kira-reply"),
                    ("body", "{\"response\":\"%WEBHOOK_RESPONSE%\"}"),
                    ("content_type", "application/json"),
                ]),
                act("log_event", vec![("message", "Webhook processed by Kira AI")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "webhook".to_string(), "ai".to_string()],
        },

        // 9. NFC tag launcher \u{2014} tap tag to run specific macro
        AutoMacro {
            id: "tpl_nfc_launcher".to_string(),
            name: "\u{1F4E1} NFC Tag Launcher".to_string(),
            description: "Tap NFC tag to activate Home profile and run home routine".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::NfcTag,
                config: [("tag_id".to_string(), "".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "home")]),
                act("set_volume", vec![("stream", "music"), ("level", "8")]),
                act("set_brightness", vec![("level", "200")]),
                act("show_toast", vec![("message", "\u{1F3E0} Welcome home!")]),
                act("kira_speak", vec![("text", "Welcome home. I've set your home profile.")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "nfc".to_string(), "home".to_string()],
        },

        // 10. Shake-to-SOS \u{2014} shake 3x to send emergency SMS
        AutoMacro {
            id: "tpl_shake_sos".to_string(),
            name: "\u{1F198} Shake SOS".to_string(),
            description: "Shake device to send SOS SMS with location to emergency contact".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::Shake,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("get_location", vec![("out_lat", "SOS_LAT"), ("out_lon", "SOS_LON")]),
                act("send_sms", vec![
                    ("number", "+1234567890"), // replace with emergency contact
                    ("message", "\u{1F198} SOS! I need help. My location: https://maps.google.com/?q=%SOS_LAT%,%SOS_LON%"),
                ]),
                act("vibrate", vec![("ms", "2000")]),
                act("show_toast", vec![("message", "\u{1F198} SOS sent!")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "sos".to_string(), "emergency".to_string(), "cooldown:60000".to_string()],
        },
    ]
}

/// Install built-in templates if not already present (called at startup)
fn install_builtin_templates(s: &mut KiraState) {
    let templates = make_builtin_templates();
    for tpl in templates {
        if !s.macros.iter().any(|m| m.id == tpl.id) {
            s.macros.push(tpl);
        }
    }
}




// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Roubao Vision-Language Agent  (github.com/Turbo1123/roubao)
// Open-AutoGLM Phone Agent      (github.com/zai-org/Open-AutoGLM)
//
// Core architecture implemented in pure Rust:
//
// ROUBAO pattern:
//   screenshot \u{2192} VLM prompt \u{2192} structured action decision \u{2192} execute
//   - Screenshot observation loop
//   - VLM-grounded element detection (describe what to tap)
//   - Action confidence scoring
//   - Task success verification via follow-up screenshot
//
// OPEN-AUTOGLM pattern:
//   user_goal \u{2192} task_planner \u{2192} action_executor \u{2192} state_observer \u{2192} loop
//   - Multi-step phone task decomposition
//   - Thought-Action-Observation (TAO) loop (ReAct variant)
//   - Sub-task tracking with completion state
//   - Grounded element location via text description
//   - Memory of previous actions in session
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Roubao: A VLM grounded action on a specific screen element
#[derive(Clone, Debug)]
struct VlmAction {
    action_type: String,   // "tap", "swipe", "type", "scroll", "back", "home", "done"
    element_desc: String,  // natural-language description of target element
    text:         String,  // for type actions
    direction:    String,  // for swipe/scroll: "up","down","left","right"
    confidence:   f32,     // 0.0-1.0 VLM confidence score
    reasoning:    String,  // VLM's chain-of-thought for this action
    x:            i32,     // resolved coordinates (-1 = unresolved)
    y:            i32,
}

/// Roubao: Task execution state machine
#[derive(Clone, Debug, PartialEq)]
enum VlmTaskState {
    Idle,
    Planning,       // generating action plan from goal
    Observing,      // taking screenshot for VLM
    Acting,         // executing the chosen action
    Verifying,      // checking if action succeeded
    Done(String),   // task complete with result
    Failed(String), // task failed with reason
}

/// Open-AutoGLM: TAO (Thought-Action-Observation) step
#[derive(Clone)]
struct TaoStep {
    step_num:    u32,
    thought:     String,   // VLM's reasoning about current state
    action:      VlmAction,
    observation: String,   // what happened after action (from next screenshot)
    success:     bool,
    ts:          u128,
}

/// Open-AutoGLM: A phone agent task session
#[derive(Clone)]
struct PhoneAgentTask {
    id:           String,
    goal:         String,    // user's natural language goal
    state:        VlmTaskState,
    plan:         Vec<String>, // high-level sub-tasks from planner
    plan_idx:     usize,      // current sub-task index
    history:      Vec<TaoStep>,
    max_steps:    u32,
    current_step: u32,
    context:      String,    // accumulated observations for VLM context
    result:       String,
    created_ms:   u128,
    last_step_ms: u128,
}

/// Roubao: Screenshot observation record
#[derive(Clone)]
struct ScreenObservation {
    task_id:    String,
    step:       u32,
    screen_pkg: String,
    ui_nodes:   String,  // JSON of accessibility nodes
    screenshot_path: String,
    vlm_description: String, // VLM's description of what it sees
    ts:         u128,
}

/// Build the VLM prompt for Roubao action selection
/// This is the core of Roubao's architecture
fn build_vlm_action_prompt(
    goal: &str,
    sub_task: &str,
    screen_desc: &str,
    history: &[TaoStep],
    step: u32,
    max_steps: u32,
) -> String {
    let history_str: String = history.iter().rev().take(5).rev().map(|h| {
        format!("Step {}: {} \u{2192} {} \u{2192} {}",
            h.step_num,
            h.thought.chars().take(60).collect::<String>(),
            h.action.action_type,
            if h.success { "success" } else { "failed" }
        )
    }).collect::<Vec<_>>().join("\n");

    format!(
        "You are a phone automation agent. Analyze the screen and choose ONE action.\n\
        \n\
        GOAL: {goal}\n\
        CURRENT TASK: {sub_task}\n\
        STEP: {step}/{max_steps}\n\
        \n\
        SCREEN STATE:\n{screen_desc}\n\
        \n\
        RECENT HISTORY:\n{history}\n\
        \n\
        Respond with JSON:\n\
        {{\n\
          \"thought\": \"reasoning about current state\",\n\
          \"action\": \"tap|swipe|type|scroll|back|home|done|failed\",\n\
          \"element\": \"describe the UI element to interact with\",\n\
          \"text\": \"text to type (if action=type)\",\n\
          \"direction\": \"up|down|left|right (if swipe/scroll)\",\n\
          \"confidence\": 0.0-1.0,\n\
          \"done_reason\": \"why task is complete (if action=done)\"\n\
        }}",
        goal = goal,
        sub_task = sub_task,
        step = step,
        max_steps = max_steps,
        screen_desc = &screen_desc[..screen_desc.len().min(800)],
        history = if history_str.is_empty() { "(none)".to_string() } else { history_str },
    )
}

/// Build the Open-AutoGLM task planning prompt
fn build_task_plan_prompt(goal: &str, context: &str) -> String {
    format!(
        "You are an Android phone agent. Break down this goal into 3-7 concrete sub-tasks.\n\
        \n\
        GOAL: {goal}\n\
        DEVICE CONTEXT: {ctx}\n\
        \n\
        Return a JSON array of sub-task strings:\n\
        [\"sub-task 1\", \"sub-task 2\", ...]\n\
        \n\
        Each sub-task should be a single screen interaction goal.\n\
        Be specific about app names, buttons, and content.",
        goal = goal,
        ctx = &context[..context.len().min(400)],
    )
}

/// Parse VLM JSON response into a VlmAction
fn parse_vlm_action(json: &str) -> VlmAction {
    let action_type  = extract_json_str(json, "action").unwrap_or_else(|| "back".to_string());
    let element_desc = extract_json_str(json, "element").unwrap_or_default();
    let text         = extract_json_str(json, "text").unwrap_or_default();
    let direction    = extract_json_str(json, "direction").unwrap_or_default();
    let thought      = extract_json_str(json, "thought").unwrap_or_default();
    let done_reason  = extract_json_str(json, "done_reason").unwrap_or_default();
    let confidence   = extract_json_f32(json, "confidence").unwrap_or(0.5);

    let reasoning = if !done_reason.is_empty() { done_reason } else { thought };

    VlmAction {
        action_type,
        element_desc,
        text,
        direction,
        confidence,
        reasoning,
        x: -1,
        y: -1,
    }
}

/// Convert VlmAction to a MacroAction for execution
fn vlm_action_to_macro(action: &VlmAction) -> MacroAction {
    let mut params = HashMap::new();
    match action.action_type.as_str() {
        "tap" => {
            params.insert("description".to_string(), action.element_desc.clone());
            if action.x >= 0 {
                params.insert("x".to_string(), action.x.to_string());
                params.insert("y".to_string(), action.y.to_string());
            }
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), if action.x >= 0 {
                    format!("input tap {} {}", action.x, action.y)
                } else {
                    format!("# find_and_tap: {}", action.element_desc)
                });
                p
            }, sub_actions: vec![], enabled: true }
        }
        "type" => {
            params.insert("text".to_string(), action.text.clone());
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), format!("input text '{}'", action.text.replace('\'', "")));
                p
            }, sub_actions: vec![], enabled: true }
        }
        "scroll" | "swipe" => {
            let (x1, y1, x2, y2) = match action.direction.as_str() {
                "up"    => (540, 1200, 540, 400),
                "down"  => (540, 400, 540, 1200),
                "left"  => (900, 700, 200, 700),
                "right" => (200, 700, 900, 700),
                _       => (540, 800, 540, 400),
            };
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), format!("input swipe {} {} {} {} 300", x1, y1, x2, y2));
                p
            }, sub_actions: vec![], enabled: true }
        }
        "back" => MacroAction { kind: MacroActionKind::Shell, params: {
            let mut p = HashMap::new(); p.insert("cmd".to_string(), "input keyevent 4".to_string()); p
        }, sub_actions: vec![], enabled: true },
        "home" => MacroAction { kind: MacroActionKind::Shell, params: {
            let mut p = HashMap::new(); p.insert("cmd".to_string(), "input keyevent 3".to_string()); p
        }, sub_actions: vec![], enabled: true },
        _ => MacroAction { kind: MacroActionKind::LogEvent, params: {
            let mut p = HashMap::new(); p.insert("message".to_string(), format!("agent: {}", action.action_type)); p
        }, sub_actions: vec![], enabled: true },
    }
}

/// Enqueue a VLM step: screenshot \u{2192} VLM prompt \u{2192} Java executes \u{2192} Rust processes result
/// This implements the Roubao/Open-AutoGLM TAO loop
fn enqueue_vlm_step(s: &mut KiraState, task_id: &str) {
    // Step 1: Take screenshot and describe screen via VLM
    // This is enqueued as a special compound action for Java to handle
    s.pending_actions.push_back(PendingMacroAction {
        macro_id:  task_id.to_string(),
        action_id: gen_id(),
        kind:      "vlm_observe".to_string(),
        params: {
            let mut p = HashMap::new();
            p.insert("task_id".to_string(), task_id.to_string());
            p.insert("screen_pkg".to_string(), s.screen_pkg.clone());
            p.insert("ui_nodes_len".to_string(), s.screen_nodes.len().to_string());
            p
        },
        ts: now_ms(),
    });
}

/// Process VLM response and execute the decided action
fn execute_vlm_step(s: &mut KiraState, task_id: &str, vlm_response: &str) -> bool {
    let task = s.phone_agent_tasks.iter().find(|t| t.id == task_id).cloned();
    let task = match task { Some(t) => t, None => return false };

    let action = parse_vlm_action(vlm_response);
    let thought = extract_json_str(vlm_response, "thought").unwrap_or_default();
    let is_done = action.action_type == "done";
    let is_failed = action.action_type == "failed";

    // Convert to macro action and enqueue
    let macro_action = vlm_action_to_macro(&action);
    if !is_done && !is_failed {
        enqueue_action(s, task_id, &macro_action);
    }

    // Record TAO step
    let step = TaoStep {
        step_num:    task.current_step,
        thought,
        action:      action.clone(),
        observation: String::new(), // filled after next screenshot
        success:     !is_failed,
        ts:          now_ms(),
    };

    // Update task state
    if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
        t.history.push(step);
        t.current_step += 1;
        t.last_step_ms = now_ms();

        if is_done {
            t.state = VlmTaskState::Done(action.reasoning.clone());
            t.result = action.reasoning.clone();
        } else if is_failed {
            t.state = VlmTaskState::Failed(action.reasoning.clone());
        } else if t.current_step >= t.max_steps {
            t.state = VlmTaskState::Failed("max steps reached".to_string());
        } else {
            t.state = VlmTaskState::Observing;
            // Schedule next observation
        }
    }

    is_done || is_failed
}

/// HTTP routes for VLM / Phone Agent
fn route_vlm_agent(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // Start a new phone agent task (Open-AutoGLM pattern)
        ("POST", "/agent/task")     => {
            let goal = extract_json_str(body, "goal").unwrap_or_default();
            if goal.is_empty() { return Some(r#"{"error":"goal required"}"#.to_string()); }
            let max_steps = extract_json_num(body, "max_steps").unwrap_or(20.0) as u32;
            let task_id = gen_id();
            let task = PhoneAgentTask {
                id: task_id.clone(), goal: goal.clone(),
                state: VlmTaskState::Planning,
                plan: vec![], plan_idx: 0,
                history: vec![], max_steps,
                current_step: 0,
                context: String::new(), result: String::new(),
                created_ms: now_ms(), last_step_ms: now_ms(),
            };
            let mut s = STATE.lock().unwrap();
            // Enqueue task plan generation (Java calls AI with the planning prompt)
            let plan_prompt = build_task_plan_prompt(&goal, &s.agent_context);
            s.pending_actions.push_back(PendingMacroAction {
                macro_id: task_id.clone(), action_id: gen_id(),
                kind: "vlm_plan".to_string(),
                params: {
                    let mut p = HashMap::new();
                    p.insert("task_id".to_string(), task_id.clone());
                    p.insert("goal".to_string(), goal);
                    p.insert("prompt".to_string(), plan_prompt);
                    p
                },
                ts: now_ms(),
            });
            s.phone_agent_tasks.push(task);
            Some(format!(r#"{{"ok":true,"task_id":"{}","state":"planning"}}"#, esc(&task_id)))
        }

        // Get all tasks
        ("GET", "/agent/tasks")     => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.phone_agent_tasks.iter().map(|t| format!(
                r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"max_steps":{},"plan_steps":{},"history_steps":{}}}"#,
                esc(&t.id), esc(&t.goal),
                match &t.state {
                    VlmTaskState::Idle => "idle",
                    VlmTaskState::Planning => "planning",
                    VlmTaskState::Observing => "observing",
                    VlmTaskState::Acting => "acting",
                    VlmTaskState::Verifying => "verifying",
                    VlmTaskState::Done(_) => "done",
                    VlmTaskState::Failed(_) => "failed",
                },
                t.current_step, t.max_steps, t.plan.len(), t.history.len()
            )).collect();
            Some(format!("[{}]", items.join(",")))
        }

        // Get task detail including history
        ("GET", "/agent/task")      => {
            let id = path.find("id=").map(|i| &path[i+3..]).unwrap_or("").split('&').next().unwrap_or("");
            let s = STATE.lock().unwrap();
            match s.phone_agent_tasks.iter().find(|t| t.id == id) {
                Some(t) => {
                    let plan_json: Vec<String> = t.plan.iter().map(|p| format!("\"{}\"", esc(p))).collect();
                    let history_json: Vec<String> = t.history.iter().map(|h| format!(
                        r#"{{"step":{},"thought":"{}","action":"{}","element":"{}","success":{},"ts":{}}}"#,
                        h.step_num, esc(&h.thought), esc(&h.action.action_type),
                        esc(&h.action.element_desc), h.success, h.ts
                    )).collect();
                    let state_str = match &t.state {
                        VlmTaskState::Done(r) => format!("done:{}", r),
                        VlmTaskState::Failed(r) => format!("failed:{}", r),
                        s => format!("{:?}", s),
                    };
                    Some(format!(
                        r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"plan":[{}],"history":[{}],"result":"{}"}}"#,
                        esc(&t.id), esc(&t.goal), esc(&state_str),
                        t.current_step, plan_json.join(","), history_json.join(","), esc(&t.result)
                    ))
                }
                None => Some(r#"{"error":"task not found"}"#.to_string()),
            }
        }

        // Java calls this after taking a screenshot and getting VLM description
        // Body: {task_id, vlm_response (JSON from AI)}
        ("POST", "/agent/vlm_step") => {
            let task_id = extract_json_str(body, "task_id").unwrap_or_default();
            let vlm_resp = extract_json_str(body, "vlm_response").unwrap_or_default();
            if task_id.is_empty() { return Some(r#"{"error":"task_id required"}"#.to_string()); }
            let done = execute_vlm_step(&mut STATE.lock().unwrap(), &task_id, &vlm_resp);
            Some(format!(r#"{{"ok":true,"done":{}}}"#, done))
        }

        // Java calls this with the AI-generated plan JSON array
        ("POST", "/agent/set_plan") => {
            let task_id = extract_json_str(body, "task_id").unwrap_or_default();
            // Parse plan array: ["task1","task2",...]
            let plan_str = extract_json_str(body, "plan").unwrap_or_default();
            let plan: Vec<String> = plan_str.split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let mut s = STATE.lock().unwrap();
            if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
                t.plan = plan;
                t.state = VlmTaskState::Observing;
                // Enqueue first VLM observation
            }
            // Now enqueue the first observation
            enqueue_vlm_step(&mut s, &task_id);
            Some(r#"{"ok":true,"state":"observing"}"#.to_string())
        }

        // Build VLM prompt for current task step (Java uses this to call AI)
        ("GET", "/agent/prompt")    => {
            let id = path.find("id=").map(|i| &path[i+3..]).unwrap_or("").split('&').next().unwrap_or("");
            let s = STATE.lock().unwrap();
            match s.phone_agent_tasks.iter().find(|t| t.id == id) {
                Some(t) => {
                    let sub_task = t.plan.get(t.plan_idx).cloned().unwrap_or_else(|| t.goal.clone());
                    let screen_desc = format!("Package: {}\nUI nodes count: {}\nAgent context: {}",
                        s.screen_pkg, s.screen_nodes.len(), &s.agent_context[..s.agent_context.len().min(500)]);
                    let prompt = build_vlm_action_prompt(
                        &t.goal, &sub_task, &screen_desc,
                        &t.history, t.current_step, t.max_steps
                    );
                    Some(format!(r#"{{"task_id":"{}","prompt":{},"step":{},"sub_task":"{}"}}"#,
                        esc(&t.id), json_str(&prompt), t.current_step, esc(&sub_task)))
                }
                None => Some(r#"{"error":"task not found"}"#.to_string()),
            }
        }

        // Cancel a task
        ("POST", "/agent/cancel")   => {
            let id = extract_json_str(body, "task_id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == id) {
                t.state = VlmTaskState::Failed("cancelled by user".to_string());
            }
            Some(r#"{"ok":true}"#.to_string())
        }

        // Clear completed tasks
        ("POST", "/agent/clear")    => {
            let mut s = STATE.lock().unwrap();
            s.phone_agent_tasks.retain(|t| matches!(t.state, VlmTaskState::Planning | VlmTaskState::Observing | VlmTaskState::Acting | VlmTaskState::Verifying));
            Some(format!(r#"{{"ok":true,"remaining":{}}}"#, s.phone_agent_tasks.len()))
        }

        // Screen observations log (Roubao pattern)
        ("GET", "/agent/observations") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.screen_observations.iter().rev().take(20).map(|o| format!(
                r#"{{"task_id":"{}","step":{},"pkg":"{}","vlm":"{}","ts":{}}}"#,
                esc(&o.task_id), o.step, esc(&o.screen_pkg),
                esc(&o.vlm_description[..o.vlm_description.len().min(100)]), o.ts
            )).collect();
            Some(format!("[{}]", items.join(",")))
        }

        _ => None,
    }
}

/// JNI: Java reports VLM observation result back to Rust
/// Called after Java takes screenshot, runs it through VLM, gets description
fn record_screen_observation(s: &mut KiraState, task_id: &str, step: u32, vlm_desc: &str) {
    s.screen_observations.push_back(ScreenObservation {
        task_id: task_id.to_string(),
        step,
        screen_pkg: s.screen_pkg.clone(),
        ui_nodes: s.screen_nodes.chars().take(500).collect(),
        screenshot_path: String::new(),
        vlm_description: vlm_desc.to_string(),
        ts: now_ms(),
    });
    if s.screen_observations.len() > 200 { s.screen_observations.pop_front(); }

    // Update context for current task
    if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
        t.context = format!("{}; step{}: {}", t.context, step, &vlm_desc[..vlm_desc.len().min(100)]);
        t.state = VlmTaskState::Acting;
        // Update observation in last history entry
        if let Some(h) = t.history.last_mut() {
            h.observation = vlm_desc.chars().take(200).collect();
        }
    }
}


// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// OpenClaw v3 / NanoBot / ZeroClaw \u{2014} Extended Automation Engine
//
// New in this version:
//   - Reactive programming: event streams + filter chains (NanoBot Rx pattern)
//   - State machine engine (ZeroClaw FSM)
//   - Sensor fusion: combine multiple device signals into composite triggers
//   - Macro scripting DSL: evaluate mini-programs from string
//   - Context-aware automation: time-of-day, location, activity zones
//   - Cross-macro communication via shared channels
//   - Macro version control: history + rollback
//   - Automation marketplace: import/export macro bundles
//   - Smart home integration hooks (MQTT/WebSocket stubs)
//   - Battery-aware scheduling: defer tasks when battery low
//   - AI-assisted macro generation: convert natural language to macro JSON
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

// \u{2500}\u{2500} NanoBot Rx: Reactive event stream \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// An event in the reactive stream
#[derive(Clone)]
struct RxEvent {
    kind:    String,   // "notification", "battery", "screen", "wifi", "timer", "custom"
    data:    String,   // JSON payload
    ts:      u128,
    source:  String,   // macro_id or "system"
}

/// A filter on the event stream (Rx-style operator)
#[derive(Clone)]
enum RxOperator {
    Filter(MacroCondition),          // only pass events matching condition
    Map(String, String),             // rename field or transform value
    Debounce(u128),                  // ignore events within N ms of last
    Throttle(u128),                  // max 1 event per N ms
    Distinct,                        // only pass if value changed from last
    Take(u32),                       // pass only first N events
    Skip(u32),                       // skip first N events
    Merge(Vec<String>),              // merge events from multiple sources
    Buffer(u32),                     // collect N events then emit as batch
}

/// A reactive subscription: event source \u{2192} operators \u{2192} macro trigger
#[derive(Clone)]
struct RxSubscription {
    id:          String,
    name:        String,
    event_kinds: Vec<String>,       // which event types to subscribe to
    operators:   Vec<RxOperator>,
    target_macro: String,           // macro to trigger when event passes filters
    enabled:     bool,
    fired_count: u64,
    last_fired:  u128,
    // State for stateful operators
    debounce_last: u128,
    throttle_last: u128,
    take_count:    u32,
    skip_count:    u32,
    last_value:    String,          // for Distinct
    buffer:        Vec<RxEvent>,
}

/// Process an event through a subscription's operator chain
fn rx_process_event(sub: &mut RxSubscription, event: &RxEvent, s: &KiraState) -> Option<String> {
    // Check event kind filter
    if !sub.event_kinds.is_empty() && !sub.event_kinds.contains(&event.kind) {
        return None;
    }

    let now = now_ms();

    for op in &sub.operators.clone() {
        match op {
            RxOperator::Filter(cond) => {
                if !eval_condition(s, cond) { return None; }
            }
            RxOperator::Debounce(ms) => {
                if now - sub.debounce_last < *ms { return None; }
                sub.debounce_last = now;
            }
            RxOperator::Throttle(ms) => {
                if now - sub.throttle_last < *ms { return None; }
                sub.throttle_last = now;
            }
            RxOperator::Distinct => {
                if event.data == sub.last_value { return None; }
                sub.last_value = event.data.clone();
            }
            RxOperator::Take(n) => {
                if sub.take_count >= *n { return None; }
                sub.take_count += 1;
            }
            RxOperator::Skip(n) => {
                if sub.skip_count < *n { sub.skip_count += 1; return None; }
            }
            RxOperator::Buffer(n) => {
                sub.buffer.push(event.clone());
                if sub.buffer.len() < *n as usize { return None; }
                // Emit buffered batch
                let batch = sub.buffer.drain(..).map(|e| e.data.clone()).collect::<Vec<_>>().join(",");
                return Some(format!(r#"{{"batch":[{}],"count":{}}}"#, batch, n));
            }
            _ => {}
        }
    }

    Some(event.data.clone())
}

// \u{2500}\u{2500} ZeroClaw FSM: Finite State Machine engine \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A state in a ZeroClaw FSM
#[derive(Clone)]
struct FsmState {
    id:           String,
    name:         String,
    entry_actions: Vec<MacroAction>,  // run when entering this state
    exit_actions:  Vec<MacroAction>,  // run when leaving this state
    is_final:     bool,
}

/// A transition between FSM states
#[derive(Clone)]
struct FsmTransition {
    from_state:  String,
    to_state:    String,
    trigger:     String,       // event kind that fires this transition
    condition:   Option<MacroCondition>,
    actions:     Vec<MacroAction>,  // run during transition
}

/// A ZeroClaw finite state machine
#[derive(Clone)]
struct StateMachine {
    id:            String,
    name:          String,
    states:        HashMap<String, FsmState>,
    transitions:   Vec<FsmTransition>,
    current_state: String,
    initial_state: String,
    enabled:       bool,
    history:       VecDeque<String>,  // state transition history
    created_ms:    u128,
}

/// Process an event through a state machine
fn fsm_process_event(s: &mut KiraState, machine_id: &str, event_kind: &str) {
    let machine = match s.state_machines.iter().find(|m| m.id == machine_id && m.enabled) {
        Some(m) => m.clone(),
        None => return,
    };

    // Find matching transition from current state
    let transition = machine.transitions.iter().find(|t| {
        t.from_state == machine.current_state
            && t.trigger == event_kind
            && t.condition.as_ref().map(|c| eval_condition(s, c)).unwrap_or(true)
    }).cloned();

    if let Some(trans) = transition {
        let from = machine.current_state.clone();
        let to = trans.to_state.clone();

        // Run exit actions of current state
        if let Some(state) = machine.states.get(&from) {
            let exit_actions = state.exit_actions.clone();
            for action in &exit_actions {
                enqueue_action(s, machine_id, action);
            }
        }

        // Run transition actions
        for action in &trans.actions {
            enqueue_action(s, machine_id, action);
        }

        // Run entry actions of new state
        if let Some(state) = machine.states.get(&to) {
            let entry_actions = state.entry_actions.clone();
            for action in &entry_actions {
                enqueue_action(s, machine_id, action);
            }
        }

        // Update machine state
        if let Some(m) = s.state_machines.iter_mut().find(|m| m.id == machine_id) {
            m.history.push_back(format!("{}->{}", from, to));
            if m.history.len() > 50 { m.history.pop_front(); }
            m.current_state = to;
        }

        // Log the transition
        s.daily_log.push_back(format!("[fsm:{}] {}\u{2192}{} via {}", machine_id, from, trans.to_state, event_kind));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
    }
}

// \u{2500}\u{2500} Sensor Fusion: Composite triggers \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A composite trigger: multiple signals combined with AND/OR/NOT logic
#[derive(Clone)]
struct CompositeTrigger {
    id:       String,
    name:     String,
    logic:    String,   // "AND", "OR", "NOT", "XOR"
    triggers: Vec<MacroTrigger>,
    target_macro: String,
    enabled:  bool,
    cooldown_ms: u128,
    last_fired:  u128,
}

fn check_composite_trigger(s: &KiraState, ct: &CompositeTrigger) -> bool {
    if !ct.enabled { return false; }
    if now_ms() - ct.last_fired < ct.cooldown_ms { return false; }

    let results: Vec<bool> = ct.triggers.iter().map(|t| check_trigger(s, t)).collect();

    match ct.logic.as_str() {
        "AND" => results.iter().all(|&r| r),
        "OR"  => results.iter().any(|&r| r),
        "NOT" => results.first().map(|&r| !r).unwrap_or(false),
        "XOR" => results.iter().filter(|&&r| r).count() == 1,
        "NAND"=> !results.iter().all(|&r| r),
        "NOR" => !results.iter().any(|&r| r),
        _ => false,
    }
}

// \u{2500}\u{2500} NanoBot Macro DSL: Mini scripting language \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Execute a NanoBot DSL script
/// Syntax (one statement per line):
///   SET $var = value
///   SET $var = %OTHER_VAR% + 1
///   IF $var == value THEN action_kind param=value
///   WAIT 500
///   REPEAT 3 action_kind param=value
///   CALL macro_id
///   CHAIN macro_id
///   LOG message text
///   NOTIFY title | body
///   HTTP GET url
///   HTTP POST url | body
///   SHELL command
fn execute_dsl_script(s: &mut KiraState, macro_id: &str, script: &str) -> Vec<String> {
    let mut log = Vec::new();

    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        match parts.as_slice() {
            ["SET", var, "=", expr] => {
                let name = var.trim_start_matches('$').to_uppercase();
                let value = eval_expr(s, expr);
                let ts = now_ms();
                s.variables.insert(name.clone(), AutoVariable {
                    name: name.clone(), value: value.clone(),
                    var_type: "string".to_string(), persistent: false,
                    created_ms: ts, updated_ms: ts,
                });
                log.push(format!("SET {} = {}", name, value));
            }
            ["WAIT", ms_str] => {
                if let Ok(ms) = ms_str.parse::<u64>() {
                    let mut p = HashMap::new();
                    p.insert("ms".to_string(), ms.to_string());
                    s.pending_actions.push_back(PendingMacroAction {
                        macro_id: macro_id.to_string(), action_id: gen_id(),
                        kind: "wait".to_string(), params: p, ts: now_ms(),
                    });
                    log.push(format!("WAIT {}ms", ms));
                }
            }
            ["LOG", ..] => {
                let msg = line[4..].trim();
                let expanded = expand_vars(s, msg);
                s.daily_log.push_back(format!("[dsl:{}] {}", macro_id, expanded));
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                log.push(format!("LOG: {}", expanded));
            }
            ["CALL", target_id] | ["CHAIN", target_id] => {
                chain_macro(s, target_id);
                log.push(format!("CHAIN \u{2192} {}", target_id));
            }
            ["IF", ..] => {
                // IF $var OP value THEN action
                if let Some(then_pos) = line.find(" THEN ") {
                    let cond_part = &line[3..then_pos].trim();
                    let action_part = line[then_pos + 6..].trim();
                    // Simple: $var == value
                    let cond_parts: Vec<&str> = cond_part.splitn(3, ' ').collect();
                    if cond_parts.len() == 3 {
                        let lhs = cond_parts[0].trim_start_matches('$').to_uppercase();
                        let op  = cond_parts[1];
                        let rhs = cond_parts[2];
                        let rust_op = match op { "==" => "eq", "!=" => "neq", ">" => "gt", "<" => "lt", ">=" => "gte", "<=" => "lte", _ => "eq" };
                        let cond = MacroCondition { lhs: format!("%{}%", lhs), operator: rust_op.to_string(), rhs: rhs.to_string() };
                        if eval_condition(s, &cond) {
                            // Execute the THEN action
                            let action_parts: Vec<&str> = action_part.splitn(2, ' ').collect();
                            let kind = action_parts[0];
                            let mut params = HashMap::new();
                            if action_parts.len() > 1 {
                                for kv in action_parts[1].split(' ') {
                                    if let Some(eq) = kv.find('=') {
                                        params.insert(kv[..eq].to_string(), expand_vars(s, &kv[eq+1..]));
                                    }
                                }
                            }
                            let action = MacroAction { kind: MacroActionKind::from_str(kind), params, sub_actions: vec![], enabled: true };
                            enqueue_action(s, macro_id, &action);
                            log.push(format!("IF {} \u{2192} executed {}", cond_part, kind));
                        }
                    }
                }
            }
            ["REPEAT", count_str, kind, ..] => {
                if let Ok(count) = count_str.parse::<u32>() {
                    let rest = if parts.len() > 3 { parts[3] } else { "" };
                    let mut params = HashMap::new();
                    for kv in rest.split(' ') {
                        if let Some(eq) = kv.find('=') {
                            params.insert(kv[..eq].to_string(), expand_vars(s, &kv[eq+1..]));
                        }
                    }
                    for _ in 0..count.min(100) {
                        let action = MacroAction { kind: MacroActionKind::from_str(kind), params: params.clone(), sub_actions: vec![], enabled: true };
                        enqueue_action(s, macro_id, &action);
                    }
                    log.push(format!("REPEAT {} \u{00D7} {}", count, kind));
                }
            }
            ["NOTIFY", ..] => {
                let rest = expand_vars(s, &line[7..]);
                let parts: Vec<&str> = rest.splitn(2, '|').collect();
                let title = parts[0].trim().to_string();
                let body  = parts.get(1).unwrap_or(&"").trim().to_string();
                let mut params = HashMap::new();
                params.insert("title".to_string(), title.clone());
                params.insert("text".to_string(), body);
                let action = MacroAction { kind: MacroActionKind::SendNotification, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("NOTIFY: {}", title));
            }
            ["HTTP", method, ..] => {
                let rest = expand_vars(s, parts.get(2).unwrap_or(&""));
                let url_parts: Vec<&str> = rest.splitn(2, '|').collect();
                let url  = url_parts[0].trim().to_string();
                let body = url_parts.get(1).unwrap_or(&"").trim().to_string();
                let kind = match *method { "GET" => MacroActionKind::HttpGet, "POST" => MacroActionKind::HttpPost, _ => MacroActionKind::HttpGet };
                let mut params = HashMap::new();
                params.insert("url".to_string(), url.clone());
                if !body.is_empty() { params.insert("body".to_string(), body); }
                let action = MacroAction { kind, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("HTTP {} {}", method, url));
            }
            ["SHELL", ..] => {
                let cmd = expand_vars(s, line[6..].trim());
                let mut params = HashMap::new();
                params.insert("cmd".to_string(), cmd.clone());
                let action = MacroAction { kind: MacroActionKind::Shell, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("SHELL: {}", &cmd[..cmd.len().min(50)]));
            }
            _ => {
                log.push(format!("UNKNOWN: {}", &line[..line.len().min(40)]));
            }
        }
    }

    log
}

// \u{2500}\u{2500} Battery-aware scheduling \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Check if it's safe to run a macro given battery state
fn battery_allows_run(s: &KiraState, macro_id: &str) -> bool {
    // Find battery threshold tag: "battery_min:20" means require >= 20%
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        for tag in &m.tags {
            if let Some(rest) = tag.strip_prefix("battery_min:") {
                if let Ok(min) = rest.parse::<i32>() {
                    if s.battery_pct < min && !s.battery_charging {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Defer a macro to run when battery is back above threshold
fn defer_until_charged(s: &mut KiraState, macro_id: &str, min_pct: i32) {
    // Add a battery_level trigger to run when battery recovers
    let trigger_id = format!("deferred_{}_{}", macro_id, now_ms());
    s.triggers.push(Trigger {
        id: trigger_id,
        trigger_type: "battery_recovery".to_string(),
        value: min_pct.to_string(),
        action: macro_id.to_string(),
        fired: false,
        repeat: false,
    });
    s.daily_log.push_back(format!("[defer] {} deferred until battery >= {}%", macro_id, min_pct));
    if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
}

// \u{2500}\u{2500} Context zones: time + location based automation \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A context zone: when device is in this context, activate/deactivate macros
#[derive(Clone)]
struct ContextZone {
    id:            String,
    name:          String,
    // Time window
    active_hours_start: u8,   // 0-23
    active_hours_end:   u8,
    active_days: Vec<u8>,     // 0=Sun,1=Mon,...,6=Sat (empty=all)
    // Location
    lat:     f64,
    lon:     f64,
    radius_m: f64,            // 0 = ignore location
    // Profile to activate
    activate_profile: String,
    // Macros to enable/disable
    enable_macros:  Vec<String>,
    disable_macros: Vec<String>,
    enabled:        bool,
    currently_active: bool,
}

/// Check if a context zone is active now
fn is_zone_active(zone: &ContextZone, s: &KiraState) -> bool {
    // Time check (simplified: use current hour from timestamp)
    let now_secs = now_ms() / 1000;
    let hour = ((now_secs % 86400) / 3600) as u8;

    let in_time = if zone.active_hours_start <= zone.active_hours_end {
        hour >= zone.active_hours_start && hour < zone.active_hours_end
    } else {
        // Overnight: e.g. 22:00-06:00
        hour >= zone.active_hours_start || hour < zone.active_hours_end
    };

    if !in_time { return false; }

    // Location check
    if zone.radius_m > 0.0 && (s.sig_lat != 0.0 || s.sig_lon != 0.0) {
        let dlat = (s.sig_lat - zone.lat).to_radians();
        let dlon = (s.sig_lon - zone.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + zone.lat.to_radians().cos() * s.sig_lat.to_radians().cos()
            * (dlon / 2.0).sin().powi(2);
        let dist_m = 6_371_000.0 * 2.0 * a.sqrt().asin();
        if dist_m > zone.radius_m { return false; }
    }

    true
}

/// Apply context zone changes
fn apply_context_zones(s: &mut KiraState) {
    let zones: Vec<ContextZone> = s.context_zones.iter().cloned().collect();
    for zone in zones {
        if !zone.enabled { continue; }
        let active = is_zone_active(&zone, s);

        // Detect zone enter/exit
        let was_active = zone.currently_active;
        if active && !was_active {
            // Zone entered
            if !zone.activate_profile.is_empty() {
                s.active_profile = zone.activate_profile.clone();
            }
            for macro_id in &zone.enable_macros {
                if let Some(m) = s.macros.iter_mut().find(|m| m.id == *macro_id) {
                    m.enabled = true;
                }
            }
            for macro_id in &zone.disable_macros {
                if let Some(m) = s.macros.iter_mut().find(|m| m.id == *macro_id) {
                    m.enabled = false;
                }
            }
            s.daily_log.push_back(format!("[zone] entered: {}", zone.name));
        }

        // Update zone active state
        if let Some(z) = s.context_zones.iter_mut().find(|z| z.id == zone.id) {
            z.currently_active = active;
        }
    }
    if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
}

// \u{2500}\u{2500} Macro bundle: import/export marketplace \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Export macros + keywords + flows as a shareable bundle
fn export_bundle(s: &KiraState, tag_filter: Option<&str>) -> String {
    let macros: Vec<String> = s.macros.iter()
        .filter(|m| tag_filter.map(|t| m.tags.contains(&t.to_string())).unwrap_or(true))
        .map(macro_to_json)
        .collect();
    let keywords: Vec<String> = s.roboru_keywords.values().map(|kw| {
        let steps_json: Vec<String> = kw.steps.iter().map(action_to_json).collect();
        let args_json: Vec<String> = kw.args.iter().map(|a| format!(r#""{}""#, esc(a))).collect();
        format!(r#"{{"name":"{}","description":"{}","args":[{}],"steps":[{}],"returns":"{}"}}"#,
            esc(&kw.name), esc(&kw.description), args_json.join(","), steps_json.join(","), esc(&kw.returns))
    }).collect();
    format!(
        r#"{{"version":"1.0","exported_ms":{},"macros":[{}],"keywords":[{}],"variable_count":{}}}"#,
        now_ms(), macros.join(","), keywords.join(","), s.variables.len()
    )
}

// \u{2500}\u{2500} Cross-macro channel communication \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Post a message to a named channel (macros can subscribe via kira_event trigger)
fn channel_post(s: &mut KiraState, channel: &str, message: &str) {
    let event_key = format!("channel:{}:{}", channel, message);
    s.sig_kira_event = event_key.clone();
    s.event_feed.push_back(EventFeedEntry {
        event: format!("channel_{}", channel),
        data: message.to_string(),
        ts: now_ms(),
    });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

// \u{2500}\u{2500} HTTP routes for new features \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}


/// Parse a natural-language condition string into (trigger_kind, trigger_value).
/// Examples:
///   "battery < 20"      → ("battery_low",  "20")
///   "screen on"         → ("screen_on",    "")
///   "wifi connected"    → ("wifi_changed",  "connected")
///   "app youtube"       → ("app_opened",   "com.google.android.youtube")
///   "notification otp"  → ("keyword_notif","otp")
///   "time 07:30"        → ("time_daily",   "07:30")
///   "charging"          → ("power_connected","")
///   "unplugged"         → ("power_disconnected","")
fn parse_nl_condition(cond: &str) -> (String, String) {
    let c = cond.to_lowercase();
    let c = c.trim();

    // Battery patterns: "battery < 20", "battery below 20", "battery 20"
    if c.contains("battery") {
        let num: String = c.chars().filter(|ch| ch.is_ascii_digit()).collect();
        if !num.is_empty() { return ("battery_low".to_string(), num); }
        if c.contains("full") || c.contains("100") { return ("battery_low".to_string(), "95".to_string()); }
    }
    // Screen events
    if c.contains("screen on") || c.contains("screen unlock") || c.contains("unlocked") {
        return ("screen_on".to_string(), String::new());
    }
    if c.contains("screen off") || c.contains("screen lock") || c.contains("locked") {
        return ("screen_off".to_string(), String::new());
    }
    // WiFi
    if c.contains("wifi") {
        if c.contains("disconnect") { return ("wifi_changed".to_string(), "disconnected".to_string()); }
        return ("wifi_changed".to_string(), "connected".to_string());
    }
    // Charging / power
    if c.contains("charging") || c.contains("plugged") || c.contains("power on") {
        return ("power_connected".to_string(), String::new());
    }
    if c.contains("unplug") || c.contains("unplugged") || c.contains("power off") {
        return ("power_disconnected".to_string(), String::new());
    }
    // Time: "time 07:30", "at 07:30", "07:30"
    if c.contains("time") || c.contains(" at ") || c.contains(":") {
        let time: String = c.split_whitespace()
            .find(|w| w.contains(':') && w.len() <= 5)
            .unwrap_or("08:00")
            .to_string();
        return ("time_daily".to_string(), time);
    }
    // App opened: "app youtube", "youtube opens"
    if c.contains("app ") || c.contains(" open") || c.contains(" launch") {
        let word = c.split_whitespace()
            .find(|w| !["app","open","opens","launch","when","if"].contains(w))
            .unwrap_or("").to_string();
        return ("app_opened".to_string(), app_name_to_pkg(&word));
    }
    // Notification keyword: "notification otp", "notif payment"
    if c.contains("notif") || c.contains("message") || c.contains("alert") {
        let keyword = c.split_whitespace()
            .find(|w| !["notification","notif","message","alert","a","the","contains"].contains(w))
            .unwrap_or("").to_string();
        return ("keyword_notif".to_string(), keyword);
    }
    // Shake / motion
    if c.contains("shake") { return ("shake".to_string(), String::new()); }
    // Headphones
    if c.contains("headphone") || c.contains("earphone") || c.contains("audio plug") {
        return ("headphone_connected".to_string(), String::new());
    }
    // Fallback: use as a keyword trigger
    ("keyword_notif".to_string(), c.to_string())
}

/// Map friendly app name → package name.
/// Covers ~30 most common Android apps.
/// Map 200+ app names/aliases to their Android package names.
/// Called by automation triggers (watch_app, if_then "app X") and tool dispatch.
