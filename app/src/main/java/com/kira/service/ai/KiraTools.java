package com.kira.service.ai;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.telephony.SmsManager;
import android.util.Log;

import com.kira.service.KiraAccessibilityService;

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.List;

/**
 * KiraTools — all tool implementations
 * Called by KiraAI when the LLM emits <tool:NAME>{args}</tool>
 */
public class KiraTools {

    private static final String TAG = "KiraTools";
    private final Context ctx;
    private final KiraMemory memory;

    public KiraTools(Context ctx) {
        this.ctx    = ctx;
        this.memory = new KiraMemory(ctx);
    }

    // ── Tool dispatcher ───────────────────────────────────────────────────────

    public String execute(String name, JSONObject args) {
        try {
            switch (name) {
                // Memory
                case "remember":        return memory.remember(args.getString("key"), args.getString("value")) + "" + "remembered: " + args.getString("key");
                case "recall":          return memory.recall(args.getString("key"));
                case "forget":          { memory.forget(args.getString("key")); return "forgot: " + args.getString("key"); }
                case "memory_list":     return memory.listAll();

                // Screen (via KiraService HTTP)
                case "read_screen":     return ks("GET", "/screenshot", null);
                case "tap_screen":      return ks("POST", "/tap", args.toString());
                case "long_press":      return ks("POST", "/long_press", args.toString());
                case "swipe_screen":    return ks("POST", "/swipe", args.toString());
                case "scroll_screen":   return ks("POST", "/scroll", args.toString());
                case "find_and_tap":    return ks("POST", "/find_and_tap", args.toString());
                case "tap_text":        return tapText(args.getString("text"));
                case "type_text":       return ks("POST", "/type", args.toString());
                case "get_focused":     return ks("GET", "/get_focused", null);
                case "clipboard_get":   return ks("GET", "/clipboard_get", null);
                case "clipboard_set":   return ks("POST", "/clipboard_set", args.toString());
                case "get_notifications": return ks("GET", "/notifications", null);

                // Navigation
                case "press_back":      return ks("GET", "/back", null);
                case "press_home":      return ks("GET", "/home", null);
                case "open_app":        return ks("POST", "/open", args.toString());
                case "installed_apps":  return ks("GET", "/installed_apps", null);

                // System
                case "set_volume":      return ks("POST", "/volume", args.toString());
                case "set_brightness":  return ks("POST", "/brightness", args.toString());
                case "torch":           return ks("POST", "/torch", args.toString());
                case "wake_screen":     return ks("GET", "/wake_screen", null);
                case "lock_screen":     return ks("GET", "/lock", null);
                case "battery_info":    return ks("GET", "/battery", null);
                case "read_sensors":    return ks("GET", "/sensors", null);

                // Shizuku / ADB
                case "sh_run":          return shRun(args.getString("cmd"));
                case "sh_tap":          return shRun("input tap " + args.getInt("x") + " " + args.getInt("y"));
                case "sh_swipe":        return shRun("input swipe " + args.getInt("x1") + " " + args.getInt("y1") + " " + args.getInt("x2") + " " + args.getInt("y2") + " " + args.optInt("duration", 300));
                case "sh_key":          return shRun("input keyevent " + keyCode(args.getString("key")));
                case "sh_type":         return shRun("input text \"" + args.getString("text").replace(" ", "%s") + "\"");
                case "sh_open_app":     return shRun("monkey -p " + args.getString("package") + " -c android.intent.category.LAUNCHER 1");
                case "sh_force_stop":   return shRun("am force-stop " + args.getString("package"));
                case "sh_install_apk":  return shRun("pm install -r \"" + args.getString("path") + "\"");
                case "sh_uninstall":    return shRun("pm uninstall " + args.getString("package"));
                case "sh_grant_perm":   return shRun("pm grant " + args.getString("package") + " " + args.getString("permission"));
                case "sh_revoke_perm":  return shRun("pm revoke " + args.getString("package") + " " + args.getString("permission"));
                case "sh_wifi":         return shRun("svc wifi " + (args.getBoolean("on") ? "enable" : "disable"));
                case "sh_mobile_data":  return shRun("svc data " + (args.getBoolean("on") ? "enable" : "disable"));
                case "sh_airplane":     return shRun("settings put global airplane_mode_on " + (args.getBoolean("on") ? "1" : "0"));
                case "sh_list_apps":    return shRun("pm list packages -3");
                case "sh_running_apps": return shRun("dumpsys activity recents | grep 'Recent #' | head -10");
                case "sh_device_info":  return shRun("getprop ro.product.model && getprop ro.build.version.release");
                case "sh_screenshot":   return shRun("screencap -p /sdcard/kira_shot.png && echo saved");

                // Communication
                case "send_sms":        return sendSms(args.getString("number"), args.getString("message"));
                case "call_number":     return makeCall(args.getString("number"));
                case "open_url":        return openUrl(args.getString("url"));

                // Web search
                case "web_search":      return webSearch(args.getString("query"));

                default: return "unknown tool: " + name;
            }
        } catch (Exception e) {
            Log.e(TAG, "tool error: " + name, e);
            return "error in " + name + ": " + e.getMessage();
        }
    }

