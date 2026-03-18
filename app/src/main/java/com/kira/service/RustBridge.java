package com.kira.service;

/**
 * JNI bridge to Kira Rust Core v9 (v40 edition).
 *
 * v40 additions — full Tasker/MacroDroid-equivalent automation engine in Rust:
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

    // ── Lifecycle ──────────────────────────────────────────────────────────────
    public static native void startServer(int port);

    // ── v40: Device signal injectors ──────────────────────────────────────────
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
    /** Call when foreground app changes — Rust also auto-signals from updateScreenPackage */
    public static native void signalAppLaunched(String packageName);
    /** Call when app is closed */
    public static native void signalAppClosed(String packageName);
    /** Call from location service. geofence = "enter:label" / "exit:label" / "" */
    public static native void signalLocation(double lat, double lon, String geofence);
    /** Fire an internal Kira event (for KiraEvent / KiraCommand triggers) */
    public static native void signalKiraEvent(String event);

    // ── v40: Macro management ─────────────────────────────────────────────────
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
     * Poll the pending action queue — call this in a tight loop from your
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

    // ── v40: Variable engine ──────────────────────────────────────────────────
    /**
     * Set a named variable. type: "string" | "number" | "boolean"
     * Variables can be referenced in action params as %VARIABLE_NAME%
     */
    public static native void setVariable(String name, String value, String type);

    /** Returns {"name":"...","value":"...","type":"..."} or {"error":"not_found"} */
    public static native String getVariable(String name);

    /** Returns JSON array of all variables */
    public static native String getVariables();

    // ── v40: Profile management ───────────────────────────────────────────────
    /**
     * Activate a profile by ID. Built-in: "default", "work", "home", "sleep", "car"
     * Macros can be scoped to a profile via their "profile" field.
     */
    public static native void setProfile(String id);

    /** Returns JSON array of all profiles with active flag */
    public static native String getProfiles();

    // ── v38: Config sync (Java SharedPrefs ↔ Rust state) ──────────────────────
    public static native void syncConfig(
        String userName, String apiKey, String baseUrl, String model,
        String visionModel, String persona, String tgToken,
        long tgAllowed, int maxSteps, boolean autoApprove,
        int heartbeat, boolean setupDone
    );
    public static native String getConfig();

    // ── v38: Setup wizard state ───────────────────────────────────────────────
    public static native void updateSetupPage(
        int page, String apiKey, String baseUrl, String model,
        String userName, String tgToken, long tgId
    );
    public static native void completeSetup();
    public static native boolean isSetupDone();

    // ── v38: Custom provider ──────────────────────────────────────────────────
    public static native void   setCustomProvider(String url, String model);
    public static native String setActiveProvider(String providerId);
    public static native String getProviders();

    // ── v38: Shizuku status ───────────────────────────────────────────────────
    public static native void   updateShizukuStatus(boolean installed, boolean permissionGranted, String errorMsg);
    public static native String getShizukuJson();

    // ── v38: Sensor / star field ──────────────────────────────────────────────
    public static native void   updateTilt(float ax, float ay);
    public static native String getStarParallax();
    public static native String getTheme();

    // ── v38: Local stats ──────────────────────────────────────────────────────
    public static native String getStatsJson();

    // ── v7: Device state ──────────────────────────────────────────────────────
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateScreenPackage(String pkg);
    public static native void updateBattery(int pct, boolean charging);
    public static native void updateAgentContext(String context);

    // ── v7: Context engine ────────────────────────────────────────────────────
    public static native void pushContextTurn(String role, String content);

    // ── v7: Memory ───────────────────────────────────────────────────────────
    public static native void indexMemory(String key, String value, String tags);

    // ── v7: Credentials ───────────────────────────────────────────────────────
    public static native void storeCredential(String name, String value);

    // ── v7: Skills ────────────────────────────────────────────────────────────
    public static native void registerSkill(String name, String description, String trigger, String content);

    // ── v7: Heartbeat ─────────────────────────────────────────────────────────
    public static native void addHeartbeatItem(String id, String check, String action, long intervalMs);

    // ── v7: Tool iteration counter ────────────────────────────────────────────
    public static native int  incrementToolIter(String sessionId);
    public static native void resetToolIter(String sessionId);

    // ── v7: Task log ──────────────────────────────────────────────────────────
    public static native void logTaskStep(String taskId, int step, String action, String result, boolean success);

    // ── v7: Command queue ─────────────────────────────────────────────────────
    public static native String nextCommand();
    public static native void   pushResult(String id, String result);

    // ── v7: Triggers ──────────────────────────────────────────────────────────
    public static native void   addTrigger(String id, String type, String value, String action, boolean repeat);
    public static native void   removeTrigger(String id);
    public static native String nextFiredTrigger();

    // ── Utility ───────────────────────────────────────────────────────────────
    public static native void freeString(String s);

    // ── OpenClaw / NanoBot / ZeroClaw extended automation ────────────────────

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
     * e.g. evalExpr("5 + %MY_NUM%") → "8"
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

}