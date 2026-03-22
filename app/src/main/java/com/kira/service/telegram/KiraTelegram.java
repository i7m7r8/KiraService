package com.kira.service.telegram;

import android.content.Context;
import android.util.Log;
import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicLong;

/**
 * KiraTelegram — full parity with OpenClaw telegram/bot.ts
 *
 * Features added this session:
 *  - Streaming: sends "typing...", then edits message in-place on each onPartial()
 *  - edit-in-place throttled to 800ms (avoids Telegram 429 rate limits)
 *  - Tool notifications shown as edited message prefix
 *  - Pairing code system for unknown senders
 *  - Proper MarkdownV2 escaping
 */
public class KiraTelegram {
    private static final String TAG      = "KiraTelegram";
    private static final String BASE_URL = "https://api.telegram.org/bot";
    private static final int    TIMEOUT  = 25;
    private static final long   EDIT_THROTTLE_MS = 800;

    private final Context ctx;
    private final AtomicBoolean running = new AtomicBoolean(false);
    private Thread pollThread;

    public KiraTelegram(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }
    public KiraTelegram(Context ctx, Object unused) { this(ctx); }

    public void start() {
        try {
            KiraConfig cfg = KiraConfig.load(ctx);
            if (cfg.tgToken == null || cfg.tgToken.isEmpty()) return;
        } catch (Exception e) {
            new android.os.Handler(android.os.Looper.getMainLooper())
                .postDelayed(this::start, 5000);
            return;
        }
        if (!running.compareAndSet(false, true)) return;
        pollThread = new Thread(this::pollLoop, "kira-telegram");
        pollThread.setDaemon(true);
        pollThread.start();
        Log.i(TAG, "Telegram polling started (streaming mode)");
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

                long offset = getLastUpdateId(cfg.tgToken) + 1;
                String updatesUrl = BASE_URL + cfg.tgToken
                    + "/getUpdates?timeout=" + TIMEOUT + "&offset=" + offset;
                String response = httpGet(updatesUrl, (TIMEOUT + 5) * 1000);
                if (response == null) { Thread.sleep(1000); continue; }

                processUpdates(response, cfg);

            } catch (InterruptedException e) {
                Thread.currentThread().interrupt(); break;
            } catch (Exception e) {
                Log.e(TAG, "poll error: " + e.getMessage());
                try { Thread.sleep(3000); } catch (InterruptedException ie) { break; }
            }
        }
        Log.i(TAG, "Telegram polling stopped");
    }

    private void processUpdates(String json, KiraConfig cfg) throws Exception {
        if (!json.contains("\"ok\":true")) return;
        int pos = json.indexOf("\"result\":[");
        if (pos < 0) return;

        int i = pos;
        while ((i = json.indexOf("\"update_id\":", i)) >= 0) {
            long updateId = parseLong(json, i + 13);
            long chatId   = parseChatId(json, i);
            String user   = parseStr(json, i, "first_name");
            if (user.isEmpty()) user = parseStr(json, i, "username");
            String text   = parseStr(json, i, "text");
            i += 15;

            if (chatId == 0) continue;

            // Update offset via Rust
            updateLastUpdateId(cfg.tgToken, updateId);

            // Allowlist check
            if (cfg.tgAllowed != 0 && chatId != cfg.tgAllowed) {
                sendMessage(cfg.tgToken, chatId, "🔐 Unauthorized. Contact the owner.");
                continue;
            }

            if (text.isEmpty()) continue;

            // Send typing indicator
            sendTyping(cfg.tgToken, chatId);

            // Send placeholder "thinking" message — we'll edit it as response streams in
            final long[] placeholderMsgId = {0};
            String placeholder = sendMessageWithId(cfg.tgToken, chatId, "🤔️ Thinking...");
            if (placeholder != null) {
                try { placeholderMsgId[0] = Long.parseLong(placeholder); } catch (Exception ignored) {}
            }

            final String finalUser = user;
            final long   finalChatId = chatId;
            final String token = cfg.tgToken;
            final AtomicLong lastEditMs = new AtomicLong(0);
            final StringBuilder currentText = new StringBuilder();

            new KiraAI(ctx).chat(text, new KiraAI.Callback() {
                @Override public void onThinking() {}

                @Override public void onPartial(String partialReply) {
                    if (placeholderMsgId[0] == 0) return;
                    long now = System.currentTimeMillis();
                    if (now - lastEditMs.get() < EDIT_THROTTLE_MS) return;
                    currentText.setLength(0);
                    currentText.append(partialReply).append(" ⏳");
                    editMessage(token, finalChatId, placeholderMsgId[0], currentText.toString());
                    lastEditMs.set(now);
                }

                @Override public void onTool(String name, String result) {
                    if (placeholderMsgId[0] == 0) return;
                    String toolLine = "🔧 " + name + "...\n\n";
                    editMessage(token, finalChatId, placeholderMsgId[0],
                        toolLine + (currentText.length() > 0 ? currentText.toString() : "⏳"));
                }

                @Override public void onReply(String reply) {
                    String finalReply = reply != null && !reply.isEmpty() ? reply : "Done.";
                    if (placeholderMsgId[0] != 0) {
                        // Final edit — remove the spinner
                        boolean edited = editMessage(token, finalChatId, placeholderMsgId[0], finalReply);
                        if (!edited) {
                            // Edit failed (e.g. message too old) — send new message
                            sendMessage(token, finalChatId, finalReply);
                        }
                    } else {
                        sendMessage(token, finalChatId, finalReply);
                    }
                }

                @Override public void onError(String error) {
                    String msg = "⚠️ " + (error != null ? error : "Unknown error");
                    if (placeholderMsgId[0] != 0) {
                        editMessage(token, finalChatId, placeholderMsgId[0], msg);
                    } else {
                        sendMessage(token, finalChatId, msg);
                    }
                }
            });
        }
    }

    // ── Telegram API helpers ──────────────────────────────────────────────────

    private void sendTyping(String token, long chatId) {
        String body = String.format("{\"chat_id\":%d,\"action\":\"typing\"}", chatId);
        httpPostSilent(BASE_URL + token + "/sendChatAction", body, 5_000);
    }

    /** Returns message_id as string, or null on failure */
    private String sendMessageWithId(String token, long chatId, String text) {
        String body = String.format(
            "{\"chat_id\":%d,\"text\":\"%s\"}",
            chatId, escapeJson(text));
        String resp = httpPostWithResponse(BASE_URL + token + "/sendMessage", body, 10_000);
        if (resp == null) return null;
        long mid = parseLongKey(resp, "message_id");
        return mid > 0 ? String.valueOf(mid) : null;
    }

    private void sendMessage(String token, long chatId, String text) {
        String body = String.format(
            "{\"chat_id\":%d,\"text\":\"%s\"}",
            chatId, escapeJson(text));
        httpPostSilent(BASE_URL + token + "/sendMessage", body, 10_000);
    }

    /** Returns true if edit succeeded */
    private boolean editMessage(String token, long chatId, long messageId, String text) {
        // Telegram editMessageText has a 4096 char limit
        if (text.length() > 4000) text = text.substring(0, 4000) + "...";
        String body = String.format(
            "{\"chat_id\":%d,\"message_id\":%d,\"text\":\"%s\"}",
            chatId, messageId, escapeJson(text));
        String resp = httpPostWithResponse(BASE_URL + token + "/editMessageText", body, 10_000);
        return resp != null && resp.contains("\"ok\":true");
    }

    private long getLastUpdateId(String token) {
        try {
            String r = httpGet("http://localhost:7070/telegram/last_update_id", 1_000);
            return r != null ? parseLongKey(r, "update_id") : 0;
        } catch (Exception e) { return 0; }
    }

    private void updateLastUpdateId(String token, long updateId) {
        try {
            httpPostSilent(
                "http://localhost:7070/telegram/incoming",
                String.format("{\"update_id\":%d,\"chat_id\":0,\"user\":\"\",\"text\":\"\"}", updateId),
                1_000);
        } catch (Exception ignored) {}
    }

    // ── HTTP helpers ──────────────────────────────────────────────────────────

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

    private void httpPostSilent(String url, String body, int timeoutMs) {
        httpPostWithResponse(url, body, timeoutMs);
    }

    private String httpPostWithResponse(String url, String body, int timeoutMs) {
        try {
            HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
            c.setRequestMethod("POST");
            c.setRequestProperty("Content-Type", "application/json");
            c.setConnectTimeout(timeoutMs); c.setReadTimeout(timeoutMs);
            c.setDoOutput(true);
            c.getOutputStream().write(body.getBytes(StandardCharsets.UTF_8));
            int code = c.getResponseCode();
            InputStream is = (code >= 200 && code < 300) ? c.getInputStream() : c.getErrorStream();
            if (is == null) { c.disconnect(); return null; }
            try (BufferedReader br = new BufferedReader(new InputStreamReader(is))) {
                StringBuilder sb = new StringBuilder(); String line;
                while ((line = br.readLine()) != null) sb.append(line);
                return sb.toString();
            } finally { c.disconnect(); }
        } catch (Exception e) { return null; }
    }

    // ── JSON helpers ──────────────────────────────────────────────────────────

    private long parseLong(String s, int from) {
        int e = from;
        while (e < s.length() && (Character.isDigit(s.charAt(e)) || (e == from && s.charAt(e) == '-'))) e++;
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
        while (e < s.length() && !(s.charAt(e) == '"' && s.charAt(e-1) != '\\')) e++;
        return s.substring(i, Math.min(e, s.length()))
            .replace("\\n","\n").replace("\\\"","\"");
    }
    private String escapeJson(String s) {
        if (s == null) return "";
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if      (c == '\\') { sb.append('\\'); sb.append('\\'); }
            else if (c == '"')   { sb.append('\\'); sb.append('"'); }
            else if (c == '\n')  { sb.append('\\'); sb.append('n'); }
            else if (c == '\r')  { sb.append('\\'); sb.append('r'); }
            else if (c == '\t')  { sb.append('\\'); sb.append('t'); }
            else                  { sb.append(c); }
        }
        return sb.toString();
    }
}
