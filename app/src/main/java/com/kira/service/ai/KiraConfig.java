package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;

public class KiraConfig {
    private static final String PREFS = "kira_config";

    public String userName  = "User";
    public String apiKey    = "";
    public String baseUrl   = "https://api.groq.com/openai/v1";
    public String model     = "llama-3.1-8b-instant";
    public String tgToken   = "";
    public long   tgAllowed = 0;
    public boolean setupDone = false;

    public static KiraConfig load(Context ctx) {
        SharedPreferences p = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        KiraConfig c = new KiraConfig();
        c.userName   = p.getString("userName", "User");
        c.apiKey     = p.getString("apiKey", "");
        c.baseUrl    = p.getString("baseUrl", "https://api.groq.com/openai/v1");
        c.model      = p.getString("model", "llama-3.1-8b-instant");
        c.tgToken    = p.getString("tgToken", "");
        c.tgAllowed  = p.getLong("tgAllowed", 0);
        c.setupDone  = p.getBoolean("setupDone", false);
        return c;
    }

    public void save(Context ctx) {
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit()
            .putString("userName",  userName)
            .putString("apiKey",    apiKey)
            .putString("baseUrl",   baseUrl)
            .putString("model",     model)
            .putString("tgToken",   tgToken)
            .putLong("tgAllowed",   tgAllowed)
            .putBoolean("setupDone", setupDone)
            .apply();
    }
}
