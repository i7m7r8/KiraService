package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;
import com.kira.service.KiraAccessibilityService;
import org.json.JSONArray;
import org.json.JSONObject;
import java.io.BufferedReader;
import java.io.InputStreamReader;

/**
 * KiraAI — Java OkHttp intelligence loop.
 *
 * Flow (Java-OkHttp, NO Rust TLS):
 *  1. getChatContext(msg)      → Rust builds LLM request JSON (api_key, messages, tools)
 *  2. callLlmStreaming(json)   → Java OkHttp SSE → fires onPartial() live
 *  3. processLlmReply(raw, n) → Rust parses tool_calls, dispatches pure-Rust tools
 *     a. {done:true, reply}   → onReply(), done
 *     b. {done:false, messages_json} → loop: call LLM again with tool results
 *  4. drainShellQueue()        → Java executes shell/app jobs queued by Rust
 *
 * This NEVER uses Rust TLS (https_post) - Java OkHttp handles all network.
 * The /ai/run endpoint is NOT used to avoid the rustls panic on Android.
 */
public class KiraAI {
    private static final String TAG       = "KiraAI";
    private static final int    MAX_STEPS = 15;

    private final Context ctx;

    public interface Callback {
        void onThinking();
        default void onPartial(String partial) {}
        default void onThinkingStep(String step) {}
        default void onTool(String name, String result) {}
        void onReply(String reply);
        void onError(String error);
    }

    public static abstract class SimpleCallback implements Callback {
        @Override public void onPartial(String p) {}
        @Override public void onThinkingStep(String s) {}
        @Override public void onTool(String n, String r) {}
    }

    private static final okhttp3.OkHttpClient HTTP = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(120,   java.util.concurrent.TimeUnit.SECONDS)
        .writeTimeout(30,   java.util.concurrent.TimeUnit.SECONDS)
        .build();

