package com.kira.service;

import android.accessibilityservice.AccessibilityService;
import android.accessibilityservice.GestureDescription;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.graphics.Path;
import android.graphics.Rect;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.accessibility.AccessibilityEvent;
import android.view.accessibility.AccessibilityNodeInfo;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.telegram.KiraTelegram;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

public class KiraAccessibilityService extends AccessibilityService {

    public static KiraAccessibilityService instance;

    private Handler handler;
    private KiraAI ai;
    private KiraTelegram telegram;

    // Notification storage
    private final List<String> recentNotifications = new CopyOnWriteArrayList<>();

    @Override
    public void onServiceConnected() {
        instance = this;
        handler = new Handler(Looper.getMainLooper());

        // Start Rust HTTP server on port 7070
        RustBridge.startServer(7070);

        // Init AI
        ai = new KiraAI(this);

        // Start Telegram if configured
        KiraConfig cfg = KiraConfig.load(this);
        if (!cfg.tgToken.isEmpty()) {
            telegram = new KiraTelegram(this, ai);
            telegram.start();
        }

        // Start command polling (Rust → Java)
        pollCommands();
    }

    // ── Rust command polling ──────────────────────────────────────────────────

    private void pollCommands() {
        handler.postDelayed(() -> {
            try {
                String cmd = RustBridge.nextCommand();
                if (cmd != null) {
                    new Thread(() -> processRustCommand(cmd)).start();
                }
            } catch (Exception ignored) {}
            pollCommands();
        }, 30);
    }

    private void processRustCommand(String cmdJson) {
        try {
            org.json.JSONObject obj  = new org.json.JSONObject(cmdJson);
            String id                = obj.getString("id");
            org.json.JSONObject body = obj.getJSONObject("body");
            String endpoint          = body.getString("endpoint");
            org.json.JSONObject data = body.optJSONObject("data");
            if (data == null) data = new org.json.JSONObject();

            String result = dispatchRustCmd(endpoint, data);
            RustBridge.pushResult(id, result);
        } catch (Exception e) {
            try {
                org.json.JSONObject obj = new org.json.JSONObject(cmdJson);
                RustBridge.pushResult(obj.getString("id"), "{\"error\":\"" + e.getMessage() + "\"}");
            } catch (Exception ignored) {}
        }
    }

    private String dispatchRustCmd(String endpoint, org.json.JSONObject data) throws Exception {
        switch (endpoint) {
            case "screenshot":    return getScreenNodesJson();
            case "notifications": return getNotificationsJson();
            case "tap":           tap(data.getInt("x"), data.getInt("y")); return "{\"ok\":true}";
            case "swipe":         swipe(data.getInt("x1"), data.getInt("y1"), data.getInt("x2"), data.getInt("y2"), data.optInt("duration",300)); return "{\"ok\":true}";
            case "type":          typeText(data.getString("text")); return "{\"ok\":true}";
            case "find_and_tap":  return findAndTapJson(data.getString("text"));
            case "open":          return openAppJson(data.getString("package"));
            case "back":          performGlobalAction(GLOBAL_ACTION_BACK); return "{\"ok\":true}";
            case "home":          performGlobalAction(GLOBAL_ACTION_HOME); return "{\"ok\":true}";
            case "recents":       performGlobalAction(GLOBAL_ACTION_RECENTS); return "{\"ok\":true}";
            case "lock":          performGlobalAction(GLOBAL_ACTION_LOCK_SCREEN); return "{\"ok\":true}";
            case "clipboard_get": return "{\"text\":\"" + getClipboard().replace("\"","\\\"") + "\"}";
            case "clipboard_set": setClipboard(data.getString("text")); return "{\"ok\":true}";
            case "battery":       return getBatteryJson();
            default: return "{\"error\":\"unknown: " + endpoint + "\"}";
        }
    }

    // ── Public methods for KiraTools ──────────────────────────────────────────

