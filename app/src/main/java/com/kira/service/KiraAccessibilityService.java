package com.kira.service;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Path;
import android.graphics.Rect;
import android.hardware.camera2.CameraManager;
import android.media.AudioManager;
import android.os.BatteryManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.telegram.KiraTelegram;

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.List;

public class KiraAccessibilityService extends AccessibilityService {

    public static KiraAccessibilityService instance;

    private Handler handler;
    private KiraAI ai;
    private KiraTelegram telegram;

    @Override
    public void onServiceConnected() {
        instance = this;
        handler  = new Handler(Looper.getMainLooper());

        // Start Rust HTTP server (localhost:7070)
        RustBridge.startServer(7070);

        // Init AI engine
        ai = new KiraAI(this);

        // Start Telegram bot
        KiraConfig cfg = KiraConfig.load(this);
        if (!cfg.tgToken.isEmpty()) {
            telegram = new KiraTelegram(this, ai);
            telegram.start();
        }

        // Start command polling loop
        pollCommands();
    }

    // ── Command polling (Rust → Java) ─────────────────────────────────────────

    private void pollCommands() {
        handler.postDelayed(() -> {
            try {
                String cmd = RustBridge.nextCommand();
                if (cmd != null) {
                    new Thread(() -> {
                        try {
                            JSONObject obj      = new JSONObject(cmd);
                            String id           = obj.getString("id");
                            JSONObject body     = obj.getJSONObject("body");
                            String endpoint     = body.getString("endpoint");
                            JSONObject data     = body.optJSONObject("data");
                            if (data == null) data = new JSONObject();
                            String result = dispatch(endpoint, data);
                            RustBridge.pushResult(id, result);
                        } catch (Exception e) {
                            try {
                                JSONObject obj = new JSONObject(cmd);
                                RustBridge.pushResult(obj.getString("id"),
                                    "{\"error\":\"" + e.getMessage() + "\"}");
                            } catch (Exception ignored) {}
                        }
                    }).start();
                }
            } catch (Exception ignored) {}
            pollCommands();
        }, 30); // 30ms poll
    }

    private String dispatch(String endpoint, JSONObject data) throws Exception {
        switch (endpoint) {
            case "tap":            return tap(data.getInt("x"), data.getInt("y"));
            case "long_press":     return longPress(data.getInt("x"), data.getInt("y"));
            case "swipe":          return swipe(data);
            case "scroll":         return scroll(data.optString("direction", "down"));
            case "type":           return typeText(data.getString("text"));
            case "find_and_tap":   return findAndTap(data.getString("text"));
            case "open":           return openApp(data.getString("package"));
            case "back":           return globalAction(GLOBAL_ACTION_BACK);
            case "home":           return globalAction(GLOBAL_ACTION_HOME);
            case "recents":        return globalAction(GLOBAL_ACTION_RECENTS);
            case "wake_screen":    return globalAction(GLOBAL_ACTION_TAKE_SCREENSHOT);
            case "lock":           return globalAction(GLOBAL_ACTION_LOCK_SCREEN);
            case "get_focused":    return getFocused();
            case "clipboard_get":  return getClipboard();
            case "clipboard_set":  return setClipboard(data.getString("text"));
            case "installed_apps": return getInstalledApps();
            case "volume":         return setVolume(data.getString("action"));
            case "brightness":     return setBrightness(data.getInt("level"));
            case "torch":          return setTorch(data.getBoolean("on"));
            case "battery":        return getBattery();
            case "sensors":        return "{\"info\":\"use termux-api for sensors\"}";
            default:               return "{\"error\":\"unknown: " + endpoint + "\"}";
        }
    }

    // ── Screen reading ────────────────────────────────────────────────────────

