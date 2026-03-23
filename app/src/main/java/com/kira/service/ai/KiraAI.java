package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;
import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;

/**
 * KiraAI — streaming intelligence loop.
 *
 * Flow:
 *  1. POST /ai/run  → Rust spawns run_agent() in background thread (returns immediately)
 *  2. Poll GET /ai/run/status every 300ms:
 *     - status="running"  → fire onPartial(partial_text) + onThinkingStep(tool info)
 *     - status="done"     → fire onReply(result.content)
 *     - status="error"    → fire onError(message)
 *  3. After done: drain shell queue for any queued Java-side tools (open_app etc.)
 *
 * This gives real streaming: text appears word-by-word as LLM generates it,
 * and tool calls appear in the UI as they execute.
 */
public class KiraAI {
    private static final String TAG       = "KiraAI";
    private static final int    MAX_STEPS = 15;
    private static final String SESSION   = "default";
    private static final int    POLL_MS   = 250;   // status poll interval
    private static final int    TIMEOUT_MS = 120_000; // 2 min max

    // Rust HTTP server port (must match startServer call)
    private static final int RUST_PORT = 7070;

    private final Context ctx;

    public interface Callback {
        void onThinking();
        default void onPartial(String partialReply) {} // streamed text so far
        default void onThinkingStep(String step) {} // tool call / reasoning line
        default void onTool(String name, String result) {} // tool executed
        void onReply(String finalReply);
        void onError(String error);
    }

    public static abstract class SimpleCallback implements Callback {
        @Override public void onPartial(String p) {}
        @Override public void onThinkingStep(String s) {}
        @Override public void onTool(String n, String r) {}
    }

