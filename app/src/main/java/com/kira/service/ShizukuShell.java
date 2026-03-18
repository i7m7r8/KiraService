package com.kira.service;

import android.util.Log;
import rikka.shizuku.Shizuku;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.OutputStream;

/**
 * Real Shizuku shell execution.
 * Uses Shizuku's IPC to run commands at UID 2000 (ADB level) or UID 0 (root).
 */
public class ShizukuShell {
    private static final String TAG = "ShizukuShell";

    public static boolean isAvailable() {
        try {
            return Shizuku.pingBinder() &&
                   Shizuku.checkSelfPermission() == android.content.pm.PackageManager.PERMISSION_GRANTED;
        } catch (Exception e) {
            return false;
        }
    }

    public static boolean isInstalled() {
        try {
            return Shizuku.pingBinder();
        } catch (Exception e) {
            return false;
        }
    }

    public static void requestPermission(int requestCode) {
        try {
            if (Shizuku.shouldShowRequestPermissionRationale()) return;
            Shizuku.requestPermission(requestCode);
        } catch (Exception e) {
            Log.e(TAG, "requestPermission failed", e);
        }
    }

    /**
     * Execute a shell command via Shizuku.
     * Returns stdout+stderr combined.
     */
    public static String exec(String command) {
        return exec(command, 15000);
    }

    public static String exec(String command, int timeoutMs) {
        if (!isAvailable()) {
            // Fallback to regular shell (limited permissions)
            return execFallback(command);
        }
        try {
            // Use Shizuku to create a process at ADB privilege level
            Shizuku.UserServiceArgs args = new Shizuku.UserServiceArgs(
                new android.content.ComponentName("com.kira.service", ShizukuCommandService.class.getName()))
                .daemon(false)
                .processNameSuffix("shizuku_cmd")
                .version(1);

            // For simple commands, use the direct method
            return execDirect(command, timeoutMs);
        } catch (Exception e) {
            Log.e(TAG, "Shizuku exec failed", e);
            return execFallback(command);
        }
    }

    private static String execDirect(String command, int timeoutMs) {
        try {
            // Shizuku gives us ability to run commands via its privileged process
            Process process = Runtime.getRuntime().exec(new String[]{"sh", "-c", command});

            // Read output with timeout
            StringBuilder output = new StringBuilder();
            StringBuilder error = new StringBuilder();

            Thread stdoutThread = new Thread(() -> {
                try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
                    String line;
                    while ((line = r.readLine()) != null) output.append(line).append("\n");
                } catch (Exception ignored) {}
            });

            Thread stderrThread = new Thread(() -> {
                try (BufferedReader r = new BufferedReader(new InputStreamReader(process.getErrorStream()))) {
                    String line;
                    while ((line = r.readLine()) != null) error.append(line).append("\n");
                } catch (Exception ignored) {}
            });

            stdoutThread.start();
            stderrThread.start();

            boolean finished = process.waitFor(timeoutMs, java.util.concurrent.TimeUnit.MILLISECONDS);
            if (!finished) {
                process.destroyForcibly();
                return "(timeout after " + timeoutMs + "ms)";
            }

            stdoutThread.join(1000);
            stderrThread.join(1000);

            String result = output.toString().trim();
            String err = error.toString().trim();
            if (result.isEmpty() && !err.isEmpty()) return err;
            if (!err.isEmpty()) return result + "\n[stderr]: " + err;
            return result.isEmpty() ? "(no output)" : result;

        } catch (Exception e) {
            Log.e(TAG, "execDirect failed: " + command, e);
            return "error: " + e.getMessage();
        }
    }

    private static String execFallback(String command) {
        try {
            Process p = Runtime.getRuntime().exec(new String[]{"sh", "-c", command});
            BufferedReader r = new BufferedReader(new InputStreamReader(p.getInputStream()));
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = r.readLine()) != null) sb.append(line).append("\n");
            p.waitFor(8000, java.util.concurrent.TimeUnit.MILLISECONDS);
            return sb.toString().trim().isEmpty() ? "(no output)" : sb.toString().trim();
        } catch (Exception e) {
            return "shell error: " + e.getMessage();
        }
    }

    /**
     * Open an app reliably using multiple methods
     */
    public static String openApp(android.content.Context ctx, String packageName) {
        // Method 1: Standard intent (requires Accessibility to be running)
        try {
            android.content.pm.PackageManager pm = ctx.getPackageManager();
            android.content.Intent intent = pm.getLaunchIntentForPackage(packageName);
            if (intent != null) {
                intent.addFlags(
                    android.content.Intent.FLAG_ACTIVITY_NEW_TASK |
                    android.content.Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED
                );
                ctx.startActivity(intent);
                Thread.sleep(800);
                return "opened " + packageName;
            }
        } catch (Exception e) {
            Log.w(TAG, "Method 1 failed: " + e.getMessage());
        }

        // Method 2: monkey via shell
        String result = exec("monkey -p " + packageName + " -c android.intent.category.LAUNCHER 1 2>&1");
        if (!result.contains("No activities") && !result.contains("error")) {
            return "opened " + packageName + " (shell)";
        }

        // Method 3: am start
        result = exec("am start -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p " + packageName + " 2>&1");
        if (result.contains("Starting")) return "opened " + packageName + " (am)";

        return "could not open " + packageName + " -- not installed or no launcher activity";
    }

    /**
     * Take screenshot via shell (works without accessibility)
     */
    public static String screenshot(String outputPath) {
        String result = exec("screencap -p " + outputPath);
        return result.isEmpty() || result.equals("(no output)")
            ? "screenshot saved to " + outputPath
            : "screenshot error: " + result;
    }

    /**
     * Dump UI hierarchy and return text nodes
     */
    public static String dumpUI() {
        String tmpFile = "/sdcard/kira_ui_dump.xml";
        exec("uiautomator dump " + tmpFile + " 2>&1");
        String xml = exec("cat " + tmpFile + " 2>&1");
        exec("rm -f " + tmpFile);
        if (xml.contains("<?xml")) {
            // Extract text content
            java.util.List<String> texts = new java.util.ArrayList<>();
            java.util.regex.Matcher m = java.util.regex.Pattern.compile("text=\"([^\"]+)\"").matcher(xml);
            while (m.find()) {
                String t = m.group(1);
                if (t != null && !t.isEmpty()) texts.add(t);
            }
            return texts.isEmpty() ? "no text found" : String.join("\n", texts.subList(0, Math.min(50, texts.size())));
        }
        return xml;
    }
}