    public KiraAI(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    // ── Main entry point ─────────────────────────────────────────────────────

    public void chat(String userMessage, Callback cb) {
        new Thread(() -> {
            try {
                if (cb != null) cb.onThinking();
                if (!RustBridge.isLoaded()) {
                    if (cb != null) cb.onError("Rust engine not loaded");
                    return;
                }

                // Step 1: Rust builds the request JSON (pushes user turn, builds system prompt)
                String requestJson = getInitialContext(userMessage, cb);
                if (requestJson == null) return;

                int step = 0;
                while (step < MAX_STEPS) {
                    // Step 2: Java calls LLM via OkHttp (SSE streaming)
                    if (cb != null) cb.onThinkingStep("Step " + (step + 1) + ": calling LLM...");
                    ParsedLlmResponse parsed = callLlmStreaming(requestJson, cb);
                    if (parsed == null) return;

                    // Drain Java shell jobs queued during this step
                    drainShellQueue(cb);

                    // Step 3: if no tool calls, we have the final reply directly in Java.
                    // No need to pass through Rust's extract_llm_content which is fragile
                    // with org.json's serialization format.
                    if (parsed.toolCalls == null || parsed.toolCalls.length() == 0) {
                        // No tool calls — content IS the final reply
                        String reply = parsed.content.trim();
                        if (reply.isEmpty()) reply = "Done.";
                        // Push to Rust history so context is maintained
                        try { RustBridge.pushContextTurn("assistant", reply); } catch (Throwable ignored) {}
                        if (cb != null) cb.onReply(reply);
                        return;
                    }

                    // Has tool calls — let Rust dispatch them and build the follow-up request
                    String processResult;
                    try {
                        processResult = RustBridge.processLlmReply(parsed.rawJson, step);
                    } catch (UnsatisfiedLinkError e) {
                        processResult = legacyProcessReply(parsed.rawJson);
                    }

                    if (processResult == null || processResult.isEmpty()) {
                        if (cb != null) cb.onError("Empty process result");
                        return;
                    }

                    JSONObject result = new JSONObject(processResult);
                    if (result.has("error")) {
                        if (cb != null) cb.onError(result.optString("error", "Unknown error"));
                        return;
                    }

                    boolean done = result.optBoolean("done", true);

                    // Show tools used
                    JSONArray toolsArr = result.optJSONArray("tools_used");
                    if (toolsArr != null && toolsArr.length() > 0 && cb != null) {
                        for (int i = 0; i < toolsArr.length(); i++) {
                            String toolName = toolsArr.optString(i, "");
                            if (!toolName.isEmpty()) cb.onThinkingStep("  - " + toolName);
                        }
                    } else {
                        String toolsStr = result.optString("tools_used", "");
                        if (!toolsStr.isEmpty() && !toolsStr.equals("[]") && cb != null) {
                            cb.onThinkingStep("Tools: " + toolsStr);
                        }
                    }

                    // Drain again after Rust dispatches
                    drainShellQueue(cb);

                    if (done) {
                        // Rust decided done=true after tool dispatch (rare — e.g. max steps in Rust)
                        String reply = result.optString("reply", "");
                        if (reply.trim().isEmpty()) reply = parsed.content.trim();
                        if (reply.trim().isEmpty()) reply = "Done.";
                        saveMemoryIfNeeded(result);
                        if (cb != null) cb.onReply(reply);
                        return;
                    }

                    // Not done — Rust has built the next request with tool results
                    String nextReqJson = result.optString("messages_json", "");
                    if (nextReqJson.isEmpty()) {
                        if (cb != null) cb.onError("Tool loop: no follow-up messages");
                        return;
                    }
                    // If messages_json contains pending_shell_result: placeholders,
                    // drain Java shell queue first, then resolve placeholders
                    if (nextReqJson.contains("pending_shell_result:")) {
                        drainShellQueue(cb);
                        try {
                            String resolved = RustBridge.resolveShellResults(nextReqJson);
                            if (resolved != null && !resolved.isEmpty()) {
                                nextReqJson = resolved;
                            }
                        } catch (Throwable ignored) {}
                    }
                    requestJson = nextReqJson;
                    step++;
                }

                if (cb != null) cb.onError("Max tool steps reached");

            } catch (Throwable e) {
                Log.e(TAG, "chat error", e);
                if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : e.getClass().getSimpleName());
            }
        }, "KiraAI-Chat").start();
    }

    // ── Parsed LLM response container ────────────────────────────────────────
    // Holds the fully parsed SSE result so we never pass raw JSON through Rust
    // for content extraction (Rust's extract_llm_content is fragile with org.json output).
    private static class ParsedLlmResponse {
        final String content;        // assembled text content (never null, may be "")
        final JSONArray toolCalls;   // null if no tool calls
        final String rawJson;        // fakeResp JSON for RustBridge.processLlmReply
        ParsedLlmResponse(String content, JSONArray toolCalls, String rawJson) {
            this.content   = content;
            this.toolCalls = toolCalls;
            this.rawJson   = rawJson;
        }
    }

    // ── SSE Streaming LLM call (Java OkHttp — no Rust TLS) ───────────────────

