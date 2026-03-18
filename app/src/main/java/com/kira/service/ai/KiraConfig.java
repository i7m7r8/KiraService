package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;

public class KiraConfig {
    private static final String PREFS = "kira_config";

    // Core
    public String  userName    = "User";
    public String  apiKey      = "";
    public String  baseUrl     = "https://api.groq.com/openai/v1";
    public String  model       = "llama-3.1-8b-instant";
    // Vision model (ZeroClaw) - e.g. meta-llama/llama-4-scout-17b-16e-instruct
    public String  visionModel = "";
    public String  persona     = "";
    // Telegram
    public String  tgToken     = "";
    public long    tgAllowed   = 0;
    // Agent settings
    public int     agentMaxSteps     = 25;
    public boolean agentAutoApprove  = true;
    public int     heartbeatInterval = 30; // minutes, 0 = disabled
    // Setup
    public boolean setupDone   = false;

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        KiraConfig c = new KiraConfig();
        c.userName         = p.getString("userName",    "User");
        c.apiKey           = p.getString("apiKey",      "");
        c.baseUrl          = p.getString("baseUrl",     "https://api.groq.com/openai/v1");
        c.model            = p.getString("model",       "llama-3.1-8b-instant");
        c.visionModel      = p.getString("visionModel", "");
        c.persona          = p.getString("persona", "");
        c.tgToken          = p.getString("tgToken",     "");
        c.tgAllowed        = p.getLong("tgAllowed",     0);
        c.agentMaxSteps    = p.getInt("agentMaxSteps",  25);
        c.agentAutoApprove = p.getBoolean("agentAutoApprove", true);
        c.heartbeatInterval= p.getInt("heartbeatInterval", 30);
        c.setupDone        = p.getBoolean("setupDone",  false);
        return c;
    }

    public void save(Context ctx) {
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putString("userName",          userName)
            .putString("apiKey",            apiKey)
            .putString("baseUrl",           baseUrl)
            .putString("model",             model)
            .putString("visionModel",       visionModel)
            .putString("persona",           persona)
            .putString("tgToken",           tgToken)
            .putLong("tgAllowed",           tgAllowed)
            .putInt("agentMaxSteps",        agentMaxSteps)
            .putBoolean("agentAutoApprove", agentAutoApprove)
            .putInt("heartbeatInterval",    heartbeatInterval)
            .putBoolean("setupDone",        setupDone)
            .apply();
        // v38: mirror to Rust state so /config + /appstats + /providers stay accurate
        try {
            com.kira.service.RustBridge.syncConfig(
                userName, apiKey, baseUrl, model,
                visionModel, persona, tgToken,
                tgAllowed, agentMaxSteps, agentAutoApprove,
                heartbeatInterval, setupDone
            );
        } catch (UnsatisfiedLinkError ignored) {
            // Rust .so not loaded yet on first cold start — safe to skip
        }
    }
}
