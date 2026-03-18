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

import org.json.JSONArray;
import org.json.JSONObject;

import java.util.List;
import java.util.ArrayList;

public class KiraAccessibilityService extends AccessibilityService {

    public static KiraAccessibilityService instance;
    private Handler handler;
    private static final int POLL_INTERVAL_MS = 50;

    @Override
    public void onServiceConnected() {
        instance = this;
        handler = new Handler(Looper.getMainLooper());

        // Start Rust HTTP server
        RustBridge.startServer(7070);

        // Start command polling loop
        pollCommands();
    }

    // ── Command polling loop ──────────────────────────────────────────────────
    // Java polls Rust for pending commands, executes them, pushes results back

    private void pollCommands() {
        handler.postDelayed(() -> {
            try {
                String cmdJson = RustBridge.nextCommand();
                if (cmdJson != null) {
                    processCommand(cmdJson);
                }
            } catch (Exception ignored) {}
            pollCommands(); // reschedule
        }, POLL_INTERVAL_MS);
    }

    private void processCommand(String cmdJson) {
        try {
            JSONObject cmd = new JSONObject(cmdJson);
            String id = cmd.getString("id");
            JSONObject body = cmd.getJSONObject("cmd").getJSONObject("body");
            String endpoint = cmd.getJSONObject("cmd").getString("endpoint");

            String result = dispatch(endpoint, body);
            RustBridge.pushResult(id, result);

        } catch (Exception e) {
            // Try to push error result
            try {
                JSONObject cmd = new JSONObject(cmdJson);
                String id = cmd.getString("id");
                RustBridge.pushResult(id, "{\"error\":\"" + e.getMessage() + "\"}");
            } catch (Exception ignored) {}
        }
    }

    private String dispatch(String endpoint, JSONObject body) throws Exception {
        switch (endpoint) {
            case "tap":           return tap(body.getInt("x"), body.getInt("y"));
            case "long_press":    return longPress(body.getInt("x"), body.getInt("y"));
            case "swipe":         return swipe(body);
            case "scroll":        return scroll(body.optString("direction", "down"));
            case "type":          return typeText(body.getString("text"));
            case "find_and_tap":  return findAndTap(body.getString("text"));
            case "open":          return openApp(body.getString("package"));
            case "back":          return globalAction(GLOBAL_ACTION_BACK);
            case "home":          return globalAction(GLOBAL_ACTION_HOME);
            case "recents":       return globalAction(GLOBAL_ACTION_RECENTS);
            case "wake_screen":   return wakeScreen();
            case "lock":          return lockScreen();
            case "get_focused":   return getFocused();
            case "clipboard_get": return getClipboard();
            case "clipboard_set": return setClipboard(body.getString("text"));
            case "installed_apps":return getInstalledApps();
            case "recent_apps":   return getRecentApps();
            case "volume":        return setVolume(body.getString("action"));
            case "brightness":    return setBrightness(body.getInt("level"));
            case "torch":         return setTorch(body.getBoolean("on"));
            case "battery":       return getBattery();
            case "sensors":       return getSensors();
            case "shizuku":       return runShizuku(body.getString("cmd"));
            default:              return "{\"error\":\"unknown endpoint: " + endpoint + "\"}";
        }
    }

    // ── Screen ────────────────────────────────────────────────────────────────