    private ParsedLlmResponse callLlmStreaming(String requestJson, Callback cb) {
        try {
            JSONObject req  = new JSONObject(requestJson);
            String apiKey   = req.optString("api_key", "");
            String baseUrl  = req.optString("base_url", "").replaceAll("/$", "").trim();
            if (baseUrl.isEmpty() || (!baseUrl.startsWith("http://") && !baseUrl.startsWith("https://"))) {
                baseUrl = "https://api.groq.com/openai/v1";
            }
            String model    = req.optString("model", "llama-3.1-8b-instant");
            if (model.isEmpty()) model = "llama-3.1-8b-instant";
            Object msgsRaw  = req.opt("messages");
            JSONArray msgs;
            if (msgsRaw instanceof JSONArray) {
                msgs = (JSONArray) msgsRaw;
            } else if (msgsRaw != null) {
                msgs = new JSONArray(msgsRaw.toString());
            } else {
                if (cb != null) cb.onError("No messages in request - check API key and retry");
                return null;
            }

            // Build streaming body
            JSONObject body = new JSONObject();
            body.put("model", model);
            body.put("max_tokens", 8192);
            body.put("stream", true);
            body.put("messages", msgs);
            try {
                if (req.has("tools") && req.opt("tools") != null)
                    body.put("tools", req.get("tools"));
                if (req.has("tool_choice") && req.opt("tool_choice") != null)
                    body.put("tool_choice", req.get("tool_choice"));
            } catch (Exception ignored) {}

            // Strip control characters only (keep all printable ASCII)
            apiKey = apiKey.replaceAll("[\u0000-\u001F\u007F]", "").trim();
            if (apiKey.isEmpty()) {
                if (cb != null) cb.onError("No API key - go to Settings");
                return null;
            }

            okhttp3.Request request = new okhttp3.Request.Builder()
                .url(baseUrl + "/chat/completions")
                .addHeader("Authorization", "Bearer " + apiKey)
                .addHeader("Content-Type",  "application/json")
                .addHeader("Accept",        "text/event-stream")
                .post(okhttp3.RequestBody.create(
                    body.toString(),
                    okhttp3.MediaType.parse("application/json")))
                .build();

            okhttp3.Response resp = HTTP.newCall(request).execute();
            if (resp.body() == null) {
                if (cb != null) cb.onError("Empty HTTP response");
                return null;
            }
            if (!resp.isSuccessful()) {
                String errMsg = "HTTP " + resp.code();
                try {
                    okhttp3.ResponseBody errBodyObj = resp.body();
                    if (errBodyObj != null) {
                        String errBody = errBodyObj.string();
                        JSONObject ej = new JSONObject(errBody);
                        if (ej.has("error")) {
                            Object e = ej.get("error");
                            if (e instanceof JSONObject) {
                                errMsg = ((JSONObject)e).optString("message", errMsg);
                            } else if (e != null && e != JSONObject.NULL) {
                                errMsg = e.toString();
                            }
                        }
                    }
                } catch (Exception ignored) {}
                if (cb != null) cb.onError(errMsg);
                return null;
            }

            // Parse SSE stream
            StringBuilder fullContent  = new StringBuilder();
            StringBuilder toolCallsRaw = new StringBuilder();
            StringBuilder rawLines     = new StringBuilder(); // collect ALL lines for non-SSE fallback
            boolean hasToolCalls = false;
            String  lastPartial  = "";
            long    lastPartialMs = 0;

            okhttp3.ResponseBody respBody = resp.body();
            if (respBody == null) {
                if (cb != null) cb.onError("Empty response body");
                return null;
            }
            BufferedReader reader = new BufferedReader(
                new InputStreamReader(respBody.byteStream(), java.nio.charset.StandardCharsets.UTF_8));
            String line;
            while ((line = reader.readLine()) != null) {
                rawLines.append(line).append("\n");
                if (!line.startsWith("data: ")) continue;
                String data = line.substring(6).trim();
                if ("[DONE]".equals(data)) break;
                try {
                    JSONObject chunk   = new JSONObject(data);
                    JSONArray  choices = chunk.optJSONArray("choices");
                    if (choices == null || choices.length() == 0) continue;
                    JSONObject choice0 = choices.optJSONObject(0);
                    if (choice0 == null) continue;
                    JSONObject delta = choice0.optJSONObject("delta");
                    if (delta == null) continue;

                    // Use isNull() check — optString returns the STRING "null" for JSON null values
                    if (!delta.isNull("content")) {
                        String contentDelta = delta.optString("content", "");
                        if (!contentDelta.isEmpty()) {
                            fullContent.append(contentDelta);
                            long now = System.currentTimeMillis();
                            if (cb != null && now - lastPartialMs > 250) {
                                String partial = fullContent.toString();
                                if (!partial.equals(lastPartial)) {
                                    cb.onPartial(partial);
                                    lastPartial   = partial;
                                    lastPartialMs = now;
                                    try { RustBridge.emitTextDelta("default", contentDelta); } catch (Throwable ignored) {}
                                }
                            }
                        }
                    }

                    JSONArray tcDeltas = delta.optJSONArray("tool_calls");
                    if (tcDeltas != null && tcDeltas.length() > 0) {
                        hasToolCalls = true;
                        toolCallsRaw.append(data).append("\n");
                    }

                } catch (Exception ignored) {}
            }
            reader.close();

            String finalContent = fullContent.toString();

            // ── Non-SSE fallback ──────────────────────────────────────────────
            // If we got zero content AND zero tool_calls, the server may have returned
            // a plain JSON response (no "data: " prefix) instead of an SSE stream.
            // Parse it directly so we never return an empty result.
            if (finalContent.isEmpty() && !hasToolCalls) {
                String raw = rawLines.toString().trim();
                try {
                    // Try parsing the whole body as one JSON object
                    JSONObject plainResp = new JSONObject(raw);
                    JSONArray plainChoices = plainResp.optJSONArray("choices");
                    if (plainChoices != null && plainChoices.length() > 0) {
                        JSONObject c0  = plainChoices.optJSONObject(0);
                        JSONObject msg = c0 != null ? c0.optJSONObject("message") : null;
                        if (msg != null) {
                            if (!msg.isNull("content")) finalContent = msg.optString("content", "");
                            JSONArray tcs = msg.optJSONArray("tool_calls");
                            if (tcs != null && tcs.length() > 0) {
                                // Return a ParsedLlmResponse with full tool_calls from plain JSON
                                String fakeRaw = buildFakeResp(finalContent, tcs);
                                return new ParsedLlmResponse(finalContent, tcs, fakeRaw);
                            }
                        }
                    }
                } catch (Exception ignored) {}
            }

            // Build fakeResp for Rust's processLlmReply (tool dispatch, history push)
            JSONArray toolCallsArray = null;
            if (hasToolCalls && toolCallsRaw.length() > 0) {
                toolCallsArray = buildToolCallsFromSse(toolCallsRaw.toString());
                if (toolCallsArray.length() == 0) toolCallsArray = null;
            }
            String fakeRespJson = buildFakeResp(finalContent, toolCallsArray);
            return new ParsedLlmResponse(finalContent, toolCallsArray, fakeRespJson);

        } catch (Exception e) {
            Log.e(TAG, "callLlmStreaming", e);
            if (cb != null) cb.onError(e.getMessage() != null ? e.getMessage() : "HTTP error");
            return null;
        }
    }

