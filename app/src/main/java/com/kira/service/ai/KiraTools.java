package com.kira.service.ai;

import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.BatteryManager;
import android.os.Build;
import android.telephony.SmsManager;
import android.util.Log;

import com.kira.service.KiraAccessibilityService;
import com.kira.service.ShizukuShell;

import org.json.JSONObject;
import org.jsoup.Jsoup;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

public class KiraTools {

    private static final String TAG = "KiraTools";
    private final Context ctx;
    private final KiraMemory memory;

    // Common app package names
    private static final Map<String, String> APP_MAP = new HashMap<String, String>() {{
        put("youtube", "com.google.android.youtube");
        put("yt", "com.google.android.youtube");
        put("whatsapp", "com.whatsapp");
        put("wa", "com.whatsapp");
        put("instagram", "com.instagram.android");
        put("ig", "com.instagram.android");
        put("facebook", "com.facebook.katana");
        put("fb", "com.facebook.katana");
        put("messenger", "com.facebook.orca");
        put("telegram", "org.telegram.messenger");
        put("tg", "org.telegram.messenger");
        put("chrome", "com.android.chrome");
        put("browser", "com.android.chrome");
        put("gmail", "com.google.android.gm");
        put("maps", "com.google.android.apps.maps");
        put("google maps", "com.google.android.apps.maps");
        put("camera", "com.android.camera2");
        put("settings", "com.android.settings");
        put("calculator", "com.google.android.calculator");
        put("calendar", "com.google.android.calendar");
        put("clock", "com.google.android.deskclock");
        put("files", "com.google.android.documentsui");
        put("photos", "com.google.android.apps.photos");
        put("spotify", "com.spotify.music");
        put("twitter", "com.twitter.android");
        put("x", "com.twitter.android");
        put("tiktok", "com.zhiliaoapp.musically");
        put("snapchat", "com.snapchat.android");
        put("netflix", "com.netflix.mediaclient");
        put("play store", "com.android.vending");
        put("playstore", "com.android.vending");
        put("dialer", "com.android.dialer");
        put("phone", "com.android.dialer");
        put("contacts", "com.android.contacts");
        put("messages", "com.google.android.apps.messaging");
        put("sms", "com.google.android.apps.messaging");
        put("music", "com.google.android.music");
        put("drive", "com.google.android.apps.docs");
        put("docs", "com.google.android.apps.docs.editors.docs");
        put("sheets", "com.google.android.apps.docs.editors.sheets");
        put("meet", "com.google.android.apps.meetings");
        put("zoom", "us.zoom.videomeetings");
        put("reddit", "com.reddit.frontpage");
        put("discord", "com.discord");
        put("linkedin", "com.linkedin.android");
        put("amazon", "com.amazon.mShop.android.shopping");
        put("nagram", "com.nextalone.nagram");
        put("vlc", "org.videolan.vlc");
        put("termux", "com.termux");
    }};

    public KiraTools(Context ctx) {
        this.ctx = ctx.getApplicationContext();
        this.memory = new KiraMemory(ctx);
    }