    // ── KiraService HTTP calls ────────────────────────────────────────────────

    private String ks(String method, String endpoint, String body) {
        try {
            URL url = new URL("http://localhost:7070" + endpoint);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setRequestMethod(method);
            conn.setConnectTimeout(8000);
            conn.setReadTimeout(8000);
            if (body != null) {
                conn.setRequestProperty("Content-Type", "application/json");
                conn.setDoOutput(true);
                conn.getOutputStream().write(body.getBytes(StandardCharsets.UTF_8));
            }
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            return sb.toString();
        } catch (Exception e) {
            return "{\"error\":\"KiraService not running: " + e.getMessage() + "\"}";
        }
    }

    // ── Smart tap by text ─────────────────────────────────────────────────────

    private String tapText(String text) {
        try {
            String screen = ks("GET", "/screenshot", null);
            org.json.JSONArray nodes = new org.json.JSONArray(screen);
            for (int i = 0; i < nodes.length(); i++) {
                JSONObject n = nodes.getJSONObject(i);
                String t = n.optString("text", "");
                if (t.toLowerCase().contains(text.toLowerCase())) {
                    String[] parts = n.getString("bounds").split(",");
                    int cx = (Integer.parseInt(parts[0].replaceAll("[^0-9]", "")) +
                              Integer.parseInt(parts[2].replaceAll("[^0-9]", ""))) / 2;
                    int cy = (Integer.parseInt(parts[1].replaceAll("[^0-9]", "")) +
                              Integer.parseInt(parts[3].replaceAll("[^0-9]", ""))) / 2;
                    JSONObject tapArgs = new JSONObject();
                    tapArgs.put("x", cx);
                    tapArgs.put("y", cy);
                    ks("POST", "/tap", tapArgs.toString());
                    return "tapped \"" + t + "\" at (" + cx + "," + cy + ")";
                }
            }
            return "\"" + text + "\" not found on screen";
        } catch (Exception e) {
            return "tap_text error: " + e.getMessage();
        }
    }

    // ── Shizuku / shell ───────────────────────────────────────────────────────

    private String shRun(String cmd) {
        try {
            Process proc = Runtime.getRuntime().exec(new String[]{"sh", "-c", cmd});
            BufferedReader stdout = new BufferedReader(new InputStreamReader(proc.getInputStream()));
            BufferedReader stderr = new BufferedReader(new InputStreamReader(proc.getErrorStream()));
            StringBuilder out = new StringBuilder();
            String line;
            while ((line = stdout.readLine()) != null) out.append(line).append("\n");
            while ((line = stderr.readLine()) != null) out.append("[err] ").append(line).append("\n");
            proc.waitFor();
            String result = out.toString().trim();
            return result.isEmpty() ? "(no output)" : result;
        } catch (Exception e) {
            return "shell error: " + e.getMessage();
        }
    }

    private String keyCode(String key) {
        switch (key.toLowerCase()) {
            case "back":        return "4";
            case "home":        return "3";
            case "recents":     return "187";
            case "power":       return "26";
            case "enter":       return "66";
            case "delete":      return "67";
            case "volume_up":   return "24";
            case "volume_down": return "25";
            case "screenshot":  return "120";
            default:            return key;
        }
    }