    private void updateScreenNodes() {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) {
                RustBridge.updateScreenNodes("[]");
                return;
            }
            JSONArray arr = new JSONArray();
            collectNodes(root, arr);
            RustBridge.updateScreenNodes(arr.toString());
        } catch (Exception e) {
            RustBridge.updateScreenNodes("[]");
        }
    }

    private void collectNodes(AccessibilityNodeInfo node, JSONArray arr) {
        if (node == null) return;
        try {
            JSONObject obj = new JSONObject();
            CharSequence text = node.getText();
            CharSequence desc = node.getContentDescription();
            obj.put("text", text != null ? text.toString() : (desc != null ? desc.toString() : ""));
            obj.put("class", node.getClassName() != null ? node.getClassName().toString() : "");
            obj.put("clickable", node.isClickable());
            obj.put("enabled", node.isEnabled());
            obj.put("focused", node.isFocused());
            Rect bounds = new Rect();
            node.getBoundsInScreen(bounds);
            obj.put("bounds", bounds.left + "," + bounds.top + "," + bounds.right + "," + bounds.bottom);
            arr.put(obj);
        } catch (Exception ignored) {}
        for (int i = 0; i < node.getChildCount(); i++) {
            collectNodes(node.getChild(i), arr);
        }
    }

    // ── Gestures ──────────────────────────────────────────────────────────────

    private String tap(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path();
        p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 50));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String longPress(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path();
        p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 800));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String swipe(JSONObject body) throws Exception {
        int x1 = body.getInt("x1"), y1 = body.getInt("y1");
        int x2 = body.getInt("x2"), y2 = body.getInt("y2");
        int dur = body.optInt("duration", 300);
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path();
        p.moveTo(x1, y1);
        p.lineTo(x2, y2);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, dur));
        dispatchGesture(b.build(), null, null);
        return "{\"ok\":true}";
    }

    private String scroll(String direction) {
        // Swipe to scroll
        int[] screen = getScreenSize();
        int cx = screen[0] / 2, cy = screen[1] / 2;
        int dy = direction.equals("up") ? 500 : -500;
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path();
        p.moveTo(cx, cy);
        p.lineTo(cx, cy + dy);
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
        collectNodes(root, nodes);
        for (int i = 0; i < nodes.length(); i++) {
            JSONObject n = nodes.getJSONObject(i);
            String t = n.optString("text", "");
            if (t.toLowerCase().contains(text.toLowerCase())) {
                String[] parts = n.getString("bounds").split(",");
                int cx = (Integer.parseInt(parts[0]) + Integer.parseInt(parts[2])) / 2;
                int cy = (Integer.parseInt(parts[1]) + Integer.parseInt(parts[3])) / 2;
                tap(cx, cy);
                return "{\"ok\":true,\"tapped\":\"" + t + "\",\"x\":" + cx + ",\"y\":" + cy + "}";
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
        if (cm == null) return "{\"error\":\"clipboard unavailable\"}";
        cm.setPrimaryClip(ClipData.newPlainText("kira", text));
        return "{\"ok\":true}";
    }

    // ── Apps ──────────────────────────────────────────────────────────────────

    private String openApp(String pkg) {
        Intent intent = getPackageManager().getLaunchIntentForPackage(pkg);
        if (intent == null) return "{\"error\":\"app not found: " + pkg + "\"}";
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        startActivity(intent);
        return "{\"ok\":true}";
    }

    private String getInstalledApps() throws Exception {
        PackageManager pm = getPackageManager();
        List<ApplicationInfo> apps = pm.getInstalledApplications(PackageManager.GET_META_DATA);
        JSONArray arr = new JSONArray();
        for (ApplicationInfo app : apps) {
            if ((app.flags & ApplicationInfo.FLAG_SYSTEM) == 0) { // user apps only
                JSONObject obj = new JSONObject();
                obj.put("name", pm.getApplicationLabel(app).toString());
                obj.put("package", app.packageName);
                arr.put(obj);
            }
        }
        return arr.toString();
    }

    private String getRecentApps() {
        return "{\"info\":\"use sh_running_apps via Shizuku for better results\"}";
    }

    // ── System controls ───────────────────────────────────────────────────────

    private String wakeScreen() {
        performGlobalAction(GLOBAL_ACTION_TAKE_SCREENSHOT); // fallback
        return "{\"ok\":true}";
    }

    private String lockScreen() {
        performGlobalAction(GLOBAL_ACTION_LOCK_SCREEN);
        return "{\"ok\":true}";
    }

    private String getFocused() throws Exception {
        AccessibilityNodeInfo focus = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (focus == null) return "{\"error\":\"nothing focused\"}";
        JSONObject obj = new JSONObject();
        obj.put("text", focus.getText() != null ? focus.getText().toString() : "");
        obj.put("class", focus.getClassName());
        return obj.toString();
    }

    private String setVolume(String action) {
        AudioManager am = (AudioManager) getSystemService(Context.AUDIO_SERVICE);
        if (am == null) return "{\"error\":\"audio unavailable\"}";
        switch (action) {
            case "up":   am.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_RAISE, AudioManager.FLAG_SHOW_UI); break;
            case "down": am.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_LOWER, AudioManager.FLAG_SHOW_UI); break;
            case "mute": am.adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_MUTE,  AudioManager.FLAG_SHOW_UI); break;
        }
        return "{\"ok\":true}";
    }

    private String setBrightness(int level) {
        try {
            int val = (int)(level / 100.0 * 255);
            Settings.System.putInt(getContentResolver(), Settings.System.SCREEN_BRIGHTNESS, val);
            return "{\"ok\":true}";
        } catch (Exception e) {
            return "{\"error\":\"" + e.getMessage() + "\"}";
        }
    }

    private String setTorch(boolean on) {
        try {
            CameraManager cm = (CameraManager) getSystemService(Context.CAMERA_SERVICE);
            String[] ids = cm.getCameraIdList();
            if (ids.length > 0) cm.setTorchMode(ids[0], on);
            return "{\"ok\":true}";
        } catch (Exception e) {
            return "{\"error\":\"" + e.getMessage() + "\"}";
        }
    }

    private String getBattery() throws Exception {
        Intent intent = registerReceiver(null, new android.content.IntentFilter(Intent.ACTION_BATTERY_CHANGED));
        if (intent == null) return "{\"error\":\"battery unavailable\"}";
        int level = intent.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
        int scale = intent.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
        int status = intent.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
        float temp = intent.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0f;
        int pct = scale > 0 ? (int)(level * 100.0f / scale) : -1;
        String statusStr = status == BatteryManager.BATTERY_STATUS_CHARGING ? "CHARGING"
                         : status == BatteryManager.BATTERY_STATUS_FULL ? "FULL" : "DISCHARGING";
        JSONObject obj = new JSONObject();
        obj.put("percentage", pct);
        obj.put("status", statusStr);
        obj.put("temperature", temp);
        return obj.toString();
    }

    private String getSensors() {
        return "{\"info\":\"sensor data available via termux-api\"}";
    }

    // ── Shizuku ───────────────────────────────────────────────────────────────

    private String runShizuku(String cmd) {
        try {
            Process proc = Runtime.getRuntime().exec(new String[]{"sh", "-c", cmd});
            byte[] out = proc.getInputStream().readAllBytes();
            proc.waitFor();
            String result = new String(out).trim();
            return "{\"result\":\"" + result.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\"}";
        } catch (Exception e) {
            return "{\"error\":\"" + e.getMessage() + "\"}";
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private int[] getScreenSize() {
        android.util.DisplayMetrics dm = getResources().getDisplayMetrics();
        return new int[]{dm.widthPixels, dm.heightPixels};
    }

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        // Update screen nodes on every window change
        if (event.getEventType() == AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED ||
            event.getEventType() == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            updateScreenNodes();
        }
        // Store notifications
        if (event.getEventType() == AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED) {
            String pkg = event.getPackageName() != null ? event.getPackageName().toString() : "";
            String title = event.getText().size() > 0 ? event.getText().get(0).toString() : "";
            String text = event.getText().size() > 1 ? event.getText().get(1).toString() : "";
            RustBridge.pushNotification(pkg, title, text);
        }
    }

    @Override
    public void onInterrupt() {}

    @Override
    public void onDestroy() {
        instance = null;
        super.onDestroy();
    }
}