    public String execute(String name, JSONObject args) {
        try {
            switch (name) {
                // ── Memory ────────────────────────────────────────────────────
                case "remember":     { memory.remember(args.getString("key"), args.getString("value")); return "remembered: " + args.getString("key"); }
                case "recall":       return memory.recall(args.getString("key"));
                case "forget":       { memory.forget(args.getString("key")); return "forgot: " + args.getString("key"); }
                case "memory_list":  return memory.listAll();

                // ── Screen (via Accessibility) ────────────────────────────────
                case "read_screen":      return readScreen();
                case "tap_screen":       return tap(args.getInt("x"), args.getInt("y"));
                case "tap_text":         return tapText(args.getString("text"));
                case "long_press":       return longPress(args.getInt("x"), args.getInt("y"));
                case "swipe_screen":     return swipe(args.getInt("x1"), args.getInt("y1"), args.getInt("x2"), args.getInt("y2"), args.optInt("duration", 300));
                case "scroll_screen":    return scroll(args.optString("direction", "down"));
                case "find_and_tap":     return findAndTap(args.getString("text"));
                case "type_text":        return typeText(args.getString("text"));
                case "get_focused":      return getFocused();
                case "clipboard_get":    return getClipboard();
                case "clipboard_set":    return setClipboard(args.getString("text"));
                case "get_notifications":return getNotifications();
                case "press_back":       return globalAction(KiraAccessibilityService.GLOBAL_ACTION_BACK);
                case "press_home":       return globalAction(KiraAccessibilityService.GLOBAL_ACTION_HOME);
                case "press_recents":    return globalAction(KiraAccessibilityService.GLOBAL_ACTION_RECENTS);
                case "wake_screen":      return globalAction(KiraAccessibilityService.GLOBAL_ACTION_TAKE_SCREENSHOT);
                case "lock_screen":      return globalAction(KiraAccessibilityService.GLOBAL_ACTION_LOCK_SCREEN);

                // ── Apps ──────────────────────────────────────────────────────
                case "open_app":      return openApp(args.getString("package"));
                case "find_app":      return findApp(args.getString("query"));
                case "list_apps":     return listApps(args.optBoolean("system", false));
                case "force_stop":    return ShizukuShell.exec("am force-stop " + args.getString("package"));
                case "install_apk":   return ShizukuShell.exec("pm install -r \"" + args.getString("path") + "\"", 30000);
                case "uninstall":     return ShizukuShell.exec("pm uninstall " + args.getString("package"));

                // ── System controls ───────────────────────────────────────────
                case "set_volume":    return setVolume(args.getString("action"));
                case "set_brightness":return ShizukuShell.exec("settings put system screen_brightness " + (args.getInt("level") * 255 / 100));
                case "torch":         return setTorch(args.getBoolean("on"));
                case "battery_info":  return getBattery();
                case "device_info":   return getDeviceInfo();
                case "wifi_on":       return ShizukuShell.exec("svc wifi " + (args.getBoolean("on") ? "enable" : "disable"));
                case "mobile_data":   return ShizukuShell.exec("svc data " + (args.getBoolean("on") ? "enable" : "disable"));
                case "airplane_mode": return ShizukuShell.exec("settings put global airplane_mode_on " + (args.getBoolean("on") ? "1" : "0"));
                case "reboot":        return ShizukuShell.exec("reboot");
                case "sleep_screen":  return ShizukuShell.exec("input keyevent 223");

                // ── Shell (Shizuku) ───────────────────────────────────────────
                case "sh_run":        return ShizukuShell.exec(args.getString("cmd"), args.optInt("timeout", 15000));
                case "sh_tap":        return ShizukuShell.exec("input tap " + args.getInt("x") + " " + args.getInt("y"));
                case "sh_swipe":      return ShizukuShell.exec("input swipe " + args.getInt("x1") + " " + args.getInt("y1") + " " + args.getInt("x2") + " " + args.getInt("y2") + " " + args.optInt("duration", 300));
                case "sh_key":        return ShizukuShell.exec("input keyevent " + keyCode(args.getString("key")));
                case "sh_type":       return ShizukuShell.exec("input text \"" + args.getString("text").replace(" ", "%s") + "\"");
                case "sh_screenshot": return ShizukuShell.screenshot("/sdcard/kira_shot_" + System.currentTimeMillis() + ".png");
                case "sh_dump_ui":    return ShizukuShell.dumpUI();
                case "grant_perm":    return ShizukuShell.exec("pm grant " + args.getString("package") + " " + args.getString("permission"));
                case "revoke_perm":   return ShizukuShell.exec("pm revoke " + args.getString("package") + " " + args.getString("permission"));
                case "running_apps":  return ShizukuShell.exec("dumpsys activity recents | grep 'Recent #' | head -15");
                case "kill_process":  return ShizukuShell.exec("pkill -f " + args.getString("name"));
                case "get_prop":      return ShizukuShell.exec("getprop " + args.getString("key"));
                case "set_setting":   return ShizukuShell.exec("settings put " + args.getString("namespace") + " " + args.getString("key") + " " + args.getString("value"));
                case "get_setting":   return ShizukuShell.exec("settings get " + args.getString("namespace") + " " + args.getString("key"));
                case "list_files":    return ShizukuShell.exec("ls -la " + args.optString("path", "/sdcard"));
                case "read_file":     return ShizukuShell.exec("cat \"" + args.getString("path") + "\"");
                case "write_file":    return writeFile(args.getString("path"), args.getString("content"));
                case "delete_file":   return ShizukuShell.exec("rm -rf \"" + args.getString("path") + "\"");
                case "wifi_scan":     return ShizukuShell.exec("dumpsys wifi | grep 'SSID'| head -20");
                case "memory_usage":  return ShizukuShell.exec("cat /proc/meminfo | head -5");

                // ── Communication ─────────────────────────────────────────────
                case "send_sms":      return sendSms(args.getString("number"), args.getString("message"));
                case "call_number":   return makeCall(args.getString("number"));
                case "open_url":      return openUrl(args.getString("url"));
                case "deep_link":     return deepLink(args.getString("uri"));

                // ── Web ───────────────────────────────────────────────────────
                case "web_search":    return webSearch(args.getString("query"));
                case "http_get":      return httpGet(args.getString("url"));

                // ── Sensors ───────────────────────────────────────────────────
                case "location":      return getLocation();


                // ── NanoBot / ZeroClaw extras ─────────────────────────────────────────
                case "read_sms":          return readSms(args.optInt("count", 10));
                case "read_contacts":     return readContacts(args.optInt("count", 20));
                case "read_call_log":     return readCallLog(args.optInt("count", 10));
                case "set_alarm":         return setAlarm(args.getInt("hour"), args.getInt("minute"), args.optString("label","Kira alarm"));
                case "set_timer":         return setTimer(args.getInt("seconds"));
                case "share_text":        return shareText(args.getString("text"), args.optString("app",""));
                case "open_notification_shade": return ShizukuShell.exec("cmd statusbar expand-notifications");
                case "close_notification_shade": return ShizukuShell.exec("cmd statusbar collapse");
                case "take_video":        return ShizukuShell.exec("screenrecord --time-limit " + args.optInt("seconds",5) + " /sdcard/kira_rec_" + System.currentTimeMillis() + ".mp4 &");
                case "get_wifi_info":     return getWifiInfo();
                case "scan_wifi":         return ShizukuShell.exec("cmd wifi list-scan-results 2>/dev/null || dumpsys wifi | grep 'SSID' | head -15");
                case "ping":              return ShizukuShell.exec("ping -c 3 " + args.getString("host"));
                case "curl":              return httpGet(args.getString("url"));
                case "scrape_web":        return scrapeWeb(args.getString("url"), args.optString("selector",""));
                case "read_clipboard":    return getClipboard();
                case "write_clipboard":   return setClipboard(args.getString("text"));
                case "press_key":         return ShizukuShell.exec("input keyevent " + keyCode(args.getString("key")));
                case "input_swipe":       return ShizukuShell.exec("input swipe "+args.getInt("x1")+" "+args.getInt("y1")+" "+args.getInt("x2")+" "+args.getInt("y2")+" "+args.optInt("ms",300));
                case "am_broadcast":      return ShizukuShell.exec("am broadcast -a " + args.getString("action") + " " + args.optString("extras",""));
                case "pm_list":           return ShizukuShell.exec("pm list packages" + (args.optBoolean("system",false) ? "" : " -3"));
                case "logcat":            return ShizukuShell.exec("logcat -d -t 50 " + args.optString("filter","*:E"));
                case "dumpsys":           return ShizukuShell.exec("dumpsys " + args.getString("service") + " | head -40");
                case "top_cpu":           return ShizukuShell.exec("top -b -n1 | head -20");
                case "disk_usage":        return ShizukuShell.exec("df -h /sdcard /data 2>/dev/null | head -5");
                case "find_files":        return ShizukuShell.exec("find " + args.optString("path","/sdcard") + " -name "" + args.getString("pattern") + "" 2>/dev/null | head -20");
                case "zip_files":         return ShizukuShell.exec("cd " + args.getString("dir") + " && zip -r /sdcard/kira_archive_" + System.currentTimeMillis() + ".zip " + args.getString("pattern"));
                case "unzip":             return ShizukuShell.exec("unzip " + args.getString("file") + " -d " + args.optString("dest","/sdcard/"));
                case "download":          return ShizukuShell.exec("curl -L -o /sdcard/" + args.optString("name","kira_download") + " "" + args.getString("url") + """);
                case "schedule_task":     { KiraProactive.scheduleReminder(ctx, args.getString("task"), args.optLong("minutes",1)); return "scheduled: " + args.getString("task"); }
                case "watch_battery":     { KiraProactive.watchBattery(ctx, args.optInt("threshold",20)); return "watching battery"; }
                case "vibrate":           { android.os.Vibrator v = (android.os.Vibrator)ctx.getSystemService(Context.VIBRATOR_SERVICE); if(v!=null)v.vibrate(args.optLong("ms",300)); return "vibrated"; }
                case "play_tone":         return playTone(args.optInt("freq",440), args.optInt("ms",500));
                case "set_wallpaper":     return setWallpaper(args.getString("path"));
                case "take_photo":        return openCameraApp();
                case "calendar_add":      return addCalendarEvent(args.getString("title"), args.optLong("start", System.currentTimeMillis()+3600000), args.optLong("end", System.currentTimeMillis()+7200000));

                default: return "unknown tool: " + name + ". Available: read_screen, tap_screen, open_app, sh_run, web_search, send_sms, remember, recall, battery_info, sh_screenshot, and 40+ more.";
            }
        } catch (Exception e) {
            Log.e(TAG, "tool error: " + name, e);
            return "error in " + name + ": " + e.getMessage();
        }
    }

