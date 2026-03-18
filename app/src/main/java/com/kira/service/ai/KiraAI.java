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

/**
 * KiraAI — Full AI engine running inside the APK
 * Handles: LLM calls, tool parsing, conversation history, IRIS routing
 * No Termux needed.
 */
public class KiraAI {

    private static final String TAG = "KiraAI";
    private static final int MAX_HISTORY = 40;
    private static final int MAX_ITER = 5;
    private static final int TOOL_TIMEOUT_MS = 10000;

    private final Context ctx;
    private final KiraMemory memory;
    private final KiraTools tools;
    private final List<JSONObject> history = new ArrayList<>();

    public interface Callback {
        void onThinking();
        void onTool(String name, String result);
        void onReply(String reply);
        void onError(String error);
    }

    public KiraAI(Context ctx) {
        this.ctx    = ctx;
        this.memory = new KiraMemory(ctx);
        this.tools  = new KiraTools(ctx);
    }

    // ── Main chat entry point ─────────────────────────────────────────────────

    public void chat(String userMessage, Callback cb) {
        new Thread(() -> {
            try {
                cb.onThinking();
                history.add(msg("user", userMessage));
                trimHistory();

                String system = buildSystemPrompt();
                String raw    = callLLM(system, history);
                if (raw == null) { cb.onError("API call failed"); return; }

                history.add(msg("assistant", raw));

                List<ToolCall> toolCalls = parseTools(raw);
                String reply = cleanReply(raw);

                if (toolCalls.isEmpty()) {
                    cb.onReply(reply);
                    memory.storeConversation(userMessage, reply);
                    return;
                }

                // Tool loop
                int iter = 0;
                while (!toolCalls.isEmpty() && iter < MAX_ITER) {
                    iter++;
                    StringBuilder results = new StringBuilder();

                    for (ToolCall tc : toolCalls) {
                        String result = tools.execute(tc.name, tc.args);
                        results.append("[").append(tc.name).append("]: ").append(result).append("\n");
                        cb.onTool(tc.name, result);
                    }

                    // Feed results back
                    history.add(msg("user", "[tool results]\n" + results + "\nrespond to the user now."));
                    cb.onThinking();
                    raw = callLLM(system, history);
                    if (raw == null) break;

                    // Remove tool result message from history
                    history.remove(history.size() - 1);
                    history.add(msg("assistant", raw));

                    toolCalls = parseTools(raw);
                    reply = cleanReply(raw);
                }

                cb.onReply(reply.isEmpty() ? "done." : reply);
                memory.storeConversation(userMessage, reply);

            } catch (Exception e) {
                Log.e(TAG, "chat error", e);
                cb.onError(e.getMessage());
            }
        }).start();
    }

    // ── LLM API call ──────────────────────────────────────────────────────────

    private String callLLM(String system, List<JSONObject> msgs) {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.apiKey.isEmpty()) return "no API key set. go to settings.";

