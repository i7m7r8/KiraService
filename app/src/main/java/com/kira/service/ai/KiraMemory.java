package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

public class KiraMemory {
    private static final String TAG       = "KiraMemory";
    private static final String PREFS_MEM  = "kira_memory";
    private static final String PREFS_CONV = "kira_conversations";
    private static final int    MAX_CONV   = 200; // unlimited-feeling

    private final Context ctx;

    public KiraMemory(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    // ── Key-value memory ──────────────────────────────────────────────────────

    public void remember(String key, String value) {
        prefs().edit().putString(key.trim(), value).apply();
    }

    public String recall(String key) {
        String val = prefs().getString(key.trim(), null);
        return val != null ? val : "nothing stored for: " + key;
    }

    public void forget(String key) {
        prefs().edit().remove(key.trim()).apply();
    }

    public String listAll() {
        SharedPreferences p = prefs();
        if (p.getAll().isEmpty()) return "(empty)";
        StringBuilder sb = new StringBuilder();
        for (java.util.Map.Entry<String, ?> e : p.getAll().entrySet()) {
            sb.append(e.getKey()).append(": ").append(e.getValue()).append("\n");
        }
        return sb.toString().trim();
    }

    public void clearFacts() {
        prefs().edit().clear().apply();
    }

    // ── Conversation history ──────────────────────────────────────────────────

    public void storeConversation(String user, String kira) {
        try {
            SharedPreferences p = convPrefs();
            String raw = p.getString("history", "[]");
            JSONArray arr = new JSONArray(raw);
            JSONObject entry = new JSONObject();
            entry.put("user", user);
            entry.put("kira", kira);
            entry.put("at", System.currentTimeMillis());
            arr.put(entry);
            while (arr.length() > MAX_CONV) arr.remove(0);
            p.edit().putString("history", arr.toString()).apply();
        } catch (Exception e) {
            Log.e(TAG, "storeConversation error", e);
        }
    }

    public JSONArray loadHistory() {
        try {
            return new JSONArray(convPrefs().getString("history", "[]"));
        } catch (Exception e) {
            return new JSONArray();
        }
    }

    public void clearHistory() {
        convPrefs().edit().remove("history").apply();
    }

    // ── Context for AI system prompt ─────────────────────────────────────────

    public String getContext() {
        try {
            StringBuilder sb = new StringBuilder();
            SharedPreferences kv = prefs();
            if (!kv.getAll().isEmpty()) {
                sb.append("## Remembered facts\n");
                for (java.util.Map.Entry<String, ?> e : kv.getAll().entrySet()) {
                    sb.append("- ").append(e.getKey()).append(": ").append(e.getValue()).append("\n");
                }
            }
            JSONArray arr = loadHistory();
            if (arr.length() > 0) {
                sb.append("\n## Recent conversations\n");
                int start = Math.max(0, arr.length() - 8);
                for (int i = start; i < arr.length(); i++) {
                    JSONObject e = arr.getJSONObject(i);
                    sb.append("you: ").append(e.getString("user")).append("\n");
                    sb.append("kira: ").append(e.getString("kira")).append("\n\n");
                }
            }
            return sb.toString().trim();
        } catch (Exception e) { return ""; }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private SharedPreferences prefs() {
        return ctx.getSharedPreferences(PREFS_MEM, Context.MODE_PRIVATE);
    }

    private SharedPreferences convPrefs() {
        return ctx.getSharedPreferences(PREFS_CONV, Context.MODE_PRIVATE);
    }
}
