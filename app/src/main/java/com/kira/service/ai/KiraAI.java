package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;

/**
 * KiraAI — Session D: thin Java wrapper over Rust /ai/chat engine.
 *
 * All AI logic (history, system prompt, LLM call, tool dispatch, memory)
 * now runs in Rust. This class:
 *   1. Calls RustBridge.chatSync() on a background thread
 *   2. Drains the shell job queue (for Shizuku/intent tools Rust can't call directly)
 *   3. Fires callbacks to update UI
 *
 * ~95% of original KiraAI.java (377 lines) is now in Rust state.rs + http.rs.
 */
public class KiraAI {
    private static final String TAG      = "KiraAI";
    private static final int    MAX_STEPS = 10;

    private final Context ctx;

    public interface Callback {
        void onThinking();
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    public KiraAI(Context ctx) {
        this.ctx = ctx.getApplicationContext();
        // No initialization needed — Rust owns all state
    }

    public void chat(String userMessage, Callback cb) {
        new Thread(null, () -> {
            try {
                if (cb != null) cb.onThinking();

                // ── Single Rust call — runs entire AI turn ─────────────────
                String resultJson = RustBridge.chatSync(userMessage, "default", MAX_STEPS);

                // ── Drain shell job queue (Shizuku/intent tools) ───────────
                drainShellQueue(cb);

                // ── Parse result ───────────────────────────────────────────
                if (resultJson == null || resultJson.isEmpty()) {
                    if (cb != null) cb.onError("no response from Rust engine");
                    return;
                }

                String error = parseJsonStr(resultJson, "error");
                if (!error.isEmpty()) {
                    if (cb != null) cb.onError(error);
                    return;
                }

                String reply      = parseJsonStr(resultJson, "content");
                String toolsUsed  = parseJsonStr(resultJson, "tools_used");

                if (!toolsUsed.isEmpty() && !toolsUsed.equals("[]") && cb != null) {
                    cb.onTool("tools", toolsUsed);
                }
                if (cb != null) cb.onReply(reply.isEmpty() ? "done." : reply);

            } catch (Exception e) {
                Log.e(TAG, "chat error", e);
                if (cb != null) cb.onError(e.getMessage());
            }
        }, "KiraAI-Chat", 8 * 1024 * 1024).start(); // 8MB stack for Rust TLS
    }

    /** Execute pending shell jobs queued by Rust AI engine */
    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 20; i++) { // max 20 shell jobs per turn
            try {
                String jobJson = RustBridge.getNextShellJob();
                if (jobJson == null || jobJson.contains("\"empty\":true")) break;

                String id  = parseJsonStr(jobJson, "id");
                String cmd = parseJsonStr(jobJson, "cmd");
                if (cmd.isEmpty()) break;

                String result = ShizukuShell.isAvailable()
                    ? ShizukuShell.exec(cmd, 10_000)
                    : "shizuku_unavailable";

                RustBridge.postShellResult(id, result != null ? result : "");
                if (cb != null) cb.onTool(cmd.split(" ")[0], result != null ? result : "");

            } catch (Exception e) {
                Log.w(TAG, "shell job error: " + e.getMessage());
                break;
            }
        }
    }

    // ── Minimal JSON parsing — no library needed ───────────────────────────

    private static String parseJsonStr(String json, String key) {
        String search = "\"" + key + "\":\"";
        int start = json.indexOf(search);
        if (start < 0) return "";
        start += search.length();
        int end = start;
        while (end < json.length()) {
            char c = json.charAt(end);
            if (c == '"' && (end == 0 || json.charAt(end-1) != '\\')) break;
            end++;
        }
        return json.substring(start, end)
            .replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\");
    }
}
