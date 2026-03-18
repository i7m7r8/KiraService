package com.kira.service;

/**
 * JNI bridge to Kira Rust Core v8 (v38 edition).
 *
 * v38 additions — all previously-Java logic now in Rust:
 *   syncConfig          — push KiraConfig to Rust on every save
 *   getConfig           — read config JSON from Rust
 *   updateSetupPage     — wizard page state lives in Rust
 *   completeSetup       — mark setup done in Rust
 *   isSetupDone         — check setup flag from Rust
 *   setCustomProvider   — register custom URL/model in Rust provider registry
 *   setActiveProvider   — switch provider by ID, Rust returns new base_url+model
 *   getProviders        — full provider list JSON from Rust (17+ entries)
 *   updateShizukuStatus — push Shizuku binder/permission state to Rust
 *   getShizukuJson      — read Shizuku status JSON
 *   updateTilt          — push accelerometer → Rust smooths parallax
 *   getStarParallax     — read smoothed parallax for star field drawing
 *   getTheme            — read crimson neon colour constants from Rust
 *   getStatsJson        — local stats (replaces all localhost:7070 UI calls)
 */
public class RustBridge {
    static { System.loadLibrary("kira_core"); }

    // ── Lifecycle ─────────────────────────────────────────────────────────────
    public static native void startServer(int port);

    // ── Device state ──────────────────────────────────────────────────────────
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateScreenPackage(String pkg);
    public static native void updateBattery(int pct, boolean charging);
    public static native void updateAgentContext(String context);

    // ── Context engine ────────────────────────────────────────────────────────
    public static native void pushContextTurn(String role, String content);

    // ── Memory ────────────────────────────────────────────────────────────────
    public static native void indexMemory(String key, String value, String tags);

    // ── Credentials ───────────────────────────────────────────────────────────
    public static native void storeCredential(String name, String value);

    // ── Skills ────────────────────────────────────────────────────────────────
    public static native void registerSkill(String name, String description, String trigger, String content);

    // ── Heartbeat ─────────────────────────────────────────────────────────────
    public static native void addHeartbeatItem(String id, String check, String action, long intervalMs);

    // ── Tool iteration counter ────────────────────────────────────────────────
    public static native int  incrementToolIter(String sessionId);
    public static native void resetToolIter(String sessionId);

    // ── Task log ──────────────────────────────────────────────────────────────
    public static native void logTaskStep(String taskId, int step, String action, String result, boolean success);

    // ── Command queue ─────────────────────────────────────────────────────────
    public static native String nextCommand();
    public static native void   pushResult(String id, String result);

    // ── Triggers ──────────────────────────────────────────────────────────────
    public static native void   addTrigger(String id, String type, String value, String action, boolean repeat);
    public static native void   removeTrigger(String id);
    public static native String nextFiredTrigger();

    // ── Memory ────────────────────────────────────────────────────────────────
    public static native void   freeString(String s);

    // ── v38: Config sync (Java SharedPrefs ↔ Rust state) ─────────────────────
    /**
     * Push all KiraConfig fields to Rust after every cfg.save().
     * Rust becomes source of truth for /config, /providers, /appstats.
     */
    public static native void syncConfig(
        String userName, String apiKey, String baseUrl, String model,
        String visionModel, String persona, String tgToken,
        long tgAllowed, int maxSteps, boolean autoApprove,
        int heartbeat, boolean setupDone
    );
    /** Returns JSON: {user_name, api_key_set, base_url, model, ...} */
    public static native String getConfig();

    // ── v38: Setup wizard state ───────────────────────────────────────────────
    /**
     * Called on each wizard page advance. Rust stores intermediate values.
     * Pass empty strings for fields not yet collected.
     */
    public static native void updateSetupPage(
        int page, String apiKey, String baseUrl, String model,
        String userName, String tgToken, long tgId
    );
    /** Mark setup as completed in Rust state. */
    public static native void completeSetup();
    /** True if Rust state has setup_done = true. */
    public static native boolean isSetupDone();

    // ── v38: Custom provider ──────────────────────────────────────────────────
    /**
     * Register a custom provider URL (from the "Custom ✎" chip in setup).
     * Rust updates the "custom" entry in the provider registry and
     * sets it as active provider, returning updated JSON.
     */
    public static native void   setCustomProvider(String url, String model);
    /**
     * Switch active provider by ID. Rust updates cfg.base_url + cfg.model
     * and returns JSON: {ok, id, base_url, model}.
     */
    public static native String setActiveProvider(String providerId);
    /** Returns JSON array of all providers (17+) with active flag. */
    public static native String getProviders();

    // ── v38: Shizuku status reporting ─────────────────────────────────────────
    /**
     * Called from MainActivity after every Shizuku check.
     * Rust stores the status; /shizuku HTTP endpoint and /appstats read it.
     */
    public static native void   updateShizukuStatus(boolean installed, boolean permissionGranted, String errorMsg);
    /** Returns JSON: {installed, permission_granted, status, last_checked_ms} */
    public static native String getShizukuJson();

    // ── v38: Sensor / star field parallax ────────────────────────────────────
    /**
     * Called from SetupActivity.onSensorChanged (TYPE_ACCELEROMETER).
     * Rust applies EMA smoothing and stores the parallax offset.
     */
    public static native void   updateTilt(float ax, float ay);
    /**
     * Read smoothed parallax back for StarFieldView.onDraw().
     * Returns JSON: {px, py, ax, ay}
     */
    public static native String getStarParallax();
    /** Returns theme colour JSON: {accent, bg, card, muted, star_count} */
    public static native String getTheme();

    // ── v38: Local stats (replaces localhost:7070/health in UI) ──────────────
    /**
     * Returns a JSON stats snapshot sourced entirely from Rust state.
     * Used by MainActivity's "refresh stats" panel instead of HTTP.
     * Returns: {facts, history, shizuku, accessibility, model, provider, uptime_ms}
     */
    public static native String getStatsJson();
}
