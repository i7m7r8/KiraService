package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.List;

/**
 * NanoClaw-style checkpoint system.
 * Saves task progress so interrupted tasks can resume.
 *
 * Also implements ZeroClaw's sendAgentEmail pattern via
 * persistent notifications and cross-session state.
 */
public class KiraCheckpoint {

    private static final String PREFS = "kira_checkpoints";
    private final SharedPreferences prefs;

    public KiraCheckpoint(Context ctx) {
        this.prefs = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    /** Save a checkpoint for a running task */
    public void save(String taskId, int step, String state, String goal) {
        try {
            JSONObject cp = new JSONObject();
            cp.put("taskId", taskId);
            cp.put("step", step);
            cp.put("state", state);
            cp.put("goal", goal);
            cp.put("ts", System.currentTimeMillis());
            cp.put("status", "paused");
            prefs.edit().putString("cp_" + taskId, cp.toString()).apply();
        } catch (Exception ignored) {}
    }

    /** Mark a task as complete */
    public void complete(String taskId, String result) {
        try {
            String existing = prefs.getString("cp_" + taskId, null);
            if (existing == null) return;
            JSONObject cp = new JSONObject(existing);
            cp.put("status", "complete");
            cp.put("result", result);
            cp.put("completedAt", System.currentTimeMillis());
            prefs.edit().putString("cp_" + taskId, cp.toString()).apply();
        } catch (Exception ignored) {}
    }

    /** Get a checkpoint by task ID */
    public JSONObject get(String taskId) {
        try {
            String s = prefs.getString("cp_" + taskId, null);
            return s != null ? new JSONObject(s) : null;
        } catch (Exception e) { return null; }
    }

    /** List all incomplete (paused) checkpoints */
    public List<JSONObject> getPaused() {
        List<JSONObject> result = new ArrayList<>();
        for (String key : prefs.getAll().keySet()) {
            if (!key.startsWith("cp_")) continue;
            try {
                JSONObject cp = new JSONObject(prefs.getString(key, "{}"));
                if ("paused".equals(cp.optString("status"))) result.add(cp);
            } catch (Exception ignored) {}
        }
        return result;
    }

    /** Delete a checkpoint */
    public void delete(String taskId) {
        prefs.edit().remove("cp_" + taskId).apply();
    }

    /** Get all checkpoints as JSON string */
    public String getAllJson() {
        JSONArray arr = new JSONArray();
        for (String key : prefs.getAll().keySet()) {
            if (!key.startsWith("cp_")) continue;
            try { arr.put(new JSONObject(prefs.getString(key, "{}"))); }
            catch (Exception ignored) {}
        }
        return arr.toString();
    }

    /**
     * ZeroClaw sendAgentEmail pattern:
     * Store a "handoff message" that the next session will see.
     */
    public void sendHandoff(String toSession, String message, String context) {
        try {
            JSONObject handoff = new JSONObject();
            handoff.put("to", toSession);
            handoff.put("message", message);
            handoff.put("context", context);
            handoff.put("ts", System.currentTimeMillis());
            handoff.put("read", false);
            String key = "handoff_" + toSession + "_" + System.currentTimeMillis();
            prefs.edit().putString(key, handoff.toString()).apply();
        } catch (Exception ignored) {}
    }

    /** Get unread handoffs for a session */
    public List<JSONObject> getUnreadHandoffs(String session) {
        List<JSONObject> result = new ArrayList<>();
        for (String key : prefs.getAll().keySet()) {
            if (!key.startsWith("handoff_" + session)) continue;
            try {
                JSONObject h = new JSONObject(prefs.getString(key, "{}"));
                if (!h.optBoolean("read", false)) {
                    result.add(h);
                    // Mark as read
                    h.put("read", true);
                    prefs.edit().putString(key, h.toString()).apply();
                }
            } catch (Exception ignored) {}
        }
        return result;
    }
}
