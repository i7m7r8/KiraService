package com.kira.service;

import android.content.Context;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;
import com.kira.service.ai.KiraTools;

import org.json.JSONObject;

/**
 * NanoBot-style proactive screen watcher.
 * Monitors screen content for keywords/patterns and fires actions.
 * Also watches for app changes (ZeroClaw foreground app tracking).
 */
public class KiraWatcher {
    private static final String TAG = "KiraWatcher";

    private final Context ctx;
    private final KiraAI ai;
    private final KiraTools tools;
    private final Handler handler;
    private volatile boolean running = false;

    private String lastPkg = "";
    private String lastScreenHash = "";
    private int checkIntervalMs = 5000; // 5 seconds

    public KiraWatcher(Context ctx, KiraAI ai) {
        this.ctx     = ctx.getApplicationContext();
        this.ai      = ai;
        this.tools   = new KiraTools(ctx);
        this.handler = new Handler(Looper.getMainLooper());
    }

    public void start() {
        running = true;
        scheduleCheck();
        Log.i(TAG, "watcher started");
    }

    public void stop() {
        running = false;
        handler.removeCallbacksAndMessages(null);
    }

    private void scheduleCheck() {
        if (!running) return;
        handler.postDelayed(this::check, checkIntervalMs);
    }

    private void check() {
        if (!running) return;
        new Thread(() -> {
            try { doCheck(); } catch (Exception e) { Log.e(TAG, "check error", e); }
        }).start();
        scheduleCheck();
    }

    private void doCheck() throws Exception {
        // Push battery to Rust
        int pct = getBatteryPct();
        if (pct > 0) RustBridge.updateBattery(pct, isCharging());

        // Track foreground app changes (ZeroClaw)
        String pkg = ShizukuShell.exec("dumpsys activity recents | grep 'Recent #0' | grep -o 'A=[^ ]*' | cut -d= -f2 | cut -d/ -f1 2>/dev/null");
        pkg = pkg.trim();
        if (!pkg.isEmpty() && !pkg.equals(lastPkg)) {
            RustBridge.updateScreenPackage(pkg);
            lastPkg = pkg;
            Log.d(TAG, "foreground app: " + pkg);
        }

        // Check watch rules from memory
        KiraMemory mem = new KiraMemory(ctx);
        String allMem = mem.listAll();
        if (allMem.contains("watch_screen_")) {
            KiraAccessibilityService svc = KiraAccessibilityService.instance;
            if (svc != null) {
                String screenText = svc.getScreenText();
                String screenHash = String.valueOf(screenText.hashCode());
                if (!screenHash.equals(lastScreenHash)) {
                    lastScreenHash = screenHash;
                    // Check watch rules
                    for (String line : allMem.split("\n")) {
                        if (!line.startsWith("watch_screen_")) continue;
                        int colon = line.indexOf(":");
                        if (colon < 0) continue;
                        String rule = line.substring(colon + 1).trim();
                        // rule format: "keyword|action"
                        String[] parts = rule.split("\\|", 2);
                        if (parts.length == 2) {
                            String keyword = parts[0].trim();
                            String action  = parts[1].trim();
                            if (screenText.toLowerCase().contains(keyword.toLowerCase())) {
                                Log.d(TAG, "screen watch triggered: " + keyword);
                                mem.forget(line.substring(0, colon).trim());
                                final String act = action;
                                ai.chat(act, new KiraAI.Callback() {
                                    @Override public void onThinking() {}
                                    @Override public void onTool(String n, String r) {}
                                    @Override public void onReply(String reply) {}
                                    @Override public void onError(String e) {}
                                });
                            }
                        }
                    }
                }
            }
        }
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

    private boolean isCharging() {
        try {
            android.content.Intent i = ctx.registerReceiver(null,
                new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return false;
            int status = i.getIntExtra(android.os.BatteryManager.EXTRA_STATUS, -1);
            return status == android.os.BatteryManager.BATTERY_STATUS_CHARGING
                || status == android.os.BatteryManager.BATTERY_STATUS_FULL;
        } catch (Exception e) { return false; }
    }
}
