package com.kira.service;

import android.content.pm.PackageManager;
import android.util.Log;
import rikka.shizuku.Shizuku;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.concurrent.TimeUnit;

/**
 * v38 \u2014 Fixed Shizuku detection:
 *  \u2022 pingBinder() is wrapped correctly with IllegalStateException catch
 *    (Shizuku throws ISE if the binder was never set, not just on disconnect).
 *  \u2022 isInstalled() no longer conflates "binder present" with "permission granted".
 *  \u2022 requestPermission now properly guards shouldShowRequestPermissionRationale.
 */
public class ShizukuShell {
    private static final String TAG = "ShizukuShell";

    /**
     * Returns true ONLY if Shizuku is running AND permission is granted.
     * This is the "god mode" check.
     */
    public static boolean isAvailable() {
        try {
            if (!Shizuku.pingBinder()) return false;
            return Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED;
        } catch (IllegalStateException e) {
            // Shizuku service not connected yet \u2014 normal on cold start
            return false;
        } catch (Exception e) {
            Log.w(TAG, "isAvailable check failed: " + e.getMessage());
            return false;
        }
    }

    /**
     * Returns true if Shizuku is running (binder alive) even if permission
     * hasn't been granted yet.
     */
    public static boolean isInstalled() {
        try {
            return Shizuku.pingBinder();
        } catch (IllegalStateException e) {
            return false;
        } catch (Exception e) {
            return false;
        }
    }

    /**
     * Request Shizuku permission.
     * Guards against shouldShowRequestPermissionRationale crash on older builds.
     */
    public static void requestPermission(int requestCode) {
        try {
            // On some Shizuku versions, shouldShowRequestPermissionRationale
            // throws when the binder isn't ready \u2014 catch separately.
            boolean shouldShow = false;
            try { shouldShow = Shizuku.shouldShowRequestPermissionRationale(); }
            catch (Exception ignored) {}

            if (!shouldShow) {
                Shizuku.requestPermission(requestCode);
            }
        } catch (Exception e) {
            Log.e(TAG, "requestPermission failed: " + e.getMessage());
        }
    }

    /** Execute a shell command. Falls back to unprivileged shell if Shizuku unavailable. */
    public static String exec(String command) {
        return exec(command, 15_000);
    }

    public static String exec(String command, int timeoutMs) {
        if (!isAvailable()) return execFallback(command);
        return execDirect(command, timeoutMs);
    }

    private static String execDirect(String command, int timeoutMs) {
        try {
            Process process = Runtime.getRuntime().exec(new String[]{"sh", "-c", command});
            StringBuilder out = new StringBuilder(), err = new StringBuilder();

            Thread t1 = new Thread(() -> {
                try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
                    String line; while ((line = r.readLine()) != null) out.append(line).append('\n');
                } catch (Exception ignored) {}
            });
            Thread t2 = new Thread(() -> {
                try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getErrorStream()))) {
                    String line; while ((line = r.readLine()) != null) err.append(line).append('\n');
                } catch (Exception ignored) {}
            });
            t1.start(); t2.start();

            boolean done = process.waitFor(timeoutMs, TimeUnit.MILLISECONDS);
            if (!done) { process.destroyForcibly(); return "(timeout after " + timeoutMs + "ms)"; }
            t1.join(1000); t2.join(1000);

            String o = out.toString().trim(), e = err.toString().trim();
            if (o.isEmpty() && !e.isEmpty()) return e;
            if (!e.isEmpty()) return o + "\n[stderr]: " + e;
            return o.isEmpty() ? "(no output)" : o;
        } catch (Exception e) {
            Log.e(TAG, "execDirect failed", e);
            return "error: " + e.getMessage();
        }
    }

    private static String execFallback(String command) {
        try {
            Process p = Runtime.getRuntime().exec(new String[]{"sh", "-c", command});
            BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line).append('\n');
            p.waitFor(8000, TimeUnit.MILLISECONDS);
            String result = sb.toString().trim();
            return result.isEmpty() ? "(no output)" : result;
        } catch (Exception e) {
            return "shell error: " + e.getMessage();
        }
    }

    public static String openApp(android.content.Context ctx, String packageName) {
        try {
            android.content.pm.PackageManager pm = ctx.getPackageManager();
            android.content.Intent intent = pm.getLaunchIntentForPackage(packageName);
            if (intent != null) {
                intent.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK |
                        android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED);
                ctx.startActivity(intent);
                Thread.sleep(800);
                return "opened " + packageName;
            }
        } catch (Exception e) {
            Log.w(TAG, "openApp method1 failed: " + e.getMessage());
        }
        String result = exec("monkey -p " + packageName + " -c android.intent.category.LAUNCHER 1 2>&1");
        if (!result.contains("No activities") && !result.contains("error"))
            return "opened " + packageName + " (shell)";
        result = exec("am start -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p " + packageName + " 2>&1");
        if (result.contains("Starting")) return "opened " + packageName + " (am)";
        return "could not open " + packageName + " \u2014 not installed or no launcher activity";
    }

    public static String screenshot(String outputPath) {
        String result = exec("screencap -p " + outputPath);
        return (result.isEmpty() || result.equals("(no output)"))
                ? "screenshot saved to " + outputPath
                : "screenshot error: " + result;
    }

    public static String dumpUI() {
        String tmp = "/sdcard/kira_ui_dump.xml";
        exec("uiautomator dump " + tmp + " 2>&1");
        String xml = exec("cat " + tmp + " 2>&1");
        exec("rm -f " + tmp);
        if (xml.contains("<?xml")) {
            java.util.List<String> texts = new java.util.ArrayList<>();
            java.util.regex.Matcher m = java.util.regex.Pattern.compile("text=\"([^\"]+)\"").matcher(xml);
            while (m.find()) { String t = m.group(1); if (t != null && !t.isEmpty()) texts.add(t); }
            return texts.isEmpty() ? "no text found"
                    : String.join("\n", texts.subList(0, Math.min(50, texts.size())));
        }
        return xml;
    }
}
