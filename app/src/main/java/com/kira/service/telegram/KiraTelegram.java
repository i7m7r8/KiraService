package com.kira.service.telegram;

import android.content.Context;
import android.util.Log;

import com.kira.service.KiraForegroundService;
import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicLong;

public class KiraTelegram {

    private static final String TAG = "KiraTelegram";
    private static final int POLL_TIMEOUT = 25; // long poll seconds
    private static final int MAX_MSG_LEN = 4096;

    private final Context ctx;
    private final KiraAI  ai;
    private volatile boolean running = false;
    private Thread pollThread;
    private final AtomicLong lastUpdateId = new AtomicLong(0);

    public KiraTelegram(Context ctx, KiraAI ai) {
        this.ctx = ctx.getApplicationContext(); // use app context to avoid leaks
        this.ai  = ai;
    }

    public void start() {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.tgToken == null || cfg.tgToken.trim().isEmpty()) {
            Log.w(TAG, "No Telegram token configured -- bot not starting");
            return;
        }

        if (running) {
            Log.i(TAG, "Already running");
            return;
        }

        running = true;

        // Start foreground service FIRST so Android doesn't kill us
        KiraForegroundService.start(ctx);

        pollThread = new Thread(this::pollLoop, "kira-telegram");
        pollThread.setDaemon(false); // keep alive
        pollThread.start();
        Log.i(TAG, "Telegram bot started with token: ****" +
            cfg.tgToken.substring(Math.max(0, cfg.tgToken.length() - 6)));
    }

    public void stop() {
        running = false;
        if (pollThread != null) pollThread.interrupt();
        Log.i(TAG, "Telegram bot stopped");
    }

    public boolean isRunning() { return running && pollThread != null && pollThread.isAlive(); }

    // -- Poll loop -------------------------------------------------------------

    private void pollLoop() {
        Log.i(TAG, "Poll loop started");
        int errorCount = 0;

        while (running && !Thread.interrupted()) {
            try {
                KiraConfig cfg = KiraConfig.load(ctx);

                if (cfg.tgToken == null || cfg.tgToken.trim().isEmpty()) {
                    Log.w(TAG, "Token cleared -- stopping bot");
                    break;
                }

                // Keep foreground service alive
                KiraForegroundService.start(ctx);

                JSONObject updates = getUpdates(cfg.tgToken, lastUpdateId.get() + 1);

                if (updates == null) {
                    errorCount++;
                    if (errorCount > 5) {
                        Log.e(TAG, "Too many errors, sleeping 30s");
                        Thread.sleep(30000);
                        errorCount = 0;
                    } else {
                        Thread.sleep(3000);
                    }
                    continue;
                }

                errorCount = 0;
                JSONArray results = updates.optJSONArray("result");
                if (results == null) continue;

                for (int i = 0; i < results.length(); i++) {
                    JSONObject update = results.getJSONObject(i);
                    long updateId = update.optLong("update_id", 0);
                    if (updateId > lastUpdateId.get()) lastUpdateId.set(updateId);
                    handleUpdate(update, cfg);
                }

            } catch (InterruptedException e) {
                Log.i(TAG, "Poll thread interrupted");
                break;
            } catch (Exception e) {
                Log.e(TAG, "Poll error: " + e.getMessage());
                try { Thread.sleep(5000); } catch (InterruptedException ie) { break; }
            }
        }

        running = false;
        Log.i(TAG, "Poll loop ended");
    }

    // -- Handle update ---------------------------------------------------------

    private void handleUpdate(JSONObject update, KiraConfig cfg) {
        try {
            JSONObject message = update.optJSONObject("message");
            if (message == null) {
                // Also handle callback queries
                JSONObject callback = update.optJSONObject("callback_query");
                if (callback != null) handleCallback(callback, cfg);
                return;
            }

            long chatId = message.getJSONObject("chat").getLong("id");
            long userId = message.getJSONObject("from").getLong("id");
            String text = message.optString("text", "").trim();
            String name = message.getJSONObject("from").optString("first_name", "user");

            // Auth -- allow if tgAllowed is 0 (any) or matches userId
            if (cfg.tgAllowed != 0 && userId != cfg.tgAllowed) {
                sendMessage(cfg.tgToken, chatId, "not authorized. your id: " + userId);
                return;
            }

            if (text.isEmpty()) return;

            // Built-in commands
            if (text.equals("/start")) {
                sendMessage(cfg.tgToken, chatId,
                    "? *Kira Agent* connected\n\nHey " + name.toLowerCase() + ". I'm running on your phone. What do you need?",
                    true);
                return;
            }

            if (text.equals("/status")) {
                String status = "? *Status*\n"
                    + "? Accessibility: " + (com.kira.service.KiraAccessibilityService.instance != null ? "?" : "?") + "\n"
                    + "? Shizuku: " + (com.kira.service.ShizukuShell.isAvailable() ? "?" : "?") + "\n"
                    + "? Bot: ? running";
                sendMessage(cfg.tgToken, chatId, status, true);
                return;
            }

            if (text.equals("/screen")) {
                String screen = ai.quickTool("read_screen", new JSONObject());
                sendMessage(cfg.tgToken, chatId, "? Screen:\n```\n" + truncate(screen, 3000) + "\n```", true);
                return;
            }

            if (text.equals("/notifs")) {
                String notifs = ai.quickTool("get_notifications", new JSONObject());
                sendMessage(cfg.tgToken, chatId, "? Notifications:\n" + truncate(notifs, 3000), true);
                return;
            }

            if (text.startsWith("/chain ")) {
                String goal = text.substring(7);
                sendTyping(cfg.tgToken, chatId);
                final long fChatId2 = chatId;
                final String fToken2 = cfg.tgToken;
                new com.kira.service.ai.KiraChain(ctx).run(goal, new com.kira.service.ai.KiraChain.ChainCallback() {
                    StringBuilder log = new StringBuilder();
                    @Override public void onThought(String t) { log.append("Think: ").append(t).append("\n"); }
                    @Override public void onAction(String tool, String a) { log.append("Act: ").append(tool).append("\n"); }
                    @Override public void onObservation(String o) { log.append("Obs: ").append(o.substring(0,Math.min(60,o.length()))).append("\n"); }
                    @Override public void onFinal(String answer) { sendMessage(fToken2, fChatId2, log.toString().trim() + "\n\nResult: " + answer, false); }
                    @Override public void onError(String e) { sendMessage(fToken2, fChatId2, "chain error: " + e, false); }
                });
                return;
            }
            if (text.startsWith("/run ")) {
                String cmd = text.substring(5);
                String result = com.kira.service.ShizukuShell.exec(cmd);
                sendMessage(cfg.tgToken, chatId, "```\n" + truncate(result, 3500) + "\n```", true);
                return;
            }

            // Send typing
            sendTyping(cfg.tgToken, chatId);

            // Chat with AI
            final long fChatId = chatId;
            final String fToken = cfg.tgToken;

            ai.chat(text, new KiraAI.Callback() {
                StringBuilder toolLog = new StringBuilder();

                @Override public void onThinking() {
                    sendTyping(fToken, fChatId);
                }

                @Override public void onTool(String name, String result) {
                    toolLog.append("? ").append(name).append("\n");
                }

                @Override public void onReply(String reply) {
                    String full = reply;
                    if (toolLog.length() > 0) {
                        full = toolLog.toString().trim() + "\n\n" + reply;
                    }
                    sendMessage(fToken, fChatId, truncate(full, MAX_MSG_LEN), true);
                }

                @Override public void onError(String error) {
                    sendMessage(fToken, fChatId, "? " + error, false);
                }
            });

        } catch (Exception e) {
            Log.e(TAG, "handleUpdate error", e);
        }
    }

    private void handleCallback(JSONObject callback, KiraConfig cfg) {
        // Future: inline keyboard callbacks
    }

    // -- API calls -------------------------------------------------------------

    private JSONObject getUpdates(String token, long offset) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token
                + "/getUpdates?timeout=" + POLL_TIMEOUT + "&offset=" + offset
                + "&allowed_updates=[\"message\",\"callback_query\"]");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout((POLL_TIMEOUT + 5) * 1000);
            conn.setRequestProperty("User-Agent", "KiraAgent/5.0");

            int code = conn.getResponseCode();
            if (code != 200) {
                Log.w(TAG, "getUpdates HTTP " + code);
                return null;
            }

            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            return new JSONObject(sb.toString());

        } catch (Exception e) {
            if (running) Log.w(TAG, "getUpdates error: " + e.getMessage());
            return null;
        }
    }

    private void sendMessage(String token, long chatId, String text, boolean markdown) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token + "/sendMessage");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("POST");
            conn.setRequestProperty("Content-Type", "application/json; charset=utf-8");
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(15000);
            conn.setDoOutput(true);

            JSONObject body = new JSONObject();
            body.put("chat_id", chatId);
            body.put("text", text.length() > MAX_MSG_LEN ? text.substring(0, MAX_MSG_LEN) : text);
            if (markdown) body.put("parse_mode", "Markdown");

            byte[] bytes = body.toString().getBytes(StandardCharsets.UTF_8);
            try (OutputStream os = conn.getOutputStream()) { os.write(bytes); }

            int code = conn.getResponseCode();
            if (code != 200) {
                // Retry without markdown if parse error
                if (markdown && code == 400) {
                    sendMessage(token, chatId, text, false);
                    return;
                }
                Log.w(TAG, "sendMessage HTTP " + code);
            }

        } catch (Exception e) {
            Log.e(TAG, "sendMessage error", e);
        }
    }

    private void sendMessage(String token, long chatId, String text) {
        sendMessage(token, chatId, text, false);
    }

    private void sendTyping(String token, long chatId) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token + "/sendChatAction");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("POST");
            conn.setRequestProperty("Content-Type", "application/json");
            conn.setConnectTimeout(5000);
            conn.setReadTimeout(5000);
            conn.setDoOutput(true);
            JSONObject body = new JSONObject();
            body.put("chat_id", chatId);
            body.put("action", "typing");
            try (OutputStream os = conn.getOutputStream()) {
                os.write(body.toString().getBytes(StandardCharsets.UTF_8));
            }
            conn.getResponseCode();
        } catch (Exception ignored) {}
    }

    private String truncate(String s, int max) {
        if (s == null) return "(null)";
        return s.length() > max ? s.substring(0, max) + "?" : s;
    }
}
