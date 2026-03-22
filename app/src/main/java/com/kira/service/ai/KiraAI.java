package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * KiraAI — full intelligence loop with pure-Rust brain, Java HTTP.
 *
 * Flow per message:
 *  1. getChatContext(msg)     → Rust pushes user turn, returns LLM request JSON
 *  2. callLlm(requestJson)   → Java OkHttp → raw LLM response string
 *  3. processLlmReply(raw,n) → Rust parses tools, dispatches, updates memory
 *     a. {done:true,  reply} → show reply, done
 *     b. {done:false, messages_json} → call LLM again with tool results (loop)
 *  4. After each step, drain Java shell queue (open_app, run_shell, etc.)
 *
 * Rust handles: history, memory, tool parsing, dispatch, system prompt
 * Java handles: HTTP/TLS (OkHttp — no rustls crash), Shizuku shell execution
 */
public class KiraAI {
    private static final String TAG      = "KiraAI";
    private static final int    MAX_STEPS = 8;

    private final Context ctx;

    public interface Callback {
        void onThinking();
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    private static final okhttp3.OkHttpClient HTTP_CLIENT = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(120, java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .build();

    public KiraAI(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    public void chat(String userMessage, Callback cb) {
        new Thread(() -> {
            try {
                if (cb != null) cb.onThinking();
                if (!RustBridge.isLoaded()) {
                    if (cb != null) cb.onError("Rust engine not loaded");
                    return;
                }

                // ── Step 1: Get initial context from Rust ──────────────────
                String requestJson = getInitialContext(userMessage, cb);
                if (requestJson == null) return;

                // ── Step 2-N: LLM call + tool loop (all in Rust) ───────────
                int step = 0;
                while (step < MAX_STEPS) {
                    // Call LLM via Java OkHttp (no rustls)
                    String rawLlmResponse = callLlm(requestJson, cb);
                    if (rawLlmResponse == null) return;

                    // Drain any shell/app jobs queued during the LLM call
                    // (tool calls may have been parsed speculatively)
                    drainShellQueue(cb);

                    // ── Step N: Let Rust process the response ──────────────
                    String processResult;
                    try {
                        processResult = RustBridge.processLlmReply(rawLlmResponse, step);
                    } catch (UnsatisfiedLinkError e) {
                        // Old .so: processLlmReply not compiled, do it in Java
                        processResult = legacyProcessReply(rawLlmResponse);
                    }

                    if (processResult == null || processResult.isEmpty()) {
                        if (cb != null) cb.onError("Empty process result");
                        return;
                    }

                    JSONObject result = new JSONObject(processResult);

                    if (result.has("error")) {
                        if (cb != null) cb.onError(result.getString("error"));
                        return;
                    }

                    boolean done = result.optBoolean("done", true);

                    // Report tools used
                    String toolsUsed = result.optString("tools_used", "[]");
                    if (!toolsUsed.isEmpty() && !toolsUsed.equals("[]") && cb != null) {
                        cb.onTool("tools", toolsUsed);
                    }

                    // Drain shell queue AFTER Rust processes (it may queue new jobs)
                    drainShellQueue(cb);

                    if (done) {
                        String reply = result.optString("reply", "done.");
                        if (cb != null) cb.onReply(reply.isEmpty() ? "done." : reply);
                        return;
                    }

                    // Not done — get updated messages for next LLM call
                    String messagesJson = result.optString("messages_json", "");
                    if (messagesJson.isEmpty()) {
                        if (cb != null) cb.onError("Tool loop error: no messages");
                        return;
                    }
                    requestJson = messagesJson;
                    step++;
                }

                if (cb != null) cb.onError("Max tool steps reached");

            } catch (Throwable e) {
                Log.e(TAG, "chat error", e);
                String msg = e.getMessage();
                if (msg == null) msg = e.getClass().getSimpleName();
                if (cb != null) cb.onError(msg);
            }
        }, "KiraAI-Chat").start();
    }

    /** Get initial request JSON from Rust, with KiraConfig fallback for old .so */
    private String getInitialContext(String userMessage, Callback cb) {
        String requestJson;
        try {
            requestJson = RustBridge.getChatContext(userMessage);
        } catch (UnsatisfiedLinkError e) {
            Log.w(TAG, "getChatContext not in .so, using KiraConfig fallback");
            requestJson = buildFallbackContext(userMessage);
        }

        if (requestJson == null || requestJson.isEmpty()) {
            if (cb != null) cb.onError("Failed to build request");
            return null;
        }

        try {
            JSONObject j = new JSONObject(requestJson);
            if (j.has("error")) {
                String err = j.getString("error");
                if ("no_api_key".equals(err)) {
                    if (cb != null) cb.onError("No API key — go to Settings");
                } else {
                    if (cb != null) cb.onError(err);
                }
                return null;
            }
        } catch (Exception ignored) {}

        return requestJson;
    }

    /** Make one LLM HTTP call. Returns raw content string or null on error. */
    private String callLlm(String requestJson, Callback cb) {
        try {
            JSONObject req = new JSONObject(requestJson);
            String apiKey  = req.getString("api_key");
            String baseUrl = req.getString("base_url").replaceAll("/$", "");
            String model   = req.getString("model");
            JSONArray msgs = req.getJSONArray("messages");

            JSONObject body = new JSONObject();
            body.put("model", model);
            body.put("max_tokens", 2048);
            body.put("messages", msgs);

            okhttp3.Request request = new okhttp3.Request.Builder()
                .url(baseUrl + "/chat/completions")
                .addHeader("Authorization", "Bearer " + apiKey)
                .addHeader("Content-Type", "application/json")
                .post(okhttp3.RequestBody.create(
                    body.toString(),
                    okhttp3.MediaType.parse("application/json")))
                .build();

            okhttp3.Response response = HTTP_CLIENT.newCall(request).execute();
            if (response.body() == null) {
                if (cb != null) cb.onError("Empty HTTP response");
                return null;
            }
            String responseStr = response.body().string();
            if (!response.isSuccessful()) {
                String errMsg = "HTTP " + response.code();
                try {
                    JSONObject errJ = new JSONObject(responseStr);
                    if (errJ.has("error")) {
                        Object e = errJ.get("error");
                        errMsg = (e instanceof JSONObject)
                            ? ((JSONObject)e).optString("message", errMsg) : e.toString();
                    }
                } catch (Exception ignored) {}
                if (cb != null) cb.onError(errMsg);
                return null;
            }
            // Extract content from OpenAI response
            return extractContent(responseStr);
        } catch (Exception e) {
            Log.e(TAG, "callLlm error", e);
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "HTTP error");
            return null;
        }
    }

    /** Drain Rust's shell job queue — execute open_app, run_shell, http_get via Java */
    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 20; i++) {
            try {
                String jobJson = RustBridge.getNextShellJob();
                if (jobJson == null || jobJson.contains("\"empty\":true")) break;

                String id  = parseJsonStr(jobJson, "id");
                String cmd = parseJsonStr(jobJson, "cmd");
                if (cmd.isEmpty()) break;

                String result = executeShellJob(cmd, cb);
                RustBridge.postShellResult(id, result != null ? result : "");
                if (cb != null) cb.onTool(cmd.split(" ")[0], result != null ? result : "");
            } catch (Throwable e) {
                Log.w(TAG, "shell job: " + e.getMessage());
                break;
            }
        }
    }

