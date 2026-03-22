package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * KiraAI — HTTP done in Java, state in Rust.
 *
 * Architecture (fixes SIGABRT from rustls on Android):
 *  1. Call RustBridge.getChatContext(msg) — Rust stores user turn, returns
 *     {api_key, base_url, model, system_prompt, messages:[...]}
 *  2. Java builds the OpenAI-compatible request and calls OkHttp
 *  3. Java parses the response
 *  4. Call RustBridge.pushAssistantTurn(reply) — Rust stores assistant turn
 *
 * OkHttp is already in the app, handles TLS natively on Android with no crashes.
 * Rust never touches the network for chat — zero rustls involvement.
 */
public class KiraAI {
    private static final String TAG = "KiraAI";

    private final Context ctx;

    public interface Callback {
        void onThinking();
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

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

                // Step 1: Get context from Rust (stores user turn, returns config+history)
                String ctxJson = RustBridge.getChatContext(userMessage);
                if (ctxJson == null || ctxJson.isEmpty()) {
                    if (cb != null) cb.onError("Failed to get chat context");
                    return;
                }

                JSONObject ctx2 = new JSONObject(ctxJson);

                if (ctx2.has("error")) {
                    String err = ctx2.getString("error");
                    if ("no_api_key".equals(err)) {
                        if (cb != null) cb.onError("No API key — go to Settings and add one");
                    } else {
                        if (cb != null) cb.onError("Context error: " + err);
                    }
                    return;
                }

                String apiKey      = ctx2.getString("api_key");
                String baseUrl     = ctx2.getString("base_url").replaceAll("/$", "");
                String model       = ctx2.getString("model");
                String systemPrompt= ctx2.getString("system_prompt");
                JSONArray messages = ctx2.getJSONArray("messages");

                // Step 2: Build request body
                JSONArray reqMessages = new JSONArray();
                // Add system message first
                if (!systemPrompt.isEmpty()) {
                    JSONObject sysMsg = new JSONObject();
                    sysMsg.put("role", "system");
                    sysMsg.put("content", systemPrompt);
                    reqMessages.put(sysMsg);
                }
                // Add conversation history
                for (int i = 0; i < messages.length(); i++) {
                    reqMessages.put(messages.getJSONObject(i));
                }

                JSONObject body = new JSONObject();
                body.put("model", model);
                body.put("max_tokens", 2048);
                body.put("messages", reqMessages);

                // Step 3: Make HTTP call with OkHttp (Android-native, no rustls)
                String url = baseUrl + "/chat/completions";
                okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(30, java.util.concurrent.TimeUnit.SECONDS)
                    .readTimeout(60, java.util.concurrent.TimeUnit.SECONDS)
                    .build();

                okhttp3.Request request = new okhttp3.Request.Builder()
                    .url(url)
                    .addHeader("Authorization", "Bearer " + apiKey)
                    .addHeader("Content-Type", "application/json")
                    .post(okhttp3.RequestBody.create(
                        body.toString(),
                        okhttp3.MediaType.parse("application/json")))
                    .build();

                okhttp3.Response response = client.newCall(request).execute();
                if (response.body() == null) {
                    if (cb != null) cb.onError("Empty response from server");
                    return;
                }

                String responseStr = response.body().string();

                if (!response.isSuccessful()) {
                    // Try to extract error message from JSON
                    String errMsg = "HTTP " + response.code();
                    try {
                        JSONObject errJson = new JSONObject(responseStr);
                        if (errJson.has("error")) {
                            Object e = errJson.get("error");
                            if (e instanceof JSONObject) {
                                errMsg = ((JSONObject) e).optString("message", errMsg);
                            } else {
                                errMsg = e.toString();
                            }
                        }
                    } catch (Exception ignored) {}
                    if (cb != null) cb.onError(errMsg);
                    return;
                }

                // Step 4: Parse response
                String reply = extractContent(responseStr);
                if (reply == null || reply.isEmpty()) {
                    if (cb != null) cb.onError("Empty reply from AI");
                    return;
                }

                // Step 5: Store assistant turn in Rust history
                try {
                    RustBridge.pushAssistantTurn(reply);
                } catch (Throwable ignored) {}

                // Drain any pending shell jobs
                drainShellQueue(cb);

                if (cb != null) cb.onReply(reply);

            } catch (Throwable e) {
                Log.e(TAG, "chat error", e);
                String msg = e.getMessage();
                if (msg == null) msg = e.getClass().getSimpleName();
                final String errMsg = msg;
                if (cb != null) cb.onError(errMsg);
            }
        }, "KiraAI-Chat").start(); // No custom stack size needed — no Rust TLS
    }

    /** Extract content from OpenAI-compatible response JSON */
    private String extractContent(String json) {
        try {
            JSONObject obj = new JSONObject(json);
            JSONArray choices = obj.getJSONArray("choices");
            if (choices.length() == 0) return null;
            JSONObject message = choices.getJSONObject(0).getJSONObject("message");
            return message.getString("content");
        } catch (Exception e) {
            // Fallback: manual parse for content field
            return parseJsonStr(json, "content");
        }
    }

    private void drainShellQueue(Callback cb) {
        for (int i = 0; i < 20; i++) {
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
            } catch (Throwable e) {
                Log.w(TAG, "shell job: " + e.getMessage());
                break;
            }
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
