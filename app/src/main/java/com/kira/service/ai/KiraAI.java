package com.kira.service.ai;

import android.content.Context;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.List;

public class KiraAI {

    private static final String TAG       = "KiraAI";
    private static final int    MAX_HIST  = 40;
    private static final int    MAX_ITER  = 5;

    private final Context     ctx;
    private final KiraMemory  memory;
    private final KiraTools   tools;
    private final List<JSONObject> history = new ArrayList<>();
    private final KiraSkillEngine skillEngine;

    public interface Callback {
        void onThinking();
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    public KiraAI(Context ctx) {
        this.ctx    = ctx.getApplicationContext();
        this.memory = new KiraMemory(ctx);
        this.skillEngine = new KiraSkillEngine(ctx);
        skillEngine.loadCustomSkillsFromMemory();
        this.tools  = new KiraTools(ctx);
        restoreHistory(); // load previous conversations on startup
    }

    // -- Restore history from persistent storage -------------------------------

    private void restoreHistory() {
        try {
            JSONArray saved = memory.loadHistory();
            int start = Math.max(0, saved.length() - 10); // last 10 exchanges
            for (int i = start; i < saved.length(); i++) {
                JSONObject entry = saved.getJSONObject(i);
                history.add(msg("user",      entry.getString("user")));
                history.add(msg("assistant", entry.getString("kira")));
            }
            Log.i(TAG, "Restored " + history.size() + " messages from history");
        } catch (Exception e) {
            Log.e(TAG, "restoreHistory error", e);
        }
    }

    // -- Main chat -------------------------------------------------------------


    /**
     * ZeroClaw: Get active provider base URL and model.
     * Falls back to config if Rust server isn't running.
     */
    private String[] getActiveProvider() {
        try {
            okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                .connectTimeout(2, java.util.concurrent.TimeUnit.SECONDS).build();
            okhttp3.Response resp = client.newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070/providers").build()
            ).execute();
            if (resp.body() != null) {
                org.json.JSONArray providers = new org.json.JSONArray(resp.body().string());
                for (int i = 0; i < providers.length(); i++) {
                    org.json.JSONObject p = providers.getJSONObject(i);
                    if (p.optBoolean("active", false)) {
                        String baseUrl = p.optString("base_url", "");
                        String model   = p.optString("model", "");
                        if (!baseUrl.isEmpty()) return new String[]{baseUrl, model};
                    }
                }
            }
        } catch (Exception ignored) {}
        // Fallback to config
        KiraConfig cfg = KiraConfig.load(ctx);
        return new String[]{
            cfg.baseUrl.isEmpty() ? "https://api.groq.com/openai/v1" : cfg.baseUrl,
            cfg.model.isEmpty()   ? "llama-3.1-8b-instant" : cfg.model
        };
    }

    public void chat(String userMessage, Callback cb) {
        new Thread(() -> {
            try {
                cb.onThinking();
                history.add(msg("user", userMessage));
                trimHistory();

                String system = buildSystemPrompt();
                String raw    = callLLM(system, history);
                if (raw == null) { cb.onError("API call failed -- check your API key in settings"); return; }

                history.add(msg("assistant", raw));

                List<ToolCall> toolCalls = parseTools(raw);
                String reply = cleanReply(raw);

                if (toolCalls.isEmpty()) {
                    cb.onReply(reply);
                    memory.storeConversation(userMessage, reply);
                    return;
                }

                // Tool execution loop
                int iter = 0;
                while (!toolCalls.isEmpty() && iter < MAX_ITER) {
                    iter++;
                    StringBuilder results = new StringBuilder();
                    for (ToolCall tc : toolCalls) {
                        String result = tools.execute(tc.name, tc.args);
                        results.append("[").append(tc.name).append("]: ").append(result).append("\n");
                        cb.onTool(tc.name, result);
                    }

                    history.add(msg("user", "[tool results]\n" + results + "\nrespond to the user now."));
                    cb.onThinking();
                    raw = callLLM(system, history);
                    if (raw == null) break;

                    // Remove tool injection from history
                    history.remove(history.size() - 1);
                    history.add(msg("assistant", raw));

                    toolCalls = parseTools(raw);
                    reply = cleanReply(raw);
                }

                if (reply.isEmpty()) reply = "done.";
                cb.onReply(reply);
                memory.storeConversation(userMessage, reply);

            } catch (Exception e) {
                Log.e(TAG, "chat error", e);
                cb.onError(e.getMessage());
            }
        }).start();
    }