    // ── Communication ─────────────────────────────────────────────────────────

    private String sendSms(String number, String message) {
        try {
            SmsManager sms = SmsManager.getDefault();
            sms.sendTextMessage(number, null, message, null, null);
            return "SMS sent to " + number;
        } catch (Exception e) {
            return "SMS error: " + e.getMessage();
        }
    }

    private String makeCall(String number) {
        try {
            Intent intent = new Intent(Intent.ACTION_CALL, Uri.parse("tel:" + number));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "calling " + number;
        } catch (Exception e) {
            return "call error: " + e.getMessage();
        }
    }

    private String openUrl(String url) {
        try {
            Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "opened " + url;
        } catch (Exception e) {
            return "open url error: " + e.getMessage();
        }
    }

    // ── Web search ────────────────────────────────────────────────────────────

    private String webSearch(String query) {
        try {
            String encoded = URLEncoder.encode(query, "UTF-8");
            URL url = new URL("https://ddg-webapp-aagd.vercel.app/search?q=" + encoded + "&format=json");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            conn.setRequestProperty("User-Agent", "KiraAgent/2.0");
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            // Parse top 3 results
            JSONObject resp = new JSONObject(sb.toString());
            org.json.JSONArray results = resp.optJSONArray("results");
            if (results == null || results.length() == 0) return "no results for: " + query;
            StringBuilder out = new StringBuilder();
            for (int i = 0; i < Math.min(3, results.length()); i++) {
                JSONObject res = results.getJSONObject(i);
                out.append(res.optString("title")).append("\n");
                out.append(res.optString("body", "")).append("\n");
                out.append(res.optString("href")).append("\n\n");
            }
            return out.toString().trim();
        } catch (Exception e) {
            return "search error: " + e.getMessage();
        }
    }

    // ── Tool list for system prompt ───────────────────────────────────────────

    public String getToolList() {
        return "PHONE CONTROL:\n"
            + "read_screen — read all text on screen\n"
            + "tap_screen {x,y} — tap coordinates\n"
            + "tap_text {text} — find and tap text on screen\n"
            + "swipe_screen {x1,y1,x2,y2} — swipe\n"
            + "scroll_screen {direction} — scroll up/down\n"
            + "find_and_tap {text} — find element by text and tap\n"
            + "type_text {text} — type into focused field\n"
            + "open_app {package} — open app by package name\n"
            + "press_back — back button\n"
            + "press_home — home button\n"
            + "get_notifications — all notifications\n"
            + "clipboard_get — read clipboard\n"
            + "clipboard_set {text} — set clipboard\n"
            + "wake_screen — wake phone\n"
            + "lock_screen — lock phone\n"
            + "set_volume {action: up/down/mute} — volume\n"
            + "set_brightness {level: 0-100} — brightness\n"
            + "torch {on: true/false} — flashlight\n"
            + "battery_info — battery status\n"
            + "installed_apps — list all apps\n\n"
            + "SHIZUKU (ADB-level, no root):\n"
            + "sh_run {cmd} — run any shell command\n"
            + "sh_tap {x,y} — tap via ADB\n"
            + "sh_key {key} — press key: back/home/enter/volume_up/volume_down/screenshot\n"
            + "sh_open_app {package} — open app via ADB\n"
            + "sh_force_stop {package} — force stop app\n"
            + "sh_install_apk {path} — install APK silently\n"
            + "sh_grant_perm {package, permission} — grant permission\n"
            + "sh_wifi {on} — toggle wifi\n"
            + "sh_mobile_data {on} — toggle data\n"
            + "sh_airplane {on} — airplane mode\n"
            + "sh_screenshot — take screenshot\n"
            + "sh_device_info — device info\n\n"
            + "COMMUNICATION:\n"
            + "send_sms {number, message} — send SMS\n"
            + "call_number {number} — make call\n"
            + "open_url {url} — open URL\n\n"
            + "WEB:\n"
            + "web_search {query} — search the web\n\n"
            + "MEMORY:\n"
            + "remember {key, value} — store fact\n"
            + "recall {key} — retrieve fact\n"
            + "forget {key} — delete fact\n"
            + "memory_list — show all stored facts\n";
    }
}