    private static final okhttp3.OkHttpClient HTTP = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(10,  java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(30,     java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(10,    java.util.concurrent.TimeUnit.SECONDS)
        .build();

    public KiraAI(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Main entry point
    // ─────────────────────────────────────────────────────────────────────────

    public void chat(String userMessage, Callback cb) {
        new Thread(() -> {
            try {
                if (cb != null) cb.onThinking();
                if (!RustBridge.isLoaded()) {
                    if (cb != null) cb.onError("Rust engine not loaded");
                    return;
                }

                // Abort any previous run
                rustPost("/ai/run/abort", "{}");

                // POST /ai/run — non-blocking, run_agent starts in background
                String startResp = rustPost("/ai/run", String.format(
                    "{\"message\":\"%s\",\"session\":\"%s\",\"max_steps\":%d}",
                    jsonEsc(userMessage), SESSION, MAX_STEPS
                ));

                if (startResp == null) {
                    // Rust server not ready or port different — fall back to chatViaRunner
                    chatViaRunnerFallback(userMessage, cb);
                    return;
                }

                JSONObject startJson = new JSONObject(startResp);
                if (startJson.has("error")) {
                    String err = startJson.getString("error");
                    if (err.contains("already running")) {
                        // Wait briefly and retry
                        Thread.sleep(1000);
                        rustPost("/ai/run/abort", "{}");
                        Thread.sleep(300);
                        rustPost("/ai/run", String.format(
                            "{\"message\":\"%s\",\"session\":\"%s\",\"max_steps\":%d}",
                            jsonEsc(userMessage), SESSION, MAX_STEPS
                        ));
                    } else if (err.contains("API key")) {
                        if (cb != null) cb.onError("No API key — go to Settings");
                        return;
                    } else {
                        if (cb != null) cb.onError(err);
                        return;
                    }
                }

                // ── Poll loop ──────────────────────────────────────────────
                String lastPartial    = "";
                int    thinkingIdx    = 0;
                long   startTime      = System.currentTimeMillis();

                while (System.currentTimeMillis() - startTime < TIMEOUT_MS) {
                    Thread.sleep(POLL_MS);

                    String statusResp = rustGet("/ai/run/status");
                    if (statusResp == null) continue;

                    JSONObject status = new JSONObject(statusResp);
                    String runStatus  = status.optString("status", "running");

                    // ── Stream partial text ──────────────────────────────
                    String partial = status.optString("partial_text", "");
                    if (!partial.isEmpty() && !partial.equals(lastPartial)) {
                        lastPartial = partial;
                        if (cb != null) cb.onPartial(partial);
                    }

                    // ── Stream thinking steps ────────────────────────────
                    JSONArray thinking = status.optJSONArray("thinking");
                    if (thinking != null) {
                        while (thinkingIdx < thinking.length()) {
                            String step = thinking.getString(thinkingIdx++);
                            if (cb != null) cb.onThinkingStep(step);
                            // Parse tool calls for onTool callback
                            if (step.startsWith("  → ")) {
                                // e.g. "  → open_app(package=com.youtube)"
                                String toolPart = step.substring(4);
                                int paren = toolPart.indexOf('(');
                                String toolName = paren > 0 ? toolPart.substring(0, paren) : toolPart;
                                if (cb != null) cb.onTool(toolName, "running...");
                            } else if (step.startsWith("    ← ")) {
                                // e.g. "    ← open_app: Opening com.youtube..."
                                String resultPart = step.substring(6);
                                int colon = resultPart.indexOf(':');
                                if (colon > 0) {
                                    String toolName = resultPart.substring(0, colon).trim();
                                    String toolResult = resultPart.substring(colon + 1).trim();
                                    if (cb != null) cb.onTool(toolName, toolResult);
                                }
                            }
                        }
                    }

                    // ── Check completion ─────────────────────────────────
                    if ("done".equals(runStatus) || "error".equals(runStatus)) {
                        // Drain shell queue before reporting done
                        drainShellQueue(cb);

                        JSONObject result = status.optJSONObject("result");
                        if (result != null) {
                            if ("error".equals(runStatus) || result.has("error")) {
                                String errMsg = result.optString("error",
                                    result.optString("content", "Unknown error"));
                                if (cb != null) cb.onError(errMsg);
                            } else {
                                String reply = result.optString("content", "");
                                if (reply.isEmpty()) reply = lastPartial;
                                if (reply.isEmpty()) reply = "Done.";
                                // Save memory if used
                                saveMemoryIfNeeded(result);
                                if (cb != null) cb.onReply(reply);
                            }
                        } else {
                            // No result object yet — use partial
                            if (!lastPartial.isEmpty()) {
                                if (cb != null) cb.onReply(lastPartial);
                            } else {
                                if (cb != null) cb.onError("No reply received");
                            }
                        }
                        return;
                    }

                    // ── Check current tool for live display ──────────────
                    String currentTool = status.optString("current_tool", "");
                    if (!currentTool.isEmpty() && cb != null) {
                        cb.onThinkingStep("⟳ " + currentTool + "...");
                    }
                }

                // Timeout
                if (cb != null) cb.onError("Timeout waiting for response");

            } catch (Throwable e) {
                Log.e(TAG, "chat error", e);
                String msg = e.getMessage();
                if (msg == null) msg = e.getClass().getSimpleName();
                if (cb != null) cb.onError(msg);
            }
        }, "KiraAI-Chat").start();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Fallback: synchronous chatViaRunner (for when HTTP server isn't ready)
    // ─────────────────────────────────────────────────────────────────────────

    private void chatViaRunnerFallback(String userMessage, Callback cb) {
        try {
            String result = RustBridge.chatViaRunner(userMessage, SESSION, MAX_STEPS);
            if (result == null) {
                if (cb != null) cb.onError("No response from Rust engine");
                return;
            }
            drainShellQueue(cb);
            JSONObject j = new JSONObject(result);
            if (j.has("error")) {
                if (cb != null) cb.onError(j.getString("error"));
            } else {
                String reply = j.optString("reply", "");
                if (reply.isEmpty()) reply = "Done.";
                saveMemoryIfNeeded(j);
                if (cb != null) cb.onReply(reply);
            }
        } catch (Throwable e) {
            Log.e(TAG, "chatViaRunnerFallback", e);
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "Error");
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Rust HTTP helpers
    // ─────────────────────────────────────────────────────────────────────────

    private String rustPost(String path, String body) {
        try {
            okhttp3.Response r = HTTP.newCall(new okhttp3.Request.Builder()
                .url("http://127.0.0.1:" + RUST_PORT + path)
                .post(okhttp3.RequestBody.create(body,
                    okhttp3.MediaType.parse("application/json")))
                .build()).execute();
            return r.body() != null ? r.body().string() : null;
        } catch (Exception e) {
            Log.w(TAG, "rustPost " + path + ": " + e.getMessage());
            return null;
        }
    }

    private String rustGet(String path) {
        try {
            okhttp3.Response r = HTTP.newCall(new okhttp3.Request.Builder()
                .url("http://127.0.0.1:" + RUST_PORT + path)
                .get().build()).execute();
            return r.body() != null ? r.body().string() : null;
        } catch (Exception e) {
            Log.w(TAG, "rustGet " + path + ": " + e.getMessage());
            return null;
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Shell queue (Java-side tools dispatched by Rust)
    // ─────────────────────────────────────────────────────────────────────────

    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 30; i++) {
            try {
                String jobJson = RustBridge.getNextShellJob();
                if (jobJson == null || jobJson.contains("\"empty\":true")) break;
                String id     = parseStr(jobJson, "id");
                String cmd    = parseStr(jobJson, "cmd");
                if (cmd.isEmpty()) break;
                String result = executeShellJob(cmd);
                RustBridge.postShellResult(id, result != null ? result : "");
                if (cb != null && result != null) {
                    String toolName = cmd.contains(":") ? cmd.substring(0, cmd.indexOf(':')) : cmd;
                    cb.onTool(toolName, result.length() > 120 ? result.substring(0, 120) + "…" : result);
                }
            } catch (Throwable e) {
                Log.w(TAG, "drainShellQueue: " + e.getMessage());
                break;
            }
        }
    }

    private String executeShellJob(String cmd) {
        try {
            if (cmd.startsWith("open_app:")) {
                String pkg = cmd.substring(9).trim();
                android.content.Intent intent = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
                if (intent != null) {
                    intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(intent);
                    return "Opened " + pkg;
                }
                try {
                    String resolved = RustBridge.appNameToPkg(pkg);
                    if (!resolved.equals(pkg)) {
                        intent = ctx.getPackageManager().getLaunchIntentForPackage(resolved);
                        if (intent != null) {
                            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                            ctx.startActivity(intent);
                            return "Opened " + resolved;
                        }
                    }
                } catch (Throwable ignored) {}
                return "App not found: " + pkg;
            }
            if (cmd.startsWith("http_get:")) {
                String url = cmd.substring(9).trim();
                okhttp3.Response r = HTTP.newCall(new okhttp3.Request.Builder()
                    .url(url).addHeader("User-Agent", "KiraAI/1.0").get().build()).execute();
                String body = r.body() != null ? r.body().string() : "";
                body = body.replaceAll("<style[^>]*>.*?</style>", " ")
                           .replaceAll("<script[^>]*>.*?</script>", " ")
                           .replaceAll("<[^>]+>", " ")
                           .replaceAll("\\s{2,}", " ").trim();
                return body.length() > 4000 ? body.substring(0, 4000) + "..." : body;
            }
            if (cmd.startsWith("send_message:")) {
                String rest = cmd.substring(13);
                int nl = rest.indexOf('\n');
                String target = nl >= 0 ? rest.substring(0, nl).trim() : rest.trim();
                android.content.Intent intent = new android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("sms:" + target));
                if (nl >= 0) intent.putExtra("sms_body", rest.substring(nl + 1).trim());
                intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(intent);
                return "Opened messaging for " + target;
            }
            if (ShizukuShell.isAvailable()) {
                String out = ShizukuShell.exec(cmd, 15_000);
                return out != null ? out : "";
            }
            return "Shell not available: " + cmd;
        } catch (Exception e) {
            return "Error: " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    private void saveMemoryIfNeeded(JSONObject result) {
        try {
            JSONArray tools = result.optJSONArray("tools_used");
            boolean usedMemory = false;
            if (tools != null) {
                for (int i = 0; i < tools.length(); i++) {
                    if ("add_memory".equals(tools.getString(i))) { usedMemory = true; break; }
                }
            }
            // Also check tools_used string
            String toolsStr = result.optString("tools_used", "");
            if (toolsStr.contains("add_memory")) usedMemory = true;

            if (usedMemory) {
                String json = RustBridge.saveMemory();
                if (json != null && !json.equals("[]")) {
                    ctx.getSharedPreferences("kira_memory", android.content.Context.MODE_PRIVATE)
                       .edit().putString("memory_json", json).apply();
                }
            }
        } catch (Throwable ignored) {}
    }

    private static String jsonEsc(String s) {
        if (s == null) return "";
        return s.replace("\\", "\\\\").replace("\"", "\\\"")
                .replace("\n", "\\n").replace("\r", "").replace("\t", "\\t");
    }

    private static String parseStr(String json, String key) {
        String search = "\"" + key + "\":\"";
        int start = json.indexOf(search);
        if (start < 0) return "";
        start += search.length();
        int end = start;
        while (end < json.length()) {
            char c = json.charAt(end);
            if (c == '"' && (end == 0 || json.charAt(end - 1) != '\\')) break;
            end++;
        }
        return json.substring(start, end)
            .replace("\\n", "\n").replace("\\\"", "\"").replace("\\\\", "\\");
    }
}