    /** Execute a shell/app/http job queued by Rust dispatch_tool. Returns result string. */
    private String executeShellJob(String cmd, Callback cb) {
        try {
            // open_app:com.package.name
            if (cmd.startsWith("open_app:")) {
                String pkg = cmd.substring("open_app:".length()).trim();
                // Also handle plain package names without prefix (from old format)
                if (pkg.isEmpty()) pkg = cmd;
                android.content.Intent intent = ctx.getPackageManager()
                    .getLaunchIntentForPackage(pkg);
                if (intent != null) {
                    intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(intent);
                    return "opened " + pkg;
                }
                // Try by app name via search
                intent = new android.content.Intent(android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("market://search?q=" + pkg));
                return "app not found: " + pkg;
            }

            // http_get:https://url
            if (cmd.startsWith("http_get:")) {
                String url = cmd.substring("http_get:".length()).trim();
                okhttp3.Response r = HTTP_CLIENT.newCall(
                    new okhttp3.Request.Builder().url(url).get().build()).execute();
                String body = r.body() != null ? r.body().string() : "";
                return body.length() > 2000 ? body.substring(0, 2000) + "..." : body;
            }

            // http_post:https://url (body in remaining args — best effort)
            if (cmd.startsWith("http_post:")) {
                String url = cmd.substring("http_post:".length()).trim();
                okhttp3.Response r = HTTP_CLIENT.newCall(
                    new okhttp3.Request.Builder().url(url)
                        .post(okhttp3.RequestBody.create("", null)).build()).execute();
                String body = r.body() != null ? r.body().string() : "";
                return body.length() > 2000 ? body.substring(0, 2000) + "..." : body;
            }

            // read_file:/path
            if (cmd.startsWith("read_file:")) {
                String path = cmd.substring("read_file:".length()).trim();
                java.io.File f = new java.io.File(path);
                if (!f.exists()) return "file not found: " + path;
                byte[] bytes = java.nio.file.Files.readAllBytes(f.toPath());
                String text = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
                return text.length() > 4000 ? text.substring(0, 4000) + "..." : text;
            }

            // list_files:/path
            if (cmd.startsWith("list_files:")) {
                String path = cmd.substring("list_files:".length()).trim();
                java.io.File dir = new java.io.File(path.isEmpty() ? "/sdcard" : path);
                String[] files = dir.list();
                return files != null ? String.join(", ", files) : "empty or not accessible";
            }

            // write_file: handled via shell (needs path:content format)
            if (cmd.startsWith("write_file:")) {
                // Fall through to shell
            }

            // send_message: (WhatsApp/Telegram deep link)
            if (cmd.startsWith("send_message:")) {
                String target = cmd.substring("send_message:".length()).trim();
                android.content.Intent intent = new android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("https://wa.me/" + target));
                intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(intent);
                return "opened messaging for " + target;
            }

            // Fallback: run via Shizuku shell
            if (ShizukuShell.isAvailable()) {
                String out = ShizukuShell.exec(cmd, 10_000);
                return out != null ? out : "";
            }

            // No Shizuku — try via AccessibilityService intent
            return "executed (no shell available): " + cmd;

        } catch (Exception e) {
            Log.w(TAG, "executeShellJob: " + cmd, e);
            return "error: " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
        }
    }

    /** Fallback for old .so: just mark as done with the raw reply */
    private String legacyProcessReply(String rawResponse) {
        // No tool execution — just clean up and return
        String cleaned = rawResponse.replaceAll("<tool[^>]*>.*?</tool>", "").trim();
        if (cleaned.isEmpty()) cleaned = rawResponse;
        try {
            return new JSONObject()
                .put("done", true)
                .put("reply", cleaned)
                .put("tools_used", "[]")
                .toString();
        } catch (Exception e) {
            return "{\"done\":true,\"reply\":\"" + cleaned.replace("\"", "\\\"") + "\",\"tools_used\":\"[]\"}";
        }
    }

    /** Fallback context for old .so without getChatContext */
    private String buildFallbackContext(String userMessage) {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.apiKey == null || cfg.apiKey.isEmpty()) return null;
            String baseUrl = (cfg.baseUrl == null || cfg.baseUrl.isEmpty())
                ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            String model = (cfg.model == null || cfg.model.isEmpty())
                ? "llama-3.1-8b-instant" : cfg.model;
            String system = (cfg.persona == null || cfg.persona.isEmpty())
                ? "You are Kira, a helpful AI agent on Android. Be concise." : cfg.persona;

            JSONArray messages = new JSONArray();
            JSONObject sysMsg = new JSONObject();
            sysMsg.put("role", "system");
            sysMsg.put("content", system);
            messages.put(sysMsg);
            JSONObject userMsg = new JSONObject();
            userMsg.put("role", "user");
            userMsg.put("content", userMessage);
            messages.put(userMsg);

            return new JSONObject()
                .put("api_key", cfg.apiKey)
                .put("base_url", baseUrl)
                .put("model", model)
                .put("messages", messages)
                .toString();
        } catch (Exception e) {
            Log.e(TAG, "buildFallbackContext", e);
            return null;
        }
    }

    private String extractContent(String json) {
        try {
            JSONObject obj = new JSONObject(json);
            JSONArray choices = obj.getJSONArray("choices");
            if (choices.length() == 0) return null;
            JSONObject message = choices.getJSONObject(0).getJSONObject("message");
            return message.getString("content");
        } catch (Exception e) {
            return parseJsonStr(json, "content");
        }
    }

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
