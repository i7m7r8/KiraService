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
 * KiraAI — routes all chat through the proper OpenAI function-calling runner in Rust.
 *
 * Flow (new):
 *   1. chatViaRunner(msg, session, maxSteps) → Rust run_agent() → reply JSON
 *   2. Drain shell queue for any Java-side tools (open_app, http_get, etc.)
 *   3. Fire callbacks: onPartial not available in synchronous mode;
 *      onTool for each tool used; onReply with final text.
 *
 * The old getChatContext/processLlmReply path is kept as a fallback for old .so files.
 * Streaming is done via a background thread that polls RUN_STATE while chatViaRunner runs.
 */
public class KiraAI {
    private static final String TAG      = "KiraAI";
    private static final int    MAX_STEPS = 15;
    private static final String SESSION   = "default";

    private final Context ctx;

    public interface Callback {
        void onThinking();
        void onPartial(String partialReply);
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    public static abstract class SimpleCallback implements Callback {
        @Override public void onPartial(String p) {}
    }

    private static final okhttp3.OkHttpClient HTTP_CLIENT = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(180, java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
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

                // ── Try the new runner first ──────────────────────────────
                String runnerResult = null;
                try {
                    runnerResult = RustBridge.chatViaRunner(userMessage, SESSION, MAX_STEPS);
                } catch (UnsatisfiedLinkError e) {
                    Log.w(TAG, "chatViaRunner not in .so, falling back to legacy path");
                }

                if (runnerResult != null) {
                    handleRunnerResult(runnerResult, cb);
                    return;
                }

                // ── Legacy fallback (old .so without chatViaRunner) ───────
                legacyChat(userMessage, cb);

            } catch (Throwable e) {
                Log.e(TAG, "chat error", e);
                if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
            }
        }, "KiraAI-Chat").start();
    }

    // ─────────────────────────────────────────────────────────────────────────
    // New runner path
    // ─────────────────────────────────────────────────────────────────────────

    private void handleRunnerResult(String runnerJson, Callback cb) {
        try {
            // Drain shell queue FIRST — runner may have queued open_app / http_get etc.
            drainShellQueue(cb);

            JSONObject result = new JSONObject(runnerJson);

            if (result.has("error")) {
                String err = result.getString("error");
                if (cb != null) cb.onError(err);
                return;
            }

            // Report tools used
            JSONArray toolsArr = result.optJSONArray("tools_used");
            if (toolsArr != null && toolsArr.length() > 0) {
                StringBuilder toolsList = new StringBuilder();
                for (int i = 0; i < toolsArr.length(); i++) {
                    if (i > 0) toolsList.append(", ");
                    toolsList.append(toolsArr.getString(i));
                }
                if (cb != null) cb.onTool("tools", toolsList.toString());

                // Persist memory if modified
                if (runnerJson.contains("\"add_memory\"") || runnerJson.contains("\"search_memory\"")) {
                    saveMemory();
                }
            }

            // Drain shell queue again after tool results are posted
            drainShellQueue(cb);

            String reply = result.optString("reply", "");
            if (reply.isEmpty()) reply = "Done.";
            if (cb != null) cb.onReply(reply);

        } catch (Throwable e) {
            Log.e(TAG, "handleRunnerResult", e);
            if (cb != null) cb.onError("Parse error: " + e.getMessage());
        }
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Shell job execution (Java-side tools dispatched by Rust)
    // ─────────────────────────────────────────────────────────────────────────

    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 30; i++) {
            try {
                String jobJson = RustBridge.getNextShellJob();
                if (jobJson == null || jobJson.contains("\"empty\":true")) break;
                String id  = parseJsonStr(jobJson, "id");
                String cmd = parseJsonStr(jobJson, "cmd");
                if (cmd.isEmpty()) break;
                String result = executeShellJob(cmd);
                RustBridge.postShellResult(id, result != null ? result : "");
                if (cb != null && result != null) {
                    // Show what tool ran
                    String toolName = cmd.contains(":") ? cmd.substring(0, cmd.indexOf(':')) : cmd;
                    cb.onTool(toolName, result.length() > 100 ? result.substring(0, 100) + "…" : result);
                }
            } catch (Throwable e) {
                Log.w(TAG, "drainShellQueue: " + e.getMessage());
                break;
            }
        }
    }

    /** Execute a Java-side tool job queued by Rust's dispatch_tool. */
    private String executeShellJob(String cmd) {
        try {
            // open_app:com.package.name
            if (cmd.startsWith("open_app:")) {
                String pkg = cmd.substring("open_app:".length()).trim();
                android.content.Intent intent = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
                if (intent != null) {
                    intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(intent);
                    return "Opened " + pkg;
                }
                // Try app name lookup via Rust
                try {
                    String resolved = RustBridge.appNameToPkg(pkg);
                    if (!resolved.equals(pkg) && !resolved.isEmpty()) {
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

            // http_get:https://url
            if (cmd.startsWith("http_get:")) {
                String url = cmd.substring("http_get:".length()).trim();
                okhttp3.Response r = HTTP_CLIENT.newCall(
                    new okhttp3.Request.Builder()
                        .url(url)
                        .addHeader("User-Agent", "Mozilla/5.0 (Android) KiraAI/1.0")
                        .get().build()).execute();
                String body = r.body() != null ? r.body().string() : "";
                // Strip HTML
                body = body.replaceAll("<style[^>]*>.*?</style>", " ")
                           .replaceAll("<script[^>]*>.*?</script>", " ")
                           .replaceAll("<[^>]+>", " ")
                           .replaceAll("\\s{2,}", " ").trim();
                if (url.contains("duckduckgo.com") || url.contains("html.duck")) {
                    body = extractSearchSnippets(body);
                }
                return body.length() > 4000 ? body.substring(0, 4000) + "..." : body;
            }

            // web_search:query (routed as http_get to DDG)
            if (cmd.startsWith("web_search:")) {
                String query   = cmd.substring("web_search:".length()).trim();
                String encoded = query.replace(' ', '+').replace("&", "%26");
                String url     = "https://html.duckduckgo.com/html/?q=" + encoded;
                return executeShellJob("http_get:" + url);
            }

            // read_file:/path
            if (cmd.startsWith("read_file:")) {
                String path = cmd.substring("read_file:".length()).trim();
                java.io.File f = new java.io.File(path);
                if (!f.exists()) return "File not found: " + path;
                byte[] bytes = java.nio.file.Files.readAllBytes(f.toPath());
                String text  = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
                return text.length() > 4000 ? text.substring(0, 4000) + "..." : text;
            }

            // list_files:/path
            if (cmd.startsWith("list_files:")) {
                String path = cmd.substring("list_files:".length()).trim();
                java.io.File dir = new java.io.File(path.isEmpty() ? "/sdcard" : path);
                String[] files   = dir.list();
                return files != null ? String.join(", ", files) : "Empty or not accessible";
            }

            // write_file:/path\ncontent
            if (cmd.startsWith("write_file:")) {
                String rest = cmd.substring("write_file:".length());
                int nl = rest.indexOf('\n');
                if (nl < 0) return "write_file: missing content (use newline separator)";
                String path    = rest.substring(0, nl).trim();
                String content = rest.substring(nl + 1);
                java.io.File f = new java.io.File(path);
                if (f.getParentFile() != null) f.getParentFile().mkdirs();
                java.nio.file.Files.write(f.toPath(),
                    content.getBytes(java.nio.charset.StandardCharsets.UTF_8));
                return "Wrote " + content.length() + " bytes to " + path;
            }

            // send_message:number\nmessage_text
            if (cmd.startsWith("send_message:")) {
                String rest = cmd.substring("send_message:".length());
                int nl = rest.indexOf('\n');
                String target  = nl >= 0 ? rest.substring(0, nl).trim() : rest.trim();
                android.content.Intent intent = new android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("sms:" + target));
                if (nl >= 0) intent.putExtra("sms_body", rest.substring(nl + 1).trim());
                intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(intent);
                return "Opened messaging for " + target;
            }

            // Fallback: run as shell command via Shizuku
            if (ShizukuShell.isAvailable()) {
                String out = ShizukuShell.exec(cmd, 10_000);
                return out != null ? out : "";
            }
            return "No shell available for: " + cmd;

        } catch (Exception e) {
            Log.w(TAG, "executeShellJob: " + cmd, e);
            return "Error: " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
        }
    }

    private String extractSearchSnippets(String text) {
        String[] lines = text.split("\n");
        StringBuilder sb = new StringBuilder();
        for (String line : lines) {
            line = line.trim();
            if (line.length() > 30) {
                sb.append(line).append("\n");
                if (sb.length() > 4000) break;
            }
        }
        return sb.length() > 0 ? sb.toString() : text.substring(0, Math.min(4000, text.length()));
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Legacy fallback (old .so without chatViaRunner)
    // ─────────────────────────────────────────────────────────────────────────

    private void legacyChat(String userMessage, Callback cb) {
        try {
            // getChatContext → OkHttp → processLlmReply loop
            String requestJson;
            try {
                requestJson = RustBridge.getChatContext(userMessage);
            } catch (UnsatisfiedLinkError e) {
                requestJson = buildFallbackContext(userMessage);
            }
            if (requestJson == null || requestJson.isEmpty()) {
                if (cb != null) cb.onError("Failed to build request");
                return;
            }
            JSONObject req = new JSONObject(requestJson);
            if (req.has("error")) {
                if (cb != null) cb.onError(req.getString("error"));
                return;
            }

            // Single non-streaming LLM call
            String rawResponse = callLlmOnce(requestJson, cb);
            if (rawResponse == null) return;

            drainShellQueue(cb);

            // Parse content
            String content = extractContent(rawResponse);
            if (content == null || content.isEmpty()) content = "Done.";
            if (cb != null) cb.onReply(content);

        } catch (Throwable e) {
            Log.e(TAG, "legacyChat error", e);
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "Error");
        }
    }

    private String callLlmOnce(String requestJson, Callback cb) {
        try {
            JSONObject req  = new JSONObject(requestJson);
            String apiKey   = req.getString("api_key");
            String baseUrl  = req.getString("base_url").replaceAll("/$", "");
            String model    = req.getString("model");
            Object msgsRaw  = req.get("messages");
            JSONArray msgs  = (msgsRaw instanceof JSONArray)
                ? (JSONArray) msgsRaw : new JSONArray(msgsRaw.toString());

            JSONObject body = new JSONObject();
            body.put("model", model);
            body.put("max_tokens", 8192);
            body.put("messages", msgs);

            okhttp3.Response response = HTTP_CLIENT.newCall(
                new okhttp3.Request.Builder()
                    .url(baseUrl + "/chat/completions")
                    .addHeader("Authorization", "Bearer " + apiKey)
                    .addHeader("Content-Type", "application/json")
                    .post(okhttp3.RequestBody.create(body.toString(),
                        okhttp3.MediaType.parse("application/json")))
                    .build()).execute();

            if (response.body() == null) { if (cb != null) cb.onError("Empty response"); return null; }
            String resp = response.body().string();
            if (!response.isSuccessful()) {
                if (cb != null) cb.onError("HTTP " + response.code());
                return null;
            }
            return resp;
        } catch (Exception e) {
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "HTTP error");
            return null;
        }
    }

    private String extractContent(String json) {
        try {
            JSONObject j = new JSONObject(json);
            JSONArray choices = j.optJSONArray("choices");
            if (choices != null && choices.length() > 0) {
                JSONObject msg = choices.getJSONObject(0).optJSONObject("message");
                if (msg != null) return msg.optString("content", "");
            }
        } catch (Exception ignored) {}
        return parseJsonStr(json, "content");
    }

    // ─────────────────────────────────────────────────────────────────────────
    // Helpers
    // ─────────────────────────────────────────────────────────────────────────

    private void saveMemory() {
        try {
            String json = RustBridge.saveMemory();
            if (json != null && !json.equals("[]")) {
                ctx.getSharedPreferences("kira_memory", android.content.Context.MODE_PRIVATE)
                   .edit().putString("memory_json", json).apply();
            }
        } catch (Throwable ignored) {}
    }

    private String buildFallbackContext(String userMessage) {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.apiKey == null || cfg.apiKey.isEmpty()) return null;
            String baseUrl = (cfg.baseUrl == null || cfg.baseUrl.isEmpty())
                ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            String model   = (cfg.model == null || cfg.model.isEmpty())
                ? "llama-3.1-8b-instant" : cfg.model;
            String system  = (cfg.persona == null || cfg.persona.isEmpty())
                ? "You are Kira, a helpful AI agent on Android." : cfg.persona;
            JSONArray messages = new JSONArray();
            JSONObject sysMsg  = new JSONObject(); sysMsg.put("role","system"); sysMsg.put("content",system); messages.put(sysMsg);
            JSONObject userMsg = new JSONObject(); userMsg.put("role","user");   userMsg.put("content",userMessage); messages.put(userMsg);
            return new JSONObject()
                .put("api_key", cfg.apiKey).put("base_url", baseUrl)
                .put("model", model).put("messages", messages).toString();
        } catch (Exception e) { Log.e(TAG, "buildFallbackContext", e); return null; }
    }

    private static String parseJsonStr(String json, String key) {
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