    public String getScreenText() {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) return "";
            List<String> texts = new ArrayList<>();
            collectText(root, texts, 0);
            return String.join("\n", texts);
        } catch (Exception e) { return ""; }
    }

    private String getScreenNodesJson() {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) return "[]";
            org.json.JSONArray arr = new org.json.JSONArray();
            collectNodes(root, arr, 0);
            // Also push to Rust state
            RustBridge.updateScreenNodes(arr.toString());
            return arr.toString();
        } catch (Exception e) { return "[]"; }
    }

    private void collectText(AccessibilityNodeInfo node, List<String> texts, int depth) {
        if (node == null || depth > 30) return;
        CharSequence text = node.getText();
        CharSequence desc = node.getContentDescription();
        String t = text != null ? text.toString() : (desc != null ? desc.toString() : "");
        if (!t.isEmpty()) texts.add(t);
        for (int i = 0; i < node.getChildCount(); i++) collectText(node.getChild(i), texts, depth+1);
    }

    private void collectNodes(AccessibilityNodeInfo node, org.json.JSONArray arr, int depth) {
        if (node == null || depth > 30) return;
        try {
            CharSequence text = node.getText();
            CharSequence desc = node.getContentDescription();
            String t = text != null ? text.toString() : (desc != null ? desc.toString() : "");
            if (!t.isEmpty() || node.isClickable()) {
                org.json.JSONObject obj = new org.json.JSONObject();
                obj.put("text", t);
                obj.put("class", node.getClassName() != null ? node.getClassName().toString() : "");
                obj.put("clickable", node.isClickable());
                Rect r = new Rect();
                node.getBoundsInScreen(r);
                obj.put("bounds", r.left + "," + r.top + "," + r.right + "," + r.bottom);
                arr.put(obj);
            }
        } catch (Exception ignored) {}
        for (int i = 0; i < node.getChildCount(); i++) collectNodes(node.getChild(i), arr, depth+1);
    }

    public boolean tap(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 50));
        return dispatchGesture(b.build(), null, null);
    }

    public boolean longPress(int x, int y) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x, y);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, 800));
        return dispatchGesture(b.build(), null, null);
    }

    public boolean swipe(int x1, int y1, int x2, int y2, int duration) {
        GestureDescription.Builder b = new GestureDescription.Builder();
        Path p = new Path(); p.moveTo(x1, y1); p.lineTo(x2, y2);
        b.addStroke(new GestureDescription.StrokeDescription(p, 0, duration));
        return dispatchGesture(b.build(), null, null);
    }

    public boolean typeText(String text) {
        AccessibilityNodeInfo focus = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (focus == null) return false;
        Bundle args = new Bundle();
        args.putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text);
        return focus.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args);
    }

    public String tapText(String text) {
        try {
            AccessibilityNodeInfo root = getRootInActiveWindow();
            if (root == null) return "no window";
            org.json.JSONArray nodes = new org.json.JSONArray();
            collectNodes(root, nodes, 0);
            for (int i = 0; i < nodes.length(); i++) {
                org.json.JSONObject n = nodes.getJSONObject(i);
                String t = n.optString("text","");
                if (t.toLowerCase().contains(text.toLowerCase())) {
                    String[] parts = n.getString("bounds").split(",");
                    int cx = (Integer.parseInt(parts[0]) + Integer.parseInt(parts[2])) / 2;
                    int cy = (Integer.parseInt(parts[1]) + Integer.parseInt(parts[3])) / 2;
                    tap(cx, cy);
                    return "tapped \"" + t + "\" at (" + cx + "," + cy + ")";
                }
            }
            return "\"" + text + "\" not found on screen";
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String findAndTapJson(String text) {
        String result = tapText(text);
        return result.startsWith("tapped") ? "{\"ok\":true,\"result\":\"" + result + "\"}" : "{\"error\":\"" + result + "\"}";
    }

    private String openAppJson(String pkg) {
        android.content.Intent intent = getPackageManager().getLaunchIntentForPackage(pkg);
        if (intent == null) return "{\"error\":\"not found: " + pkg + "\"}";
        intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK | android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
        startActivity(intent);
        return "{\"ok\":true}";
    }

    public String getClipboard() {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm == null || cm.getPrimaryClip() == null) return "";
        ClipData.Item item = cm.getPrimaryClip().getItemAt(0);
        return item.getText() != null ? item.getText().toString() : "";
    }

    public void setClipboard(String text) {
        ClipboardManager cm = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
        if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("kira", text));
    }

    public String getFocusedText() {
        AccessibilityNodeInfo f = findFocus(AccessibilityNodeInfo.FOCUS_INPUT);
        if (f == null) return "nothing focused";
        CharSequence t = f.getText();
        return t != null ? t.toString() : "focused but empty";
    }

    public String getNotificationsText() {
        if (recentNotifications.isEmpty()) return "no recent notifications";
        return String.join("\n", recentNotifications.subList(
            Math.max(0, recentNotifications.size() - 20), recentNotifications.size()));
    }

    private String getNotificationsJson() {
        org.json.JSONArray arr = new org.json.JSONArray();
        for (String n : recentNotifications) {
            try { arr.put(new org.json.JSONObject().put("text", n)); } catch (Exception ignored) {}
        }
        return arr.toString();
    }

    private String getBatteryJson() {
        try {
            android.content.Intent i = registerReceiver(null, new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return "{\"error\":\"unavailable\"}";
            int level = i.getIntExtra(android.os.BatteryManager.EXTRA_LEVEL, -1);
            int scale = i.getIntExtra(android.os.BatteryManager.EXTRA_SCALE, -1);
            int status = i.getIntExtra(android.os.BatteryManager.EXTRA_STATUS, -1);
            float temp = i.getIntExtra(android.os.BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0f;
            int pct = scale > 0 ? level * 100 / scale : -1;
            String s = status == android.os.BatteryManager.BATTERY_STATUS_CHARGING ? "CHARGING" : "DISCHARGING";
            return "{\"percentage\":" + pct + ",\"status\":\"" + s + "\",\"temperature\":" + temp + "}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage() + "\"}"; }
    }

    // ── Events ────────────────────────────────────────────────────────────────

    @Override
    public void onAccessibilityEvent(AccessibilityEvent event) {
        int type = event.getEventType();

        if (type == AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED ||
            type == AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) {
            // Update screen nodes for Rust HTTP server
            handler.post(this::getScreenNodesJson);
        }

        if (type == AccessibilityEvent.TYPE_NOTIFICATION_STATE_CHANGED) {
            String pkg   = event.getPackageName() != null ? event.getPackageName().toString() : "unknown";
            String title = event.getText().size() > 0 ? event.getText().get(0).toString() : "";
            String text  = event.getText().size() > 1 ? event.getText().get(1).toString() : "";
            String entry = "[" + pkg + "] " + title + (text.isEmpty() ? "" : ": " + text);
            recentNotifications.add(entry);
            if (recentNotifications.size() > 100) recentNotifications.remove(0);
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