    /** Build a fake non-streaming response JSON for Rust's processLlmReply. */
    private String buildFakeResp(String content, JSONArray toolCalls) {
        try {
            JSONObject fakeMessage = new JSONObject();
            fakeMessage.put("role", "assistant");
            fakeMessage.put("content", content);
            if (toolCalls != null && toolCalls.length() > 0) {
                fakeMessage.put("tool_calls", toolCalls);
            }
            JSONObject fakeChoice = new JSONObject();
            fakeChoice.put("message", fakeMessage);
            fakeChoice.put("finish_reason", (toolCalls != null && toolCalls.length() > 0) ? "tool_calls" : "stop");
            JSONArray fakeChoices = new JSONArray();
            fakeChoices.put(fakeChoice);
            JSONObject fakeResp = new JSONObject();
            fakeResp.put("choices", fakeChoices);
            return fakeResp.toString();
        } catch (Exception e) {
            return "{\"choices\":[{\"message\":{\"role\":\"assistant\",\"content\":\"" + content.replace("\"","\\\"") + "\"},\"finish_reason\":\"stop\"}]}";
        }
    }

    /**
     * Assemble tool_calls array from SSE delta chunks.
     * Each chunk has: {"index":0,"id":"call_xxx","type":"function","function":{"name":"...","arguments":"..."}}
     * Arguments arrive in fragments and must be concatenated per index.
     */
    private JSONArray buildToolCallsFromSse(String rawSseLines) {
        try {
            // Map from index -> assembled call
            java.util.TreeMap<Integer, JSONObject> calls = new java.util.TreeMap<>();
            java.util.TreeMap<Integer, StringBuilder> argBuilders = new java.util.TreeMap<>();

            for (String line : rawSseLines.split("\n")) {
                if (line.trim().isEmpty()) continue;
                try {
                    JSONObject chunk = new JSONObject(line.trim());
                    JSONArray choices = chunk.optJSONArray("choices");
                    if (choices == null) continue;
                    if (choices.length() == 0) continue;
                    JSONObject choice0 = choices.optJSONObject(0);
                    if (choice0 == null) continue;
                    JSONObject delta = choice0.optJSONObject("delta");
                    if (delta == null) continue;
                    JSONArray tcArr = delta.optJSONArray("tool_calls");
                    if (tcArr == null) continue;

                    for (int i = 0; i < tcArr.length(); i++) {
                        JSONObject tc = tcArr.getJSONObject(i);
                        int idx = tc.optInt("index", 0);

                        if (!calls.containsKey(idx)) {
                            JSONObject call = new JSONObject();
                            call.put("id",   tc.optString("id", "call_" + idx));
                            call.put("type", "function");
                            call.put("function", new JSONObject()
                                .put("name", "")
                                .put("arguments", ""));
                            calls.put(idx, call);
                            argBuilders.put(idx, new StringBuilder());
                        }

                        JSONObject fn = tc.optJSONObject("function");
                        if (fn != null) {
                            String nameDelta = fn.optString("name", "");
                            String argsDelta = fn.optString("arguments", "");
                            if (!nameDelta.isEmpty()) {
                                calls.get(idx).getJSONObject("function").put("name", nameDelta);
                            }
                            if (!argsDelta.isEmpty()) {
                                argBuilders.get(idx).append(argsDelta);
                            }
                        }
                    }
                } catch (Exception ignored) {}
            }

            // Finalize: set assembled arguments
            JSONArray result = new JSONArray();
            for (int idx : calls.keySet()) {
                JSONObject call = calls.get(idx);
                call.getJSONObject("function").put("arguments",
                    argBuilders.get(idx).toString());
                result.put(call);
            }
            return result;

        } catch (Exception e) {
            Log.w(TAG, "buildToolCallsFromSse: " + e.getMessage());
            return new JSONArray();
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private String getInitialContext(String userMessage, Callback cb) {
        String requestJson;
        try {
            requestJson = RustBridge.getChatContext(userMessage);
        } catch (UnsatisfiedLinkError e) {
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
                // If Rust has no api_key yet (timing race), fall back to loading from prefs
                if ("no_api_key".equals(err)) {
                    KiraConfig cfg = KiraConfig.load(ctx);
                    if (cfg.apiKey != null && !cfg.apiKey.isEmpty()) {
                        // Sync config to Rust and retry getChatContext directly.
                        // Do NOT call buildFallbackContext here — it would push the user
                        // turn a second time if getChatContext is still called inside it.
                        try { cfg.save(ctx); } catch (Throwable ignored2) {}
                        // Retry: Rust now has the key, so getChatContext will push the
                        // turn exactly once and return the full context with tools + history.
                        try {
                            requestJson = RustBridge.getChatContext(userMessage);
                            JSONObject retryCheck = new JSONObject(requestJson);
                            if (retryCheck.has("error")) {
                                // getChatContext still failing — use plain fallback (no Rust push)
                                requestJson = buildFallbackContext(userMessage);
                            }
                        } catch (Throwable e2) {
                            requestJson = buildFallbackContext(userMessage);
                        }
                        if (requestJson == null) {
                            if (cb != null) cb.onError("No API key - go to Settings");
                            return null;
                        }
                        // CRITICAL: re-parse j from the updated requestJson so the
                        // base_url fixup below operates on the NEW valid JSON, not the
                        // stale {"error":"no_api_key"} object. Without this, j.toString()
                        // overwrites requestJson with the error JSON (no "messages" key)
                        // and callLlmStreaming fires "No messages in request".
                        try { j = new JSONObject(requestJson); } catch (Exception ignored3) { return requestJson; }
                    } else {
                        if (cb != null) cb.onError("No API key - go to Settings");
                        return null;
                    }
                } else {
                    if (cb != null) cb.onError(err);
                    return null;
                }
            }
            // Validate base_url: only patch if clearly wrong, use string replace not j.toString()
            // j.toString() can corrupt large nested JSON (tools schema), so avoid it
            String baseUrl = j.optString("base_url", "");
            if (baseUrl.isEmpty() || (!baseUrl.startsWith("http://") && !baseUrl.startsWith("https://"))) {
                // Safe string-level patch: replace the base_url value without re-serializing
                requestJson = patchJsonString(requestJson, "base_url", "https://api.groq.com/openai/v1");
            }
        } catch (Exception ignored) {}
        return requestJson;
    }

    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 30; i++) {
            try {
                String job = RustBridge.getNextShellJob();
                if (job == null || job.contains("\"empty\":true")) break;
                String id  = parseStr(job, "id");
                String cmd = parseStr(job, "cmd");
                if (cmd.isEmpty()) break;
                String res = executeShellJob(cmd);
                RustBridge.postShellResult(id, res != null ? res : "");
                if (cb != null && res != null) {
                    String toolName = cmd.contains(":") ? cmd.substring(0, cmd.indexOf(':')) : cmd;
                    cb.onTool(toolName, res.length() > 120 ? res.substring(0, 120) + "..." : res);
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
                String rest   = cmd.substring(13);
                int nl        = rest.indexOf('\n');
                String target = nl >= 0 ? rest.substring(0, nl).trim() : rest.trim();
                android.content.Intent i = new android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("sms:" + target));
                if (nl >= 0) i.putExtra("sms_body", rest.substring(nl + 1).trim());
                i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(i);
                return "Opened messaging for " + target;
            }
            // ── Session 21: Accessibility commands ───────────────────────────
            KiraAccessibilityService acc = KiraAccessibilityService.instance;
            if (acc != null) {
                if (cmd.startsWith("tap:")) {
                    String[] parts = cmd.substring(4).split(",");
                    if (parts.length >= 2) {
                        try {
                            int x = Integer.parseInt(parts[0].trim());
                            int y = Integer.parseInt(parts[1].trim());
                            return acc.tap(x, y) ? "Tapped (" + x + "," + y + ")" : "Tap failed";
                        } catch (Exception ignored) {}
                    }
                }
                if (cmd.startsWith("find_and_tap:")) {
                    String text = cmd.substring(13);
                    return acc.tapText(text);
                }
                if (cmd.startsWith("swipe:")) {
                    String[] p = cmd.substring(6).split(",");
                    if (p.length >= 4) {
                        try {
                            int x1 = Integer.parseInt(p[0].trim()), y1 = Integer.parseInt(p[1].trim());
                            int x2 = Integer.parseInt(p[2].trim()), y2 = Integer.parseInt(p[3].trim());
                            int dur = p.length >= 5 ? Integer.parseInt(p[4].trim()) : 300;
                            return acc.swipe(x1, y1, x2, y2, dur) ? "Swiped" : "Swipe failed";
                        } catch (Exception ignored) {}
                    }
                }
                if (cmd.startsWith("type:")) {
                    String text = cmd.substring(5);
                    return acc.typeText(text) ? "Typed: " + text : "Type failed (no focused field)";
                }
                if (cmd.equals("back:")) {
                    acc.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_BACK);
                    return "Back pressed";
                }
                if (cmd.equals("home:")) {
                    acc.performGlobalAction(android.accessibilityservice.AccessibilityService.GLOBAL_ACTION_HOME);
                    return "Home pressed";
                }
                if (cmd.equals("screen_read:")) {
                    return acc.getScreenText();
                }
                if (cmd.startsWith("clipboard_set:")) {
                    String text = cmd.substring(14);
                    android.content.ClipboardManager cm =
                        (android.content.ClipboardManager) ctx.getSystemService(android.content.Context.CLIPBOARD_SERVICE);
                    if (cm != null) {
                        cm.setPrimaryClip(android.content.ClipData.newPlainText("kira", text));
                        return "Clipboard set";
                    }
                }
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

    private String legacyProcessReply(String raw) {
        try {
            JSONObject j = new JSONObject(raw);
            JSONArray choices = j.optJSONArray("choices");
            if (choices != null && choices.length() > 0) {
                JSONObject c0 = choices.optJSONObject(0);
                JSONObject msg = c0 != null ? c0.optJSONObject("message") : null;
                if (msg != null) {
                    String content = msg.optString("content", "Done.").trim();
                    return new JSONObject().put("done", true).put("reply",
                        content.isEmpty() ? "Done." : content).put("tools_used", "[]").toString();
                }
            }
        } catch (Exception ignored) {}
        return "{\"done\":true,\"reply\":\"Done.\",\"tools_used\":\"[]\"}";
    }

    /** Safely patch a string field in a JSON object without re-serializing the whole object. */
    private static String patchJsonString(String json, String key, String newValue) {
        // Find "key":"value" and replace the value part
        String searchKey = "\"" + key + "\":\"";
        int start = json.indexOf(searchKey);
        if (start < 0) {
            // Key not present - insert before the last }
            int last = json.lastIndexOf('}');
            if (last < 0) return json;
            boolean hasFields = json.indexOf(':') >= 0;
            String insert = (hasFields ? "," : "") + "\"" + key + "\":\"" + newValue + "\"";
            return json.substring(0, last) + insert + "}";
        }
        int valueStart = start + searchKey.length();
        // Find the closing quote (skip escaped quotes)
        int valueEnd = valueStart;
        while (valueEnd < json.length()) {
            char c = json.charAt(valueEnd);
            if (c == '\\') { valueEnd += 2; continue; }
            if (c == '"') break;
            valueEnd++;
        }
        return json.substring(0, valueStart) + newValue + json.substring(valueEnd);
    }

    private String buildFallbackContext(String msg) {
        // NOTE: This method must NOT call RustBridge.getChatContext() — doing so would
        // push the user turn into Rust history a second time (double-push bug).
        // It is only called when Rust is unavailable or has no API key yet.
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.apiKey == null || cfg.apiKey.isEmpty()) return null;
            String base = (cfg.baseUrl == null || cfg.baseUrl.isEmpty())
                ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            String model = (cfg.model == null || cfg.model.isEmpty())
                ? "llama-3.1-8b-instant" : cfg.model;
            String persona = (cfg.persona == null || cfg.persona.isEmpty())
                ? "You are Kira, a smart AI assistant on Android. Use tools to get real data." : cfg.persona;
            JSONArray messages = new JSONArray();
            messages.put(new JSONObject().put("role","system").put("content", persona));
            messages.put(new JSONObject().put("role","user").put("content", msg));
            return new JSONObject()
                .put("api_key", cfg.apiKey)
                .put("base_url", base)
                .put("model", model)
                .put("messages", messages)
                .toString();
        } catch (Exception e) { return null; }
    }

    private void saveMemoryIfNeeded(JSONObject result) {
        try {
            String toolsStr = result.optString("tools_used", "");
            JSONArray toolsArr = result.optJSONArray("tools_used");
            boolean usedMemory = toolsStr.contains("add_memory");
            if (!usedMemory && toolsArr != null) {
                for (int i = 0; i < toolsArr.length(); i++) {
                    if ("add_memory".equals(toolsArr.getString(i))) { usedMemory = true; break; }
                }
            }
            if (usedMemory) {
                String json = RustBridge.saveMemory();
                if (json != null && !json.equals("[]")) {
                    ctx.getSharedPreferences("kira_memory", android.content.Context.MODE_PRIVATE)
                       .edit().putString("memory_json", json).apply();
                }
            }
        } catch (Throwable ignored) {}
    }

    private static String parseStr(String json, String key) {
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
            .replace("\\n","\n").replace("\\\"","\"").replace("\\\\","\\");
    }
}