    private void updateScreenNodes() {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) { RustBridge.updateScreenNodes("[]"); return; }
            JSONArray arr = new JSONArray();
            collectNodes(root, arr, 0);
            RustBridge.updateScreenNodes(arr.toString());
        } catch (Exception e) { RustBridge.updateScreenNodes("[]"); }
    }

    private void collectNodes(AccessibilityNodeInfo node, JSONArray arr, int depth) {
        if (node == null || depth > 30) return;
        try {
            JSONObject obj = new JSONObject();
            CharSequence text = node.getText();
            CharSequence desc = node.getContentDescription();
            String t = text != null ? text.toString() : (desc != null ? desc.toString() : "");
            if (!t.isEmpty() || node.isClickable()) {
                obj.put("text", t);
                obj.put("class", node.getClassName() != null ? node.getClassName().toString() : "");
                obj.put("clickable", node.isClickable());
                Rect bounds = new Rect();
                node.getBoundsInScreen(bounds);
                obj.put("bounds", bounds.left + "," + bounds.top + "," + bounds.right + "," + bounds.bottom);
                arr.put(obj);
            }
        } catch (Exception ignored) {}
        for (int i = 0; i < node.getChildCount(); i++) {
            collectNodes(node.getChild(i), arr, depth + 1);
        }
    }

    // ── Gestures ──────────────────────────────────────────────────────────────

    private String tap(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 50));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String longPress(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 800));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String swipe(JSONObject d) throws Exception {
        int x1 = d.getInt("x1"), y1 = d.getInt("y1");
        int x2 = d.getInt("x2"), y2 = d.getInt("y2");
        int dur = d.optInt("duration", 300);
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x1, y1); p.lineTo(x2, y2);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, dur));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String scroll(String direction) {
        android.util.DisplayMetrics dm = getResources().getDisplayMetrics();
        int cx = dm.widthPixels / 2, cy = dm.heightPixels / 2;
        int dy = direction.equals("up") ? 600 : -600;
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(cx, cy); p.lineTo(cx, cy + dy);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 300));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String globalAction(int action) {
        performGlobalAction(action);
        return "{\"ok\":true}";
    }

    private String findAndTap(String text) throws Exception {
        AccessibilityNodeInfo root = getRootInActiveWindow();
        if (root == null) return "{\"error\":\"no window\"}";
        JSONArray nodes = new JSONArray();
        collectNodes(root, nodes, 0);
        for (int i = 0; i < nodes.length(); i++) {
            JSONObject n = nodes.getJSONObject(i);
            String t = n.optString("text", "");
            if (t.toLowerCase().contains(text.toLowerCase())) {
                String[] parts = n.getString("bounds").split(",");
                int cx = (Integer.parseInt(parts[0]) + Integer.parseInt(parts[2])) / 2;
                int cy = (Integer.parseInt(parts[1]) + Integer.parseInt(parts[3])) / 2;
                tap(cx, cy);
                return "{\"ok\":true,\"tapped\":\"" + t + "\"}";
            }
        }
        return "{\"error\":\"not found\"}";
    }

    // ── Input ─────────────────────────────────────────────────────────────────

    private String typeText(String text) {
        AccessibilityNodeInfo focus = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (focus == null) return "{\"error\":\"no focused input\"}";
        Bundle args = new Bundle();
        args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text);
        focus.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args);
        return "{\"ok\":true}";
    }

    private String getClipboard() {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm == null || cm.getPrimaryClip() == null) return "{\"text\":\"\"}";
        ClipData.Item item = cm.getPrimaryClip().getItemAt(0);
        return "{\"text\":\"" + (item.getText() != null ? item.getText().toString() : "") + "\"}";
    }

    private String setClipboard(String text) {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm == null) return "{\"error\":\"unavailable\"}";
        cm.setPrimaryClip(ClipData.newPlainText("kira", text));
        return "{\"ok\":true}";
    }

    private String getFocused() throws Exception {
        AccessibilityNodeInfo f = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (f == null) return "{\"error\":\"nothing focused\"}";
        JSONObject obj = new JSONObject();
        obj.put("text", f.getText() != null ? f.getText().toString() : "");
        obj.put("class", f.getClassName());
        return obj.toString();
    }

    // ── Apps ──────────────────────────────────────────────────────────────────

    private String openApp(String pkg) {
        Intent intent = getPackageManager().getLaunchIntentForPackage(pkg);
        if (intent == null) return "{\"error\":\"not found: " + pkg + "\"}";
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED | Intent.FLAG_ACTIVITY_CLEAR_TOP);
        startActivity(intent);
        return "{\"ok\":true}";
    }

    private String getInstalledApps() throws Exception {
        PackageManager pm = getPackageManager();
        List<ApplicationInfo> apps = pm.getInstalledApplications(0);
        JSONArray arr = new JSONArray();
        for (ApplicationInfo app : apps) {
            if ((app.flags & ApplicationInfo.FLAG_SYSTEM) == 0) {
                JSONObject obj = new JSONObject();
                obj.put("name", pm.getApplicationLabel(app).toString());
                obj.put("package", app.packageName);
                arr.put(obj);
            }
        }
        return arr.toString();
    }

    // ── System ────────────────────────────────────────────────────────────────

    private String setVolume(String action) {
        AudioManager am = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
        if (am == null) return "{\"error\":\"unavailable\"}";
        int adj = action.equals("up") ? AudioManager.ADJUST_RAISE
                : action.equals("down") ? AudioManager.ADJUST_LOWER
                : AudioManager.ADJUST_MUTE;
        am.adjustStreamVolume(AudioManager.STREAM_MUSIC, adj, AudioManager.FLAG_SHOW_UI);
        return "{\"ok\":true}";
    }

    private String setBrightness(int level) {
        try {
            Settings.System.putInt(getContentResolver(),
                Settings.System.SCREEN_BRIGHTNESS, level * 255 / 100);
            return "{\"ok\":true}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage() + "\"}"; }
    }

    private String setTorch(boolean on) {
        try {
            CameraManager cm = (CameraManager) getSystemService(Context.CAMERA_SERVICE);
            String[] ids = cm.getCameraIdList();
            if (ids.length > 0) cm.setTorchMode(ids[0], on);
            return "{\"ok\":true}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage() + "\"}"; }
    }

    private String getBattery() throws Exception {
        Intent i = registerReceiver(null, new android.content.IntentFilter(Intent.ACTION_BATTERY_CHANGED));
        if (i == null) return "{\"error\":\"unavailable\"}";
        int level = i.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
        int scale = i.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
        int status = i.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
        float temp = i.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0f;
        int pct = scale > 0 ? level * 100 / scale : -1;
        String s = status == BatteryManager.BATTERY_STATUS_CHARGING ? "CHARGING"
                 : status == BatteryManager.BATTERY_STATUS_FULL ? "FULL" : "DISCHARGING";
        JSONObject obj = new JSONObject();
        obj.put("percentage", pct); obj.put("status", s); obj.put("temperature", temp);
        return obj.toString();
    }

    // ── Events ────────────────────────────────────────────────────────────────

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        int type = event.getEventType();
        if (type == AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED ||
            type == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            updateScreenNodes();
        }
        if (type == AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED) {
            String pkg   = event.getPackageName() != null ? event.getPackageName().toString() : "";
            String title = event.getText().size() > 0 ? event.getText().get(0).toString() : "";
            String text  = event.getText().size() > 1 ? event.getText().get(1).toString() : "";
            RustBridge.pushNotification(pkg, title, text);
        }
    }

    @Override public void onInterrupt() {}

    @Override
    public void onDestroy() {
        instance = null;
        if (telegram != null) telegram.stop();
        super.onDestroy();
    }
}
