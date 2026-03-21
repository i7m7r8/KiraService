package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;

/**
 * KiraConfig — stores app configuration in SharedPreferences.
 * Plain text storage (no encryption) — encryption was causing
 * key override and crash bugs due to Rust timing issues.
 */
public class KiraConfig {
    private static final String PREFS = "kira_config";

    public String  userName          = "User";
    public String  apiKey            = "";
    public String  baseUrl           = "https://api.groq.com/openai/v1";
    public String  model             = "llama-3.1-8b-instant";
    public String  visionModel       = "";
    public String  persona           = "";
    public String  tgToken           = "";
    public long    tgAllowed         = 0;
    public int     agentMaxSteps     = 25;
    public boolean agentAutoApprove  = true;
    public int     heartbeatInterval = 30;
    public boolean setupDone         = false;
    public String  otaRepo           = "i7m7r8/KiraService";

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        KiraConfig c = new KiraConfig();
        c.userName          = p.getString ("userName",          "User");
        c.apiKey            = p.getString ("apiKey",            "");
        c.baseUrl           = p.getString ("baseUrl",           "https://api.groq.com/openai/v1");
        c.model             = p.getString ("model",             "llama-3.1-8b-instant");
        c.visionModel       = p.getString ("visionModel",       "");
        c.persona           = p.getString ("persona",           "");
        c.tgToken           = p.getString ("tgToken",           "");
        c.tgAllowed         = p.getLong   ("tgAllowed",         0);
        c.agentMaxSteps     = p.getInt    ("agentMaxSteps",     25);
        c.agentAutoApprove  = p.getBoolean("agentAutoApprove",  true);
        c.heartbeatInterval = p.getInt    ("heartbeatInterval", 30);
        c.setupDone         = p.getBoolean("setupDone",         false);
        c.otaRepo           = p.getString ("otaRepo",           "i7m7r8/KiraService");

        // Migration: read from old encrypted keys if plain key is empty
        if (c.apiKey.isEmpty()) {
            String enc = p.getString("apiKey_enc", "");
            if (!enc.isEmpty()) c.apiKey = enc; // best-effort plain fallback
        }
        if (c.tgToken.isEmpty()) {
            String enc = p.getString("tgToken_enc", "");
            if (!enc.isEmpty()) c.tgToken = enc;
        }
        return c;
    }

    public void save(Context ctx) {
        SharedPreferences.Editor e =
            ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit();
        e.putString ("userName",          userName);
        e.putString ("apiKey",            apiKey);
        e.putString ("baseUrl",           baseUrl);
        e.putString ("model",             model);
        e.putString ("visionModel",       visionModel);
        e.putString ("persona",           persona);
        e.putString ("tgToken",           tgToken);
        e.putLong   ("tgAllowed",         tgAllowed);
        e.putInt    ("agentMaxSteps",     agentMaxSteps);
        e.putBoolean("agentAutoApprove",  agentAutoApprove);
        e.putInt    ("heartbeatInterval", heartbeatInterval);
        e.putBoolean("setupDone",         setupDone);
        e.putString ("otaRepo",           otaRepo);
        // Clear old encrypted keys to avoid migration confusion
        e.remove("apiKey_enc");
        e.remove("tgToken_enc");
        e.commit(); // synchronous

        // Mirror to Rust — best effort
        try {
            com.kira.service.RustBridge.syncConfig(
                userName, apiKey, baseUrl, model, visionModel,
                persona, tgToken, tgAllowed, agentMaxSteps, agentAutoApprove,
                heartbeatInterval, setupDone);
        } catch (Throwable ignored) {}
    }
}
