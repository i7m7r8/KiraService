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
import java.util.HashSet;
import java.util.Set;

/**
 * Built-in Telegram bot — no Termux needed
 * Long-polls Telegram API, sends messages to KiraAI, replies
 */
public class KiraTelegram {

    private static final String TAG = "KiraTelegram";

    private final Context ctx;
    private final KiraAI ai;
    private volatile boolean running = false;
    private long lastUpdateId = 0;
    private final Set<Long> busyChats = new HashSet<>();

    public KiraTelegram(Context ctx, KiraAI ai) {
        this.ctx = ctx;
        this.ai  = ai;
    }

    public void start() {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.tgToken.isEmpty()) return;
        running = true;
        new Thread(this::pollLoop).start();
        Log.i(TAG, "Telegram bot started");
    }

    public void stop() {
        running = false;
    }

    private void pollLoop() {
        while (running) {
            try {
                KiraConfig cfg = KiraConfig.load(ctx);
                if (cfg.tgToken.isEmpty()) { Thread.sleep(5000); continue; }

                // Keep foreground service alive
                try { KiraForegroundService.start(ctx); } catch (Exception ignored) {}

                JSONObject updates = getUpdates(cfg.tgToken, lastUpdateId + 1);
                if (updates == null) { Thread.sleep(2000); continue; }

                JSONArray results = updates.optJSONArray("result");
                if (results == null) { Thread.sleep(1000); continue; }

                for (int i = 0; i < results.length(); i++) {
                    JSONObject update = results.getJSONObject(i);
                    long updateId = update.getLong("update_id");
                    if (updateId > lastUpdateId) lastUpdateId = updateId;
                    handleUpdate(update, cfg);
                }

            } catch (Exception e) {
                Log.e(TAG, "poll error", e);
                try { Thread.sleep(3000); } catch (InterruptedException ignored) {}
            }
        }
    }

    private void handleUpdate(JSONObject update, KiraConfig cfg) {
        try {
            JSONObject message = update.optJSONObject("message");
            if (message == null) return;

            long chatId  = message.getJSONObject("chat").getLong("id");
            long userId  = message.getJSONObject("from").getLong("id");
            String text  = message.optString("text", "");
            String name  = message.getJSONObject("from").optString("first_name", "user");

            // Auth check
            if (cfg.tgAllowed != 0 && userId != cfg.tgAllowed) {
                sendMessage(cfg.tgToken, chatId, "not authorized.");
                return;
            }

            if (text.equals("/start")) {
                sendMessage(cfg.tgToken, chatId, "kira connected. what do you need, " + name.toLowerCase() + "?");
                return;
            }

            if (text.isEmpty()) return;

            // Rate limit
            if (busyChats.contains(chatId)) {
                sendMessage(cfg.tgToken, chatId, "still working on that...");
                return;
            }

            busyChats.add(chatId);
            sendTyping(cfg.tgToken, chatId);

            final long fChatId = chatId;
            ai.chat(text, new KiraAI.Callback() {
                @Override public void onThinking() { sendTyping(cfg.tgToken, fChatId); }
                @Override public void onTool(String name, String result) {}
                @Override public void onReply(String reply) {
                    busyChats.remove(fChatId);
                    sendMessage(cfg.tgToken, fChatId, reply != null ? reply : "done.");
                }
                @Override public void onError(String error) {
                    busyChats.remove(fChatId);
                    sendMessage(cfg.tgToken, fChatId, "error: " + error);
                }
            });

        } catch (Exception e) {
            Log.e(TAG, "handle update error", e);
        }
    }

    private JSONObject getUpdates(String token, long offset) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token
                + "/getUpdates?timeout=25&offset=" + offset);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(30000);
            conn.setReadTimeout(30000);
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            return new JSONObject(sb.toString());
        } catch (Exception e) {
            Log.e(TAG, "getUpdates error", e);
            return null;
        }
    }

    private void sendMessage(String token, long chatId, String text) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token + "/sendMessage");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("POST");
            conn.setRequestProperty("Content-Type", "application/json");
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            conn.setDoOutput(true);
            JSONObject body = new JSONObject();
            body.put("chat_id", chatId);
            body.put("text", text.length() > 4096 ? text.substring(0, 4096) : text);
            try (OutputStream os = conn.getOutputStream()) {
                os.write(body.toString().getBytes(StandardCharsets.UTF_8));
            }
            conn.getResponseCode();
        } catch (Exception e) {
            Log.e(TAG, "sendMessage error", e);
        }
    }

    private void sendTyping(String token, long chatId) {
        try {
            URL url = new URL("https://api.telegram.org/bot" + token + "/sendChatAction");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod("POST");
            conn.setRequestProperty("Content-Type", "application/json");
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
}