    // ── Accessibility wrappers ────────────────────────────────────────────────

    private String readScreen() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility service not running — enable it in Settings → Accessibility → Kira";
        String nodes = svc.getScreenText();
        return nodes.isEmpty() ? "screen is empty or unreadable" : nodes;
    }

    private String tap(int x, int y) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc != null) return svc.tap(x, y) ? "tapped (" + x + "," + y + ")" : "tap failed";
        return ShizukuShell.exec("input tap " + x + " " + y);
    }

    private String longPress(int x, int y) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc != null) return svc.longPress(x, y) ? "long pressed (" + x + "," + y + ")" : "failed";
        return ShizukuShell.exec("input swipe " + x + " " + y + " " + x + " " + y + " 800");
    }

    private String swipe(int x1, int y1, int x2, int y2, int dur) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc != null) return svc.swipe(x1, y1, x2, y2, dur) ? "swiped" : "failed";
        return ShizukuShell.exec("input swipe " + x1 + " " + y1 + " " + x2 + " " + y2 + " " + dur);
    }

    private String scroll(String direction) {
        android.util.DisplayMetrics dm = ctx.getResources().getDisplayMetrics();
        int cx = dm.widthPixels / 2, cy = dm.heightPixels / 2;
        int dy = direction.equals("up") ? 500 : -500;
        return swipe(cx, cy, cx, cy + dy, 300);
    }

    private String typeText(String text) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc != null) return svc.typeText(text) ? "typed text" : "no focused input found";
        return ShizukuShell.exec("input text \"" + text.replace(" ", "%s") + "\"");
    }

    private String tapText(String text) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return ShizukuShell.dumpUI().contains(text) ? ShizukuShell.exec("input tap 540 960") : "\"" + text + "\" not found";
        return svc.tapText(text);
    }

    private String findAndTap(String text) { return tapText(text); }

    private String globalAction(int action) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        svc.performGlobalAction(action);
        return "done";
    }

    private String getFocused() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        return svc.getFocusedText();
    }

    private String getClipboard() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        return svc.getClipboard();
    }

    private String setClipboard(String text) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        svc.setClipboard(text);
        return "clipboard set";
    }

    private String getNotifications() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        return svc.getNotificationsText();
    }

    // ── App management ────────────────────────────────────────────────────────

    private String openApp(String input) {
        // Resolve common name
        String pkg = APP_MAP.getOrDefault(input.toLowerCase().trim(), input.trim());

        // Check if it's actually installed
        if (!isInstalled(pkg)) {
            // Try to find by searching
            String found = findPackage(input);
            if (found != null) pkg = found;
            else return "\"" + input + "\" not found. Try: find_app {\"query\": \"" + input + "\"}";
        }

        return ShizukuShell.openApp(ctx, pkg);
    }

    private String findApp(String query) {
        String found = findPackage(query);
        if (found != null) {
            return "found: " + found + "\nuse: <tool:open_app>{\"package\": \"" + found + "\"}</tool>";
        }
        // Search via pm list
        String result = ShizukuShell.exec("pm list packages | grep -i " + query.toLowerCase().replaceAll("[^a-z0-9]", ""));
        return result.isEmpty() ? "no app found matching: " + query : result;
    }

    private String findPackage(String query) {
        try {
            PackageManager pm = ctx.getPackageManager();
            String q = query.toLowerCase().trim();
            List<ApplicationInfo> apps = pm.getInstalledApplications(0);
            for (ApplicationInfo app : apps) {
                String label = pm.getApplicationLabel(app).toString().toLowerCase();
                if (label.contains(q) || app.packageName.contains(q)) return app.packageName;
            }
        } catch (Exception ignored) {}
        return null;
    }

    private boolean isInstalled(String pkg) {
        try {
            ctx.getPackageManager().getApplicationInfo(pkg, 0);
            return true;
        } catch (Exception e) { return false; }
    }

    private String listApps(boolean includeSystem) {
        try {
            PackageManager pm = ctx.getPackageManager();
            List<ApplicationInfo> apps = pm.getInstalledApplications(0);
            StringBuilder sb = new StringBuilder();
            for (ApplicationInfo app : apps) {
                if (!includeSystem && (app.flags & ApplicationInfo.FLAG_SYSTEM) != 0) continue;
                sb.append(pm.getApplicationLabel(app)).append(": ").append(app.packageName).append("\n");
            }
            return sb.toString().trim();
        } catch (Exception e) {
            return "error listing apps: " + e.getMessage();
        }
    }

    // ── System ────────────────────────────────────────────────────────────────

    private String setVolume(String action) {
        android.media.AudioManager am = (android.media.AudioManager) ctx.getSystemService(Context.AUDIO_SERVICE);
        if (am == null) return "audio unavailable";
        int adj = action.equals("up") ? android.media.AudioManager.ADJUST_RAISE
                : action.equals("down") ? android.media.AudioManager.ADJUST_LOWER
                : android.media.AudioManager.ADJUST_MUTE;
        am.adjustStreamVolume(android.media.AudioManager.STREAM_MUSIC, adj, android.media.AudioManager.FLAG_SHOW_UI);
        return "volume " + action;
    }

    private String setTorch(boolean on) {
        try {
            android.hardware.camera2.CameraManager cm = (android.hardware.camera2.CameraManager) ctx.getSystemService(Context.CAMERA_SERVICE);
            String[] ids = cm.getCameraIdList();
            if (ids.length > 0) { cm.setTorchMode(ids[0], on); return "torch " + (on ? "on" : "off"); }
            return "no camera found";
        } catch (Exception e) { return "torch error: " + e.getMessage(); }
    }

    private String getBattery() {
        try {
            Intent i = ctx.registerReceiver(null, new android.content.IntentFilter(Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return "battery info unavailable";
            int level = i.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale = i.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            int status = i.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            float temp = i.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0f;
            int pct = scale > 0 ? level * 100 / scale : -1;
            String s = status == BatteryManager.BATTERY_STATUS_CHARGING ? "charging"
                     : status == BatteryManager.BATTERY_STATUS_FULL ? "full" : "discharging";
            return pct + "% — " + s + " — " + temp + "°C";
        } catch (Exception e) { return "battery error: " + e.getMessage(); }
    }

    private String getDeviceInfo() {
        return "Model: " + Build.MODEL + "\n"
            + "Brand: " + Build.BRAND + "\n"
            + "Android: " + Build.VERSION.RELEASE + " (SDK " + Build.VERSION.SDK_INT + ")\n"
            + "CPU: " + Build.HARDWARE + "\n"
            + ShizukuShell.exec("getprop ro.product.cpu.abi");
    }

    // ── Communication ─────────────────────────────────────────────────────────

    private String sendSms(String number, String message) {
        try {
            SmsManager sms = SmsManager.getDefault();
            sms.sendTextMessage(number, null, message, null, null);
            return "SMS sent to " + number;
        } catch (Exception e) { return "SMS error: " + e.getMessage(); }
    }

    private String makeCall(String number) {
        try {
            Intent intent = new Intent(Intent.ACTION_CALL, Uri.parse("tel:" + number));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "calling " + number;
        } catch (Exception e) { return "call error: " + e.getMessage(); }
    }

    private String openUrl(String url) {
        try {
            if (!url.startsWith("http")) url = "https://" + url;
            Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "opened " + url;
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String deepLink(String uri) {
        try {
            Intent intent = new Intent(Intent.ACTION_VIEW, Uri.parse(uri));
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "opened deep link: " + uri;
        } catch (Exception e) { return "deep link error: " + e.getMessage(); }
    }

    // ── Web ───────────────────────────────────────────────────────────────────

    private String webSearch(String query) {
        try {
            String encoded = URLEncoder.encode(query, "UTF-8");
            // DuckDuckGo instant answers API
            URL url = new URL("https://api.duckduckgo.com/?q=" + encoded + "&format=json&no_html=1&skip_disambig=1");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            conn.setRequestProperty("User-Agent", "KiraAgent/3.0");
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            org.json.JSONObject resp = new org.json.JSONObject(sb.toString());
            String abstract_ = resp.optString("Abstract", "");
            String answer = resp.optString("Answer", "");
            String result = !answer.isEmpty() ? answer : (!abstract_.isEmpty() ? abstract_ : "no direct answer found");
            // Also return related topics
            org.json.JSONArray related = resp.optJSONArray("RelatedTopics");
            if (related != null && related.length() > 0 && result.length() < 50) {
                StringBuilder extras = new StringBuilder(result + "\n\nRelated:\n");
                for (int i = 0; i < Math.min(3, related.length()); i++) {
                    try {
                        extras.append("• ").append(related.getJSONObject(i).optString("Text", "")).append("\n");
                    } catch (Exception ignored) {}
                }
                return extras.toString().trim();
            }
            return result;
        } catch (Exception e) {
            // Fallback: open in browser
            openUrl("https://duckduckgo.com/?q=" + query.replace(" ", "+"));
            return "opened search for: " + query;
        }
    }

    private String httpGet(String urlStr) {
        try {
            URL url = new URL(urlStr);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line).append("\n");
            String result = sb.toString().trim();
            return result.length() > 500 ? result.substring(0, 500) + "... (truncated)" : result;
        } catch (Exception e) { return "http error: " + e.getMessage(); }
    }

    // ── File ─────────────────────────────────────────────────────────────────

    private String writeFile(String path, String content) {
        try {
            java.io.FileWriter fw = new java.io.FileWriter(path);
            fw.write(content);
            fw.close();
            return "written to " + path;
        } catch (Exception e) {
            // Try via shell
            return ShizukuShell.exec("echo '" + content.replace("'", "'\"'\"'") + "' > " + path);
        }
    }

    // ── Sensors ───────────────────────────────────────────────────────────────

    private String getLocation() {
        // Returns last known location from settings
        String result = ShizukuShell.exec("dumpsys location | grep 'last location' | head -3");
        return result.isEmpty() ? "location unavailable (need GPS permission)" : result;
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private String keyCode(String key) {
        switch (key.toLowerCase()) {
            case "back": return "4";
            case "home": return "3";
            case "recents": case "recent": return "187";
            case "power": return "26";
            case "enter": return "66";
            case "delete": case "del": return "67";
            case "volume_up": case "vol+": return "24";
            case "volume_down": case "vol-": return "25";
            case "screenshot": return "120";
            case "menu": return "82";
            case "search": return "84";
            case "tab": return "61";
            case "up": return "19";
            case "down": return "20";
            case "left": return "21";
            case "right": return "22";
            default: return key;
        }
    }


    // ── NanoBot / ZeroClaw methods ────────────────────────────────────────────

    private String readSms(int count) {
        try {
            android.database.Cursor c = ctx.getContentResolver().query(
                android.net.Uri.parse("content://sms/inbox"), null, null, null, "date DESC LIMIT " + count);
            if (c == null) return "cannot read SMS (permission denied?)";
            StringBuilder sb = new StringBuilder();
            while (c.moveToNext()) {
                String addr = c.getString(c.getColumnIndexOrThrow("address"));
                String body = c.getString(c.getColumnIndexOrThrow("body"));
                sb.append("From: ").append(addr).append("\n").append(body.substring(0, Math.min(100, body.length()))).append("\n---\n");
            }
            c.close();
            return sb.length() == 0 ? "no SMS" : sb.toString().trim();
        } catch (Exception e) { return "SMS error: " + e.getMessage(); }
    }

    private String readContacts(int count) {
        try {
            android.database.Cursor c = ctx.getContentResolver().query(
                android.provider.ContactsContract.CommonDataKinds.Phone.CONTENT_URI,
                new String[]{android.provider.ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME,
                             android.provider.ContactsContract.CommonDataKinds.Phone.NUMBER},
                null, null, android.provider.ContactsContract.CommonDataKinds.Phone.DISPLAY_NAME + " ASC");
            if (c == null) return "contacts unavailable";
            StringBuilder sb = new StringBuilder();
            int n = 0;
            while (c.moveToNext() && n++ < count) {
                sb.append(c.getString(0)).append(": ").append(c.getString(1)).append("\n");
            }
            c.close();
            return sb.length() == 0 ? "no contacts" : sb.toString().trim();
        } catch (Exception e) { return "contacts error: " + e.getMessage(); }
    }

    private String readCallLog(int count) {
        try {
            android.database.Cursor c = ctx.getContentResolver().query(
                android.provider.CallLog.Calls.CONTENT_URI, null, null, null,
                android.provider.CallLog.Calls.DATE + " DESC LIMIT " + count);
            if (c == null) return "call log unavailable";
            StringBuilder sb = new StringBuilder();
            while (c.moveToNext()) {
                String name   = c.getString(c.getColumnIndexOrThrow(android.provider.CallLog.Calls.CACHED_NAME));
                String number = c.getString(c.getColumnIndexOrThrow(android.provider.CallLog.Calls.NUMBER));
                int type      = c.getInt(c.getColumnIndexOrThrow(android.provider.CallLog.Calls.TYPE));
                String t = type == android.provider.CallLog.Calls.INCOMING_TYPE ? "in" :
                           type == android.provider.CallLog.Calls.OUTGOING_TYPE ? "out" : "missed";
                sb.append("[").append(t).append("] ").append(name != null ? name : number).append("\n");
            }
            c.close();
            return sb.length() == 0 ? "no calls" : sb.toString().trim();
        } catch (Exception e) { return "call log error: " + e.getMessage(); }
    }

    private String setAlarm(int hour, int minute, String label) {
        try {
            android.content.Intent i = new android.content.Intent(android.provider.AlarmClock.ACTION_SET_ALARM);
            i.putExtra(android.provider.AlarmClock.EXTRA_HOUR, hour);
            i.putExtra(android.provider.AlarmClock.EXTRA_MINUTES, minute);
            i.putExtra(android.provider.AlarmClock.EXTRA_MESSAGE, label);
            i.putExtra(android.provider.AlarmClock.EXTRA_SKIP_UI, true);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "alarm set for " + hour + ":" + String.format("%02d", minute);
        } catch (Exception e) { return "alarm error: " + e.getMessage(); }
    }

    private String setTimer(int seconds) {
        try {
            android.content.Intent i = new android.content.Intent(android.provider.AlarmClock.ACTION_SET_TIMER);
            i.putExtra(android.provider.AlarmClock.EXTRA_LENGTH, seconds);
            i.putExtra(android.provider.AlarmClock.EXTRA_SKIP_UI, true);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "timer set for " + seconds + "s";
        } catch (Exception e) { return "timer error: " + e.getMessage(); }
    }

    private String shareText(String text, String app) {
        try {
            android.content.Intent i = new android.content.Intent(android.content.Intent.ACTION_SEND);
            i.setType("text/plain");
            i.putExtra(android.content.Intent.EXTRA_TEXT, text);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            if (!app.isEmpty()) {
                i.setPackage(APP_MAP.getOrDefault(app.toLowerCase(), app));
            }
            ctx.startActivity(i);
            return "shared text";
        } catch (Exception e) { return "share error: " + e.getMessage(); }
    }

    private String getWifiInfo() {
        android.net.wifi.WifiManager wm = (android.net.wifi.WifiManager) ctx.getSystemService(Context.WIFI_SERVICE);
        if (wm == null) return "wifi manager unavailable";
        android.net.wifi.WifiInfo info = wm.getConnectionInfo();
        if (info == null) return "not connected to WiFi";
        return "SSID: " + info.getSSID() + "\nBSSID: " + info.getBSSID()
            + "\nIP: " + android.text.format.Formatter.formatIpAddress(info.getIpAddress())
            + "\nSignal: " + info.getRssi() + " dBm"
            + "\nSpeed: " + info.getLinkSpeed() + " Mbps";
    }

    private String scrapeWeb(String url, String selector) {
        try {
            if (!url.startsWith("http")) url = "https://" + url;
            org.jsoup.nodes.Document doc = org.jsoup.Jsoup.connect(url)
                .userAgent("Mozilla/5.0 KiraAgent/6.0")
                .timeout(10000).get();
            if (!selector.isEmpty()) {
                return doc.select(selector).text().substring(0, Math.min(1000, doc.select(selector).text().length()));
            }
            String text = doc.body().text();
            return text.substring(0, Math.min(1500, text.length()));
        } catch (Exception e) { return "scrape error: " + e.getMessage(); }
    }

    private String getClipboard() {
        com.kira.service.KiraAccessibilityService svc = com.kira.service.KiraAccessibilityService.instance;
        if (svc != null) return svc.getClipboard();
        return ShizukuShell.exec("service call clipboard 2 2>/dev/null | grep -o 'String.*' | head -1");
    }

    private String setClipboard(String text) {
        com.kira.service.KiraAccessibilityService svc = com.kira.service.KiraAccessibilityService.instance;
        if (svc != null) { svc.setClipboard(text); return "clipboard set"; }
        return ShizukuShell.exec("am broadcast -a clipper.set --es text '" + text.replace("'","") + "' 2>/dev/null");
    }

    private String playTone(int freq, int durationMs) {
        try {
            android.media.AudioTrack track = new android.media.AudioTrack.Builder()
                .setAudioAttributes(new android.media.AudioAttributes.Builder()
                    .setUsage(android.media.AudioAttributes.USAGE_MEDIA).build())
                .setAudioFormat(new android.media.AudioFormat.Builder()
                    .setEncoding(android.media.AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(44100).setChannelMask(android.media.AudioFormat.CHANNEL_OUT_MONO).build())
                .setBufferSizeInBytes(44100 * durationMs / 1000 * 2)
                .setTransferMode(android.media.AudioTrack.MODE_STATIC).build();
            int numSamples = 44100 * durationMs / 1000;
            short[] samples = new short[numSamples];
            for (int i = 0; i < numSamples; i++) {
                samples[i] = (short)(32767 * Math.sin(2 * Math.PI * freq * i / 44100));
            }
            track.write(samples, 0, numSamples);
            track.play();
            return "playing " + freq + "Hz tone for " + durationMs + "ms";
        } catch (Exception e) { return "tone error: " + e.getMessage(); }
    }

    private String setWallpaper(String path) {
        return ShizukuShell.exec("am broadcast -a android.intent.action.WALLPAPER_CHANGED 2>/dev/null; "
            + "settings put system wallpaper_component null 2>/dev/null");
    }

    private String openCameraApp() {
        try {
            android.content.Intent i = new android.content.Intent(android.provider.MediaStore.ACTION_IMAGE_CAPTURE);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "opened camera";
        } catch (Exception e) { return ShizukuShell.openApp(ctx, "com.android.camera2"); }
    }

    private String addCalendarEvent(String title, long startMs, long endMs) {
        try {
            android.content.Intent i = new android.content.Intent(android.content.Intent.ACTION_INSERT);
            i.setData(android.provider.CalendarContract.Events.CONTENT_URI);
            i.putExtra(android.provider.CalendarContract.EXTRA_EVENT_BEGIN_TIME, startMs);
            i.putExtra(android.provider.CalendarContract.EXTRA_EVENT_END_TIME, endMs);
            i.putExtra(android.provider.CalendarContract.Events.TITLE, title);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "calendar event created: " + title;
        } catch (Exception e) { return "calendar error: " + e.getMessage(); }
    }

    public String getToolList() {
        return
            "SCREEN & UI:\n" +
            "  read_screen — read all visible text on screen\n" +
            "  tap_screen {x, y} — tap coordinates\n" +
            "  tap_text {text} — find and tap element by text\n" +
            "  swipe_screen {x1,y1,x2,y2,duration} — swipe gesture\n" +
            "  scroll_screen {direction: up/down} — scroll\n" +
            "  type_text {text} — type into focused field\n" +
            "  press_back / press_home / press_recents — navigation\n" +
            "  lock_screen / wake_screen — screen power\n" +
            "  clipboard_get / clipboard_set {text} — clipboard\n" +
            "  get_notifications — all notification content\n" +
            "\nAPPS:\n" +
            "  open_app {package} — open by name or package. Common: youtube, whatsapp, instagram, telegram, chrome, settings, camera\n" +
            "  find_app {query} — search for installed app\n" +
            "  list_apps {system: false} — list installed apps\n" +
            "  force_stop {package} — force stop app\n" +
            "  install_apk {path} — install APK silently\n" +
            "  uninstall {package} — uninstall app\n" +
            "\nSYSTEM:\n" +
            "  battery_info — battery level, status, temp\n" +
            "  device_info — model, android version, CPU\n" +
            "  set_volume {action: up/down/mute} — volume\n" +
            "  set_brightness {level: 0-100} — screen brightness\n" +
            "  torch {on: true/false} — flashlight\n" +
            "  wifi_on {on: true/false} — toggle WiFi\n" +
            "  mobile_data {on: true/false} — toggle data\n" +
            "  airplane_mode {on: true/false} — airplane mode\n" +
            "\nSHELL (Shizuku/ADB):\n" +
            "  sh_run {cmd, timeout?} — run ANY shell command\n" +
            "  sh_tap {x, y} — tap via ADB input\n" +
            "  sh_key {key} — keyevent: back/home/enter/volume_up/volume_down/screenshot\n" +
            "  sh_screenshot — save screenshot to /sdcard\n" +
            "  sh_dump_ui — dump full UI text content\n" +
            "  grant_perm {package, permission} — grant permission\n" +
            "  running_apps — list running apps\n" +
            "  list_files {path} — list files\n" +
            "  read_file {path} — read file content\n" +
            "  write_file {path, content} — write file\n" +
            "  get_setting/set_setting {namespace, key, value} — system settings\n" +
            "  memory_usage — RAM info\n" +
            "\nCOMMUNICATION:\n" +
            "  send_sms {number, message} — send SMS\n" +
            "  call_number {number} — make phone call\n" +
            "  open_url {url} — open URL in browser\n" +
            "  deep_link {uri} — open deep link/intent URI\n" +
            "\nWEB:\n" +
            "  web_search {query} — search DuckDuckGo\n" +
            "  http_get {url} — HTTP GET request\n" +
            "\nMEMORY:\n" +
            "  remember {key, value} — store fact\n" +
            "  recall {key} — retrieve fact\n" +
            "  forget {key} — delete fact\n" +
            "  memory_list — show all stored facts\n";
    }
}
