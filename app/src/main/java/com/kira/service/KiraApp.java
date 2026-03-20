package com.kira.service;

import android.app.Application;
import android.content.Intent;
import android.util.Log;
import java.io.PrintWriter;
import java.io.StringWriter;

/**
 * Global Application class.
 * Installs an UncaughtExceptionHandler that:
 *   1. Writes the crash to Rust (/crash POST)
 *   2. Saves it to SharedPreferences for CrashActivity
 *   3. Launches CrashActivity instead of the system "App stopped" dialog
 */
public class KiraApp extends Application {

    private static final String TAG = "KiraApp";
    static final String PREFS_CRASH = "kira_crash";
    static final String KEY_TRACE   = "last_trace";
    static final String KEY_TS      = "last_crash_ts";
    static final String KEY_THREAD  = "last_crash_thread";

    @Override
    public void onCreate() {
        super.onCreate();
        installCrashHandler();
    }

    private void installCrashHandler() {
        Thread.UncaughtExceptionHandler defaultHandler =
            Thread.getDefaultUncaughtExceptionHandler();

        Thread.setDefaultUncaughtExceptionHandler((thread, throwable) -> {
            try {
                String trace = stackTrace(throwable);
                long   ts    = System.currentTimeMillis();

                // 1. Persist locally so CrashActivity can display without network
                getSharedPreferences(PREFS_CRASH, MODE_PRIVATE).edit()
                    .putString(KEY_TRACE,  trace)
                    .putLong  (KEY_TS,     ts)
                    .putString(KEY_THREAD, thread.getName())
                    .commit();

                // 2. Push to Rust (best-effort — may fail if Rust server not up)
                try {
                    pushCrashToRust(thread.getName(), trace, ts);
                } catch (Throwable ignored) {}

                // 3. Launch CrashActivity
                Intent intent = new Intent(getApplicationContext(), CrashActivity.class);
                intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                              | Intent.FLAG_ACTIVITY_CLEAR_TASK
                              | Intent.FLAG_ACTIVITY_CLEAR_TOP);
                intent.putExtra("trace",  trace);
                intent.putExtra("ts",     ts);
                intent.putExtra("thread", thread.getName());
                startActivity(intent);

            } catch (Throwable t) {
                Log.e(TAG, "Crash handler itself crashed: " + t);
            } finally {
                // Let the system default handler run too (writes to logcat)
                if (defaultHandler != null) {
                    defaultHandler.uncaughtException(thread, throwable);
                }
            }
        });
    }

    /** Flatten Throwable chain to a readable string */
    static String stackTrace(Throwable t) {
        StringWriter sw = new StringWriter(4096);
        t.printStackTrace(new PrintWriter(sw));
        // Include cause chain
        Throwable cause = t.getCause();
        int depth = 0;
        while (cause != null && depth++ < 5) {
            sw.write("\nCaused by: ");
            cause.printStackTrace(new PrintWriter(sw));
            cause = cause.getCause();
        }
        return sw.toString();
    }

    /** POST crash data to Rust /crash endpoint */
    static void pushCrashToRust(String thread, String trace, long ts) throws Exception {
        // Escape for JSON
        String safeTrace  = trace.replace("\\", "\\\\").replace("\"", "'")
            .replace("\r", "").replace("\n", "\\n").replace("\t", "  ");
        String safeThread = thread.replace("\"", "'");
        String body = "{\"thread\":\"" + safeThread
            + "\",\"trace\":\"" + safeTrace
            + "\",\"ts\":" + ts + "}";

        java.net.URL url = new java.net.URL("http://localhost:7070/crash");
        java.net.HttpURLConnection conn =
            (java.net.HttpURLConnection) url.openConnection();
        conn.setRequestMethod("POST");
        conn.setRequestProperty("Content-Type", "application/json");
        conn.setConnectTimeout(1000);
        conn.setReadTimeout(1000);
        conn.setDoOutput(true);
        byte[] data = body.getBytes("UTF-8");
        conn.getOutputStream().write(data);
        conn.getResponseCode(); // flush
        conn.disconnect();
    }
}
