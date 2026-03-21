package com.kira.service;

import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.os.BatteryManager;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import com.kira.service.ai.KiraAI;
import org.json.JSONObject;

/**
 * KiraWatcher — Session H: thin 5s polling wrapper.
 * Original: 229 lines. Rewritten: ~80 lines.
 *
 * Java responsibility (only things requiring Android APIs):
 *   1. Read battery level + charging state → POST /macro/tick
 *   2. Read foreground package (via Shizuku) → POST /macro/tick
 *   3. Read screen text (via AccessibilityService) → POST /macro/tick
 *   4. Rust evaluates all triggers, queues fired actions
 *   5. Drain /macro/pending_results for any intent-based actions
 *
 * All trigger evaluation logic now in Rust /macro/tick.
 */
public class KiraWatcher {
    private static final String TAG      = "KiraWatcher";
    private static final int    INTERVAL = 5_000;

    private final Context ctx;
    private final Handler handler;
    private volatile boolean running = false;

    public KiraWatcher(Context ctx, KiraAI unused) {
        this.ctx     = ctx.getApplicationContext();
        this.handler = new Handler(Looper.getMainLooper());
    }

    public void start() {
        if (running) return;
        running = true;
        handler.postDelayed(this::tick, INTERVAL);
        Log.i(TAG, "watcher started");
    }

    public void stop() {
        running = false;
        handler.removeCallbacksAndMessages(null);
    }

    private void tick() {
        if (!running) return;
        new Thread(this::doTick, "kira-watcher-tick").start();
        handler.postDelayed(this::tick, INTERVAL);
    }

    private void doTick() {
        try {
            // Collect device state
            int     battery   = getBattery();
            boolean charging  = isCharging();
            String  pkg       = getForegroundPkg();
            String  screenTxt = getScreenText();
            String  screenHash= String.valueOf(screenTxt.hashCode());

            // POST to Rust — Rust evaluates all macro triggers
            String body = String.format(
                "{\"battery\":%d,\"charging\":%b,\"pkg\":\"%s\","
                + "\"screen_hash\":\"%s\",\"screen_text\":\"%s\"}",
                battery, charging,
                pkg.replace("\"",""),
                screenHash,
                screenTxt.length() > 500
                    ? screenTxt.substring(0,500).replace("\"","'")
                    : screenTxt.replace("\"","'")
            );
            httpPost("http://localhost:7070/macro/tick", body);

            // Drain any intent-based actions Rust queued
            drainPendingResults();

        } catch (Exception e) {
            Log.e(TAG, "tick error: " + e.getMessage());
        }
    }

    private void drainPendingResults() {
        for (int i = 0; i < 10; i++) {
            try {
                String resp = httpGet("http://localhost:7070/macro/pending_results");
                if (resp == null || resp.contains("\"has_action\":false")) break;
                String action = parseStr(resp, "action");
                if (action.isEmpty()) break;
                // Handle intent-based actions
                if (action.startsWith("open_app:")) {
                    String pkg = action.substring(9);
                    Intent intent = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
                    if (intent != null) { intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK); ctx.startActivity(intent); }
                }
            } catch (Exception e) { break; }
        }
    }

    // ── Device state helpers (Android APIs — must stay Java) ──────────────

    private int getBattery() {
        try {
            Intent intent = ctx.registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            if (intent == null) return -1;
            int l = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int s = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            return s > 0 ? l * 100 / s : -1;
        } catch (Exception e) { return -1; }
    }

    private boolean isCharging() {
        try {
            Intent i = ctx.registerReceiver(null, new IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return false;
            int st = i.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            return st == BatteryManager.BATTERY_STATUS_CHARGING
                || st == BatteryManager.BATTERY_STATUS_FULL;
        } catch (Exception e) { return false; }
    }

    private String getForegroundPkg() {
        try {
            if (!ShizukuShell.isAvailable()) return "";
            String r = ShizukuShell.exec(
                "dumpsys activity recents | grep 'Recent #0' | grep -o 'A=[^ ]*'"
                + " | cut -d= -f2 | cut -d/ -f1 2>/dev/null", 3_000);
            return r != null ? r.trim() : "";
        } catch (Exception e) { return ""; }
    }

    private String getScreenText() {
        try {
            KiraAccessibilityService svc = KiraAccessibilityService.instance;
            return svc != null ? svc.getScreenText() : "";
        } catch (Exception e) { return ""; }
    }

    private void httpPost(String url, String body) {
        try {
            java.net.HttpURLConnection c =
                (java.net.HttpURLConnection) new java.net.URL(url).openConnection();
            c.setRequestMethod("POST");
            c.setRequestProperty("Content-Type","application/json");
            c.setConnectTimeout(2_000); c.setReadTimeout(2_000); c.setDoOutput(true);
            c.getOutputStream().write(body.getBytes());
            c.getResponseCode(); c.disconnect();
        } catch (Exception ignored) {}
    }

    private String httpGet(String url) {
        try {
            java.net.HttpURLConnection c =
                (java.net.HttpURLConnection) new java.net.URL(url).openConnection();
            c.setConnectTimeout(1_000); c.setReadTimeout(1_000);
            try (java.io.BufferedReader br = new java.io.BufferedReader(
                    new java.io.InputStreamReader(c.getInputStream()))) {
                StringBuilder sb = new StringBuilder(); String line;
                while ((line = br.readLine()) != null) sb.append(line);
                return sb.toString();
            } finally { c.disconnect(); }
        } catch (Exception e) { return null; }
    }

    private String parseStr(String json, String key) {
        String k = "\"" + key + "\":\"";
        int s = json.indexOf(k); if (s < 0) return "";
        s += k.length(); int e = s;
        while (e < json.length() && json.charAt(e) != '"') e++;
        return json.substring(s, e);
    }
}
