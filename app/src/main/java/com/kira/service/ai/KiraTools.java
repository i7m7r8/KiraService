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
                case "vlm_agent":     { // Roubao/Open-AutoGLM: VLM-guided task
                    String goal = args.optString("goal", args.optString("task",""));
                    int maxSteps = args.optInt("max_steps", 20);
                    try {
                        String result = com.kira.service.RustBridge.startAgentTask(goal, maxSteps);
                        return "Agent task started: " + result;
                    } catch (Exception e) { return "Agent error: " + e.getMessage(); }
                }
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

                case "watch_battery": { RustBridge.addTrigger("bat_" + args.optInt("threshold",20), "battery_low", String.valueOf(args.optInt("threshold",20)), args.getString("action"), args.optBoolean("repeat", true)); return "watching battery < " + args.optInt("threshold",20) + "%"; }
                case "watch_notif":   { RustBridge.addTrigger("notif_" + System.currentTimeMillis(), "keyword_notif", args.getString("keyword"), args.getString("action"), args.optBoolean("repeat", true)); return "watching notifications for: " + args.getString("keyword"); }
                case "if_then":       { String ifC=args.optString("if",""); String thenA=args.optString("then",""); if(ifC.isEmpty()||thenA.isEmpty()) return "need 'if' and 'then'"; return postRust("/auto/if_then","{\"if\":\""+ifC.replace("\"","'")+"\",\"then\":\""+thenA.replace("\"","'")+"\"}"); }
                case "watch_app":     { String app=args.optString("app",""); String act=args.optString("action",""); return postRust("/auto/watch_app","{\"app\":\""+app+"\",\"action\":\""+act.replace("\"","'")+"\"}"); }
                case "repeat_task":   { String task=args.optString("task",""); int min=args.optInt("every_minutes",30); return postRust("/auto/repeat","{\"task\":\""+task.replace("\"","'")+"\",\"every_minutes\":"+min+"}"); }
                case "on_notif":      { String kw=args.optString("keyword",""); String act=args.optString("action",""); String app2=args.optString("app",""); return postRust("/auto/on_notif","{\"keyword\":\""+kw+"\",\"action\":\""+act.replace("\"","'")+"\",\"app\":\""+app2+"\"}"); }
                case "on_time":       { String t=args.optString("time","08:00"); String act=args.optString("action",""); return postRust("/auto/on_time","{\"time\":\""+t+"\",\"action\":\""+act.replace("\"","'")+"\"}"); }
                case "on_charge":     { String act=args.optString("action",""); String st=args.optString("state","plugged"); return postRust("/auto/on_charge","{\"action\":\""+act.replace("\"","'")+"\",\"state\":\""+st+"\"}"); }
                case "list_automations": case "list_macros": { return getRust("/auto/list"); }
                case "delete_automation": { return deleteRust("/auto/"+args.optString("id","")); }
                case "enable_automation": { return postRust("/auto/enable","{\"id\":\""+args.optString("id","")+"\",\"enabled\":"+args.optBoolean("enabled",true)+"}"); }
                case "list_watches":  { return ShizukuShell.exec("echo watches active"); }
                case "stop_watch":    { RustBridge.removeTrigger(args.getString("id")); return "stopped watch: " + args.getString("id"); }



                // OpenClaw: Agent coordination
                case "agent_handoff":  { String to = args.optString("to","main"); String msg = args.getString("message"); com.kira.service.RustBridge.pushContextTurn("system","handoff->"+to+": "+msg); return "handed off to " + to; }
                case "post_skill":     { com.kira.service.RustBridge.registerSkill(args.getString("name"),args.optString("description",""),args.optString("trigger",""),args.getString("content")); return "skill registered: " + args.getString("name"); }
                case "list_skills":    { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/skills").build()).execute().body().string(); } catch(Exception e){ return "error: "+e.getMessage(); } }


                // OpenClaw: Knowledge base population (RAG)
                case "kb_add":        { try { String b2="{\"id\":\""+args.optString("id",String.valueOf(System.currentTimeMillis()))+"\""+",\"title\":\""+args.getString("title").replace("\"","\\\"")+"\""+",\"content\":\""+args.getString("content").replace("\"","\\\"")+"\""+",\"tags\":\""+args.optString("tags","")+"\""+"}"; new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/kb/add").post(okhttp3.RequestBody.create(b2,okhttp3.MediaType.parse("application/json"))).build()).execute(); return "kb entry added: "+args.getString("title"); } catch(Exception e){ return "error: "+e.getMessage(); } }
                case "kb_search":     { try { String q=java.net.URLEncoder.encode(args.getString("query"),"UTF-8"); return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/kb/search?q="+q).build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                // OpenClaw: Event feed
                case "post_to_feed":  { try { String b2="{\"event\":\""+args.getString("event")+"\",\"data\":\""+args.optString("data","")+"\"}"; new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/events").post(okhttp3.RequestBody.create(b2,okhttp3.MediaType.parse("application/json"))).build()).execute(); return "event posted"; } catch(Exception e){ return "error"; } }
                case "get_events":    { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/events").build()).execute().body().string(); } catch(Exception e){ return "offline"; } }
                // ZeroClaw: Cache
                case "cache_set":     { try { String b2="{\"key\":\""+args.getString("key")+"\",\"value\":\""+args.getString("value").replace("\"","\\\"")+"\",\"ttl_ms\":"+args.optLong("ttl_ms",300000)+"}"; new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/cache").post(okhttp3.RequestBody.create(b2,okhttp3.MediaType.parse("application/json"))).build()).execute(); return "cached: "+args.getString("key"); } catch(Exception e){ return "error"; } }
                case "cache_get":     { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/cache?key="+java.net.URLEncoder.encode(args.getString("key"),"UTF-8")).build()).execute().body().string(); } catch(Exception e){ return "cache miss"; } }
                // NanoClaw: budget
                case "get_budget":    { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/budget").build()).execute().body().string(); } catch(Exception e){ return "offline"; } }
                case "reset_budget":  { com.kira.service.RustBridge.resetToolIter(args.optString("session","default")); return "budget reset"; }
                // Rou Bao: streaming
                case "stream_chunk":  { try { String b2="{\"text\":\""+args.getString("text").replace("\"","\\\"")+"\"}"; new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/stream/chunk").post(okhttp3.RequestBody.create(b2,okhttp3.MediaType.parse("application/json"))).build()).execute(); return "streamed"; } catch(Exception e){ return "error"; } }
                // OpenClaw: relay to another channel
                case "relay_to":      { try { String b2="{\"channel\":\""+args.getString("channel")+"\",\"message\":\""+args.getString("message").replace("\"","\\\"")+"\"}"; new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/relay").post(okhttp3.RequestBody.create(b2,okhttp3.MediaType.parse("application/json"))).build()).execute(); return "relayed to: "+args.getString("channel"); } catch(Exception e){ return "error"; } }
                // OpenClaw: Workflows (.agent/workflows pattern)
                case "list_workflows":   return new KiraWorkflow(ctx).listJson();
                case "run_workflow":     { String goal = new KiraWorkflow(ctx).buildGoal(args.getString("name")); return "workflow goal: " + goal + " (use /agent or /chain to run)"; }
                case "save_workflow":    { new KiraWorkflow(ctx).save(args.getString("name"), args.getString("description"), args.optString("steps","")); return "workflow saved: " + args.getString("name"); }

                // NanoClaw: Checkpoints
                case "save_checkpoint": { new KiraCheckpoint(ctx).save(args.optString("id","task_"+System.currentTimeMillis()), args.optInt("step",0), args.optString("state",""), args.optString("goal","")); return "checkpoint saved"; }
                case "list_checkpoints":{ return new KiraCheckpoint(ctx).getAllJson(); }
                case "resume_task":     { org.json.JSONObject cp = new KiraCheckpoint(ctx).get(args.getString("id")); return cp != null ? "Resume from step " + cp.optInt("step") + ": " + cp.optString("goal") : "checkpoint not found"; }
                // ZeroClaw: Provider switching
                case "switch_provider": { try { String pid=args.getString("id"); new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/providers/set").post(okhttp3.RequestBody.create("{\"id\":\""+pid+"\"}",okhttp3.MediaType.parse("application/json"))).build()).execute(); return "switched to provider: "+pid; } catch(Exception e){ return "error: "+e.getMessage(); } }
                case "list_providers":  { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/providers").build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                // OpenClaw: Custom skill registration
                case "register_skill":  { new KiraSkillEngine(ctx).registerCustomSkill(args.getString("name"), args.getString("trigger"), args.getString("content")); return "skill registered: " + args.getString("name"); }
                // OpenClaw: SOUL.md - agent identity
                case "set_persona":     { com.kira.service.RustBridge.pushContextTurn("system","[PERSONA] " + args.getString("persona")); return "persona updated"; }
                // ZeroClaw: Agent handoff
                case "handoff":         { new KiraCheckpoint(ctx).sendHandoff(args.optString("to","main"), args.getString("message"), args.optString("context","")); return "handoff sent"; }
                case "check_handoffs":  { java.util.List<org.json.JSONObject> hs = new KiraCheckpoint(ctx).getUnreadHandoffs(args.optString("session","main")); return hs.isEmpty() ? "no handoffs" : hs.toString(); }

                // NanoClaw: Session management
                case "new_session":    { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/sessions/new").post(okhttp3.RequestBody.create(args.toString(),okhttp3.MediaType.parse("application/json"))).build()).execute().body().string(); } catch(Exception e){ return "error: "+e.getMessage(); } }
                // AndyClaw: Policy
                case "allow_tool":     { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/policy/allow").post(okhttp3.RequestBody.create("{\"tool\":\""+args.getString("tool")+"\"}",okhttp3.MediaType.parse("application/json"))).build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                case "deny_tool":      { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/policy/deny").post(okhttp3.RequestBody.create("{\"tool\":\""+args.getString("tool")+"\"}",okhttp3.MediaType.parse("application/json"))).build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                // ZeroClaw: Credentials
                case "store_credential":{ com.kira.service.RustBridge.storeCredential(args.getString("name"),args.getString("value")); return "credential stored: " + args.getString("name"); }
                case "get_credential": { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/credentials/get").post(okhttp3.RequestBody.create("{\"name\":\""+args.getString("name")+"\"}",okhttp3.MediaType.parse("application/json"))).build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                // OpenClaw: SOUL.md (agent identity)
                case "set_soul":       { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/soul").post(okhttp3.RequestBody.create("{\"content\":\""+args.getString("content")+"\"}",okhttp3.MediaType.parse("application/json"))).build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                case "get_soul":       { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/soul").build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                // Audit
                case "audit_log":      { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/audit_log").build()).execute().body().string(); } catch(Exception e){ return "error"; } }
                case "memory_search":  { try { return new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder().url("http://localhost:7070/memory/search?q="+java.net.URLEncoder.encode(args.getString("query"),"UTF-8")).build()).execute().body().string(); } catch(Exception e){ return "error"; } }

                // OpenClaw event system
                case "post_event":    { com.kira.service.KiraEventBus.post(new com.kira.service.KiraEventBus.AgentStarted(args.optString("goal","custom"))); return "event posted"; }
                // NanoBot context tools
                case "set_context":   { memory.remember("_agent_context", args.getString("text")); com.kira.service.RustBridge.updateAgentContext(args.getString("text")); return "context set"; }
                case "get_context":   return memory.recall("_agent_context");
                case "task_log":      return getTaskLog();
                case "task_status":   return getTaskLog();
                // Extended shell
                case "sh_broadcast":  return ShizukuShell.exec("am broadcast -a " + args.getString("action") + " " + args.optString("extras","") + " 2>&1");
                case "sh_service":    return ShizukuShell.exec("am startservice " + args.optString("pkg","") + "/" + args.getString("class") + " 2>&1");
                case "sh_activity":   return ShizukuShell.exec("am start -n " + args.getString("component") + " 2>&1");
                case "sh_input_text": return ShizukuShell.exec("input text '" + args.getString("text").replace("'","").replace(" ","%s") + "'");
                case "sh_back":       return ShizukuShell.exec("input keyevent 4");
                case "sh_home":       return ShizukuShell.exec("input keyevent 3");
                case "sh_recents":    return ShizukuShell.exec("input keyevent 187");
                case "sh_lock":       return ShizukuShell.exec("input keyevent 26");
                case "sh_wakeup":     return ShizukuShell.exec("input keyevent 224");
                case "sh_vol_up":     return ShizukuShell.exec("input keyevent 24");
                case "sh_vol_down":   return ShizukuShell.exec("input keyevent 25");
                case "sh_mute":       return ShizukuShell.exec("input keyevent 164");
                case "sh_brightness": return ShizukuShell.exec("settings put system screen_brightness " + (int)(args.optInt("level",50) * 2.55));
                case "sh_dnd":        return ShizukuShell.exec("cmd notification set_dnd " + (args.optBoolean("on",true) ? "on" : "off") + " 2>&1");
                case "sh_rotation":   return ShizukuShell.exec("settings put system accelerometer_rotation " + (args.optBoolean("auto",true) ? "1" : "0"));
                case "sh_font_scale": return ShizukuShell.exec("settings put system font_scale " + args.optString("scale","1.0"));
                case "sh_battery_saver": return ShizukuShell.exec("settings put global low_power " + (args.optBoolean("on",true) ? "1" : "0"));
                case "sh_wifi_scan":  return ShizukuShell.exec("cmd wifi start-scan && sleep 1 && cmd wifi list-scan-results 2>&1 | head -20");
                case "sh_bluetooth":  return ShizukuShell.exec("svc bluetooth " + (args.optBoolean("on",true) ? "enable" : "disable") + " 2>&1");
                case "sh_nfc":        return ShizukuShell.exec("svc nfc " + (args.optBoolean("on",true) ? "enable" : "disable") + " 2>&1");
                case "sh_hotspot":    return ShizukuShell.exec("cmd connectivity tether wifi " + (args.optBoolean("on",true) ? "start" : "stop") + " 2>&1");
                case "sh_clear_app":  return ShizukuShell.exec("pm clear " + args.getString("package") + " 2>&1");
                case "sh_app_info":   return ShizukuShell.exec("dumpsys package " + args.getString("package") + " | grep -E 'versionName|firstInstallTime|lastUpdateTime|dataDir' | head -8");
                case "sh_cpu_info":   return ShizukuShell.exec("cat /proc/cpuinfo | grep -E 'processor|model name|Hardware' | head -10");
                case "sh_ram_info":   return ShizukuShell.exec("cat /proc/meminfo | head -8");
                case "sh_storage":    return ShizukuShell.exec("df -h 2>&1 | head -8");
                case "sh_netstat":    return ShizukuShell.exec("ss -tuln 2>/dev/null || netstat -tuln 2>/dev/null | head -15");
                case "sh_ps":         return ShizukuShell.exec("ps -A 2>/dev/null | head -20 || ps | head -20");
                case "sh_ifconfig":   return ShizukuShell.exec("ifconfig 2>/dev/null || ip addr show 2>/dev/null | head -20");
                case "sh_crontab":    { String cmd = "echo '" + args.getString("cmd") + "' >> /sdcard/kira_cron.sh && chmod +x /sdcard/kira_cron.sh"; return ShizukuShell.exec(cmd); }

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
        String normalized = input.toLowerCase().trim();
        String pkg = APP_MAP.getOrDefault(normalized, input.trim());

        // Special case: YouTube — try multiple methods in order
        if ("com.google.android.youtube".equals(pkg) || normalized.equals("youtube") || normalized.equals("yt")) {
            pkg = "com.google.android.youtube";
            // Method 1: Direct getLaunchIntentForPackage (most reliable)
            try {
                android.content.pm.PackageManager pm2 = ctx.getPackageManager();
                android.content.Intent launch = pm2.getLaunchIntentForPackage(pkg);
                if (launch != null) {
                    launch.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK
                        | android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
                    ctx.startActivity(launch);
                    android.os.SystemClock.sleep(500);
                    return "opened YouTube";
                }
            } catch (Exception e1) {
                android.util.Log.w("KiraTools", "YouTube method1: " + e1.getMessage());
            }
            // Method 2: am start with explicit activity via Shizuku
            String amR = ShizukuShell.exec(
                "am start -n com.google.android.youtube/com.google.android.youtube.HomeActivity 2>&1");
            if (amR.contains("Starting")) return "opened YouTube (am)";
            // Method 3: am start with package
            amR = ShizukuShell.exec(
                "am start -a android.intent.action.MAIN -c android.intent.category.LAUNCHER" +
                " -p com.google.android.youtube 2>&1");
            if (amR.contains("Starting")) return "opened YouTube (am2)";
            // Method 4: monkey
            amR = ShizukuShell.exec(
                "monkey -p com.google.android.youtube -c android.intent.category.LAUNCHER 1 2>&1");
            if (!amR.contains("No activities") && !amR.contains("error"))
                return "opened YouTube (monkey)";
            return "YouTube not installed. Install from Play Store.";
        }

        // Resolve package if fuzzy name was given
        if (!isInstalled(pkg)) {
            String found = findPackage(input);
            if (found != null) {
                pkg = found;
            } else {
                // Try am start with the fuzzy name as package
                String amResult = ShizukuShell.exec(
                    "am start -a android.intent.action.MAIN -c android.intent.category.LAUNCHER " +
                    "$(pm list packages | grep -i " + input.toLowerCase().replaceAll("[^a-z0-9]","") +
                    " | head -1 | cut -d: -f2) 2>&1");
                if (amResult.contains("Starting")) return "opened " + input + " (am-fuzzy)";
                return "app not found: " + input + ". Is it installed? Try: find_app " + input;
            }
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


    private String getTaskLog() {
        try {
            String result = new okhttp3.OkHttpClient().newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070/task_log").build()
            ).execute().body().string();
            return result.length() > 500 ? result.substring(0, 500) + "..." : result;
        } catch (Exception e) {
            return "task log unavailable: " + e.getMessage();
        }
    }

    // ── HTTP helpers for Rust engine calls ──────────────────────────────────────
    private static final okhttp3.OkHttpClient RUST_CLIENT = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(3, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(6, java.util.concurrent.TimeUnit.SECONDS)
        .build();

    private String getRust(String path) {
        try {
            okhttp3.Response r = RUST_CLIENT.newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070" + path).get().build()).execute();
            return r.body() != null ? r.body().string() : "{\"error\":\"empty\"}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage().replace("\"","'") + "\"}"; }
    }

    private String postRust(String path, String jsonBody) {
        try {
            okhttp3.Response r = RUST_CLIENT.newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070" + path)
                    .post(okhttp3.RequestBody.create(jsonBody,
                        okhttp3.MediaType.parse("application/json"))).build()).execute();
            return r.body() != null ? r.body().string() : "{\"ok\":true}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage().replace("\"","'") + "\"}"; }
    }

    private String deleteRust(String path) {
        try {
            okhttp3.Response r = RUST_CLIENT.newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070" + path)
                    .delete().build()).execute();
            return r.body() != null ? r.body().string() : "{\"ok\":true}";
        } catch (Exception e) { return "{\"error\":\"" + e.getMessage().replace("\"","'") + "\"}"; }
    }

    // ── Single-command automation shortcuts ─────────────────────────────────────
    // Called when AI detects a compound intent like "open YouTube and search for music"
    // These compose multiple tools into one call so the AI doesn't need to chain steps.

    /** Execute a sequence of tool calls from a plain-English compound command */
    public String runScenario(String scenario) {
        String s = scenario.toLowerCase().trim();
        StringBuilder log = new StringBuilder();

        // ── Media commands ────────────────────────────────────────────────
        if (s.contains("youtube") && (s.contains("play") || s.contains("open") || s.contains("watch"))) {
            log.append(openApp("youtube")).append("\n");
            return log.toString().trim();
        }
        if (s.contains("spotify") && (s.contains("play") || s.contains("music") || s.contains("open"))) {
            log.append(openApp("spotify")).append("\n");
            return log.toString().trim();
        }
        if ((s.contains("play music") || s.contains("music player")) && !s.contains("youtube")) {
            log.append(openApp("spotify")).append("\n");
            return log.toString().trim();
        }

        // ── Navigation ────────────────────────────────────────────────────
        if (s.contains("navigate to") || s.contains("directions to") || s.contains("take me to")) {
            String dest = s.replaceAll(".*(?:navigate to|directions to|take me to)\s*", "").trim();
            try {
                android.content.Intent nav = new android.content.Intent(
                    android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("google.navigation:q=" + java.net.URLEncoder.encode(dest, "UTF-8")));
                nav.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(nav);
                return "navigating to: " + dest;
            } catch (Exception e) { return openApp("maps"); }
        }

        // ── Communication ─────────────────────────────────────────────────
        if (s.contains("call") && !s.contains("video call")) {
            String num = s.replaceAll("[^0-9+]", "");
            if (!num.isEmpty()) {
                try {
                    android.content.Intent call = new android.content.Intent(
                        android.content.Intent.ACTION_CALL,
                        android.net.Uri.parse("tel:" + num));
                    call.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(call);
                    return "calling " + num;
                } catch (Exception e) { return "call failed: " + e.getMessage(); }
            }
        }
        if (s.contains("whatsapp") && s.contains("message")) {
            log.append(openApp("whatsapp")).append("\n");
            return log.toString().trim();
        }
        if (s.contains("telegram") && (s.contains("open") || s.contains("message"))) {
            log.append(openApp("telegram")).append("\n");
            return log.toString().trim();
        }

        // ── Search ────────────────────────────────────────────────────────
        if (s.contains("search") || s.contains("google")) {
            String query = s.replaceAll(".*(?:search for|google|look up|find)\s*", "").trim();
            if (!query.isEmpty()) {
                try {
                    android.content.Intent web = new android.content.Intent(
                        android.content.Intent.ACTION_VIEW,
                        android.net.Uri.parse("https://www.google.com/search?q=" +
                            java.net.URLEncoder.encode(query, "UTF-8")));
                    web.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(web);
                    return "searching: " + query;
                } catch (Exception e) { return "search failed"; }
            }
        }

        // ── System shortcuts ──────────────────────────────────────────────
        if (s.contains("screenshot") || s.contains("screen shot")) {
            return ShizukuShell.exec("screencap -p /sdcard/Pictures/Screenshots/kira_" +
                System.currentTimeMillis() + ".png 2>&1");
        }
        if (s.contains("brightness") && s.contains("max")) {
            return ShizukuShell.exec("settings put system screen_brightness 255 2>&1");
        }
        if (s.contains("brightness") && (s.contains("min") || s.contains("low"))) {
            return ShizukuShell.exec("settings put system screen_brightness 10 2>&1");
        }
        if (s.contains("volume") && (s.contains("mute") || s.contains("silent") || s.contains("off"))) {
            return ShizukuShell.exec("media volume --stream 2 --set 0 2>&1");
        }
        if (s.contains("volume") && s.contains("max")) {
            return ShizukuShell.exec("media volume --stream 2 --set 15 2>&1");
        }
        if (s.contains("wifi") && s.contains("on")) {
            return ShizukuShell.exec("svc wifi enable 2>&1");
        }
        if (s.contains("wifi") && s.contains("off")) {
            return ShizukuShell.exec("svc wifi disable 2>&1");
        }
        if (s.contains("bluetooth") && s.contains("on")) {
            return ShizukuShell.exec("svc bluetooth enable 2>&1");
        }
        if (s.contains("bluetooth") && s.contains("off")) {
            return ShizukuShell.exec("svc bluetooth disable 2>&1");
        }
        if (s.contains("flashlight") || s.contains("torch")) {
            return ShizukuShell.exec(s.contains("off") ?
                "settings put secure camera_torch_on 0 2>&1" :
                "am broadcast -a android.intent.action.ACTION_POWER_SAVE_MODE_CHANGED 2>&1");
        }
        if (s.contains("airplane") || s.contains("flight mode")) {
            String state = s.contains("off") ? "0" : "1";
            return ShizukuShell.exec("settings put global airplane_mode_on " + state +
                " && am broadcast -a android.intent.action.AIRPLANE_MODE 2>&1");
        }
        if (s.contains("dark mode") || s.contains("night mode")) {
            String ui = s.contains("off") ? "1" : "2";
            return ShizukuShell.exec("cmd uimode night " + (ui.equals("2") ? "yes" : "no") + " 2>&1");
        }
        if (s.contains("reboot") || s.contains("restart phone")) {
            return ShizukuShell.exec("reboot 2>&1");
        }
        if (s.contains("lock screen") || s.contains("lock phone")) {
            return ShizukuShell.exec("input keyevent 26 2>&1");
        }
        if (s.contains("home screen") || s.contains("go home")) {
            return ShizukuShell.exec("input keyevent 3 2>&1");
        }
        if (s.contains("recent apps") || s.contains("app switcher")) {
            return ShizukuShell.exec("input keyevent 187 2>&1");
        }
        if (s.contains("battery") || s.contains("charge")) {
            return getBatteryInfo();
        }
        if (s.contains("notification") || s.contains("alerts")) {
            return ShizukuShell.exec("cmd statusbar expand-notifications 2>&1");
        }
        if (s.contains("clear notifications")) {
            return ShizukuShell.exec("service call notification 1 2>&1");
        }

        // ── App opening ───────────────────────────────────────────────────
        // Catch-all: try to open any mentioned app
        String[] knownApps = {"whatsapp","telegram","instagram","youtube","spotify","netflix",
            "gmail","chrome","maps","camera","settings","calculator","clock","contacts","phone",
            "messages","drive","photos","calendar","discord","reddit","twitter","facebook",
            "snapchat","tiktok","linkedin","amazon","uber","zoom","slack","notion","teams",
            "paypal","netflix","prime video","disney plus","twitch","zoom","spotify"};
        for (String app : knownApps) {
            if (s.contains(app)) {
                return openApp(app);
            }
        }

        return "scenario not recognised. Try: 'open YouTube', 'navigate to [place]', 'search for [query]', 'mute volume', 'take screenshot'";
    }

    public String getToolList() {
        return "SCREEN: read_screen, tap_screen, tap_text, swipe_screen, scroll_screen, type_text, "
            + "press_back, press_home, press_recents, lock_screen, clipboard_get, clipboard_set, get_notifications\n"
            + "APPS: open_app, find_app, list_apps, force_stop, install_apk, uninstall\n"
                     + "AGENT: vlm_agent {goal, max_steps} - AI vision agent, flow {id}, keyword {name}\n"
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
            + "PROACTIVE: schedule_task, watch_battery, watch_notif\n"
            + "AUTOMATION: if_then, watch_app, repeat_task, on_notif, on_time, on_charge, list_automations, delete_automation, enable_automation\n"
            + "MEMORY: remember, recall, forget, memory_list\n";
    }
}
