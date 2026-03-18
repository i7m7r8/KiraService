package com.kira.service;

import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;

import org.json.JSONObject;

/**
 * AndyClaw-inspired autonomous heartbeat.
 * Wakes up periodically and checks device status, runs scheduled tasks,
 * monitors conditions, and can proactively message via Telegram.
 */
public class KiraHeartbeat {
    private static final String TAG = "KiraHeartbeat";

    private final Context ctx;
    private final KiraAI  ai;
    private final Handler handler;
    private volatile boolean running = false;
    private int interval = 30 * 60 * 1000; // 30 minutes default

    public KiraHeartbeat(Context ctx, KiraAI ai) {
        this.ctx     = ctx.getApplicationContext();
        this.ai      = ai;
        this.handler = new Handler(Looper.getMainLooper());
    }

    public void start(int intervalMinutes) {
        this.interval = intervalMinutes * 60 * 1000;
        this.running  = true;
        scheduleNext();
        Log.i(TAG, "heartbeat started, interval=" + intervalMinutes + "m");
    }

    public void stop() {
        running = false;
        handler.removeCallbacksAndMessages(null);
    }

    private void scheduleNext() {
        if (!running) return;
        handler.postDelayed(this::beat, interval);
    }

    private void beat() {
        if (!running) return;
        Log.d(TAG, "heartbeat");

        try {
            // Update Rust state
            int pct = getBatteryPct();
            RustBridge.pushEvent("{\"type\":\"heartbeat\",\"battery\":" + pct + ",\"ts\":" + System.currentTimeMillis() + "}");

            // Check battery watch
            KiraMemory mem = new KiraMemory(ctx);
            String watchStr = mem.recall("battery_watch_threshold");
            if (!watchStr.startsWith("nothing")) {
                try {
                    int threshold = Integer.parseInt(watchStr.trim());
                    if (pct <= threshold) {
                        notifyTelegram("? Battery at " + pct + "% (below threshold " + threshold + "%)");
                    }
                } catch (Exception ignored) {}
            }

            // Check scheduled tasks
            String allMem = mem.listAll();
            if (allMem.contains("scheduled_task_")) {
                for (String line : allMem.split("\n")) {
                    if (line.startsWith("scheduled_task_")) {
                        String task = line.substring(line.indexOf(":") + 1).trim();
                        Log.d(TAG, "running scheduled task: " + task);
                        // Execute async
                        new Thread(() -> {
                            try {
                                String key = line.substring(0, line.indexOf(":")).trim();
                                ai.chat(task, new KiraAI.Callback() {
                                    @Override public void onThinking() {}
                                    @Override public void onTool(String n, String r) {}
                                    @Override public void onReply(String reply) {
                                        notifyTelegram("? Scheduled task result:\n" + reply.substring(0, Math.min(200, reply.length())));
                                        mem.forget(key); // Remove after execution
                                    }
                                    @Override public void onError(String e) {}
                                });
                            } catch (Exception e) { Log.e(TAG, "scheduled task failed", e); }
                        }).start();
                    }
                }
            }
        } catch (Exception e) {
            Log.e(TAG, "heartbeat error", e);
        }

        scheduleNext();
    }

    private int getBatteryPct() {
        try {
            android.content.Intent i = ctx.registerReceiver(null,
                new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return -1;
            int level = i.getIntExtra(android.os.BatteryManager.EXTRA_LEVEL, -1);
            int scale = i.getIntExtra(android.os.BatteryManager.EXTRA_SCALE, -1);
            return scale > 0 ? level * 100 / scale : -1;
        } catch (Exception e) { return -1; }
    }

    private void notifyTelegram(String msg) {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.tgToken.isEmpty() || cfg.tgAllowed == 0) return;
        new Thread(() -> {
            try {
                java.net.URL url = new java.net.URL(
                    "https://api.telegram.org/bot" + cfg.tgToken + "/sendMessage");
                java.net.HttpURLConnection conn = (java.net.HttpURLConnection) url.openConnection();
                conn.setRequestMethod("POST");
                conn.setRequestProperty("Content-Type", "application/json");
                conn.setDoOutput(true);
                conn.setConnectTimeout(8000);
                JSONObject body = new JSONObject();
                body.put("chat_id", cfg.tgAllowed);
                body.put("text", "? Kira heartbeat\n" + msg);
                conn.getOutputStream().write(body.toString().getBytes());
                conn.getResponseCode();
            } catch (Exception e) { Log.e(TAG, "telegram notify failed", e); }
        }).start();
    }
}