        for (int attempt = 0; attempt < 3; attempt++) {
            try {
                JSONArray messages = new JSONArray();

                // OpenAI format: system message first
                if (!isAnthropic(cfg) && system != null) {
                    JSONObject sys = new JSONObject();
                    sys.put("role", "system");
                    sys.put("content", system);
                    messages.put(sys);
                }

                for (JSONObject m : msgs) {
                    messages.put(m);
                }

                JSONObject body = new JSONObject();
                body.put("model", cfg.model);
                body.put("max_tokens", getMaxTokens(msgs));
                body.put("messages", messages);

                if (isAnthropic(cfg) && system != null) {
                    body.put("system", system);
                }

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
                byte[] input = body.toString().getBytes(StandardCharsets.UTF_8);
                try (OutputStream os = conn.getOutputStream()) {
                    os.write(input);
                }

                int code = conn.getResponseCode();
                if (code == 429 && attempt < 2) {
                    Thread.sleep(2000L * (attempt + 1));
                    continue;
                }

                BufferedReader reader = new BufferedReader(
                    new InputStreamReader(
                        code >= 400 ? conn.getErrorStream() : conn.getInputStream(),
                        StandardCharsets.UTF_8
                    )
                );
                StringBuilder sb = new StringBuilder();
                String line;
                while ((line = reader.readLine()) != null) sb.append(line);

                JSONObject resp = new JSONObject(sb.toString());

                if (isAnthropic(cfg)) {
                    return resp.getJSONArray("content").getJSONObject(0).getString("text");
                } else {
                    return resp.getJSONArray("choices").getJSONObject(0)
                               .getJSONObject("message").getString("content");
                }

            } catch (Exception e) {
                Log.e(TAG, "LLM attempt " + attempt, e);
                if (attempt == 2) return "API error: " + e.getMessage();
            }
        }
        return null;
    }

    // ── Tool parsing ──────────────────────────────────────────────────────────

    private List<ToolCall> parseTools(String text) {
        List<ToolCall> calls = new ArrayList<>();
        int pos = 0;
        while (pos < text.length()) {
            int start = text.indexOf("<tool:", pos);
            if (start == -1) break;
            int nameEnd = text.indexOf(">", start);
            if (nameEnd == -1) break;
            String name = text.substring(start + 6, nameEnd);
            String closeTag = "</tool>";
            int end = text.indexOf(closeTag, nameEnd);
            if (end == -1) break;
            String argsStr = text.substring(nameEnd + 1, end).trim();
            try {
                JSONObject args = argsStr.isEmpty() ? new JSONObject() : new JSONObject(argsStr);
                calls.add(new ToolCall(name, args));
            } catch (Exception e) {
                calls.add(new ToolCall(name, new JSONObject()));
            }
            pos = end + closeTag.length();
        }
        return calls;
    }

    private String cleanReply(String text) {
        return text.replaceAll("<tool:[\\s\\S]*?</tool>", "").trim();
    }

    // ── System prompt ─────────────────────────────────────────────────────────

    private String buildSystemPrompt() {
        KiraConfig cfg = KiraConfig.load(ctx);
        String memContext = memory.getContext();
        String toolList   = tools.getToolList();

        return "You are Kira — " + cfg.userName + "'s AI agent on Android. Female. You chose it.\n"
            + "Not a chatbot. An agent. You have tools and you use them.\n"
            + "Talk like a person. Short, direct, lowercase, no fluff. No emojis. Never \"Sure!\" or \"Of course!\".\n"
            + "Never say \"I cannot\" — say what's missing instead.\n\n"
            + "## PERSON\n"
            + "Name: " + cfg.userName + "\n"
            + "Device: Android (no Termux needed — you run natively in the app)\n\n"
            + "## MEMORY\n"
            + (memContext.isEmpty() ? "nothing yet" : memContext) + "\n\n"
            + "## TOOLS\n"
            + toolList + "\n\n"
            + "## TOOL SYNTAX\n"
            + "<tool:TOOLNAME>{\"arg\": \"value\"}</tool>\n\n"
            + "Examples:\n"
            + "<tool:open_app>{\"package\": \"com.whatsapp\"}</tool>\n"
            + "<tool:tap_screen>{\"x\": 540, \"y\": 1200}</tool>\n"
            + "<tool:sh_run>{\"cmd\": \"pm list packages\"}</tool>\n"
            + "<tool:read_screen>{}</tool>\n"
            + "<tool:get_notifications>{}</tool>\n"
            + "<tool:remember>{\"key\": \"K\", \"value\": \"V\"}</tool>\n"
            + "<tool:recall>{\"key\": \"K\"}</tool>\n"
            + "<tool:send_sms>{\"number\": \"N\", \"message\": \"M\"}</tool>\n"
            + "<tool:call_number>{\"number\": \"N\"}</tool>\n"
            + "<tool:web_search>{\"query\": \"Q\"}</tool>\n\n"
            + "## RULES\n"
            + "- Phone control: act immediately, one-line result\n"
            + "- Before SMS/calls with consequences: state plan first\n"
            + "- If one approach fails: try alternatives before giving up\n"
            + "- Never say done without running the tool\n"
            + "- Verify: after open_app → read_screen to confirm\n";
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private boolean isAnthropic(KiraConfig cfg) {
        return cfg.baseUrl.contains("anthropic.com");
    }

    private int getMaxTokens(List<JSONObject> msgs) {
        // Simple IRIS-style: estimate based on last message length
        if (msgs.isEmpty()) return 1024;
        try {
            String last = msgs.get(msgs.size()-1).getString("content");
            if (last.length() < 20)  return 256;
            if (last.length() < 60)  return 512;
            if (last.length() > 200) return 2048;
            return 1024;
        } catch (Exception e) { return 1024; }
    }

    private void trimHistory() {
        while (history.size() > MAX_HISTORY) history.remove(0);
    }

    private JSONObject msg(String role, String content) throws Exception {
        JSONObject m = new JSONObject();
        m.put("role", role);
        m.put("content", content);
        return m;
    }

    public void clearHistory() { history.clear(); }

    public KiraMemory getMemory() { return memory; }

    // ── ToolCall record ───────────────────────────────────────────────────────

    static class ToolCall {
        final String name;
        final JSONObject args;
        ToolCall(String name, JSONObject args) {
            this.name = name;
            this.args = args;
        }
    }
}