    // -- LLM call --------------------------------------------------------------

    private String callLLM(String system, List<JSONObject> msgs) {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.apiKey.isEmpty()) return "no API key -- go to settings and add one.";

        for (int attempt = 0; attempt < 3; attempt++) {
            try {
                JSONArray messages = new JSONArray();

                if (!isAnthropic(cfg) && system != null) {
                    JSONObject sys = new JSONObject();
                    sys.put("role", "system");
                    sys.put("content", system);
                    messages.put(sys);
                }
                for (JSONObject m : msgs) messages.put(m);

                JSONObject body = new JSONObject();
                body.put("model", cfg.model);
                body.put("max_tokens", getMaxTokens());
                body.put("messages", messages);
                if (isAnthropic(cfg) && system != null) body.put("system", system);

                String endpoint = isAnthropic(cfg)
                    ? cfg.baseUrl + "/messages"
                    : cfg.baseUrl + "/chat/completions";

                URL url = new URL(endpoint);
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("POST");
                conn.setRequestProperty("Content-Type", "application/json");
                conn.setConnectTimeout(30000);
                conn.setReadTimeout(60000);
                if (isAnthropic(cfg)) {
                    conn.setRequestProperty("x-api-key", cfg.apiKey);
                    conn.setRequestProperty("anthropic-version", "2023-06-01");
                } else {
                    conn.setRequestProperty("Authorization", "Bearer " + cfg.apiKey);
                }
                conn.setDoOutput(true);
                conn.getOutputStream().write(body.toString().getBytes(StandardCharsets.UTF_8));

                int code = conn.getResponseCode();
                if (code == 429 && attempt < 2) {
                    Thread.sleep(2000L * (attempt + 1));
                    continue;
                }

                BufferedReader reader = new BufferedReader(new InputStreamReader(
                    code >= 400 ? conn.getErrorStream() : conn.getInputStream(),
                    StandardCharsets.UTF_8));
                StringBuilder sb = new StringBuilder();
                String line;
                while ((line = reader.readLine()) != null) sb.append(line);

                if (code >= 400) {
                    return "API error " + code + ": " + sb.toString().substring(0, Math.min(200, sb.length()));
                }

                JSONObject resp = new JSONObject(sb.toString());
                if (isAnthropic(cfg)) {
                    return resp.getJSONArray("content").getJSONObject(0).getString("text");
                } else {
                    return resp.getJSONArray("choices").getJSONObject(0)
                               .getJSONObject("message").getString("content");
                }

            } catch (Exception e) {
                Log.e(TAG, "LLM attempt " + attempt, e);
                if (attempt == 2) return "connection error: " + e.getMessage();
            }
        }
        return null;
    }

    // -- Tool parsing ----------------------------------------------------------

    private List<ToolCall> parseTools(String text) {
        List<ToolCall> calls = new ArrayList<>();
        int pos = 0;
        while (pos < text.length()) {
            int start = text.indexOf("<tool:", pos);
            if (start == -1) break;
            int nameEnd = text.indexOf(">", start);
            if (nameEnd == -1) break;
            String name = text.substring(start + 6, nameEnd);
            int end = text.indexOf("</tool>", nameEnd);
            if (end == -1) break;
            String argsStr = text.substring(nameEnd + 1, end).trim();
            try {
                JSONObject args = argsStr.isEmpty() ? new JSONObject() : new JSONObject(argsStr);
                calls.add(new ToolCall(name, args));
            } catch (Exception e) {
                calls.add(new ToolCall(name, new JSONObject()));
            }
            pos = end + 7;
        }
        return calls;
    }

    private String cleanReply(String text) {
        return text.replaceAll("<tool:[\\s\\S]*?</tool>", "").trim();
    }

    // -- System prompt ---------------------------------------------------------

    private String buildSystemPrompt() {
        KiraConfig cfg = KiraConfig.load(ctx);
        String memCtx  = memory.getContext();
        String toolList = tools.getToolList();

        return "You are Kira -- " + cfg.userName + "'s AI agent running natively on Android. Female.\n"
            + "Not a chatbot. An agent with real tools. You control this phone.\n"
            + "Talk like a person. Short, direct, lowercase, no fluff. No emojis unless asked.\n"
            + "Never say 'I cannot' -- say what's missing instead.\n\n"
            + "## PERSON\nName: " + cfg.userName + "\n\n"
            + "## MEMORY\n" + (memCtx.isEmpty() ? "nothing yet" : memCtx) + "\n\n"
            + "## TOOLS\n" + toolList + "\n\n"
            + "## TOOL SYNTAX\n"
            + "<tool:TOOLNAME>{\"arg\": \"value\"}</tool>\n\n"
            + "## KEY EXAMPLES\n"
            + "<tool:open_app>{\"package\": \"com.google.android.youtube\"}</tool>\n"
            + "<tool:tap_screen>{\"x\": 540, \"y\": 1200}</tool>\n"
            + "<tool:read_screen>{}</tool>\n"
            + "<tool:sh_run>{\"cmd\": \"pm list packages | grep youtube\"}</tool>\n"
            + "<tool:get_notifications>{}</tool>\n"
            + "<tool:remember>{\"key\": \"user_city\", \"value\": \"Dhaka\"}</tool>\n"
            + "<tool:recall>{\"key\": \"user_city\"}</tool>\n"
            + "<tool:send_sms>{\"number\": \"+880...\", \"message\": \"text\"}</tool>\n"
            + "<tool:web_search>{\"query\": \"weather Dhaka\"}</tool>\n\n"
            + "## RULES\n"
            + "- open_app: use exact package name. If unsure, use sh_run to find it first\n"
            + "- After opening app, wait and verify with read_screen\n"
            + "- Before SMS/calls: state plan and wait for confirmation\n"
            + "- Never say done without running the tool\n"
            + "- If a tool fails: try sh_run as fallback\n"
            + "- remember important facts the user tells you\n";
    }

    // -- Helpers ---------------------------------------------------------------

    private boolean isAnthropic(KiraConfig cfg) {
        return cfg.baseUrl.contains("anthropic.com");
    }

    private int getMaxTokens() {
        return 1024;
    }

    private void trimHistory() {
        while (history.size() > MAX_HIST) history.remove(0);
    }

    private JSONObject msg(String role, String content) {
        try {
            JSONObject m = new JSONObject();
            m.put("role", role);
            m.put("content", content);
            return m;
        } catch (Exception e) { return new JSONObject(); }
    }



    // Single-turn chat without tool loop - for agent planning/reflection
    public String simpleChat(String prompt) {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            org.json.JSONArray messages = new org.json.JSONArray();
            org.json.JSONObject msg = new org.json.JSONObject();
            msg.put("role", "user");
            msg.put("content", prompt);
            messages.put(msg);
            org.json.JSONObject body = new org.json.JSONObject();
            body.put("model", cfg.model.isEmpty() ? "llama-3.1-8b-instant" : cfg.model);
            body.put("max_tokens", 500);
            body.put("messages", messages);
            String baseUrl = cfg.baseUrl.isEmpty() ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                .connectTimeout(15, java.util.concurrent.TimeUnit.SECONDS)
                .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS).build();
            okhttp3.Request req = new okhttp3.Request.Builder()
                .url(baseUrl + "/chat/completions")
                .addHeader("Authorization", "Bearer " + cfg.apiKey)
                .addHeader("Content-Type", "application/json")
                .post(okhttp3.RequestBody.create(body.toString(), okhttp3.MediaType.parse("application/json")))
                .build();
            okhttp3.Response resp = client.newCall(req).execute();
            if (resp.body() == null) return "(no response)";
            org.json.JSONObject respJson = new org.json.JSONObject(resp.body().string());
            return respJson.getJSONArray("choices").getJSONObject(0)
                .getJSONObject("message").getString("content");
        } catch (Exception e) {
            return "error: " + e.getMessage();
        }
    }

    // Quick tool call without AI -- for direct Telegram commands
    public String quickTool(String toolName, org.json.JSONObject args) {
        try {
            return tools.execute(toolName, args);
        } catch (Exception e) {
            return "error: " + e.getMessage();
        }
    }

    public void clearHistory() {
        history.clear();
        memory.clearHistory();
    }

    public KiraMemory getMemory() { return memory; }

    static class ToolCall {
        final String name;
        final JSONObject args;
        ToolCall(String name, JSONObject args) { this.name = name; this.args = args; }
    }
}
