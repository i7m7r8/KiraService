package com.kira.service.telegram;

import android.content.Context;
import android.util.Log;
import com.kira.service.ai.KiraConfig;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * KiraTelegram — Session F: thin Java polling wrapper.
 * Original: 333 lines. Rewritten: ~120 lines.
 *
 * Java responsibility (only things Rust cannot do — raw HTTP to Telegram API):
 *   1. Long-poll getUpdates from Telegram
 *   2. POST /telegram/incoming to Rust (Rust runs AI + queues reply)
 *   3. Poll GET /telegram/next_send from Rust
 *   4. Send reply via Telegram sendMessage API
 */
public class KiraTelegram {
    private static final String TAG      = "KiraTelegram";
    private static final String BASE_URL = "https://api.telegram.org/bot";
    private static final int    TIMEOUT  = 25;

    private final Context ctx;
    private final AtomicBoolean running = new AtomicBoolean(false);
    private Thread pollThread;

    public KiraTelegram(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    // Legacy constructor
    public KiraTelegram(Context ctx, Object unused) { this(ctx); }

    public void start() {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.tgToken == null || cfg.tgToken.isEmpty()) return;
        } catch (Exception e) {
            Log.w(TAG, "config load failed, deferring telegram start: " + e.getMessage());
            // Retry after 5s — Rust may not be loaded yet
            new android.os.Handler(android.os.Looper.getMainLooper())
                .postDelayed(this::start, 5000);
            return;
        }
        if (!running.compareAndSet(false, true)) return;
        pollThread = new Thread(this::pollLoop, "kira-telegram");
        pollThread.setDaemon(true);
        pollThread.start();
        Log.i(TAG, "Telegram polling started");
    }

    public void stop() {
        running.set(false);
        if (pollThread != null) pollThread.interrupt();
    }

    private void pollLoop() {
        while (running.get()) {
            try {
                KiraConfig cfg;
                try { cfg = KiraConfig.load(ctx); }
                catch (Exception ex) { Thread.sleep(5000); continue; }
                if (cfg.tgToken.isEmpty()) { Thread.sleep(5000); continue; }

                // Get last update id from Rust
                long offset = getLastUpdateId() + 1;

                // Long-poll Telegram
                String updatesUrl = BASE_URL + cfg.tgToken
                    + "/getUpdates?timeout=" + TIMEOUT + "&offset=" + offset;
                String response = httpGet(updatesUrl, (TIMEOUT + 5) * 1000);
                if (response == null) { Thread.sleep(1000); continue; }

                processUpdates(response, cfg.tgToken);

                // Drain Rust send queue
                drainSendQueue(cfg.tgToken);

            } catch (InterruptedException e) {
                Thread.currentThread().interrupt(); break;
            } catch (Exception e) {
                Log.e(TAG, "poll error: " + e.getMessage());
                try { Thread.sleep(3000); } catch (InterruptedException ie) { break; }
            }
        }
        Log.i(TAG, "Telegram polling stopped");
    }

    private void processUpdates(String json, String token) throws Exception {
        if (!json.contains("\"ok\":true")) return;
        int pos = json.indexOf("\"result\":[");
        if (pos < 0) return;
        // Simple array scan
        int i = pos;
        while ((i = json.indexOf("\"update_id\":", i)) >= 0) {
            long updateId = parseLong(json, i + 13);
            long chatId   = parseChatId(json, i);
            String user   = parseStr(json, i, "first_name");
            String text   = parseStr(json, i, "text");
            i += 15; // advance past this update
            if (text.isEmpty() || chatId == 0) continue;

            // POST to Rust — Rust runs AI, queues reply
            String body = String.format(
                "{\"update_id\":%d,\"chat_id\":%d,\"user\":\"%s\",\"text\":\"%s\"}",
                updateId, chatId, user.replace("\"","\\\""), text.replace("\"","\\\""));
            httpPost("http://localhost:7070/telegram/incoming", body, 10_000);
        }
    }

    private void drainSendQueue(String token) throws Exception {
        for (int i = 0; i < 10; i++) {
            String msg = httpGet("http://localhost:7070/telegram/next_send", 2_000);
            if (msg == null || msg.contains("\"has_message\":false")) break;
            long chatId = parseLongKey(msg, "chat_id");
            String text = parseStr(msg, 0, "text");
            if (chatId == 0 || text.isEmpty()) break;
            sendMessage(token, chatId, text);
        }
    }

    private void sendMessage(String token, long chatId, String text) throws Exception {
        String body = String.format(
            "{\"chat_id\":%d,\"text\":\"%s\"}",
            chatId, text.replace("\\","\\\\").replace("\"","\\\"").replace("\n","\\n"));
        httpPost(BASE_URL + token + "/sendMessage", body, 10_000);
    }

    private long getLastUpdateId() {
        try {
            String r = httpGet("http://localhost:7070/telegram/last_update_id", 1_000);
            if (r == null) return 0;
            return parseLongKey(r, "update_id");
        } catch (Exception e) { return 0; }
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────

    private String httpGet(String url, int timeoutMs) {
        try {
            HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(timeoutMs); c.setReadTimeout(timeoutMs);
            try (BufferedReader br = new BufferedReader(new InputStreamReader(c.getInputStream()))) {
                StringBuilder sb = new StringBuilder(); String line;
                while ((line = br.readLine()) != null) sb.append(line);
                return sb.toString();
            } finally { c.disconnect(); }
        } catch (Exception e) { return null; }
    }

    private void httpPost(String url, String body, int timeoutMs) {
        try {
            HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
            c.setRequestMethod("POST");
            c.setRequestProperty("Content-Type","application/json");
            c.setConnectTimeout(timeoutMs); c.setReadTimeout(timeoutMs);
            c.setDoOutput(true);
            c.getOutputStream().write(body.getBytes(StandardCharsets.UTF_8));
            c.getResponseCode(); c.disconnect();
        } catch (Exception ignored) {}
    }

    // ── Minimal JSON helpers ──────────────────────────────────────────────

    private long parseLong(String s, int from) {
        int e = from; while (e < s.length() && Character.isDigit(s.charAt(e))) e++;
        try { return Long.parseLong(s.substring(from, e)); } catch (Exception ex) { return 0; }
    }
    private long parseLongKey(String s, String key) {
        String k = "\"" + key + "\":"; int i = s.indexOf(k);
        return i < 0 ? 0 : parseLong(s, i + k.length());
    }
    private long parseChatId(String s, int from) {
        int i = s.indexOf("\"id\":", from);
        return i < 0 ? 0 : parseLong(s, i + 6);
    }
    private String parseStr(String s, int from, String key) {
        String k = "\"" + key + "\":\"";
        int i = s.indexOf(k, from); if (i < 0) return "";
        i += k.length(); int e = i;
        while (e < s.length() && !(s.charAt(e)=='"' && s.charAt(e-1)!='\\')) e++;
        return s.substring(i, Math.min(e, s.length())).replace("\\n","\n");
    }
}
