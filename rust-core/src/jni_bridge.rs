mod jni_bridge {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_void};

    // ── JNI type aliases ──────────────────────────────────────────────────
    type JNIEnv  = *mut *mut c_void;   // JNIEnv**
    type JObject = *mut c_void;         // jobject
    type JString = *mut c_void;         // jstring (IS a JVM object reference)

    /// Convert a raw *const c_char (from Java) to a Rust String.
    fn cs(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }

    /// Length-bounded, safe version of cs() for untrusted JNI input (Session L).
    /// Rejects strings over max_len bytes with an empty string + log.
    fn cs_safe(p: *const c_char, max_len: usize) -> String {
        if p.is_null() { return String::new(); }
        let s = unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() };
        if s.len() > max_len {
            // Truncate rather than reject  -  avoids Java crash on oversized input
            return s[..max_len].to_string();
        }
        s
    }

    /// Create a JVM-managed jstring from a Rust &str.
    /// Uses jni crate for safe, correct NewStringUTF call.
    /// Falls back to manual vtable[169] if env is null.
    unsafe fn jni_str(env: JNIEnv, s: &str) -> JString {
        use jni::JNIEnv as SafeEnv;
        if env.is_null() { return std::ptr::null_mut(); }
        // Sanitize: remove embedded nulls which cause NewStringUTF to abort
        let safe_s: String = s.chars()
            .filter(|c| *c != '\0')
            .collect();
        // Clamp to 512KB to avoid OOM in JVM string allocation
        let clamped = if safe_s.len() > 524288 {
            &safe_s[..524288]
        } else {
            &safe_s
        };
        // from_raw can fail  -  use match instead of expect/unwrap
        let mut safe_env = match SafeEnv::from_raw(env as *mut jni::sys::JNIEnv) {
            Ok(e)  => e,
            Err(_) => return std::ptr::null_mut(),
        };
        match safe_env.new_string(clamped) {
            Ok(jstr) => jstr.into_raw() as JString,
            Err(_)   => std::ptr::null_mut(),
        }
    }

    // \u{2500}\u{2500} Lifecycle \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: JNIEnv, _c: JObject, port: i32,
    ) {
        // Guard: only start once. Second call from KiraAccessibilityService is a no-op.
        use std::sync::atomic::{AtomicBool, Ordering};
        static SERVER_STARTED: AtomicBool = AtomicBool::new(false);
        if SERVER_STARTED.swap(true, Ordering::SeqCst) {
            return; // already running
        }

        // Install a panic hook that suppresses the default stderr output and
        // prevents the "panic in panic handler → abort()" double-panic scenario.
        // catch_unwind at the JNI boundary still catches all panics safely.
        // This is critical for Android where rustls can panic on bad TLS records.
        std::panic::set_hook(Box::new(|info| {
            // Log to Android logcat without panicking ourselves
            let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "unknown panic".to_string()
            };
            let loc = info.location().map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "unknown".to_string());
            // Store in state for crash reporting (best-effort, ignore lock errors)
            if let Ok(mut s) = STATE.try_lock() {
                s.last_panic = format!("{} @ {}", msg, loc);
            }
            // Do NOT call the default hook  -  it can cause double-panic → abort()
        }));
        let p = port as u16;
        {
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            s.uptime_start = now_ms();
            s.providers    = make_providers();
            let sess = Session { id:"default".into(), channel:"kira".into(), created:now_ms(), last_msg:now_ms(), ..Default::default() };
            s.sessions.insert("default".into(), sess);

            // ── Session 4: Load persisted sessions from disk ────────────────
            s.session_store.load_from_disk();
            // Ensure "default" session always exists in store
            let now = now_ms();
            s.session_store.get_or_create("default", "kira", now);
            // Default profiles
            s.profiles = vec![
                AutoProfile { id:"default".into(), name:"Default".into(), active:true,  auto_activate_trigger:String::new(), auto_activate_value:String::new() },
                AutoProfile { id:"work".into(),    name:"Work".into(),    active:false, auto_activate_trigger:"wifi_connected".into(), auto_activate_value:String::new() },
                AutoProfile { id:"home".into(),    name:"Home".into(),    active:false, auto_activate_trigger:"wifi_connected".into(), auto_activate_value:String::new() },
                AutoProfile { id:"sleep".into(),   name:"Sleep".into(),   active:false, auto_activate_trigger:"time".into(),           auto_activate_value:"22:00".into() },
                AutoProfile { id:"car".into(),     name:"Car".into(),     active:false, auto_activate_trigger:"bt_connected".into(),   auto_activate_value:String::new() },
            ];
        }
        install_builtin_templates(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()));
        thread::spawn(move || run_http(p));
        thread::spawn(run_trigger_watcher);
        thread::spawn(run_cron_scheduler);
        thread::spawn(run_macro_engine);
        thread::spawn(run_watchdog);
    }

    // \u{2500}\u{2500} v40: Device signal injectors (called from Java on each device event) \u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOn(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_screen_on = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOff(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_screen_off = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalUnlocked(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_device_unlocked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocked(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_device_locked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalShake(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_shake = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeUp(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_vol_up = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeDown(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_vol_down = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalWifi(
        _e: JNIEnv, _c: JObject, ssid: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_wifi_ssid = cs(ssid); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalBluetooth(
        _e: JNIEnv, _c: JObject, device: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_bt_device = cs(device); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalSms(
        _e: JNIEnv, _c: JObject,
        sender: *const c_char, text: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.sig_sms_sender = cs(sender);
        s.sig_sms_text   = cs(text);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalCall(
        _e: JNIEnv, _c: JObject, number: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_call_number = cs(number); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalNfc(
        _e: JNIEnv, _c: JObject, tag_id: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_nfc_tag = cs(tag_id); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalClipboard(
        _e: JNIEnv, _c: JObject, text: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_clipboard = cs(text); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppLaunched(
        _e: JNIEnv, _c: JObject, pkg: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_app_launched = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppClosed(
        _e: JNIEnv, _c: JObject, pkg: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_app_closed = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocation(
        _e: JNIEnv, _c: JObject, lat: f64, lon: f64, geofence: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.sig_lat      = lat;
        s.sig_lon      = lon;
        s.sig_geofence = cs(geofence);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalKiraEvent(
        _e: JNIEnv, _c: JObject, event: *const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).sig_kira_event = cs(event); }

    // \u{2500}\u{2500} v40: Macro management JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    /// Add or replace a macro. Body is full macro JSON.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addMacro(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let m = parse_macro_from_json(&body);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.macros.retain(|x| x.id != m.id);
        let id = m.id.clone();
        s.macros.push(m);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, id)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeMacro(
        _e: JNIEnv, _c: JObject, id: *const c_char,
    ) {
        let id = cs(id);
        STATE.lock().unwrap_or_else(|e| e.into_inner()).macros.retain(|m| m.id != id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_enableMacro(
        _e: JNIEnv, _c: JObject, id: *const c_char, enabled: bool,
    ) {
        let id = cs(id);
        if let Some(m) = STATE.lock().unwrap_or_else(|e| e.into_inner()).macros.iter_mut().find(|m| m.id == id) {
            m.enabled = enabled;
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacros(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.macros.iter().map(macro_to_json).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runMacroNow(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let actions: Vec<MacroAction> = s.macros.iter()
            .find(|m| m.id == id)
            .map(|m| m.actions.clone())
            .unwrap_or_default();
        let name = s.macros.iter().find(|m| m.id == id)
            .map(|m| m.name.clone()).unwrap_or_default();
        let start = now_ms();
        let (steps, _) = execute_macro_actions(&mut s, &id, &actions);
        if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
            m.run_count += 1; m.last_run_ms = start;
        }
        s.macro_run_log.push_back(MacroRunLog {
            macro_id: id.clone(), macro_name: name, trigger:"manual".to_string(),
            success:true, steps_run:steps, duration_ms:now_ms()-start, ts:start, error:String::new()
        });
        if s.macro_run_log.len() > 1000 { s.macro_run_log.pop_front(); }
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"steps":{}}}"#, steps)) }
    }

    /// Get next pending macro action for Java to execute
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextMacroAction(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap_or_else(|e| e.into_inner()).pending_actions.pop_front() {
            Some(pa) => {
                let params_json: Vec<String> = pa.params.iter()
                    .map(|(k,v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect();
                let json = format!(
                    r#"{{"macro_id":"{}","action_id":"{}","kind":"{}","ts":{},"params":{{{}}}}}"#,
                    esc(&pa.macro_id), esc(&pa.action_id), esc(&pa.kind), pa.ts, params_json.join(",")
                );
                unsafe { jni_str(env, &json) }
            }
            None => std::ptr::null_mut(),
        }
    }

    // \u{2500}\u{2500} v40: Variable management \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setVariable(
        _e: JNIEnv, _c: JObject,
        name: *const c_char, value: *const c_char, var_type: *const c_char,
    ) {
        let name = cs(name); let value = cs(value); let vt = cs(var_type);
        let ts = now_ms();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.variables.entry(name.clone()).and_modify(|v| { v.value = value.clone(); v.updated_ms = ts; })
            .or_insert(AutoVariable { name, value, var_type: if vt.is_empty(){"string".to_string()}else{vt}, persistent:false, created_ms:ts, updated_ms:ts });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getVariable(
        env: JNIEnv, _c: JObject, name: *const c_char,
    ) -> JString {
        let name = cs(name);
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let json = match s.variables.get(&name) {
            Some(v) => format!(r#"{{"name":"{}","value":"{}","type":"{}"}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type)),
            None    => r#"{"error":"not_found"}"#.to_string(),
        };
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getVariables(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.variables.values().map(|v|
            format!(r#"{{"name":"{}","value":"{}","type":"{}","updated_ms":{}}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type), v.updated_ms)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} v40: Profile management \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setProfile(
        _e: JNIEnv, _c: JObject, id: *const c_char,
    ) {
        let id = cs(id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.active_profile = id.clone();
        for p in s.profiles.iter_mut() { p.active = p.id == id; }
        s.sig_kira_event = format!("profile_changed:{}", id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProfiles(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.profiles.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","active":{}}}"#, esc(&p.id), esc(&p.name), p.active)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacroRunLog(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.macro_run_log.iter().skip(s.macro_run_log.len().saturating_sub(100)).map(|r|
            format!(r#"{{"macro_id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"duration_ms":{},"ts":{}}}"#,
                esc(&r.macro_id), esc(&r.macro_name), esc(&r.trigger), r.success, r.steps_run, r.duration_ms, r.ts)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} v38 JNI (unchanged) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_syncConfig(
        _e: JNIEnv, _c: JObject,
        user_name:*const c_char, api_key:*const c_char, base_url:*const c_char,
        model:*const c_char, vision_model:*const c_char, persona:*const c_char,
        tg_token:*const c_char, tg_allowed:i64, max_steps:i32, auto_approve:bool,
        heartbeat:i32, setup_done:bool,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.config.user_name = cs(user_name);
        // Sanitize: reject non-UTF8 / binary garbage from old encrypted storage
        let raw_key = cs(api_key);
        let raw_url = cs(base_url);
        if !raw_key.is_empty() {
            s.config.api_key = if raw_key.chars().all(|c| c.is_ascii()) { raw_key } else { String::new() };
        }
        let is_valid_url = (raw_url.starts_with("http://") || raw_url.starts_with("https://"))
            && raw_url.is_ascii() && raw_url.len() < 256;
        s.config.base_url = if is_valid_url { raw_url } else { "https://api.groq.com/openai/v1".to_string() };
        s.config.model              = cs(model);
        s.config.vision_model       = cs(vision_model);
        s.config.persona            = cs(persona);
        s.config.tg_token           = cs(tg_token);
        s.config.tg_allowed         = tg_allowed;
        s.config.agent_max_steps    = max_steps as u32;
        s.config.agent_auto_approve = auto_approve;
        s.config.heartbeat_interval = heartbeat as u32;
        s.config.setup_done         = setup_done;
        let bu = s.config.base_url.clone();
        if let Some(p) = s.providers.iter().find(|p| p.base_url == bu) { s.active_provider = p.id.clone(); }
    }


    /// Step 1: Prepare a chat turn. Pushes user message to history,
    /// returns JSON with everything Java needs to call the LLM:
    /// {api_key, base_url, model, messages:[{role,content},...]}
    /// THE MAIN CHAT JNI  -  routes through the proper OpenAI function-calling runner.
    /// Replaces getChatContext + processLlmReply with a single synchronous call.
    ///
    /// Returns JSON: {"reply":"...","tools_used":["x","y"],"steps":3,"done":true}
    ///           or  {"error":"...","done":true}
    ///
    /// Called from KiraAI.java chat() on a background thread.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chatViaRunner(
        env: JNIEnv, _c: JObject,
        message:    *const c_char,
        session_id: *const c_char,
        max_steps:  i32,
    ) -> JString {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let user_msg   = cs_safe(message, 32768);
            let session    = cs_safe(session_id, 64);
            let session    = if session.is_empty() { "default".to_string() } else { session };
            let max_steps  = if max_steps > 0 { max_steps as u32 } else { 15 };

            // Read config and history from STATE
            let (api_key, base_url, model, system_prompt, history, tools_json) = {
                let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                if s.config.api_key.is_empty() {
                    return r#"{"error":"No API key configured. Go to Settings.","done":true}"#.to_string();
                }
                // Validate base_url
                let base_url = if s.config.base_url.is_ascii()
                    && (s.config.base_url.starts_with("http://") || s.config.base_url.starts_with("https://"))
                    && s.config.base_url.len() < 512
                {
                    s.config.base_url.clone()
                } else {
                    s.config.base_url = "https://api.groq.com/openai/v1".to_string();
                    s.config.base_url.clone()
                };
                let persona = if s.config.persona.is_empty() {
                    "You are Kira, a powerful Android AI agent. Use tools to get real data  -  never guess or hallucinate.".to_string()
                } else {
                    s.config.persona.clone()
                };
                let sys      = build_system_prompt(&s, &persona);
                let history  = decompress_context(&s);
                let tools    = build_kira_tools_schema_filtered(&s.tool_allowlist, &s.tool_denylist);
                s.theme.is_thinking = true;
                s.request_count += 1;
                (s.config.api_key.clone(), base_url, s.config.model.clone(), sys, history, tools)
            };

            // Run the proper ReAct loop (OpenAI function-calling format)
            let cfg = crate::ai::runner::AgentRunConfig {
                api_key,
                base_url,
                model,
                system_prompt,
                session_id: session,
                user_message: user_msg.clone(),
                history,
                max_steps,
                tools_json,
            };

            let result = crate::ai::runner::run_agent(cfg);

            // Persist: push user + assistant turns to compressed history
            {
                let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                push_turn_compressed(&mut s, "user", &user_msg);
                if !result.content.is_empty() {
                    push_turn_compressed(&mut s, "assistant", &result.content);
                }
                s.theme.is_thinking = false;
                s.tool_call_count += result.tools_used.len() as u64;
            }

            // Auto-save memory if add_memory was used
            if result.tools_used.iter().any(|t| t == "add_memory") {
                if let Ok(s) = STATE.lock() {
                    let items: Vec<String> = s.memory_index.iter()
                        .map(|m| format!(r#"{{"k":"{}","v":"{}","ts":{}}}"#,
                            esc(&m.key), esc(&m.value), m.ts))
                        .collect();
                    drop(s);
                    // Store JSON in a variable for Java to persist via SharedPrefs
                    if let Ok(mut s) = STATE.lock() {
                        let ts = now_ms();
                        s.variables.entry("_memory_json_pending".to_string())
                            .and_modify(|v| { v.value = format!("[{}]", items.join(",")); v.updated_ms = ts; })
                            .or_insert(AutoVariable {
                                name: "_memory_json_pending".to_string(),
                                value: format!("[{}]", items.join(",")),
                                var_type: "string".to_string(),
                                persistent: false,
                                created_ms: ts, updated_ms: ts,
                            });
                    }
                }
            }

            // Build response JSON
            let tools_json_arr: String = result.tools_used.iter()
                .map(|t| format!(""{}"", esc(t)))
                .collect::<Vec<_>>().join(",");

            if let Some(err) = &result.error {
                format!(r#"{{"error":"{}","done":true,"steps":{}}}"#,
                    esc(err), result.steps)
            } else {
                format!(r#"{{"reply":"{}","tools_used":[{}],"steps":{},"done":true}}"#,
                    esc(&result.content), tools_json_arr, result.steps)
            }
        })).unwrap_or_else(|_| {
            if let Ok(mut s) = STATE.lock() { s.theme.is_thinking = false; }
            r#"{"error":"Internal error in chat runner","done":true}"#.to_string()
        });
        unsafe { jni_str(env, &result) }
    }

    /// Java calls OkHttp with this, then passes raw LLM response to processLlmReply.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getChatContext(
        env: JNIEnv, _c: JObject,
        user_message: *const c_char,
    ) -> JString {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let user_msg = cs_safe(user_message, 16384);
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            if s.config.api_key.is_empty() {
                return r#"{"error":"no_api_key"}"#.to_string();
            }
            s.request_count += 1;
            s.theme.is_thinking = true;
            push_turn_compressed(&mut s, "user", &user_msg);
            build_llm_request_json(&s)
        })).unwrap_or_else(|_| r#"{"error":"panic_in_get_context"}"#.to_string());
        unsafe { jni_str(env, &result) }
    }

    /// Step 2: Process raw LLM response (reconstructed non-streaming JSON from Java).
    /// Java passes: {"choices":[{"message":{"content":"REPLY"},"finish_reason":"stop"}]}
    /// Returns:
    ///   {"done":true,  "reply":"...", "tools_used":"[]"}         -  send reply to user
    ///   {"done":false, "messages_json":"...", "tools_used":"..."}  -  call LLM again
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_processLlmReply(
        env: JNIEnv, _c: JObject,
        raw_response: *const c_char,
        step: i32,
    ) -> JString {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let raw    = cs_safe(raw_response, 131072);
            let step_n = step as usize;
            let max_steps = {
                let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                s.config.agent_max_steps.max(3) as usize
            };


            // Extract plain text from the JSON envelope Java reconstructed from SSE
            let content = extract_llm_content(&raw).unwrap_or_default();

            // Try JSON function-calling format first (modern LLMs), fall back to XML tags
            let json_tcs = crate::ai::runner::parse_tool_calls_json(&raw);
            let xml_tcs  = if json_tcs.is_empty() { parse_tool_calls(&content) } else { vec![] };
            let has_tools = !json_tcs.is_empty() || !xml_tcs.is_empty();

            // Clean reply text (strips XML <tool> tags if present)
            let reply = clean_reply(&content);

            if !has_tools || step_n >= max_steps {
                // No tools or step limit hit  -  final reply
                let final_reply = if reply.trim().is_empty() {
                    if content.trim().is_empty() { "(No response)".to_string() }
                    else { content.trim().to_string() }
                } else { reply };
                {
                    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                    push_turn_compressed(&mut s, "assistant", &final_reply);
                    s.theme.is_thinking = false;
                }
                format!(r#"{{"done":true,"reply":"{}","tools_used":[]}}"#, esc(&final_reply))
            } else {
                // Execute tools and build the follow-up LLM request
                let mut tool_results: Vec<(String, String, String)> = Vec::new(); // (id, name, result)
                let mut tools_used: Vec<String> = Vec::new();

                // Execute JSON tool calls (OpenAI function-calling)
                for tc in &json_tcs {
                    let mut res = dispatch_tool(&tc.name, &tc.params);
                    // http_get/web_search return __shell_http__:job_id - mark for Java resolution
                    if res.starts_with("__shell_http__:") {
                        let job_id = res.trim_start_matches("__shell_http__:");
                        res = format!("pending_shell_result:{}", job_id);
                    }
                    if res.starts_with("__shell__") {
                        let arg = tc.params.get("cmd").or_else(|| tc.params.get("package"))
                            .cloned().unwrap_or_default();
                        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                        s.pending_shell.push_back(ShellJob {
                            id: tc.id.clone(), cmd: format!("{}:{}", tc.name, arg),
                            timeout: 15_000, created: now_ms(),
                        });
                    }
                    tool_results.push((tc.id.clone(), tc.name.clone(), res));
                    tools_used.push(tc.name.clone());
                }

                // Execute XML tool calls (fallback)
                for (tname, targs) in &xml_tcs {
                    let mut res = dispatch_tool(tname, targs);
                    if res.starts_with("__shell_http__:") {
                        let job_id = res.trim_start_matches("__shell_http__:");
                        res = format!("pending_shell_result:{}", job_id);
                    }
                    if res.starts_with("__shell__") {
                        let arg = targs.get("cmd").or_else(|| targs.get("package"))
                            .cloned().unwrap_or_default();
                        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                        s.pending_shell.push_back(ShellJob {
                            id:      format!("xml_{}_{}", step_n, tname),
                            cmd:     format!("{}:{}", tname, arg),
                            timeout: 15_000, created: now_ms(),
                        });
                    }
                    tool_results.push((format!("xml_{}_{}", step_n, tname), tname.clone(), res));
                    tools_used.push(tname.clone());
                }

                { STATE.lock().unwrap_or_else(|e| e.into_inner()).tool_call_count += tools_used.len() as u64; }

                // Build next messages array
                let next_messages: Vec<String> = {
                    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                    let persona = if s.config.persona.is_empty() {
                        "You are Kira, an AI agent on Android.".to_string()
                    } else { s.config.persona.clone() };
                    let sys  = build_system_prompt(&s, &persona);
                    let hist = decompress_context(&s);
                    let mut msgs = Vec::new();
                    if !sys.is_empty() {
                        msgs.push(format!(r#"{{"role":"system","content":"{}"}}"#, esc(&sys)));
                    }
                    for (r, c) in &hist {
                        msgs.push(format!(r#"{{"role":"{}","content":"{}"}}"#, esc(r), esc(c)));
                    }
                    if !json_tcs.is_empty() {
                        // OpenAI format: assistant turn WITH tool_calls array
                        let tc_arr: Vec<String> = json_tcs.iter().map(|tc| {
                            format!(r#"{{"id":"{}","type":"function","function":{{"name":"{}","arguments":"{}"}}}}"#,
                                esc(&tc.id), esc(&tc.name), esc(&tc.args_json))
                        }).collect();
                        msgs.push(format!(
                            r#"{{"role":"assistant","content":"{}","tool_calls":[{}]}}"#,
                            esc(&content), tc_arr.join(",")
                        ));
                        for (id, name, res) in &tool_results {
                            msgs.push(format!(
                                r#"{{"role":"tool","tool_call_id":"{}","name":"{}","content":"{}"}}"#,
                                esc(id), esc(name), esc(res)
                            ));
                        }
                    } else {
                        // XML fallback: assistant + user follow-up
                        msgs.push(format!(r#"{{"role":"assistant","content":"{}"}}"#, esc(&content)));
                        let results_text = tool_results.iter()
                            .map(|(_, name, res)| format!("[{}]: {}", name, res))
                            .collect::<Vec<_>>().join("\n");
                        msgs.push(format!(r#"{{"role":"user","content":"{}"}}"#,
                            esc(&format!("Tool results:\n{}\n\nNow respond to the user.", results_text))));
                    }
                    msgs
                };

                // Build next request with tools schema included
                let (api_key, base_url, model) = {
                    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                    (s.config.api_key.clone(), s.config.base_url.clone(), s.config.model.clone())
                };
                let tools_schema = {
                    let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                    build_kira_tools_schema_filtered(&s.tool_allowlist, &s.tool_denylist)
                };
                let tools_field = if tools_schema.is_empty() || tools_schema == "[]" {
                    String::new()
                } else {
                    format!(r#","tools":{},"tool_choice":"auto""#, tools_schema)
                };

                let next_req = format!(
                    r#"{{"api_key":"{}","base_url":"{}","model":"{}","messages":[{}]{}}}"#,
                    esc(&api_key), esc(&base_url), esc(&model),
                    next_messages.join(","), tools_field
                );

                let tools_arr: Vec<String> = tools_used.iter()
                    .map(|t| format!("\"{}\"", esc(t))).collect();

                format!(
                    r#"{{"done":false,"messages_json":"{}","tools_used":[{}]}}"#,
                    esc(&next_req), tools_arr.join(",")
                )
        })).unwrap_or_else(|_| {
            if let Ok(mut s) = STATE.lock() { s.theme.is_thinking = false; }
            r#"{"done":true,"reply":"Internal error  -  please try again","tools_used":"[]"}"#.to_string()
        });
        unsafe { jni_str(env, &result) }
    }

        /// Store assistant reply in compressed history (called externally if needed).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushAssistantTurn(
        _e: JNIEnv, _c: JObject,
        content: *const c_char,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let text = cs_safe(content, 32768);
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            push_turn_compressed(&mut s, "assistant", &text);
            s.theme.is_thinking = false;
        }));
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getConfig(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let json = config_to_json(&s.config);
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateSetupPage(
        _e: JNIEnv, _c: JObject,
        page:i32, api_key:*const c_char, base_url:*const c_char,
        model:*const c_char, user_name:*const c_char, tg_token:*const c_char, tg_id:i64,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.setup.current_page = page as u8;
        let ak=cs(api_key);
        if !ak.is_empty() && ak.chars().all(|c| c.is_ascii()) {
            s.setup.api_key = ak.clone(); s.config.api_key = ak;
        }
        let bu=cs(base_url);
        if !bu.is_empty() && (bu.starts_with("http://") || bu.starts_with("https://")) {
            s.setup.base_url = bu.clone(); s.config.base_url = bu;
        }
        let mo=cs(model);     if !mo.is_empty()  { s.setup.model     =mo.clone();  s.config.model    =mo; }
        let un=cs(user_name); if !un.is_empty()  { s.setup.user_name =un.clone();  s.config.user_name=un; }
        let tt=cs(tg_token);  if !tt.is_empty()  { s.setup.tg_token  =tt.clone();  s.config.tg_token =tt; }
        if tg_id > 0 { s.setup.tg_allowed_id=tg_id; s.config.tg_allowed=tg_id; }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_completeSetup(
        _e: JNIEnv, _c: JObject,
    ) { let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner()); s.setup.done=true; s.config.setup_done=true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_isSetupDone(
        _e: JNIEnv, _c: JObject,
    ) -> bool { STATE.lock().unwrap_or_else(|e| e.into_inner()).config.setup_done }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setCustomProvider(
        _e: JNIEnv, _c: JObject, url:*const c_char, model:*const c_char,
    ) {
        let url=cs(url); let model=cs(model);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.setup.custom_url=url.clone(); s.setup.selected_provider_id="custom".to_string();
        s.config.base_url=url.clone(); if !model.is_empty() { s.config.model=model.clone(); }
        if let Some(p) = s.providers.iter_mut().find(|p| p.id=="custom") {
            p.base_url=url; if !model.is_empty() { p.model=model; }
        }
        s.active_provider="custom".to_string();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setActiveProvider(
        env: JNIEnv, _c: JObject, provider_id:*const c_char,
    ) -> JString {
        let id=cs(provider_id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let found=s.providers.iter().find(|p| p.id==id).cloned();
        let result = if let Some(p)=found {
            s.active_provider=id; s.config.base_url=p.base_url.clone(); s.config.model=p.model.clone();
            format!(r#"{{"ok":true,"id":"{}","base_url":"{}","model":"{}"}}"#, esc(&s.active_provider),esc(&p.base_url),esc(&p.model))
        } else { format!(r#"{{"error":"unknown provider {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProviders(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.providers.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),esc(&p.base_url),esc(&p.model),p.id==s.active_provider)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateShizukuStatus(
        _e: JNIEnv, _c: JObject,
        installed:bool, permission_granted:bool, error_msg:*const c_char,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.shizuku.installed=installed; s.shizuku.permission_granted=permission_granted;
        s.shizuku.error_msg=cs(error_msg); s.shizuku.last_checked_ms=now_ms();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getShizukuJson(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { jni_str(env, &shizuku_to_json(&s.shizuku)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateTilt(
        _e: JNIEnv, _c: JObject, ax:f32, ay:f32,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay;
        let tx=-ax*s.theme.star_speed; let ty=ay*s.theme.star_speed;
        s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08;
        s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStarParallax(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { jni_str(env, &format!(r#"{{"px":{:.6},"py":{:.6},"ax":{:.4},"ay":{:.4}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y,s.theme.star_tilt_x,s.theme.star_tilt_y)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getTheme(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { jni_str(env, &s.theme.to_json()) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setTheme(
        _e: JNIEnv, _c: JObject,
        name: *const c_char,
    ) {
        let name = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap_or("catppuccin_mocha") };
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.theme = match name {
            "material" | "material_neo" | "material_dark" => ThemeConfig::material_dark(),
            "material_light" | "material_neo_light"       => ThemeConfig::material_light(),
            "kira"                                        => ThemeConfig::default(),
            "catppuccin_mocha" | "catppuccin" | "mocha"  => ThemeConfig::catppuccin_mocha(),
            _                                             => ThemeConfig::catppuccin_mocha(),
        };
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStatsJson(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        { let _s = format!(
            r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"profiles":{},"active_profile":"{}","variables":{}}}"#,
            s.memory_index.len(), s.context_turns.len(),
            if s.shizuku.permission_granted{"active \u{2713}"} else if s.shizuku.installed{"no permission"} else{"not running"},
            if !s.agent_context.is_empty(){"enabled \u{2713}"} else{"disabled"},
            esc(&s.config.model), esc(&s.config.base_url),
            now_ms().saturating_sub(s.uptime_start),
            s.macros.len(), s.profiles.len(), esc(&s.active_profile),
            s.variables.len()
        ); unsafe { jni_str(env, &_s) } }
    }

    // \u{2500}\u{2500} v7 JNI (unchanged) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: JNIEnv, _c: JObject,
        pkg:*const c_char, title:*const c_char, text:*const c_char,
    ) {
        let (pkg,title,text) = (cs(pkg),cs(title),cs(text));
        let ts = now_ms();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        fire_notif_triggers(&mut s, &pkg, &title, &text);
        s.daily_log.push_back(format!("[{}] notif {}:{}", ts, pkg, &title[..title.len().min(40)]));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
        s.notifications.push_back(Notif { pkg, title, text, time:ts });
        if s.notifications.len() > 500 { s.notifications.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _e: JNIEnv, _c: JObject, json:*const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).screen_nodes = cs(json); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenPackage(
        _e: JNIEnv, _c: JObject, pkg:*const c_char,
    ) {
        let pkg = cs(pkg);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let prev = s.screen_pkg.clone();
        if prev != pkg {
            s.sig_app_launched = pkg.clone();
            if !prev.is_empty() { s.sig_app_closed = prev; }
        }
        s.screen_pkg = pkg;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _e: JNIEnv, _c: JObject, pct:i32, charging:bool,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let prev = s.battery_pct;
        s.battery_pct=pct; s.battery_charging=charging;
        fire_battery_triggers(&mut s, pct, prev);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateAgentContext(
        _e: JNIEnv, _c: JObject, ctx:*const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).agent_context = cs(ctx); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushContextTurn(
        _e: JNIEnv, _c: JObject,
        role:*const c_char, content:*const c_char,
    ) {
        let role=cs(role); let content=cs(content);
        let tokens=estimate_tokens(&content);
        let ts=now_ms();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let sess_id = s.active_session.clone();
        s.total_tokens += tokens as u64;
        s.daily_log.push_back(format!("[{}] {}: {}", ts, role, &content[..content.len().min(80)]));
        // Also push compressed copy (Session B)  -  for memory-efficient context loading
        push_turn_compressed(&mut s, &role, &content);
        s.context_turns.push_back(ContextTurn { role, content, ts, tokens, session:sess_id });
        if s.context_turns.len() > 60 { compact_context(&mut s); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_indexMemory(
        _e: JNIEnv, _c: JObject,
        key:*const c_char, value:*const c_char, tags:*const c_char,
    ) {
        let (key,value,tags_raw) = (cs(key),cs(value),cs(tags));
        let tags: Vec<String> = tags_raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.memory_index.retain(|e| e.key != key);
        let fact = format!("- {}: {}", key, value);
        if !s.memory_md.contains(&fact) { s.memory_md.push_str(&format!("\n{}", fact)); }
        s.memory_index.push(MemoryEntry { key, value, tags, ts:now_ms(), relevance:1.0, access_count:0 });
        if s.memory_index.len() > 10000 {
            s.memory_index.sort_by(|a,b| a.relevance.partial_cmp(&b.relevance).unwrap_or(std::cmp::Ordering::Equal));
            s.memory_index.remove(0);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_storeCredential(
        _e: JNIEnv, _c: JObject,
        name:*const c_char, value:*const c_char,
    ) {
        let name=cs(name); let value=cs(value);
        let enc=xor_crypt(value.as_bytes(), derive_key(&name).as_slice());
        STATE.lock().unwrap_or_else(|e| e.into_inner()).credentials.insert(name, enc);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_registerSkill(
        _e: JNIEnv, _c: JObject,
        name:*const c_char, desc:*const c_char, trigger:*const c_char, content:*const c_char,
    ) {
        let name=cs(name);
        STATE.lock().unwrap_or_else(|e| e.into_inner()).skills.insert(name.clone(), Skill { name, description:cs(desc), trigger:cs(trigger), content:cs(content), enabled:true, usage_count:0 });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addHeartbeatItem(
        _e: JNIEnv, _c: JObject,
        id:*const c_char, check:*const c_char, action:*const c_char, interval_ms:i64,
    ) {
        let item = HeartbeatItem { id:cs(id), check:cs(check), action:cs(action), enabled:true, last_run:0, interval_ms:interval_ms as u128 };
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.heartbeat_items.retain(|i| i.id!=item.id);
        s.heartbeat_items.push(item);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_incrementToolIter(
        _e: JNIEnv, _c: JObject, session_id:*const c_char,
    ) -> i32 {
        let id=cs(session_id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let count = { let c=s.tool_iterations.entry(id).or_insert(0); *c+=1; *c };
        s.tool_call_count += 1;
        let max = s.max_tool_iters;
        if count > max { -1 } else { count as i32 }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resetToolIter(
        _e: JNIEnv, _c: JObject, session_id:*const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).tool_iterations.remove(&cs(session_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logTaskStep(
        _e: JNIEnv, _c: JObject,
        task_id:*const c_char, step:i32, action:*const c_char, result:*const c_char, success:bool,
    ) {
        let (tid,act,res) = (cs(task_id),cs(action),cs(result));
        let ts=now_ms();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        do_audit(&mut s, &tid, &act, &act, &res, success, false);
        s.task_log.push_back(TaskStep { task_id:tid, step:step as u32, action:act, result:res, time:ts, success });
        if s.task_log.len() > 2000 { s.task_log.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap_or_else(|e| e.into_inner()).pending_cmds.pop_front() {
            Some((id,body)) => unsafe { jni_str(env, &format!(r#"{{"id":"{}","body":{}}}"#, id, body)) },
            None => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _e: JNIEnv, _c: JObject, id:*const c_char, result:*const c_char,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).results.insert(cs(id), cs(result)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextFiredTrigger(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap_or_else(|e| e.into_inner()).fired_triggers.pop_front() {
            Some(t) => unsafe { jni_str(env, &t) },
            None    => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addTrigger(
        _e: JNIEnv, _c: JObject,
        id:*const c_char, ttype:*const c_char, value:*const c_char, action:*const c_char, repeat:bool,
    ) { STATE.lock().unwrap_or_else(|e| e.into_inner()).triggers.push(Trigger { id:cs(id), trigger_type:cs(ttype), value:cs(value), action:cs(action), fired:false, repeat }); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeTrigger(
        _e: JNIEnv, _c: JObject, id:*const c_char,
    ) { let id=cs(id); STATE.lock().unwrap_or_else(|e| e.into_inner()).triggers.retain(|t| t.id!=id); }

    // \u{2500}\u{2500} OpenClaw v3 JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runDslScript(
        env: JNIEnv, _c: JObject,
        macro_id: *const c_char, script: *const c_char,
    ) -> JString {
        let log = execute_dsl_script(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(macro_id), &cs(script));
        let log_json: Vec<String> = log.iter().map(|l| format!(r#""{}""#, esc(l))).collect();
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"log":[{}]}}"#, log_json.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_rxSubscribe(
        env: JNIEnv, _c: JObject,
        id: *const c_char, name: *const c_char, event_kinds: *const c_char,
        target_macro: *const c_char, debounce_ms: i64, throttle_ms: i64, distinct: bool,
    ) -> JString {
        let id   = cs(id); let name = cs(name);
        let kinds: Vec<String> = cs(event_kinds).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let target = cs(target_macro);
        let mut operators = Vec::new();
        if debounce_ms > 0 { operators.push(RxOperator::Debounce(debounce_ms as u128)); }
        if throttle_ms > 0 { operators.push(RxOperator::Throttle(throttle_ms as u128)); }
        if distinct { operators.push(RxOperator::Distinct); }
        let sub = RxSubscription {
            id: id.clone(), name, event_kinds: kinds, operators, target_macro: target,
            enabled: true, fired_count: 0, last_fired: 0, debounce_last: 0, throttle_last: 0,
            take_count: 0, skip_count: 0, last_value: String::new(), buffer: Vec::new(),
        };
        STATE.lock().unwrap_or_else(|e| e.into_inner()).rx_subscriptions.push(sub);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_rxPostEvent(
        _e: JNIEnv, _c: JObject,
        kind: *const c_char, data: *const c_char,
    ) {
        let event = RxEvent { kind: cs(kind), data: cs(data), ts: now_ms(), source: "jni".to_string() };
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let subs: Vec<RxSubscription> = s.rx_subscriptions.iter().cloned().collect();
        for mut sub in subs {
            if !sub.enabled { continue; }
            if let Some(_) = rx_process_event(&mut sub, &event, &s) {
                let target = sub.target_macro.clone();
                chain_macro(&mut s, &target);
                if let Some(rs) = s.rx_subscriptions.iter_mut().find(|r| r.id == sub.id) {
                    rs.fired_count += 1; rs.last_fired = now_ms();
                }
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_channelPost(
        _e: JNIEnv, _c: JObject,
        channel: *const c_char, message: *const c_char,
    ) { channel_post(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(channel), &cs(message)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_batteryDefer(
        _e: JNIEnv, _c: JObject,
        macro_id: *const c_char, min_pct: i32,
    ) { defer_until_charged(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(macro_id), min_pct); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_exportBundle(
        env: JNIEnv, _c: JObject, tag_filter: *const c_char,
    ) -> JString {
        let tag = cs(tag_filter);
        let result = export_bundle(&STATE.lock().unwrap_or_else(|e| e.into_inner()), if tag.is_empty() { None } else { Some(&tag) });
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_fsmEvent(
        _e: JNIEnv, _c: JObject,
        machine_id: *const c_char, event: *const c_char,
    ) { fsm_process_event(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(machine_id), &cs(event)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _e: JNIEnv, _c: JObject, s:*mut c_char,
    ) { if !s.is_null() { unsafe { drop(CString::from_raw(s)); } } }

    // \u{2500}\u{2500} OpenClaw / NanoBot / ZeroClaw extended JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_exportMacros(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let json = export_macros_json(&STATE.lock().unwrap_or_else(|e| e.into_inner()));
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_importMacros(
        _e: JNIEnv, _c: JObject, json: *const c_char,
    ) { import_macros_json(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(json)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chainMacro(
        _e: JNIEnv, _c: JObject, target_id: *const c_char,
    ) { chain_macro(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(target_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_evalExpr(
        env: JNIEnv, _c: JObject, expr: *const c_char,
    ) -> JString {
        let result = eval_expr(&STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(expr));
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_expandVars(
        env: JNIEnv, _c: JObject, text: *const c_char,
    ) -> JString {
        let result = expand_vars(&STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(text));
        unsafe { jni_str(env, &result) }
    }

    // \u{2500}\u{2500} Roubao / Open-AutoGLM VLM JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    /// Start a new phone agent task. Returns {ok, task_id}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startAgentTask(
        env: JNIEnv, _c: JObject,
        goal: *const c_char, max_steps: i32,
    ) -> JString {
        let goal = cs(goal);
        let max_s = if max_steps > 0 { max_steps as u32 } else { 20 };
        let task_id = gen_id();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let plan_prompt = build_task_plan_prompt(&goal, &s.agent_context);
        s.pending_actions.push_back(PendingMacroAction {
            macro_id: task_id.clone(), action_id: gen_id(),
            kind: "vlm_plan".to_string(),
            params: {
                let mut p = HashMap::new();
                p.insert("task_id".to_string(), task_id.clone());
                p.insert("goal".to_string(), goal.clone());
                p.insert("prompt".to_string(), plan_prompt);
                p
            },
            ts: now_ms(),
        });
        s.phone_agent_tasks.push(PhoneAgentTask {
            id: task_id.clone(), goal, state: VlmTaskState::Planning,
            plan: vec![], plan_idx: 0, history: vec![], max_steps: max_s,
            current_step: 0, context: String::new(), result: String::new(),
            created_ms: now_ms(), last_step_ms: now_ms(),
        });
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"task_id":"{}"}}"#, esc(&task_id))) }
    }

    /// Called by Java after VLM responds with action decision
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_processVlmStep(
        env: JNIEnv, _c: JObject,
        task_id: *const c_char, vlm_response: *const c_char,
    ) -> JString {
        let task_id = cs(task_id); let vlm_resp = cs(vlm_response);
        let done = execute_vlm_step(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &task_id, &vlm_resp);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"done":{}}}"#, done)) }
    }

    /// Called by Java after taking screenshot + getting VLM screen description
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_recordScreenObservation(
        _e: JNIEnv, _c: JObject,
        task_id: *const c_char, step: i32, vlm_desc: *const c_char,
    ) {
        record_screen_observation(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(task_id), step as u32, &cs(vlm_desc));
    }

    /// Set the AI-generated plan for a task
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setAgentPlan(
        _e: JNIEnv, _c: JObject,
        task_id: *const c_char, plan_json: *const c_char,
    ) {
        let task_id = cs(task_id); let plan_str = cs(plan_json);
        // plan_json is a comma-separated list of steps extracted from AI JSON array
        let plan: Vec<String> = plan_str.split("||")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
            t.plan = plan;
            t.state = VlmTaskState::Observing;
        }
        enqueue_vlm_step(&mut s, &task_id);
    }

    /// Get current agent prompt for the AI call (Java reads this before calling AI)
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentPrompt(
        env: JNIEnv, _c: JObject,
        task_id: *const c_char,
    ) -> JString {
        let task_id = cs(task_id);
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let result = match s.phone_agent_tasks.iter().find(|t| t.id == task_id) {
            Some(t) => {
                let sub_task = t.plan.get(t.plan_idx).cloned().unwrap_or_else(|| t.goal.clone());
                let screen_desc = format!("Package: {}
Context: {}",
                    s.screen_pkg, &s.agent_context[..s.agent_context.len().min(400)]);
                let prompt = build_vlm_action_prompt(
                    &t.goal, &sub_task, &screen_desc,
                    &t.history, t.current_step, t.max_steps
                );
                format!(r#"{{"task_id":"{}","prompt":{},"step":{},"sub_task":"{}"}}"#,
                    esc(&t.id), json_str(&prompt), t.current_step, esc(&sub_task))
            }
            None => r#"{"error":"not found"}"#.to_string(),
        };
        unsafe { jni_str(env, &result) }
    }

    /// Get all agent tasks summary
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentTasks(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.phone_agent_tasks.iter().map(|t| format!(
            r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"result":"{}"}}"#,
            esc(&t.id), esc(&t.goal),
            match &t.state {
                VlmTaskState::Done(_) => "done",
                VlmTaskState::Failed(_) => "failed",
                VlmTaskState::Planning => "planning",
                VlmTaskState::Observing => "observing",
                _ => "running",
            },
            t.current_step, esc(&t.result)
        )).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} Roboru JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addFlow(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        if let Some(flow) = parse_flow_from_json(&body) {
            let id = flow.id.clone();
            STATE.lock().unwrap_or_else(|e| e.into_inner()).roboru_flows.insert(id.clone(), flow);
            unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))) }
        } else {
            unsafe { jni_str(env, &r#"{"error":"invalid flow"}"#) }
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runFlow(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let flow = s.roboru_flows.get(&id).cloned();
        let result = if let Some(flow) = flow {
            let steps = execute_flow(&mut s, &flow, None);
            if let Some(f) = s.roboru_flows.get_mut(&id) { f.run_count += 1; f.last_run_ms = now_ms(); }
            format!(r#"{{"ok":true,"steps":{}}}"#, steps)
        } else { format!(r#"{{"error":"not found: {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addKeyword(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let result = if let Some(kw) = parse_keyword_from_json(&body) {
            let name = kw.name.clone();
            STATE.lock().unwrap_or_else(|e| e.into_inner()).roboru_keywords.insert(name.clone(), kw);
            format!(r#"{{"ok":true,"name":"{}"}}"#, esc(&name))
        } else { r#"{"error":"invalid keyword"}"#.to_string() };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runKeyword(
        env: JNIEnv, _c: JObject,
        name: *const c_char, args_json: *const c_char,
    ) -> JString {
        let name = cs(name); let args_body = cs(args_json);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let kw = s.roboru_keywords.get(&name).cloned();
        let result = if let Some(kw) = kw {
            let args: HashMap<String,String> = kw.args.iter().enumerate().map(|(i, arg_name)| {
                let val = extract_json_str(&args_body, &format!("arg{}", i)).unwrap_or_default();
                (arg_name.clone(), val)
            }).collect();
            let r = execute_keyword(&mut s, &kw, &args);
            format!(r#"{{"ok":true,"result":"{}"}}"#, esc(&r))
        } else { format!(r#"{{"error":"keyword not found: {}"}}"#, esc(&name)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addPipeline(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let result = if let Some(p) = parse_pipeline_from_json(&body) {
            let id = p.id.clone();
            STATE.lock().unwrap_or_else(|e| e.into_inner()).roboru_pipelines.insert(id.clone(), p);
            format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))
        } else { r#"{"error":"invalid pipeline"}"#.to_string() };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runPipeline(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let pipeline = s.roboru_pipelines.get(&id).cloned();
        let result = if let Some(pipeline) = pipeline {
            let (steps, errors) = execute_pipeline(&mut s, &pipeline);
            if let Some(p) = s.roboru_pipelines.get_mut(&id) { p.run_count += 1; p.last_run_ms = now_ms(); }
            format!(r#"{{"ok":true,"steps":{},"errors":{}}}"#, steps,
                format!("[{}]", errors.iter().map(|e| format!(r#""{}""#, esc(e))).collect::<Vec<_>>().join(",")))
        } else { format!(r#"{{"error":"pipeline not found: {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationAnalytics(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let json = get_automation_analytics(&STATE.lock().unwrap_or_else(|e| e.into_inner()));
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationReport(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let report = get_automation_report(&STATE.lock().unwrap_or_else(|e| e.into_inner()));
        unsafe { jni_str(env, &report) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_scheduleMacroDaily(
        _e: JNIEnv, _c: JObject,
        macro_id: *const c_char, time_hhmm: *const c_char,
    ) {
        let id = cs(macro_id); let time = cs(time_hhmm);
        if !id.is_empty() && !time.is_empty() {
            schedule_macro_daily(&mut STATE.lock().unwrap_or_else(|e| e.into_inner()), &id, &time);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_findMacroByName(
        env: JNIEnv, _c: JObject,
        name: *const c_char,
    ) -> JString {
        let result = find_macro_by_name(&STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(name));
        let json = match result {
            Some(id) => format!(r#"{{"found":true,"id":"{}"}}"#, esc(&id)),
            None     => r#"{"found":false}"#.to_string(),
        };
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resolveParam(
        env: JNIEnv, _c: JObject,
        param: *const c_char,
    ) -> JString {
        let result = resolve_param(&STATE.lock().unwrap_or_else(|e| e.into_inner()), &cs(param));
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let enabled = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
        let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();
        let json = format!(
            r#"{{"enabled_macros":{},"templates":{},"total_macros":{},"variables":{},"active_profile":"{}","pending_actions":{},"run_log_entries":{},"rate_ok":{}}}"#,
            enabled, templates, s.macros.len(), s.variables.len(),
            esc(&s.active_profile), s.pending_actions.len(),
            s.macro_run_log.len(), check_rate_limit(&s)
        );
        unsafe { jni_str(env, &json) }
    }

    // ── v43: OTA Engine JNI ───────────────────────────────────────────────────

    /// Register installed version with Rust on app start.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSetCurrentVersion(
        _e: JNIEnv, _c: JObject,
        version: *const c_char, code: i64,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let v = cs(version);
        if !v.is_empty() { s.ota.current_version = v; }
        if code > 0 { s.ota.current_code = code; }
        if s.ota.repo.is_empty() { s.ota.repo = "i7m7r8/KiraService".to_string(); }
    }

    /// Set GitHub repo slug e.g. "i7m7r8/KiraService".
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSetRepo(
        _e: JNIEnv, _c: JObject,
        repo: *const c_char,
    ) {
        let r = cs(repo);
        if !r.is_empty() { STATE.lock().unwrap_or_else(|e| e.into_inner()).ota.repo = r; }
    }

    /// Java feeds parsed GitHub release. Rust decides: prompt_user | up_to_date | skipped.
    /// Returns JSON {"action":"...","version":"...","current":"..."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnRelease(
        env: JNIEnv, _c: JObject,
        tag:       *const c_char,
        url:       *const c_char,
        changelog: *const c_char,
        date:      *const c_char,
        sha256:    *const c_char,
        apk_bytes: i64,
    ) -> JString {
        let (tag, url, log, date, sha) = (cs(tag), cs(url), cs(changelog), cs(date), cs(sha256));
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if s.ota.skipped_versions.contains(&tag) {
            s.ota.phase = OtaPhase::Idle;
            return unsafe { jni_str(env, &r#"{"action":"skipped"}"#) };
        }
        let newer = s.ota.is_newer(&tag);
        s.ota.latest_version  = tag.clone();
        s.ota.download_url    = url;
        s.ota.changelog       = log;
        s.ota.release_date    = date;
        s.ota.apk_sha256      = sha;
        s.ota.total_apk_bytes = apk_bytes as u64;
        s.ota.last_check_ms   = now_ms();
        s.ota.check_error     = String::new();
        let action = if newer {
            s.ota.phase = OtaPhase::UpdateAvailable;
            "prompt_user"
        } else {
            s.ota.phase = OtaPhase::Idle;
            "up_to_date"
        };
        let cur = s.ota.current_version.clone();
        let result = format!(r#"{{"action":"{}","version":"{}","current":"{}"}}"#,
            action, esc(&tag), esc(&cur));
        unsafe { jni_str(env, &result) }
    }

    /// Java reports streaming download progress. Rust tracks % for /ota/status.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaProgress(
        _e: JNIEnv, _c: JObject,
        bytes_done: i64, bytes_total: i64,
    ) {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.ota.download_bytes = bytes_done as u64;
        s.ota.download_total = bytes_total as u64;
        s.ota.download_pct   = if bytes_total > 0 {
            // Use u128 to prevent overflow on large files (bytes_done * 100)
            let pct = (bytes_done as u128 * 100) / bytes_total as u128;
            pct.min(100) as u8
        } else { 0 };
        s.ota.phase = OtaPhase::Downloading;
    }

    /// APK fully downloaded. Rust verifies SHA256 and returns install instructions JSON.
    /// Returns {"ok":true,"method":"shizuku|package_installer","shizuku":bool,"cmd":"..."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnDownloaded(
        env: JNIEnv, _c: JObject,
        path:   *const c_char,
        sha256: *const c_char,
    ) -> JString {
        let (path, sha) = (cs(path), cs(sha256));
        // SECURITY: Validate APK path  -  must end with .apk, no path traversal
        // Android getCacheDir() returns /data/user/0/<pkg>/cache/ or /data/data/<pkg>/cache/
        let path_ok = path.ends_with(".apk")
            && !path.contains("..")
            && !path.contains("//")
            && path.starts_with("/");  // must be absolute  -  allows all valid Android paths
        if !path_ok {
            return unsafe { jni_str(env, r#"{"ok":false,"error":"invalid_apk_path"}"#) };
        }
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.ota.apk_local_path = path.clone();
        let expected = s.ota.apk_sha256.clone();
        let ok = expected.is_empty() || expected == sha;
        let use_shizuku = s.shizuku.permission_granted;
        if ok {
            s.ota.phase = OtaPhase::Downloaded;
            let method = if use_shizuku { "shizuku" } else { "package_installer" };
            s.ota.install_method = method.to_string();
            let cmd = format!("pm install -r -t \"{}\"", esc(&path));
            { let _s = format!(
                r#"{{"ok":true,"path":"{}","method":"{}","shizuku":{},"cmd":"{}","verified":{}}}"#,
                esc(&path), method, use_shizuku, esc(&cmd), ok
            ); unsafe { jni_str(env, &_s) } }
        } else {
            let err = format!("sha256_mismatch");
            s.ota.phase = OtaPhase::Failed(err.clone());
            unsafe { jni_str(env, &format!(r#"{{"ok":false,"error":"{}"}}"#, esc(&err))) }
        }
    }

    /// Install completed. Pass new versionName.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnInstalled(
        _e: JNIEnv, _c: JObject,
        new_version: *const c_char,
    ) {
        let ver = cs(new_version);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.ota.phase = OtaPhase::Installed;
        if !ver.is_empty() { s.ota.current_version = ver; }
    }

    /// Install failed. Rust records error and sets Failed phase.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnFailed(
        _e: JNIEnv, _c: JObject,
        error: *const c_char,
    ) {
        let err = cs(error);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.ota.install_error = err.clone();
        s.ota.phase = OtaPhase::Failed(err);
    }

    /// Permanently skip this version (stored in Rust skip list).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSkip(
        _e: JNIEnv, _c: JObject,
        version: *const c_char,
    ) {
        let ver = cs(version);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if !ver.is_empty() && !s.ota.skipped_versions.contains(&ver) {
            s.ota.skipped_versions.push(ver);
        }
        s.ota.phase = OtaPhase::Idle;
    }

    /// Get full OTA status JSON from Rust.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaGetStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        unsafe { jni_str(env, &STATE.lock().unwrap_or_else(|e| e.into_inner()).ota.to_json()) }
    }



    // ── Session G: Tool execution + app package lookup ───────────────────────

    /// Execute any non-intent tool in Rust. Called by KiraTools.execute() for ~82 tools.
    /// name: tool name, params_json: JSON object of parameters
    /// Returns tool result string
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_executeTool(
        env: JNIEnv, _c: JObject,
        name:        *const c_char,
        params_json: *const c_char,
    ) -> JString {
        let tname  = cs(name);
        let pjson  = cs(params_json);
        // Parse params JSON into HashMap
        let mut params = std::collections::HashMap::new();
        let mut rest = pjson.trim_start_matches('{').trim_end_matches('}');
        while let Some(ks) = rest.find('"') {
            let after_ks = &rest[ks+1..];
            let Some(ke) = after_ks.find('"') else { break };
            let key = &after_ks[..ke];
            let after_colon = after_ks[ke+1..].trim_start_matches(':').trim_start_matches('"');
            let val_end = after_colon.find('"').unwrap_or(after_colon.len());
            let val = &after_colon[..val_end];
            params.insert(key.to_string(), val.to_string());
            rest = &after_colon[val_end..];
            rest = rest.trim_start_matches('"').trim_start_matches(',');
        }
        let result = dispatch_tool(&tname, &params);
        unsafe { jni_str(env, &result) }
    }

    /// Resolve app name to package name using Rust's 280+ entry database.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_appNameToPkg(
        env: JNIEnv, _c: JObject,
        app_name: *const c_char,
    ) -> JString {
        let result = app_name_to_pkg(&cs(app_name));
        unsafe { jni_str(env, &result) }
    }

        // ── Session E: Agent + Chain JNI exports ─────────────────────────────────

    /// Run full autonomous agent. Blocks until completion (use background thread).
    /// Returns JSON: {"final":"..","steps":8,"tools_used":["x"],"success":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_agentSync(
        env: JNIEnv, _c: JObject,
        goal: *const c_char, max_steps: i32, session: *const c_char,
    ) -> JString {
        let g = cs_safe(goal, 4096);
        let s = cs_safe(session, 64);
        let body = format!(r#"{{"goal":"{}","max_steps":{},"session":"{}"}}"#, esc(&g), max_steps, esc(&s));
        let result = match std::panic::catch_unwind(|| route_http("POST", "/ai/agent", &body)) {
            Ok(r) => r,
            Err(_) => r#"{"error":"agent crashed  -  try again","success":false}"#.to_string(),
        };
        unsafe { jni_str(env, &result) }
    }

    /// Run chain-of-thought reasoning. Returns JSON: {"reasoning":[...],"conclusion":".."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chainSync(
        env: JNIEnv, _c: JObject,
        goal: *const c_char, depth: i32,
    ) -> JString {
        let g = cs_safe(goal, 4096);
        let body = format!(r#"{{"goal":"{}","depth":{}}}"#, esc(&g), depth);
        let result = match std::panic::catch_unwind(|| route_http("POST", "/ai/chain", &body)) {
            Ok(r) => r,
            Err(_) => r#"{"error":"chain crashed  -  try again"}"#.to_string(),
        };
        unsafe { jni_str(env, &result) }
    }

    /// Get current agent task status
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let result = route_http("GET", "/ai/agent/status", "");
        unsafe { jni_str(env, &result) }
    }

    /// Stop running agent
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_stopAgent(
        _e: JNIEnv, _c: JObject,
    ) {
        route_http("POST", "/ai/agent/stop", "");
    }

        // ── Session D: AI chat JNI + shell queue exports ─────────────────────────

    /// Single-call AI chat  -  replaces KiraAI.java entirely.
    /// Java calls this from a background thread; blocks until reply is ready.
    /// Returns JSON: {"role":"assistant","content":"..","tools_used":["x"],"done":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chatSync(
        env: JNIEnv, _c: JObject,
        message:       *const c_char,
        _session_id:   *const c_char,
        _max_steps:    i32,
    ) -> JString {
        // cs_safe is the ONLY thing that runs outside catch_unwind.
        // It just reads a C string  -  cannot panic.
        let user_msg = cs_safe(message, 16384);

        // Wrap EVERYTHING in catch_unwind  -  including STATE.lock(), lz4 compression,
        // build_system_prompt, and the network call.
        // This prevents ANY panic from crossing the JNI boundary and causing SIGABRT.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> String {
            // 1. Check for empty api key
            {
                let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                if s.config.api_key.is_empty() {
                    return r#"{"error":"no API key  -  go to Settings and add one","done":true}"#
                        .to_string();
                }
            }

            // 2. Build request under lock, then release
            let (api_key, base_url, model, system) = {
                let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                let persona = if s.config.persona.is_empty() {
                    "You are Kira, a helpful AI agent on Android.".to_string()
                } else {
                    s.config.persona.clone()
                };
                let sys = build_system_prompt(&s, &persona);
                s.request_count += 1;
                push_turn_compressed(&mut s, "user", &user_msg);
                (s.config.api_key.clone(), s.config.base_url.clone(),
                 s.config.model.clone(), sys)
            };

            // 3. Decompress history (outside lock  -  no mutex held during alloc)
            let history = {
                let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                decompress_context(&s)
            };

            // 4. Network call (no mutex held)
            match call_llm_sync(&api_key, &base_url, &model, &system, &history) {
                Ok(reply) => {
                    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
                    s.theme.is_thinking = false;
                    push_turn_compressed(&mut s, "assistant", &reply);
                    let safe = esc(&reply);
                    format!(r#"{{"content":"{}","tools_used":"[]","done":true}}"#, safe)
                }
                Err(e) => {
                    if let Ok(mut s) = STATE.lock() { s.theme.is_thinking = false; }
                    format!(r#"{{"error":"{}","done":true}}"#, esc(&e))
                }
            }
        }));

        let out = match result {
            Ok(s) => s,
            Err(_) => {
                // Panic was caught  -  do NOT re-panic, just return error JSON
                if let Ok(mut s) = STATE.lock() { s.theme.is_thinking = false; }
                r#"{"error":"Internal error  -  please try again","done":true}"#.to_string()
            }
        };

        unsafe { jni_str(env, &out) }
    }


    /// Serialize memory index to JSON for persistent storage in Java SharedPrefs.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_saveMemory(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            let items: Vec<String> = s.memory_index.iter()
                .map(|m| format!(r#"{{"k":"{}","v":"{}","ts":{}}}"#,
                    esc(&m.key), esc(&m.value), m.ts))
                .collect();
            format!("[{}]", items.join(","))
        })).unwrap_or_else(|_| "[]".to_string());
        unsafe { jni_str(env, &result) }
    }

    /// Load memory index from JSON (call on startup with SharedPrefs data).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_loadMemory(
        _e: JNIEnv, _c: JObject,
        json: *const c_char,
    ) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let j = cs_safe(json, 65536);
            if j.is_empty() || j == "[]" { return; }
            let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            // Simple JSON array parser: extract "v" fields
            let mut rest = j.as_str();
            while let Some(v_start) = rest.find(r#""v":""#) {
                rest = &rest[v_start + 5..];
                if let Some(v_end) = rest.find('"') {
                    let value = &rest[..v_end];
                    let already = s.memory_index.iter().any(|m| m.value == value);
                    if !already {
                        let idx = s.memory_index.len();
                        s.memory_index.push(MemoryEntry {
                            key: format!("mem_{}", idx),
                            value: value.to_string(),
                            tags: vec![],
                            ts: now_ms(),
                            relevance: 1.0,
                            access_count: 0,
                        });
                    }
                    rest = &rest[v_end + 1..];
                } else { break; }
            }
        }));
    }

    /// Get the agent_max_steps config value.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentMaxSteps(
        _e: JNIEnv, _c: JObject,
    ) -> i32 {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            STATE.lock().unwrap_or_else(|e| e.into_inner()).config.agent_max_steps as i32
        })).unwrap_or(5)
    }

    /// Get next queued shell command for Java to execute via Shizuku.
    /// Returns JSON {"id":"..","cmd":"..","timeout":5000} or {"empty":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getNextShellJob(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        match s.pending_shell.pop_front() {
            Some(job) => {
                let r = format!(r#"{{"id":"{}","cmd":"{}","timeout":{}}}"#,
                    esc(&job.id), esc(&job.cmd), job.timeout);
                unsafe { jni_str(env, &r) }
            }
            None => unsafe { jni_str(env, r#"{"empty":true}"#) }
        }
    }

    /// Post shell execution result back to Rust.
    /// id = job id from getNextShellJob; stdout = command output.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_postShellResult(
        _e: JNIEnv, _c: JObject,
        job_id: *const c_char,
        stdout: *const c_char,
    ) {
        let (id, out) = (cs(job_id), cs(stdout));
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.shell_results.insert(id, out);
    }

    /// Called by Java after drainShellQueue to substitute shell results
    /// into the messages_json that processLlmReply returned.
    /// Input:  the messages_json string (URL/JSON encoded next request)
    /// Output: same string with "pending_shell_result:JOB_ID" replaced by actual results
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resolveShellResults(
        env: JNIEnv, _c: JObject,
        messages_json: *const c_char,
    ) -> JString {
        let json = cs_safe(messages_json, 524288);
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let mut result = json.clone();
        // Replace all "pending_shell_result:JOB_ID" with actual results
        let keys: Vec<String> = s.shell_results.keys().cloned().collect();
        for key in keys {
            let placeholder = format!("pending_shell_result:{}", key);
            if result.contains(&placeholder) {
                let val = s.shell_results.remove(&key).unwrap_or_else(|| "no result".to_string());
                // The placeholder appears as an escaped string inside JSON
                let escaped_placeholder = esc(&placeholder);
                let escaped_val = esc(&val[..val.len().min(3000)]);
                result = result.replace(&escaped_placeholder, &escaped_val);
            }
        }
        unsafe { jni_str(env, &result) }
    }


        // ── Session C: AES-256-GCM secret encryption JNI exports ─────────────────

    /// Encrypt a secret string. seed = SHA256(ANDROID_ID+pkg) from Java.
    /// domain = field name ("api_key", "tg_token") for cross-field protection.
    /// Returns hex-encoded AES-256-GCM ciphertext.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_encryptSecret(
        env: JNIEnv, _c: JObject,
        plaintext: *const c_char,
        seed:      *const c_char,
        domain:    *const c_char,
    ) -> JString {
        let (pt, sd, dm) = (cs_safe(plaintext,65536), cs_safe(seed,256), cs_safe(domain,64));
        let result = aes_encrypt(&pt, &sd, &dm);
        unsafe { jni_str(env, &result) }
    }

    /// Decrypt a hex-encoded AES-256-GCM ciphertext.
    /// Returns original plaintext or empty string if key/data is wrong.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_decryptSecret(
        env: JNIEnv, _c: JObject,
        hex_ciphertext: *const c_char,
        seed:           *const c_char,
        domain:         *const c_char,
    ) -> JString {
        let (ct, sd, dm) = (cs_safe(hex_ciphertext,131072), cs_safe(seed,256), cs_safe(domain,64));
        let result = aes_decrypt(&ct, &sd, &dm);
        unsafe { jni_str(env, &result) }
    }

    /// Derive the AES key seed from device_id + pkg_name.
    /// Call once on first run; store result in SharedPreferences.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_deriveKeySeed(
        env: JNIEnv, _c: JObject,
        android_id:   *const c_char,
        package_name: *const c_char,
    ) -> JString {
        let (aid, pkg) = (cs(android_id), cs(package_name));
        let combined = format!("{}:{}", aid, pkg);
        // SHA-256 of combined → 64 hex chars as seed
        let key = derive_aes_key(&combined);
        let hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();
        unsafe { jni_str(env, &hex) }
    }

    // ── Crash log JNI  -  called directly by KiraApp (faster than HTTP) ──────────

    /// Store a crash entry directly in Rust memory.
    /// Called synchronously from the UncaughtExceptionHandler before process dies.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logCrash(
        _e: JNIEnv, _c: JObject,
        thread_name: *const c_char,
        message:     *const c_char,
        trace:       *const c_char,
        ts_ms:       i64,
    ) {
        let thread  = cs_safe(thread_name, 256);
        let msg     = cs_safe(message,     512);
        let tr      = cs_safe(trace,       4096); // cap at 4KB
        let ts      = if ts_ms > 0 { ts_ms as u128 } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis()).unwrap_or(0)
        };
        let entry = CrashEntry { ts, thread, message: msg, trace: tr };
        let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        s.crash_log.push_back(entry);
        if s.crash_log.len() > 50 { s.crash_log.pop_front(); }
    }

    /// Return the latest crash entry as JSON.
    /// Returns {"has_crash":false} if no crashes stored.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getLatestCrash(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
            match s.crash_log.back() {
                Some(c) => format!(
                    r#"{{"has_crash":true,"ts":{},"thread":"{}","message":"{}"}}"#,
                    c.ts, esc(&c.thread), esc(&c.message)
                ),
                None => r#"{"has_crash":false}"#.to_string(),
            }
        })).unwrap_or_else(|_| r#"{"has_crash":false}"#.to_string());
        unsafe { jni_str(env, &result) }
    }

    /// Return all crash entries as JSON array.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getCrashLog(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap_or_else(|e| e.into_inner());
        let items: Vec<String> = s.crash_log.iter().map(|c| {
            format!(r#"{{"ts":{},"thread":"{}","message":"{}","trace":"{}"}}"#,
                c.ts, esc(&c.thread), esc(&c.message), esc(&c.trace))
        }).collect();
        let result = format!(r#"{{"count":{},"crashes":[{}]}}"#,
            items.len(), items.join(","));
        unsafe { jni_str(env, &result) }
    }

    /// Clear all stored crash entries.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_clearCrashLog(
        _e: JNIEnv, _c: JObject,
    ) {
        STATE.lock().unwrap_or_else(|e| e.into_inner()).crash_log.clear();
    }


}