package com.kira.service;

/**
 * JNI bridge to Kira Rust Core v9 (v40 edition).
 *
 * v40 additions \u2014 full Tasker/MacroDroid-equivalent automation engine in Rust:
 *   Macro management:   addMacro, removeMacro, enableMacro, getMacros,
 *                       runMacroNow, nextMacroAction, getMacroRunLog
 *   Variables engine:   setVariable, getVariable, getVariables
 *   Profiles:           setProfile, getProfiles
 *   Device signals:     signalScreenOn/Off, signalUnlocked/Locked, signalShake,
 *                       signalVolumeUp/Down, signalWifi, signalBluetooth,
 *                       signalSms, signalCall, signalNfc, signalClipboard,
 *                       signalAppLaunched, signalAppClosed, signalLocation,
 *                       signalKiraEvent
 *
 * All v38 methods (syncConfig, getConfig, updateSetupPage, completeSetup,
 * isSetupDone, setCustomProvider, setActiveProvider, getProviders,
 * updateShizukuStatus, getShizukuJson, updateTilt, getStarParallax,
 * getTheme, getStatsJson) are preserved unchanged.
 */
public class RustBridge {
    static { System.loadLibrary("kira_core"); }

    // \u2500\u2500 Lifecycle \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void startServer(int port);

    // \u2500\u2500 v40: Device signal injectors \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    /** Call from BroadcastReceiver / AccessibilityService when screen turns on */
    public static native void signalScreenOn();
    /** Call when screen turns off */
    public static native void signalScreenOff();
    /** Call from KeyguardManager listener when device unlocked */
    public static native void signalUnlocked();
    /** Call when device locked */
    public static native void signalLocked();
    /** Call from SensorEventListener when shake detected */
    public static native void signalShake();
    /** Call when volume-up button pressed */
    public static native void signalVolumeUp();
    /** Call when volume-down button pressed */
    public static native void signalVolumeDown();
    /** Call when wifi connects; pass SSID or "" for disconnect */
    public static native void signalWifi(String ssid);
    /** Call when BT device connects; pass device name or "" for disconnect */
    public static native void signalBluetooth(String device);
    /** Call when SMS received */
    public static native void signalSms(String sender, String text);
    /** Call when incoming call */
    public static native void signalCall(String number);
    /** Call when NFC tag scanned */
    public static native void signalNfc(String tagId);
    /** Call when clipboard changes */
    public static native void signalClipboard(String text);
    /** Call when foreground app changes \u2014 Rust also auto-signals from updateScreenPackage */
    public static native void signalAppLaunched(String packageName);
    /** Call when app is closed */
    public static native void signalAppClosed(String packageName);
    /** Call from location service. geofence = "enter:label" / "exit:label" / "" */
    public static native void signalLocation(double lat, double lon, String geofence);
    /** Fire an internal Kira event (for KiraEvent / KiraCommand triggers) */
    public static native void signalKiraEvent(String event);

    // \u2500\u2500 v40: Macro management \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    /**
     * Add or replace a macro. JSON format:
     * {
     *   "id": "...",          // optional, auto-generated if missing
     *   "name": "My Macro",
     *   "description": "...",
     *   "enabled": true,
     *   "profile": "",        // "" = any profile
     *   "tags": "tag1,tag2",
     *   "triggers": [{"kind":"screen_on","enabled":true,"config":{}}],
     *   "conditions": [{"lhs":"%BATTERY%","op":"lte","rhs":"20"}],
     *   "actions": [{"kind":"show_toast","enabled":true,"params":{"message":"Hi!"},"sub_actions":[]}]
     * }
     * Returns: {"ok":true,"id":"..."}
     */
    public static native String addMacro(String json);

    /** Remove macro by ID */
    public static native void removeMacro(String id);

    /** Enable or disable a macro */
    public static native void enableMacro(String id, boolean enabled);

    /** Returns JSON array of all macros */
    public static native String getMacros();

    /**
     * Run a macro immediately (ignores triggers/conditions).
     * Returns: {"ok":true,"steps":N}
     */
    public static native String runMacroNow(String id);

