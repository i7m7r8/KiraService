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
        // Poll Rust for any macro actions that fired
        pollAndExecuteMacroActions();
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

    /** Poll Rust for fired macro actions and execute them */
    private void pollAndExecuteMacroActions() {
        try {
            String next = RustBridge.nextMacroAction(null);
            while (next != null && !next.isEmpty()) {
                Log.d(TAG, "macro action: " + next);
                org.json.JSONObject action = new org.json.JSONObject(next);
                String type    = action.optString("type", "");
                org.json.JSONObject params = action.optJSONObject("params");
                if (params == null) params = new org.json.JSONObject();

                switch (type) {
                    case "kira_chat": {
                        // Fired macro sends a message to Kira AI
                        final String msg = params.optString("message","");
                        if (!msg.isEmpty()) {
                            final org.json.JSONObject p = params;
                            handler.post(() -> {
                                ai.chat(msg, new com.kira.service.ai.KiraAI.Callback() {
                                    @Override public void onThinking() {}
                                    @Override public void onTool(String n, String r) {}
                                    @Override public void onReply(String reply) {
                                        // Post result to Kira event bus
                                        KiraEventBus.post("macro_result", reply);
                                    }
                                    @Override public void onError(String e) {
                                        Log.w(TAG, "macro chat error: " + e);
                                    }
                                });
                            });
                        }
                        break;
                    }
                    case "run_tool": {
                        // Fired macro runs a tool directly
                        String toolName = params.optString("tool", "");
                        if (!toolName.isEmpty()) {
                            final String finalTool = toolName;
                            final org.json.JSONObject finalParams = params;
                            new Thread(() -> {
                                try {
                                    String result = tools.runTool(finalTool, finalParams);
                                    RustBridge.logTaskStep("macro_tool", 1, finalTool, result, true);
                                } catch (Exception e) {
                                    Log.w(TAG, "macro tool error: " + e.getMessage());
                                }
                            }).start();
                        }
                        break;
                    }
                    case "send_notification": {
                        String title = params.optString("title","Kira");
                        String msg2  = params.optString("message","Automation triggered");
                        sendNotification(title, msg2);
                        break;
                    }
                    default:
                        Log.d(TAG, "unhandled macro action type: " + type);
                }
                next = RustBridge.nextMacroAction(null);
            }
        } catch (Exception e) {
            Log.w(TAG, "pollMacroActions error: " + e.getMessage());
        }
    }

    private void sendNotification(String title, String message) {
        try {
            android.app.NotificationManager nm = (android.app.NotificationManager)
                ctx.getSystemService(android.content.Context.NOTIFICATION_SERVICE);
            if (nm == null) return;
            String chId = "kira_automation";
            if (android.os.Build.VERSION.SDK_INT >= 26) {
                nm.createNotificationChannel(new android.app.NotificationChannel(
                    chId, "Kira Automations",
                    android.app.NotificationManager.IMPORTANCE_DEFAULT));
            }
            android.app.Notification notif = new android.app.Notification.Builder(ctx, chId)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle(title)
                .setContentText(message)
                .setAutoCancel(true)
                .build();
            nm.notify((int)(System.currentTimeMillis() % 10000), notif);
        } catch (Exception e) {
            Log.w(TAG, "sendNotification error: " + e.getMessage());
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
