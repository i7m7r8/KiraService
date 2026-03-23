package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;

/**
 * KiraConfig — plain text SharedPreferences storage.
 * v55_fixed: removed AES encryption (was causing API key override bugs).
 */
public class KiraConfig {
    private static final String PREFS   = "kira_config";
    private static final int    VERSION = 6; // v6: stricter api_key validation - reject encrypted garbage

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

    /** Returns true if string contains only printable ASCII chars */
    private static boolean isAscii(String s) {
        if (s == null || s.isEmpty()) return true;
        for (int i = 0; i < s.length(); i++) {
            char ch = s.charAt(i);
            if (ch < 32 || ch > 126) return false;
        }
        return true;
    }

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);

        // One-time migration: remove known encrypted keys from old versions
        if (p.getInt("config_version", 0) < VERSION) {
            SharedPreferences.Editor me = p.edit();
            // Remove old encrypted fields (they contain binary garbage)
            me.remove("apiKey_enc");
            me.remove("tgToken_enc");
            me.remove("baseUrl_enc");
            // If current apiKey fails ASCII check, clear it so user re-enters
            String existingKey = p.getString("apiKey", "");
            if (!isAscii(existingKey)) {
                me.remove("apiKey");
            }
            // If baseUrl is invalid, reset to default
            String existingUrl = p.getString("baseUrl", "");
            if (!existingUrl.isEmpty() && !isAscii(existingUrl)) {
                me.putString("baseUrl", "https://api.groq.com/openai/v1");
            }
            me.putInt("config_version", VERSION);
            me.commit();
        }

        KiraConfig c = new KiraConfig();
        c.userName          = p.getString ("userName",          "User");
        String rawKey = p.getString("apiKey", "");
        // Strict validation: reject encrypted garbage
        // Accept any printable ASCII key of reasonable length
        c.apiKey = (isAscii(rawKey) && !rawKey.isEmpty() && rawKey.length() <= 512)
            ? rawKey : "";
        String rawUrl = p.getString("baseUrl", "https://api.groq.com/openai/v1");
        c.baseUrl = (isAscii(rawUrl) && (rawUrl.startsWith("http://") || rawUrl.startsWith("https://")))
            ? rawUrl : "https://api.groq.com/openai/v1";
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
        e.putInt    ("config_version",    VERSION);
        // Always clear old encrypted keys
        e.remove("apiKey_enc");
        e.remove("tgToken_enc");
        e.commit();

        // Mirror to Rust
        try {
            com.kira.service.RustBridge.syncConfig(
                userName, apiKey, baseUrl, model, visionModel,
                persona, tgToken, tgAllowed, agentMaxSteps, agentAutoApprove,
                heartbeatInterval, setupDone);
        } catch (Throwable ignored) {}
    }
}