    /**
     * Poll the pending action queue \u2014 call this in a tight loop from your
     * foreground service. Returns null when queue is empty.
     * Each result is:
     * {
     *   "macro_id": "...", "action_id": "...", "kind": "open_app",
     *   "ts": 1234567890, "params": {"package":"com.example"}
     * }
     * Java must execute the action and may call signalKiraEvent("action_done:...")
     * to chain into further macros.
     */
    public static native String nextMacroAction();

    /** Returns last 100 macro run log entries as JSON array */
    public static native String getMacroRunLog();

    // \u2500\u2500 v40: Variable engine \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    /**
     * Set a named variable. type: "string" | "number" | "boolean"
     * Variables can be referenced in action params as %VARIABLE_NAME%
     */
    public static native void setVariable(String name, String value, String type);

    /** Returns {"name":"...","value":"...","type":"..."} or {"error":"not_found"} */
    public static native String getVariable(String name);

    /** Returns JSON array of all variables */
    public static native String getVariables();

    // \u2500\u2500 v40: Profile management \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    /**
     * Activate a profile by ID. Built-in: "default", "work", "home", "sleep", "car"
     * Macros can be scoped to a profile via their "profile" field.
     */
    public static native void setProfile(String id);

    /** Returns JSON array of all profiles with active flag */
    public static native String getProfiles();

