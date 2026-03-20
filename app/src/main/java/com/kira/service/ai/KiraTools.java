package com.kira.service.ai;

import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;
import java.util.Map;

/**
 * KiraTools — Session G: shrunk from 1073 to ~120 lines.
 *
 * Only 8 tools that REQUIRE Android Intent/API remain here.
 * All other tools (~82) now live in Rust dispatch_tool() in state.rs.
 *
 * Tool routing:
 *   Java handles: open_app, call_number, send_sms, open_url,
 *                 share_text, set_clipboard, press_home, press_back
 *   Rust handles: everything else (shell, file, memory, http, vars, macros…)
 */
public class KiraTools {
    private static final String TAG = "KiraTools";

    private final Context ctx;

    public KiraTools(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    /**
     * Execute a tool by name.
     * Java handles 8 intent-based tools; delegates everything else to Rust.
     */
    public String execute(String name, Map<String, String> args) {
        if (name == null) return "error: null tool name";
        switch (name) {
            case "open_app":     return openApp(args.getOrDefault("pkg",""),
                                               args.getOrDefault("app",""));
            case "call_number":  return call(args.getOrDefault("number",""));
            case "send_sms":     return sendSms(args.getOrDefault("to",""),
                                               args.getOrDefault("body",""));
            case "open_url":     return openUrl(args.getOrDefault("url",""));
            case "share_text":   return shareText(args.getOrDefault("text",""));
            case "set_clipboard":return setClipboard(args.getOrDefault("text",""));
            case "press_home":   return pressHome();
            case "press_back":   return pressBack();
            default:
                // Delegate to Rust for all other tools
                return RustBridge.executeTool(name, argsToJson(args));
        }
    }

    // ── 8 Intent-based tools (require Android APIs) ────────────────────────

    private String openApp(String pkg, String appName) {
        try {
            // Resolve package from name if needed
            if (pkg.isEmpty() && !appName.isEmpty()) {
                pkg = RustBridge.appNameToPkg(appName);
            }
            if (pkg.isEmpty()) return "error: unknown app: " + appName;
            Intent i = ctx.getPackageManager().getLaunchIntentForPackage(pkg);
            if (i == null) return "error: app not installed: " + pkg;
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "opened: " + pkg;
        } catch (Exception e) {
            return "error: " + e.getMessage();
        }
    }

    private String call(String number) {
        try {
            Intent i = new Intent(Intent.ACTION_CALL, Uri.parse("tel:" + number));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "calling: " + number;
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String sendSms(String to, String body) {
        try {
            android.telephony.SmsManager.getDefault().sendTextMessage(to, null, body, null, null);
            return "sms sent to " + to;
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String openUrl(String url) {
        try {
            Intent i = new Intent(Intent.ACTION_VIEW, Uri.parse(url));
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
            return "opened: " + url;
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String shareText(String text) {
        try {
            Intent i = new Intent(Intent.ACTION_SEND);
            i.setType("text/plain"); i.putExtra(Intent.EXTRA_TEXT, text);
            i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(Intent.createChooser(i, "Share").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            return "shared";
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String setClipboard(String text) {
        try {
            ClipboardManager cm = (ClipboardManager) ctx.getSystemService(Context.CLIPBOARD_SERVICE);
            if (cm != null) { cm.setPrimaryClip(ClipData.newPlainText("kira", text)); return "copied"; }
            return "error: clipboard unavailable";
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String pressHome() {
        try {
            Intent i = new Intent(Intent.ACTION_MAIN);
            i.addCategory(Intent.CATEGORY_HOME); i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i); return "home pressed";
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    private String pressBack() {
        try {
            // Use Shizuku if available for clean back press
            if (ShizukuShell.isAvailable()) {
                ShizukuShell.exec("input keyevent 4", 2_000);
                return "back pressed";
            }
            return "back: shizuku required";
        } catch (Exception e) { return "error: " + e.getMessage(); }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    private static String argsToJson(Map<String, String> args) {
        if (args == null || args.isEmpty()) return "{}";
        StringBuilder sb = new StringBuilder("{");
        for (Map.Entry<String, String> e : args.entrySet()) {
            sb.append("\"").append(e.getKey().replace("\"","\\\"")).append("\":\"")
              .append(e.getValue().replace("\\","\\\\").replace("\"","\\\"")).append("\",");
        }
        if (sb.charAt(sb.length()-1) == ',') sb.setLength(sb.length()-1);
        sb.append("}");
        return sb.toString();
    }
}
