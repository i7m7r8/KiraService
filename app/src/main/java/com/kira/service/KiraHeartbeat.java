package com.kira.service;

import android.content.Context;
import android.os.BatteryManager;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;

import org.json.JSONObject;

import java.net.HttpURLConnection;
import java.net.URL;

/**
 * NanoBot-inspired autonomous heartbeat.
 * Runs on a schedule, checks device state, fires proactive actions,
 * executes scheduled tasks, monitors triggers.
 */
public class KiraHeartbeat {
    private static final String TAG = "KiraHeartbeat";

    private final Context ctx;
    private final KiraAI  ai;
    private final Handler handler;
    private volatile boolean running = false;
    private int intervalMs = 30 * 60 * 1000; // 30 min default

    public KiraHeartbeat(Context ctx, KiraAI ai) {
        this.ctx     = ctx.getApplicationContext();
        this.ai      = ai;
        this.handler = new Handler(Looper.getMainLooper());
    }

    public void start(int intervalMinutes) {
        this.intervalMs = intervalMinutes * 60 * 1000;
        this.running    = true;
        scheduleNext();
        Log.i(TAG, "heartbeat started every " + intervalMinutes + "m");
    }

    public void stop() {
        running = false;
        handler.removeCallbacksAndMessages(null);
    }

    private void scheduleNext() {
        if (!running) return;
        handler.postDelayed(this::beat, intervalMs);
    }

    private void beat() {
        if (!running) return;
        Log.d(TAG, "beat");
        new Thread(() -> {
            try { doBeat(); } catch (Exception e) { Log.e(TAG, "beat error", e); }
        }).start();
        scheduleNext();
    }

    private void doBeat() throws Exception {
        int pct = getBatteryPct();
        boolean charging = isCharging();

        // Push to Rust state
        RustBridge.updateBattery(pct, charging);

        KiraMemory mem = new KiraMemory(ctx);

        // Battery watch
        String watchStr = mem.recall("battery_watch_threshold");
        if (!watchStr.startsWith("nothing")) {
            try {
                int threshold = Integer.parseInt(watchStr.trim());
                if (pct > 0 && pct <= threshold && !charging) {
                    sendTelegram("Battery at " + pct + "% (below " + threshold + "%). Plugging in soon?");
                }
            } catch (NumberFormatException ignored) {}
        }

        // Run scheduled tasks stored in memory
        String allMem = mem.listAll();
        if (!allMem.equals("(empty)") && allMem.contains("scheduled_task_")) {
            for (String line : allMem.split("\n")) {
                if (line.startsWith("scheduled_task_")) {
                    int colon = line.indexOf(":");
                    if (colon < 0) continue;
                    String key  = line.substring(0, colon).trim();
                    String task = line.substring(colon + 1).trim();
                    mem.forget(key); // remove before running to avoid re-run
                    Log.d(TAG, "running scheduled task: " + task);
                    ai.chat(task, new KiraAI.Callback() {
                        @Override public void onThinking() {}
                        @Override public void onTool(String n, String r) {}
                        @Override public void onReply(String reply) {
                            sendTelegram("Scheduled task done:\n" + reply.substring(0, Math.min(300, reply.length())));
                        }
                        @Override public void onError(String e) {}
                    });
                }
            }
        }

        // Check Rust fired triggers
        String triggered;
        while ((triggered = RustBridge.nextFiredTrigger()) != null) {
            Log.d(TAG, "trigger fired: " + triggered);
            try {
                JSONObject obj = new JSONObject(triggered);
                String action = obj.optString("action", "");
                if (!action.isEmpty()) {
                    final String finalAction = action;
                    ai.chat(finalAction, new KiraAI.Callback() {
                        @Override public void onThinking() {}
                        @Override public void onTool(String n, String r) {}
                        @Override public void onReply(String reply) {
                            sendTelegram("Trigger action result:\n" + reply.substring(0, Math.min(300, reply.length())));
                        }
                        @Override public void onError(String e) {}
                    });
                }
            } catch (Exception ignored) {}
        }
    }

    private int getBatteryPct() {
        try {
            android.content.Intent i = ctx.registerReceiver(null,
                new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return -1;
            int level = i.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale = i.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            return scale > 0 ? level * 100 / scale : -1;
        } catch (Exception e) { return -1; }
    }

    private boolean isCharging() {
        try {
            android.content.Intent i = ctx.registerReceiver(null,
                new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return false;
            int status = i.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            return status == BatteryManager.BATTERY_STATUS_CHARGING
                || status == BatteryManager.BATTERY_STATUS_FULL;
        } catch (Exception e) { return false; }
    }

    private void sendTelegram(String msg) {
        KiraConfig cfg = KiraConfig.load(ctx);
        if (cfg.tgToken.isEmpty() || cfg.tgAllowed == 0) return;
        new Thread(() -> {
            try {
                URL url = new URL("https://api.telegram.org/bot" + cfg.tgToken + "/sendMessage");
                HttpURLConnection conn = (HttpURLConnection) url.openConnection();
                conn.setRequestMethod("POST");
                conn.setRequestProperty("Content-Type", "application/json");
                conn.setConnectTimeout(8000);
                conn.setReadTimeout(8000);
                conn.setDoOutput(true);
                JSONObject body = new JSONObject();
                body.put("chat_id", cfg.tgAllowed);
                body.put("text", "Kira heartbeat\n" + msg);
                conn.getOutputStream().write(body.toString().getBytes("UTF-8"));
                conn.getResponseCode();
            } catch (Exception e) { Log.e(TAG, "telegram failed", e); }
        }).start();
    }
}
