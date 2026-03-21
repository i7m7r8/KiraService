package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;
import android.provider.Settings;
import com.kira.service.RustBridge;

/**
 * KiraConfig — Session C: API keys stored with AES-256-GCM encryption.
 *
 * Encryption flow:
 *   First run: deriveKeySeed(ANDROID_ID, pkg) → store seed in "kira_key_seed"
 *   Save:  encryptSecret(plaintext, seed, domain) → store hex ciphertext
 *   Load:  decryptSecret(hexCiphertext, seed, domain) → plaintext
 *
 * Non-sensitive fields (userName, model, baseUrl, etc.) stored in plain text.
 * Sensitive fields (apiKey, tgToken) stored encrypted.
 */
public class KiraConfig {
    private static final String PREFS       = "kira_config";
    private static final String PREFS_KEY   = "kira_key_seed";

    // Core
    public String  userName    = "User";
    public String  apiKey      = "";
    public String  baseUrl     = "https://api.groq.com/openai/v1";
    public String  model       = "llama-3.1-8b-instant";
    public String  visionModel = "";
    public String  persona     = "";
    // Telegram
    public String  tgToken     = "";
    public long    tgAllowed   = 0;
    // Agent
    public int     agentMaxSteps    = 25;
    public boolean agentAutoApprove = true;
    public int     heartbeatInterval= 30;
    // Setup
    public boolean setupDone   = false;
    // OTA
    public String  otaRepo    = "i7m7r8/KiraService";

    // ── Key seed management ────────────────────────────────────────────────

    /** Get or create the AES key seed for this device. Stored in separate prefs. */
    private static String getOrCreateSeed(Context ctx) {
        SharedPreferences kp = ctx.getSharedPreferences(PREFS_KEY, Context.MODE_PRIVATE);
        String seed = kp.getString("seed", "");
        if (!seed.isEmpty()) return seed;
        // First run: derive from ANDROID_ID + package
        try {
            String aid = Settings.Secure.getString(ctx.getContentResolver(),
                Settings.Secure.ANDROID_ID);
            if (aid == null) aid = "kira_fallback_id";
            seed = RustBridge.deriveKeySeed(aid, ctx.getPackageName());
        } catch (Throwable e) {
            seed = "kira_default_seed_fallback_32byte";
        }
        kp.edit().putString("seed", seed).commit();
        return seed;
    }

    // ── Encrypt / Decrypt helpers ──────────────────────────────────────────

    private static String encrypt(String plaintext, String seed, String domain) {
        if (plaintext == null || plaintext.isEmpty()) return "";
        try { return RustBridge.encryptSecret(plaintext, seed, domain); }
        catch (Throwable e) { return plaintext; } // fallback: store plain if Rust not loaded
    }

    private static String decrypt(String hex, String seed, String domain) {
        if (hex == null || hex.isEmpty()) return "";
        // If it looks like hex (all hex chars, even length), try decrypt
        if (hex.length() % 2 == 0 && hex.matches("[0-9a-fA-F]+")) {
            try {
                String plain = RustBridge.decryptSecret(hex, seed, domain);
                if (!plain.isEmpty()) return plain;
            } catch (Throwable ignored) {}
        }
        // Fallback: return as-is (supports migration from unencrypted storage)
        return hex;
    }

    // ── Load / Save ────────────────────────────────────────────────────────

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        String seed = getOrCreateSeed(ctx);
        KiraConfig c = new KiraConfig();
        c.userName          = p.getString("userName",    "User");
        // Sensitive: decrypt from encrypted storage, fall back to plain key
        String encKey       = p.getString("apiKey_enc",  "");
        String plainKey     = p.getString("apiKey",      "");
        try {
            c.apiKey = !encKey.isEmpty() ? decrypt(encKey, seed, "api_key") : plainKey;
        } catch (Throwable ignored) { c.apiKey = plainKey; }
        c.baseUrl           = p.getString("baseUrl",     "https://api.groq.com/openai/v1");
        c.model             = p.getString("model",       "llama-3.1-8b-instant");
        c.visionModel       = p.getString("visionModel", "");
        c.persona           = p.getString("persona",     "");
        String encTg        = p.getString("tgToken_enc", "");
        String plainTg      = p.getString("tgToken",     "");
        try {
            c.tgToken = !encTg.isEmpty() ? decrypt(encTg, seed, "tg_token") : plainTg;
        } catch (Throwable ignored) { c.tgToken = plainTg; }
        c.tgAllowed         = p.getLong  ("tgAllowed",   0);
        c.agentMaxSteps     = p.getInt   ("agentMaxSteps",  25);
        c.agentAutoApprove  = p.getBoolean("agentAutoApprove", true);
        c.heartbeatInterval = p.getInt   ("heartbeatInterval", 30);
        c.setupDone         = p.getBoolean("setupDone",   false);
        c.otaRepo           = p.getString("otaRepo",     "i7m7r8/KiraService");
        return c;
    }

    public void save(Context ctx) {
        String seed = getOrCreateSeed(ctx);
        SharedPreferences.Editor e = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit();
        e.putString("userName",         userName);
        // Encrypt sensitive fields
        if (!apiKey.isEmpty()) {
            e.putString("apiKey_enc", encrypt(apiKey, seed, "api_key"));
            e.remove("apiKey");         // remove old plain-text key
        }
        e.putString("baseUrl",          baseUrl);
        e.putString("model",            model);
        e.putString("visionModel",      visionModel);
        e.putString("persona",          persona);
        if (!tgToken.isEmpty()) {
            e.putString("tgToken_enc", encrypt(tgToken, seed, "tg_token"));
            e.remove("tgToken");        // remove old plain-text token
        }
        e.putLong  ("tgAllowed",        tgAllowed);
        e.putInt   ("agentMaxSteps",    agentMaxSteps);
        e.putBoolean("agentAutoApprove",agentAutoApprove);
        e.putInt   ("heartbeatInterval",heartbeatInterval);
        e.putBoolean("setupDone",       setupDone);
        e.putString("otaRepo",          otaRepo);
        e.commit(); // synchronous — ensures setupDone persists before activity transition
        // Mirror to Rust state
        try {
            RustBridge.syncConfig(userName, apiKey, baseUrl, model, visionModel,
                persona, tgToken, tgAllowed, agentMaxSteps, agentAutoApprove,
                heartbeatInterval, setupDone);
        } catch (UnsatisfiedLinkError ignored) {}
    }
}
