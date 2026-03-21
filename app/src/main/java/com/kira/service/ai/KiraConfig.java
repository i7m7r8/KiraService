package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;
import android.provider.Settings;
import com.kira.service.RustBridge;

public class KiraConfig {
    private static final String PREFS     = "kira_config";
    private static final String PREFS_KEY = "kira_key_seed";

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

    // ── Key seed ────────────────────────────────────────────────────────────

    private static String getOrCreateSeed(Context ctx) {
        SharedPreferences kp = ctx.getSharedPreferences(PREFS_KEY, Context.MODE_PRIVATE);
        String seed = kp.getString("seed", "");
        if (!seed.isEmpty()) return seed;
        // derive from ANDROID_ID — fully wrapped, never throws
        try {
            String aid = Settings.Secure.getString(
                ctx.getContentResolver(), Settings.Secure.ANDROID_ID);
            if (aid == null || aid.isEmpty()) aid = "kira_fallback_id";
            String derived = RustBridge.deriveKeySeed(aid, ctx.getPackageName());
            if (derived != null && !derived.isEmpty()) seed = derived;
        } catch (Throwable ignored) {}
        if (seed.isEmpty()) seed = "kira_default_seed_fallback_32bytex";
        kp.edit().putString("seed", seed).commit();
        return seed;
    }

    // ── Encrypt / Decrypt ───────────────────────────────────────────────────

    private static String encrypt(String plaintext, String seed, String domain) {
        if (plaintext == null || plaintext.isEmpty()) return "";
        try {
            String result = RustBridge.encryptSecret(plaintext, seed, domain);
            return (result != null && !result.isEmpty()) ? result : plaintext;
        } catch (Throwable e) { return plaintext; }
    }

    private static String decrypt(String hex, String seed, String domain) {
        if (hex == null || hex.isEmpty()) return "";
        if (hex.length() % 2 == 0 && hex.matches("[0-9a-fA-F]+")) {
            try {
                String plain = RustBridge.decryptSecret(hex, seed, domain);
                if (plain != null && !plain.isEmpty()) return plain;
            } catch (Throwable ignored) {}
        }
        return hex; // migration fallback: return as-is
    }

    // ── Load ────────────────────────────────────────────────────────────────

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        KiraConfig c = new KiraConfig();
        c.userName          = p.getString("userName",    "User");
        c.baseUrl           = p.getString("baseUrl",     "https://api.groq.com/openai/v1");
        c.model             = p.getString("model",       "llama-3.1-8b-instant");
        c.visionModel       = p.getString("visionModel", "");
        c.persona           = p.getString("persona",     "");
        c.tgAllowed         = p.getLong  ("tgAllowed",   0);
        c.agentMaxSteps     = p.getInt   ("agentMaxSteps",     25);
        c.agentAutoApprove  = p.getBoolean("agentAutoApprove", true);
        c.heartbeatInterval = p.getInt   ("heartbeatInterval", 30);
        c.setupDone         = p.getBoolean("setupDone",  false);
        c.otaRepo           = p.getString("otaRepo",     "i7m7r8/KiraService");

        // Decrypt sensitive fields — seed derived lazily, fully wrapped
        try {
            String seed     = getOrCreateSeed(ctx);
            String encKey   = p.getString("apiKey_enc", "");
            String plainKey = p.getString("apiKey",     "");
            c.apiKey = !encKey.isEmpty() ? decrypt(encKey, seed, "api_key") : plainKey;
            String encTg    = p.getString("tgToken_enc", "");
            String plainTg  = p.getString("tgToken",     "");
            c.tgToken = !encTg.isEmpty() ? decrypt(encTg, seed, "tg_token") : plainTg;
        } catch (Throwable ignored) {
            // If anything goes wrong with crypto, just use plain values
            c.apiKey  = p.getString("apiKey",  "");
            c.tgToken = p.getString("tgToken", "");
        }
        return c;
    }

    // ── Save ────────────────────────────────────────────────────────────────

    public void save(Context ctx) {
        SharedPreferences.Editor e =
            ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit();
        e.putString ("userName",          userName);
        e.putString ("baseUrl",           baseUrl);
        e.putString ("model",             model);
        e.putString ("visionModel",       visionModel);
        e.putString ("persona",           persona);
        e.putLong   ("tgAllowed",         tgAllowed);
        e.putInt    ("agentMaxSteps",     agentMaxSteps);
        e.putBoolean("agentAutoApprove",  agentAutoApprove);
        e.putInt    ("heartbeatInterval", heartbeatInterval);
        e.putBoolean("setupDone",         setupDone);
        e.putString ("otaRepo",           otaRepo);

        // Encrypt sensitive fields — wrapped, never crashes save()
        try {
            String seed = getOrCreateSeed(ctx);
            if (!apiKey.isEmpty()) {
                e.putString("apiKey_enc", encrypt(apiKey, seed, "api_key"));
                e.remove("apiKey");
            }
            if (!tgToken.isEmpty()) {
                e.putString("tgToken_enc", encrypt(tgToken, seed, "tg_token"));
                e.remove("tgToken");
            }
        } catch (Throwable ex) {
            // Fallback: store plain text
            if (!apiKey.isEmpty())  e.putString("apiKey",   apiKey);
            if (!tgToken.isEmpty()) e.putString("tgToken",  tgToken);
        }

        e.commit(); // synchronous — ensures setupDone written before activity transition

        // Mirror to Rust — best effort, never crashes save()
        try {
            RustBridge.syncConfig(userName, apiKey, baseUrl, model, visionModel,
                persona, tgToken, tgAllowed, agentMaxSteps, agentAutoApprove,
                heartbeatInterval, setupDone);
        } catch (Throwable ignored) {}
    }
}
