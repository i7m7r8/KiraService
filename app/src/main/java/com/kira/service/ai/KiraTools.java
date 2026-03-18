package com.kira.service.ai;

import android.content.Context;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.BatteryManager;
import android.os.Build;
import android.telephony.SmsManager;
import android.util.Log;

import com.kira.service.KiraAccessibilityService;
import com.kira.service.ShizukuShell;
import com.kira.service.RustBridge;

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

    // Common app package name mappings
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
        put("meet", "com.google.android.apps.meetings");
        put("zoom", "us.zoom.videomeetings");
        put("reddit", "com.reddit.frontpage");
        put("discord", "com.discord");
        put("linkedin", "com.linkedin.android");
        put("amazon", "com.amazon.mShop.android.shopping");
        put("vlc", "org.videolan.vlc");
        put("termux", "com.termux");
        put("nagram", "com.nextalone.nagram");
    }};

    public KiraTools(Context ctx) {
        this.ctx = ctx.getApplicationContext();
        this.memory = new KiraMemory(ctx);
    }

    public String execute(String name, JSONObject args) {
        try {
            switch (name) {
                // Memory
                case "remember":     memory.remember(args.getString("key"), args.getString("value")); return "remembered: " + args.getString("key");
                case "recall":       return memory.recall(args.getString("key"));
                case "forget":       memory.forget(args.getString("key")); return "forgot: " + args.getString("key");
                case "memory_list":  return memory.listAll();

                // Screen control via Accessibility
                case "read_screen":      return readScreen();
                case "tap_screen":       return tap(args.getInt("x"), args.getInt("y"));
                case "tap_text":         return tapText(args.getString("text"));
                case "long_press":       return longPress(args.getInt("x"), args.getInt("y"));
                case "swipe_screen":     return swipe(args.getInt("x1"), args.getInt("y1"), args.getInt("x2"), args.getInt("y2"), args.optInt("duration", 300));
                case "scroll_screen":    return scroll(args.optString("direction", "down"));
                case "type_text":        return typeText(args.getString("text"));
                case "get_focused":      return getFocused();
                case "clipboard_get":    return getClipboard();
                case "clipboard_set":    return setClipboard(args.getString("text"));
                case "get_notifications":return getNotifications();
                case "press_back":       return globalAction(KiraAccessibilityService.GLOBAL_ACTION_BACK);
                case "press_home":       return globalAction(KiraAccessibilityService.GLOBAL_ACTION_HOME);
                case "press_recents":    return globalAction(KiraAccessibilityService.GLOBAL_ACTION_RECENTS);
                case "lock_screen":      return globalAction(KiraAccessibilityService.GLOBAL_ACTION_LOCK_SCREEN);
                case "find_and_tap":     return tapText(args.getString("text"));

                // App management
                case "open_app":      return openApp(args.getString("package"));
                case "find_app":      return findApp(args.getString("query"));
                case "list_apps":     return listApps(args.optBoolean("system", false));
                case "force_stop":    return ShizukuShell.exec("am force-stop " + args.getString("package"));
                case "install_apk":   return ShizukuShell.exec("pm install -r \"" + args.getString("path") + "\"", 30000);
                case "uninstall":     return ShizukuShell.exec("pm uninstall " + args.getString("package"));

                // System
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
                case "get_wifi_info": return getWifiInfo();
                case "scan_wifi":     return ShizukuShell.exec("cmd wifi list-scan-results 2>/dev/null || dumpsys wifi | grep SSID | head -15");

                // Shell (Shizuku/ADB)
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
                case "get_prop":      return ShizukuShell.exec("getprop " + args.getString("key"));
                case "set_setting":   return ShizukuShell.exec("settings put " + args.getString("namespace") + " " + args.getString("key") + " " + args.getString("value"));
                case "get_setting":   return ShizukuShell.exec("settings get " + args.getString("namespace") + " " + args.getString("key"));
                case "list_files":    return ShizukuShell.exec("ls -la " + args.optString("path", "/sdcard"));
                case "read_file":     return ShizukuShell.exec("cat \"" + args.getString("path") + "\"");
                case "write_file":    return writeFile(args.getString("path"), args.getString("content"));
                case "delete_file":   return ShizukuShell.exec("rm -rf \"" + args.getString("path") + "\"");
                case "memory_usage":  return ShizukuShell.exec("cat /proc/meminfo | head -5");
                case "top_cpu":       return ShizukuShell.exec("top -b -n1 | head -20");
                case "disk_usage":    return ShizukuShell.exec("df -h /sdcard /data 2>/dev/null | head -5");
                case "logcat":        return ShizukuShell.exec("logcat -d -t 50 " + args.optString("filter", "*:E"));
                case "dumpsys":       return ShizukuShell.exec("dumpsys " + args.getString("service") + " | head -40");
                case "am_broadcast":  return ShizukuShell.exec("am broadcast -a " + args.getString("action") + " " + args.optString("extras", ""));
                case "pm_list":       return ShizukuShell.exec("pm list packages" + (args.optBoolean("system", false) ? "" : " -3"));
                case "find_files":    return ShizukuShell.exec("find " + args.optString("path", "/sdcard") + " -name " + args.getString("pattern") + " 2>/dev/null | head -20");
                case "zip_files":     return ShizukuShell.exec("cd " + args.getString("dir") + " && zip -r /sdcard/kira_archive_" + System.currentTimeMillis() + ".zip " + args.getString("pattern"));
                case "unzip":         return ShizukuShell.exec("unzip " + args.getString("file") + " -d " + args.optString("dest", "/sdcard/"));
                case "download":      return ShizukuShell.exec("curl -L -o /sdcard/" + args.optString("name", "kira_dl") + " " + args.getString("url"));
                case "ping":          return ShizukuShell.exec("ping -c 3 " + args.getString("host"));
                case "input_swipe":   return ShizukuShell.exec("input swipe " + args.getInt("x1") + " " + args.getInt("y1") + " " + args.getInt("x2") + " " + args.getInt("y2") + " " + args.optInt("ms", 300));
                case "press_key":     return ShizukuShell.exec("input keyevent " + keyCode(args.getString("key")));
                case "open_notification_shade":  return ShizukuShell.exec("cmd statusbar expand-notifications");
                case "close_notification_shade": return ShizukuShell.exec("cmd statusbar collapse");
                case "take_video":    return ShizukuShell.exec("screenrecord --time-limit " + args.optInt("seconds", 5) + " /sdcard/kira_rec_" + System.currentTimeMillis() + ".mp4");

                // Communication
                case "send_sms":      return sendSms(args.getString("number"), args.getString("message"));
                case "call_number":   return makeCall(args.getString("number"));
                case "open_url":      return openUrl(args.getString("url"));
                case "deep_link":     return deepLink(args.getString("uri"));
                case "share_text":    return shareText(args.getString("text"), args.optString("app", ""));

                // Read data
                case "read_sms":      return readSms(args.optInt("count", 10));
                case "read_contacts": return readContacts(args.optInt("count", 20));
                case "read_call_log": return readCallLog(args.optInt("count", 10));

                // System actions
                case "set_alarm":     return setAlarm(args.getInt("hour"), args.getInt("minute"), args.optString("label", "Kira alarm"));
                case "set_timer":     return setTimer(args.getInt("seconds"));
                case "vibrate":       { android.os.Vibrator v = (android.os.Vibrator) ctx.getSystemService(Context.VIBRATOR_SERVICE); if (v != null) v.vibrate(args.optLong("ms", 300)); return "vibrated"; }
                case "play_tone":     return playTone(args.optInt("freq", 440), args.optInt("ms", 500));
                case "take_photo":    return openCameraApp();
                case "calendar_add":  return addCalendarEvent(args.getString("title"), args.optLong("start", System.currentTimeMillis() + 3600000), args.optLong("end", System.currentTimeMillis() + 7200000));

                // Web
                case "web_search":    return webSearch(args.getString("query"));
                case "http_get":      return httpGet(args.getString("url"));
                case "curl":          return httpGet(args.getString("url"));
                case "scrape_web":    return scrapeWeb(args.getString("url"), args.optString("selector", ""));



                case "schedule_task": { RustBridge.addTrigger("sched_" + System.currentTimeMillis(), "time", String.valueOf(System.currentTimeMillis() + args.optLong("minutes",1)*60000), args.getString("task"), false); return "scheduled in " + args.optLong("minutes",1) + "m: " + args.getString("task"); }
                case "watch_screen":  { String keyword = args.getString("keyword"); String action = args.getString("action"); memory.remember("watch_screen_" + System.currentTimeMillis(), keyword + "|" + action); return "watching screen for: " + keyword; }
                case "watch_app":     { RustBridge.addTrigger("app_" + args.getString("package"), "app_notif", args.getString("package"), args.getString("action"), args.optBoolean("repeat", false)); return "watching app: " + args.getString("package"); }
                case "watch_battery": { RustBridge.addTrigger("bat_" + args.optInt("threshold",20), "battery_low", String.valueOf(args.optInt("threshold",20)), args.getString("action"), args.optBoolean("repeat", true)); return "watching battery < " + args.optInt("threshold",20) + "%"; }
                case "watch_notif":   { RustBridge.addTrigger("notif_" + System.currentTimeMillis(), "keyword_notif", args.getString("keyword"), args.getString("action"), args.optBoolean("repeat", true)); return "watching notifications for: " + args.getString("keyword"); }
                case "list_watches":  { return ShizukuShell.exec("echo watches active"); }
                case "stop_watch":    { RustBridge.removeTrigger(args.getString("id")); return "stopped watch: " + args.getString("id"); }

                // Vision (ZeroClaw-style)
                case "analyze_screen": return analyzeScreen(args.optString("question", ""));
                case "find_element":   return findAndTapVisual(args.getString("description"));
                case "read_image":     return analyzeScreen("Extract and return all text from this image.");
                case "describe_screen":return analyzeScreen("Describe every UI element visible.");

                // Proactive

                // Location
                case "location":      return getLocation();

                default: return "unknown tool: " + name;
            }
        } catch (Exception e) {
            Log.e(TAG, "tool error: " + name, e);
            return "error in " + name + ": " + e.getMessage();
        }
    }

    // Accessibility wrappers
    private String readScreen() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility service not running";
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
        if (svc != null) return svc.longPress(x, y) ? "long pressed" : "failed";
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
        if (svc == null) return ShizukuShell.exec("input tap 540 960");
        return svc.tapText(text);
    }

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
        if (svc != null) return svc.getClipboard();
        return ShizukuShell.exec("service call clipboard 2 2>/dev/null | head -1");
    }

    private String setClipboard(String text) {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc != null) { svc.setClipboard(text); return "clipboard set"; }
        return "accessibility needed for clipboard";
    }

    private String getNotifications() {
        KiraAccessibilityService svc = KiraAccessibilityService.instance;
        if (svc == null) return "accessibility not running";
        return svc.getNotificationsText();
    }

    // App management
    private String openApp(String input) {
        String pkg = APP_MAP.getOrDefault(input.toLowerCase().trim(), input.trim());
        if (!isInstalled(pkg)) {
            String found = findPackage(input);
            if (found != null) pkg = found;
            else return "app not found: " + input;
        }
        return ShizukuShell.openApp(ctx, pkg);
    }

    private String findApp(String query) {
        String found = findPackage(query);
        if (found != null) return "found: " + found;
        return ShizukuShell.exec("pm list packages | grep -i " + query.toLowerCase().replaceAll("[^a-z0-9]", ""));
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
        try { ctx.getPackageManager().getApplicationInfo(pkg, 0); return true; }
        catch (Exception e) { return false; }
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
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    // System
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
            return "no camera";
        } catch (Exception e) { return "torch error: " + e.getMessage(); }
    }

    private String getBattery() {
        try {
            android.content.Intent i = ctx.registerReceiver(null, new android.content.IntentFilter(android.content.Intent.ACTION_BATTERY_CHANGED));
            if (i == null) return "battery unavailable";
            int level  = i.getIntExtra(BatteryManager.EXTRA_LEVEL, -1);
            int scale  = i.getIntExtra(BatteryManager.EXTRA_SCALE, -1);
            int status = i.getIntExtra(BatteryManager.EXTRA_STATUS, -1);
            float temp = i.getIntExtra(BatteryManager.EXTRA_TEMPERATURE, 0) / 10.0f;
            int pct = scale > 0 ? level * 100 / scale : -1;
            String s = status == BatteryManager.BATTERY_STATUS_CHARGING ? "charging"
                     : status == BatteryManager.BATTERY_STATUS_FULL ? "full" : "discharging";
            return pct + "% - " + s + " - " + temp + "C";
        } catch (Exception e) { return "battery error: " + e.getMessage(); }
    }

    private String getDeviceInfo() {
        return "Model: " + Build.MODEL + "\nBrand: " + Build.BRAND
            + "\nAndroid: " + Build.VERSION.RELEASE + " (SDK " + Build.VERSION.SDK_INT + ")\n"
            + ShizukuShell.exec("getprop ro.product.cpu.abi");
    }

    private String getWifiInfo() {
        android.net.wifi.WifiManager wm = (android.net.wifi.WifiManager) ctx.getSystemService(Context.WIFI_SERVICE);
        if (wm == null) return "wifi unavailable";
        android.net.wifi.WifiInfo info = wm.getConnectionInfo();
        if (info == null) return "not connected";
        return "SSID: " + info.getSSID() + "\nBSSID: " + info.getBSSID()
            + "\nIP: " + android.text.format.Formatter.formatIpAddress(info.getIpAddress())
            + "\nSignal: " + info.getRssi() + " dBm";
    }

    // Communication
    private String sendSms(String number, String message) {
        try {
            SmsManager.getDefault().sendTextMessage(number, null, message, null, null);
            return "SMS sent to " + number;
        } catch (Exception e) { return "SMS error: " + e.getMessage(); }
    }

    private String makeCall(String number) {
        try {
            android.content.Intent intent = new android.content.Intent(android.content.Intent.ACTION_CALL, Uri.parse("tel:" + number));
            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "calling " + number;
        } catch (Exception e) { return "call error: " + e.getMessage(); }
    }

    private String openUrl(String url) {
        try {
            if (!url.startsWith("http")) url = "https://" + url;
            android.content.Intent intent = new android.content.Intent(android.content.Intent.ACTION_VIEW, Uri.parse(url));
            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "opened " + url;
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String deepLink(String uri) {
        try {
            android.content.Intent intent = new android.content.Intent(android.content.Intent.ACTION_VIEW, Uri.parse(uri));
            intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
            return "opened: " + uri;
        } catch (Exception e) { return "deep link error: " + e.getMessage(); }
    }

    private String shareText(String text, String app) {
        try {
            android.content.Intent i = new android.content.Intent(android.content.Intent.ACTION_SEND);
            i.setType("text/plain");
            i.putExtra(android.content.Intent.EXTRA_TEXT, text);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            if (!app.isEmpty()) i.setPackage(APP_MAP.getOrDefault(app.toLowerCase(), app));
            ctx.startActivity(i);
            return "shared text";
        } catch (Exception e) { return "share error: " + e.getMessage(); }
    }

    // Read data
    private String readSms(int count) {
        try {
            android.database.Cursor c = ctx.getContentResolver().query(
                Uri.parse("content://sms/inbox"), null, null, null, "date DESC LIMIT " + count);
            if (c == null) return "SMS permission denied";
            StringBuilder sb = new StringBuilder();
            while (c.moveToNext()) {
                String addr = c.getString(c.getColumnIndexOrThrow("address"));
                String body = c.getString(c.getColumnIndexOrThrow("body"));
                sb.append("From: ").append(addr).append("\n")
                  .append(body.substring(0, Math.min(100, body.length()))).append("\n---\n");
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
                String t = type == android.provider.CallLog.Calls.INCOMING_TYPE ? "in"
                         : type == android.provider.CallLog.Calls.OUTGOING_TYPE ? "out" : "missed";
                sb.append("[").append(t).append("] ").append(name != null ? name : number).append("\n");
            }
            c.close();
            return sb.length() == 0 ? "no calls" : sb.toString().trim();
        } catch (Exception e) { return "call log error: " + e.getMessage(); }
    }

    // Alarms & timers
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

    private String playTone(int freq, int durationMs) {
        try {
            int sampleRate = 44100;
            int numSamples = sampleRate * durationMs / 1000;
            short[] samples = new short[numSamples];
            for (int i = 0; i < numSamples; i++) {
                samples[i] = (short)(32767 * Math.sin(2 * Math.PI * freq * i / sampleRate));
            }
            android.media.AudioTrack track = new android.media.AudioTrack.Builder()
                .setAudioAttributes(new android.media.AudioAttributes.Builder()
                    .setUsage(android.media.AudioAttributes.USAGE_MEDIA).build())
                .setAudioFormat(new android.media.AudioFormat.Builder()
                    .setEncoding(android.media.AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(sampleRate)
                    .setChannelMask(android.media.AudioFormat.CHANNEL_OUT_MONO).build())
                .setBufferSizeInBytes(numSamples * 2)
                .setTransferMode(android.media.AudioTrack.MODE_STATIC).build();
            track.write(samples, 0, numSamples);
            track.play();
            return "playing " + freq + "Hz for " + durationMs + "ms";
        } catch (Exception e) { return "tone error: " + e.getMessage(); }
    }

    private String openCameraApp() {
        try {
            android.content.Intent i = new android.content.Intent(android.provider.MediaStore.ACTION_IMAGE_CAPTURE);
            i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "camera opened";
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
            return "calendar event: " + title;
        } catch (Exception e) { return "calendar error: " + e.getMessage(); }
    }

    // Web
    private String webSearch(String query) {
        try {
            String encoded = URLEncoder.encode(query, "UTF-8");
            URL url = new URL("https://api.duckduckgo.com/?q=" + encoded + "&format=json&no_html=1&skip_disambig=1");
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            conn.setRequestProperty("User-Agent", "KiraAgent/6.0");
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line);
            org.json.JSONObject resp = new org.json.JSONObject(sb.toString());
            String answer   = resp.optString("Answer", "");
            String abstract_ = resp.optString("Abstract", "");
            String result = !answer.isEmpty() ? answer : (!abstract_.isEmpty() ? abstract_ : "no direct answer");
            return result;
        } catch (Exception e) {
            openUrl("https://duckduckgo.com/?q=" + query.replace(" ", "+"));
            return "opened search for: " + query;
        }
    }

    private String httpGet(String urlStr) {
        try {
            if (!urlStr.startsWith("http")) urlStr = "https://" + urlStr;
            URL url = new URL(urlStr);
            HttpURLConnection conn = (HttpURLConnection) url.openConnection();
            conn.setConnectTimeout(10000);
            conn.setReadTimeout(10000);
            conn.setRequestProperty("User-Agent", "KiraAgent/6.0");
            BufferedReader r = new BufferedReader(new InputStreamReader(conn.getInputStream(), StandardCharsets.UTF_8));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line).append("\n");
            String result = sb.toString().trim();
            return result.length() > 2000 ? result.substring(0, 2000) + "... (truncated)" : result;
        } catch (Exception e) { return "http error: " + e.getMessage(); }
    }

    private String scrapeWeb(String url, String selector) {
        try {
            if (!url.startsWith("http")) url = "https://" + url;
            org.jsoup.nodes.Document doc = Jsoup.connect(url)
                .userAgent("Mozilla/5.0 KiraAgent/6.0")
                .timeout(10000).get();
            String text = selector.isEmpty() ? doc.body().text() : doc.select(selector).text();
            return text.substring(0, Math.min(2000, text.length()));
        } catch (Exception e) { return "scrape error: " + e.getMessage(); }
    }

    // Files
    private String writeFile(String path, String content) {
        try {
            java.io.FileWriter fw = new java.io.FileWriter(path);
            fw.write(content);
            fw.close();
            return "written to " + path;
        } catch (Exception e) {
            return ShizukuShell.exec("echo '" + content.replace("'", "") + "' > " + path);
        }
    }

    // Sensors
    private String getLocation() {
        return ShizukuShell.exec("dumpsys location | grep 'last location' | head -3");
    }

    // Key codes
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
            case "menu":        return "82";
            case "tab":         return "61";
            case "up":          return "19";
            case "down":        return "20";
            case "left":        return "21";
            case "right":       return "22";
            default:            return key;
        }
    }


    private String analyzeScreen(String question) {
        KiraConfig cfg = KiraConfig.load(ctx);
        return new KiraVision(ctx).analyzeScreen(question, cfg);
    }

    private String findAndTapVisual(String description) {
        KiraConfig cfg = KiraConfig.load(ctx);
        int[] coords = new KiraVision(ctx).findElementCoords(description, cfg);
        if (coords != null) {
            tap(coords[0], coords[1]);
            return "found and tapped: " + description + " at (" + coords[0] + "," + coords[1] + ")";
        }
        return "element not found visually: " + description;
    }

    public String getToolList() {
        return "SCREEN: read_screen, tap_screen, tap_text, swipe_screen, scroll_screen, type_text, "
            + "press_back, press_home, press_recents, lock_screen, clipboard_get, clipboard_set, get_notifications\n"
            + "APPS: open_app, find_app, list_apps, force_stop, install_apk, uninstall\n"
            + "SYSTEM: battery_info, device_info, set_volume, set_brightness, torch, wifi_on, mobile_data, "
            + "airplane_mode, reboot, sleep_screen, get_wifi_info, scan_wifi\n"
            + "SHELL: sh_run, sh_tap, sh_swipe, sh_key, sh_type, sh_screenshot, sh_dump_ui, grant_perm, "
            + "running_apps, list_files, read_file, write_file, delete_file, find_files, download, ping, "
            + "top_cpu, disk_usage, logcat, dumpsys, am_broadcast, pm_list, zip_files, unzip, "
            + "open_notification_shade, close_notification_shade, take_video\n"
            + "COMMS: send_sms, call_number, open_url, deep_link, share_text\n"
            + "DATA: read_sms, read_contacts, read_call_log\n"
            + "ACTIONS: set_alarm, set_timer, vibrate, play_tone, take_photo, calendar_add\n"
            + "WEB: web_search, http_get, curl, scrape_web\n"
            + "PROACTIVE: schedule_task, watch_battery\n"
            + "MEMORY: remember, recall, forget, memory_list\n";
    }
}