    // \u2500\u2500 v38: Config sync (Java SharedPrefs \u2194 Rust state) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void syncConfig(
        String userName, String apiKey, String baseUrl, String model,
        String visionModel, String persona, String tgToken,
        long tgAllowed, int maxSteps, boolean autoApprove,
        int heartbeat, boolean setupDone
    );
    public static native String getConfig();

    // \u2500\u2500 v38: Setup wizard state \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void updateSetupPage(
        int page, String apiKey, String baseUrl, String model,
        String userName, String tgToken, long tgId
    );
    public static native void completeSetup();
    public static native boolean isSetupDone();

    // \u2500\u2500 v38: Custom provider \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void   setCustomProvider(String url, String model);
    public static native String setActiveProvider(String providerId);
    public static native String getProviders();

    // \u2500\u2500 v38: Shizuku status \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void   updateShizukuStatus(boolean installed, boolean permissionGranted, String errorMsg);
    public static native String getShizukuJson();

    // \u2500\u2500 v38: Sensor / star field \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void   updateTilt(float ax, float ay);
    public static native String getStarParallax();
    public static native String getTheme();

    // \u2500\u2500 v38: Local stats \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native String getStatsJson();

    // \u2500\u2500 v7: Device state \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateScreenPackage(String pkg);
    public static native void updateBattery(int pct, boolean charging);
    public static native void updateAgentContext(String context);

    // \u2500\u2500 v7: Context engine \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void pushContextTurn(String role, String content);

    // \u2500\u2500 v7: Memory \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void indexMemory(String key, String value, String tags);

    // \u2500\u2500 v7: Credentials \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void storeCredential(String name, String value);

    // \u2500\u2500 v7: Skills \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void registerSkill(String name, String description, String trigger, String content);

    // \u2500\u2500 v7: Heartbeat \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void addHeartbeatItem(String id, String check, String action, long intervalMs);

    // \u2500\u2500 v7: Tool iteration counter \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native int  incrementToolIter(String sessionId);
    public static native void resetToolIter(String sessionId);

    // \u2500\u2500 v7: Task log \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void logTaskStep(String taskId, int step, String action, String result, boolean success);

    // \u2500\u2500 v7: Command queue \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native String nextCommand();
    public static native void   pushResult(String id, String result);

    // \u2500\u2500 v7: Triggers \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void   addTrigger(String id, String type, String value, String action, boolean repeat);
    public static native void   removeTrigger(String id);
    public static native String nextFiredTrigger();

    // \u2500\u2500 Utility \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    public static native void freeString(String s);

    // \u2500\u2500 OpenClaw / NanoBot / ZeroClaw extended automation \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Export all macros as JSON string (backup / share) */
    public static native String exportMacros();

    /** Import macros from JSON (merged, not wiped) */
    public static native void importMacros(String json);

    /**
     * Chain-trigger another macro by ID.
     * Respects cooldown + rate limiter.
     */
    public static native void chainMacro(String targetId);

    /**
     * Evaluate a %VAR% expression.
     * e.g. evalExpr("5 + %MY_NUM%") \u2192 "8"
     */
    public static native String evalExpr(String expression);

    /** Expand %VAR% tokens in a string */
    public static native String expandVars(String text);

    /**
     * Get full automation status JSON:
     * {enabled_macros, templates, total_macros, variables,
     *  active_profile, pending_actions, run_log_entries, rate_ok}
     */
    public static native String getAutomationStatus();


    // \u2500\u2500 OpenClaw v2: Advanced automation \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Analytics: runs per hour/day, success rate, most active macro */
    public static native String getAutomationAnalytics();

    /** Full text automation report (for AI to summarize) */
    public static native String getAutomationReport();

    /** Schedule macro to run daily at HH:MM */
    public static native void scheduleMacroDaily(String macroId, String timeHHMM);

    /** Find macro by name (fuzzy match) \u2014 returns {found, id} */
    public static native String findMacroByName(String name);

    /** Resolve %VAR% tokens and math expressions in a string */
    public static native String resolveParam(String param);


    // \u2500\u2500 Roboru / E-Robot / Automate visual automation engine \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Add or replace a visual flowchart flow. Returns {ok, id} */
    public static native String addFlow(String json);
    /** Run a flow by ID. Returns {ok, steps} */
    public static native String runFlow(String id);

    /** Add a keyword (Robot Framework pattern). Returns {ok, name} */
    public static native String addKeyword(String json);
    /** Run a keyword. argsJson: {"arg0":"val0","arg1":"val1"} */
    public static native String runKeyword(String name, String argsJson);

    /** Add a hyper-automation pipeline. Returns {ok, id} */
    public static native String addPipeline(String json);
    /** Run a pipeline by ID. Returns {ok, steps, errors:[]} */
    public static native String runPipeline(String id);


    // \u2500\u2500 Roubao / Open-AutoGLM VLM Phone Agent \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /**
     * Start a VLM-guided phone agent task (Open-AutoGLM pattern).
     * Returns: {"ok":true,"task_id":"..."}
     * Java must poll nextMacroAction() for "vlm_plan" and "vlm_observe" actions,
     * call the AI with the prompt, then call setAgentPlan() or processVlmStep().
     */
    public static native String startAgentTask(String goal, int maxSteps);

    /** Feed VLM action response back to Rust. Returns {"ok":true,"done":bool} */
    public static native String processVlmStep(String taskId, String vlmResponseJson);

    /** Record screen observation from VLM screenshot analysis */
    public static native void recordScreenObservation(String taskId, int step, String vlmDescription);

    /** Set AI-generated plan. planSteps: "step1||step2||step3" (pipe-separated) */
    public static native void setAgentPlan(String taskId, String planSteps);

    /** Get the VLM prompt for the current step. Java passes this to AI. */
    public static native String getAgentPrompt(String taskId);

    /** Get all phone agent tasks as JSON array */
    public static native String getAgentTasks();


    // \u2500\u2500 OpenClaw v3 / NanoBot / ZeroClaw extended automation \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Execute a NanoBot DSL script. Returns {"ok":true,"log":["..."]} */
    public static native String runDslScript(String macroId, String script);

    /** Subscribe to reactive event stream. Returns {"ok":true,"id":"..."} */
    public static native String rxSubscribe(String id, String name, String eventKinds,
        String targetMacro, long debounceMs, long throttleMs, boolean distinct);

    /** Post an event to the reactive stream */
    public static native void rxPostEvent(String kind, String data);

    /** Post a message to a named channel (cross-macro communication) */
    public static native void channelPost(String channel, String message);

    /** Defer a macro until battery recovers to minPct */
    public static native void batteryDefer(String macroId, int minPct);

    /** Export macros/keywords as a bundle JSON */
    public static native String exportBundle(String tagFilter);

    /** Process a state machine event */
    public static native void fsmEvent(String machineId, String event);

}