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
 * KiraAI — full intelligence loop with streaming support.
 *
 * Flow per message:
 *  1. getChatContext(msg)      → Rust pushes user turn, returns LLM request JSON
 *  2. callLlmStreaming(...)    → Java OkHttp SSE stream → onPartial callbacks
 *  3. processLlmReply(raw, n) → Rust parses tools, dispatches, updates memory
 *     a. {done:true, reply}   → show reply, done
 *     b. {done:false, messages_json} → loop with tool results
 *  4. After each step, drain Java shell queue
 *
 * Streaming: each SSE chunk fires onPartial(text) so UI can update in real-time.
 * Telegram: KiraTelegram calls editMessage() on each onPartial for live updates.
 */
public class KiraAI {
    private static final String TAG      = "KiraAI";
    private static final int    MAX_STEPS = 12;

    private final Context ctx;

    public interface Callback {
        void onThinking();
        void onPartial(String partialReply);   // NEW: streaming chunk
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    // Adapter for callers that don't need streaming
    public static abstract class SimpleCallback implements Callback {
        @Override public void onPartial(String partialReply) {}
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

                String requestJson = getInitialContext(userMessage, cb);
                if (requestJson == null) return;

                int step = 0;
                while (step < MAX_STEPS) {
                    // ── Streaming LLM call ─────────────────────────────────
                    String rawLlmResponse = callLlmStreaming(requestJson, cb);
                    if (rawLlmResponse == null) return;

                    drainShellQueue(cb);

                    String processResult;
                    try {
                        processResult = RustBridge.processLlmReply(rawLlmResponse, step);
                    } catch (UnsatisfiedLinkError e) {
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
                    String toolsUsed = result.optString("tools_used", "[]");
                    if (!toolsUsed.isEmpty() && !toolsUsed.equals("[]") && cb != null) {
                        cb.onTool("tools", toolsUsed);
                    }

                    drainShellQueue(cb);

                    if (done) {
                        String reply = result.optString("reply", "Done.");
                        if (toolsUsed.contains("add_memory") || toolsUsed.contains("search_memory")) {
                            saveMemory();
                        }
                        if (reply == null || reply.trim().isEmpty()) reply = "Done.";
                        if (cb != null) cb.onReply(reply);
                        return;
                    }

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

    /**
     * SSE streaming LLM call.
     * Fires cb.onPartial(text) for each chunk so the UI / Telegram can update live.
     * Returns the full assembled content string when done.
     */
    private String callLlmStreaming(String requestJson, Callback cb) {
        try {
            JSONObject req      = new JSONObject(requestJson);
            String apiKey       = req.getString("api_key");
            String baseUrl      = req.getString("base_url").replaceAll("/$", "");
            String model        = req.getString("model");
            Object msgsRaw      = req.get("messages");
            JSONArray msgs      = (msgsRaw instanceof JSONArray)
                ? (JSONArray) msgsRaw
                : new JSONArray(msgsRaw.toString());

            // Build streaming request body
            JSONObject body = new JSONObject();
            body.put("model", model);
            body.put("max_tokens", 8192);
            body.put("stream", true);
            body.put("messages", msgs);
            // Include tools if present
            if (req.has("tools")) body.put("tools", req.get("tools"));

            okhttp3.Request request = new okhttp3.Request.Builder()
                .url(baseUrl + "/chat/completions")
                .addHeader("Authorization", "Bearer " + apiKey)
                .addHeader("Content-Type", "application/json")
                .addHeader("Accept", "text/event-stream")
                .post(okhttp3.RequestBody.create(
                    body.toString(),
                    okhttp3.MediaType.parse("application/json")))
                .build();

            okhttp3.Response response = HTTP_CLIENT.newCall(request).execute();
            if (response.body() == null) {
                if (cb != null) cb.onError("Empty HTTP response");
                return null;
            }

            if (!response.isSuccessful()) {
                String errBody = response.body().string();
                String errMsg  = "HTTP " + response.code();
                try {
                    JSONObject errJ = new JSONObject(errBody);
                    if (errJ.has("error")) {
                        Object e = errJ.get("error");
                        errMsg = (e instanceof JSONObject)
                            ? ((JSONObject)e).optString("message", errMsg) : e.toString();
                    }
                } catch (Exception ignored) {}
                if (cb != null) cb.onError(errMsg);
                return null;
            }

            // ── Parse SSE stream ─────────────────────────────────────────
            StringBuilder fullContent  = new StringBuilder();
            StringBuilder toolCallBuf  = new StringBuilder();
            boolean hasToolCalls       = false;
            String lastThrottledPartial = "";
            long   lastPartialMs       = 0;

            BufferedReader reader = new BufferedReader(
                new InputStreamReader(response.body().byteStream()));
            String line;
            while ((line = reader.readLine()) != null) {
                if (!line.startsWith("data: ")) continue;
                String data = line.substring(6).trim();
                if ("[DONE]".equals(data)) break;

                try {
                    JSONObject chunk  = new JSONObject(data);
                    JSONArray choices = chunk.optJSONArray("choices");
                    if (choices == null || choices.length() == 0) continue;

                    JSONObject delta = choices.getJSONObject(0).optJSONObject("delta");
                    if (delta == null) continue;

                    // Text content delta
                    String contentDelta = delta.optString("content", "");
                    if (!contentDelta.isEmpty()) {
                        fullContent.append(contentDelta);
                        // Throttle partial callbacks to ~200ms to avoid flooding Telegram API
                        long now = System.currentTimeMillis();
                        if (cb != null && now - lastPartialMs > 200) {
                            String partial = fullContent.toString();
                            if (!partial.equals(lastThrottledPartial)) {
                                cb.onPartial(partial);
                                lastThrottledPartial = partial;
                                lastPartialMs = now;
                            }
                        }
                    }

                    // Tool call deltas (accumulate)
                    JSONArray toolCallDeltas = delta.optJSONArray("tool_calls");
                    if (toolCallDeltas != null && toolCallDeltas.length() > 0) {
                        hasToolCalls = true;
                        // Accumulate raw tool call JSON for Rust to parse
                        toolCallBuf.append(data).append("\n");
                    }

                } catch (Exception ignored) {}
            }
            reader.close();

            // Build final response object for Rust processLlmReply
            // Rust's extract_llm_content handles the choices[0].message.content format
            // We reconstruct a minimal non-streaming response for Rust to parse
            String finalContent = fullContent.toString();
            JSONObject fakeResponse = new JSONObject();
            JSONArray fakeChoices   = new JSONArray();
            JSONObject fakeChoice   = new JSONObject();
            JSONObject fakeMessage  = new JSONObject();
            fakeMessage.put("role", "assistant");
            fakeMessage.put("content", finalContent);
            if (hasToolCalls && toolCallBuf.length() > 0) {
                // Pass tool call buffer in a special field Rust can check
                fakeMessage.put("tool_calls_raw_sse", toolCallBuf.toString());
            }
            fakeChoice.put("message", fakeMessage);
            fakeChoice.put("finish_reason", hasToolCalls ? "tool_calls" : "stop");
            fakeChoices.put(fakeChoice);
            fakeResponse.put("choices", fakeChoices);

            return fakeResponse.toString();

        } catch (Exception e) {
            Log.e(TAG, "callLlmStreaming error", e);
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "HTTP error");
            return null;
        }
    }

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

    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 20; i++) {
            try {
                String jobJson = RustBridge.getNextShellJob();
                if (jobJson == null || jobJson.contains("\"empty\":true")) break;
                String id     = parseJsonStr(jobJson, "id");
                String cmd    = parseJsonStr(jobJson, "cmd");
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

    private String executeShellJob(String cmd, Callback cb) {
        try {
            if (cmd.startsWith("open_app:")) {
                String pkg = cmd.substring("open_app:".length()).trim();
                android.content.Intent intent = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
                if (intent != null) {
                    intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(intent);
                    return "opened " + pkg;
                }
                return "app not found: " + pkg;
            }
            if (cmd.startsWith("http_get:")) {
                String url = cmd.substring("http_get:".length()).trim();
                okhttp3.Response r = HTTP_CLIENT.newCall(
                    new okhttp3.Request.Builder()
                        .url(url)
                        .addHeader("User-Agent", "Mozilla/5.0 (Android) KiraAI/1.0")
                        .get().build()).execute();
                String body = r.body() != null ? r.body().string() : "";
                body = body.replaceAll("<style[^>]*>.*?</style>", " ")
                           .replaceAll("<script[^>]*>.*?</script>", " ")
                           .replaceAll("<[^>]+>", " ")
                           .replaceAll("\\s{2,}", " ").trim();
                if (url.contains("duckduckgo.com")) body = extractSearchSnippets(body);
                return body.length() > 3000 ? body.substring(0, 3000) + "..." : body;
            }
            if (cmd.startsWith("read_file:")) {
                String path = cmd.substring("read_file:".length()).trim();
                java.io.File f = new java.io.File(path);
                if (!f.exists()) return "file not found: " + path;
                byte[] bytes = java.nio.file.Files.readAllBytes(f.toPath());
                String text  = new String(bytes, java.nio.charset.StandardCharsets.UTF_8);
                return text.length() > 4000 ? text.substring(0, 4000) + "..." : text;
            }
            if (cmd.startsWith("list_files:")) {
                String path = cmd.substring("list_files:".length()).trim();
                java.io.File dir = new java.io.File(path.isEmpty() ? "/sdcard" : path);
                String[] files   = dir.list();
                return files != null ? String.join(", ", files) : "empty or not accessible";
            }
            if (cmd.startsWith("write_file:")) {
                // format: write_file:/path/to/file\ncontent here
                String rest = cmd.substring("write_file:".length());
                int nl = rest.indexOf('\n');
                if (nl < 0) return "write_file: no content separator";
                String path    = rest.substring(0, nl).trim();
                String content = rest.substring(nl + 1);
                java.io.File f = new java.io.File(path);
                if (f.getParentFile() != null) f.getParentFile().mkdirs();
                java.nio.file.Files.write(f.toPath(),
                    content.getBytes(java.nio.charset.StandardCharsets.UTF_8));
                return "written " + content.length() + " bytes to " + path;
            }
            if (ShizukuShell.isAvailable()) {
                String out = ShizukuShell.exec(cmd, 10_000);
                return out != null ? out : "";
            }
            return "no shell available: " + cmd;
        } catch (Exception e) {
            Log.w(TAG, "executeShellJob: " + cmd, e);
            return "error: " + (e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
        }
    }

    private String extractSearchSnippets(String text) {
        String[] lines = text.split("\n");
        StringBuilder sb = new StringBuilder();
        for (String line : lines) {
            line = line.trim();
            if (line.length() > 30) {
                sb.append(line).append("\n");
                if (sb.length() > 3000) break;
            }
        }
        return sb.length() > 0 ? sb.toString() : text.substring(0, Math.min(3000, text.length()));
    }

    private String legacyProcessReply(String rawResponse) {
        // Extract actual text content from the OpenAI JSON envelope
        String content = rawResponse;
        try {
            JSONObject j = new JSONObject(rawResponse);
            JSONArray choices = j.optJSONArray("choices");
            if (choices != null && choices.length() > 0) {
                JSONObject msg = choices.getJSONObject(0).optJSONObject("message");
                if (msg != null) content = msg.optString("content", rawResponse);
            }
        } catch (Exception ignored) {}
        // Strip any <tool> XML tags
        String cleaned = content.replaceAll("<tool[^>]*>.*?</tool>", "").trim();
        if (cleaned.isEmpty()) cleaned = "Done.";
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

    private String buildFallbackContext(String userMessage) {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.apiKey == null || cfg.apiKey.isEmpty()) return null;
            String baseUrl = (cfg.baseUrl == null || cfg.baseUrl.isEmpty())
                ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            String model   = (cfg.model == null || cfg.model.isEmpty())
                ? "llama-3.1-8b-instant" : cfg.model;
            String system  = (cfg.persona == null || cfg.persona.isEmpty())
                ? "You are Kira, a helpful AI agent on Android. Be concise." : cfg.persona;
            JSONArray messages = new JSONArray();
            JSONObject sysMsg  = new JSONObject(); sysMsg.put("role","system"); sysMsg.put("content",system); messages.put(sysMsg);
            JSONObject userMsg = new JSONObject(); userMsg.put("role","user");   userMsg.put("content",userMessage); messages.put(userMsg);
            return new JSONObject()
                .put("api_key", cfg.apiKey).put("base_url", baseUrl)
                .put("model", model).put("messages", messages).toString();
        } catch (Exception e) { Log.e(TAG, "buildFallbackContext", e); return null; }
    }

    private void saveMemory() {
        try {
            String json = RustBridge.saveMemory();
            if (json != null && !json.equals("[]")) {
                ctx.getSharedPreferences("kira_memory", android.content.Context.MODE_PRIVATE)
                   .edit().putString("memory_json", json).apply();
            }
        } catch (Throwable ignored) {}
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
