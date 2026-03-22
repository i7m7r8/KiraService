package com.kira.service;

import android.app.Application;
import com.kira.service.RustBridge;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.util.Log;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

/**
 * KiraApp — Application entry point.
 *
 * Crash handler system:
 *  1. Catches ALL uncaught exceptions (main + background threads)
 *  2. Persists full trace to SharedPrefs (survives process death)
 *  3. Posts to Rust /crash endpoint for in-memory log (up to 50 entries)
 *  4. Posts a "Kira crashed" notification — tap opens CrashActivity
 *  5. Starts CrashActivity in :crash process (separate from dead main process)
 *  6. Falls through to default handler so system gets the death signal
 */
public class KiraApp extends Application {

    static final String PREFS_CRASH   = "kira_crash";
    static final String KEY_TRACE     = "last_trace";
    static final String KEY_TS        = "last_crash_ts";
    static final String KEY_THREAD    = "last_crash_thread";
    static final String CHANNEL_CRASH = "kira_crash";

    private static final String TAG = "KiraApp";

    @Override
    public void onCreate() {
        super.onCreate();
        createCrashChannel();
        installCrashHandler();
    }

    // ── Notification channel ──────────────────────────────────────────────────
    private void createCrashChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel ch = new NotificationChannel(
                CHANNEL_CRASH, "Kira Crash Reporter",
                NotificationManager.IMPORTANCE_HIGH);
            ch.setDescription("Shows when Kira crashes with full stack trace");
            ch.enableVibration(true);
            NotificationManager nm =
                (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm != null) nm.createNotificationChannel(ch);
        }
    }

    // ── Crash handler ─────────────────────────────────────────────────────────
    private void installCrashHandler() {
        Thread.UncaughtExceptionHandler defaultHandler =
            Thread.getDefaultUncaughtExceptionHandler();

        Thread.setDefaultUncaughtExceptionHandler((thread, throwable) -> {
            try {
                String trace  = stackTrace(throwable);
                long   ts     = System.currentTimeMillis();
                String tsStr  = new SimpleDateFormat("yyyy-MM-dd HH:mm:ss",
                    Locale.getDefault()).format(new Date(ts));
                String message = throwable.toString();
                String tname   = thread.getName();

                // 1. Persist to SharedPrefs (survives process death, readable next launch)
                getSharedPreferences(PREFS_CRASH, MODE_PRIVATE).edit()
                    .putString(KEY_TRACE,  trace)
                    .putLong  (KEY_TS,     ts)
                    .putString(KEY_THREAD, tname)
                    .commit(); // commit() not apply() — must be synchronous before process dies

                // 1b. Log to Rust JNI directly (synchronous, no HTTP, fastest path)
                try {
                    RustBridge.logCrash(tname, message, trace, ts);
                } catch (Throwable rustDied) {
                    // Rust may have died too — that's fine, SharedPrefs has the data
                    Log.w(TAG, "Rust JNI unavailable during crash: " + rustDied.getMessage());
                }

                // 2. Post to Rust HTTP endpoint (async — backup, ensures /crash/log works)
                new Thread(() -> {
                    try {
                        String body = "{" +
                            "\"thread\":\"" + esc(tname)    + "\"," +
                            "\"message\":\"" + esc(message) + "\"," +
                            "\"trace\":\"" + esc(trace)    + "\"," +
                            "\"ts\":" + ts +
                        "}";
                        okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                            .connectTimeout(1, java.util.concurrent.TimeUnit.SECONDS)
                            .readTimeout(2, java.util.concurrent.TimeUnit.SECONDS)
                            .build();
                        client.newCall(new okhttp3.Request.Builder()
                            .url("http://localhost:7070/crash")
                            .post(okhttp3.RequestBody.create(
                                body, okhttp3.MediaType.parse("application/json")))
                            .build()).execute();
                    } catch (Throwable ignored) {}
                }).start();

                // 3. Post crash notification (works even from background / :crash process)
                postCrashNotification(trace, ts, tname, message);

                // 4. Launch CrashActivity in :crash process
                try {
                    Intent intent = new Intent(getApplicationContext(), CrashActivity.class);
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                                  | Intent.FLAG_ACTIVITY_CLEAR_TASK
                                  | Intent.FLAG_ACTIVITY_EXCLUDE_FROM_RECENTS);
                    intent.putExtra("trace",   trace);
                    intent.putExtra("ts",      ts);
                    intent.putExtra("thread",  tname);
                    intent.putExtra("message", message);
                    getApplicationContext().startActivity(intent);
                } catch (Throwable t) {
                    Log.e(TAG, "Could not start CrashActivity: " + t);
                }

                // Small delay to let CrashActivity start before process dies
                try { Thread.sleep(400); } catch (Throwable ignored) {}

            } catch (Throwable t) {
                Log.e(TAG, "Crash handler failed: " + t);
            } finally {
                // Always call default handler — system needs to know process died
                if (defaultHandler != null)
                    defaultHandler.uncaughtException(thread, throwable);
            }
        });
    }

    private void postCrashNotification(String trace, long ts, String thread, String message) {
        try {
            Context ctx = getApplicationContext();

            // Intent to open CrashActivity from notification tap
            Intent openIntent = new Intent(ctx, CrashActivity.class);
            openIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                              | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            openIntent.putExtra("trace",   trace);
            openIntent.putExtra("ts",      ts);
            openIntent.putExtra("thread",  thread);
            openIntent.putExtra("message", message);

            PendingIntent pi = PendingIntent.getActivity(ctx, 0, openIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

            String shortMsg = message.length() > 120 ? message.substring(0, 120) + "…" : message;

            Notification.Builder nb;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                nb = new Notification.Builder(ctx, CHANNEL_CRASH);
            } else {
                nb = new Notification.Builder(ctx);
            }

            nb.setSmallIcon(android.R.drawable.ic_dialog_alert)
              .setContentTitle("💀  Kira crashed")
              .setContentText(shortMsg)
              .setStyle(new Notification.BigTextStyle()
                  .bigText("Thread: " + thread + "\n" + shortMsg))
              .setContentIntent(pi)
              .setAutoCancel(true)
              .setPriority(Notification.PRIORITY_MAX)
              .setColor(0xFFF38BA8);  // Catppuccin Pink

            NotificationManager nm =
                (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm != null) {
                nm.notify(0xCA5E, nb.build()); // 0xCA5E = "CASE" in hex, crash notification ID
            }
        } catch (Throwable t) {
            Log.e(TAG, "postCrashNotification failed: " + t);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    static String stackTrace(Throwable t) {
        StringWriter sw = new StringWriter(8192);
        t.printStackTrace(new PrintWriter(sw));
        Throwable cause = t.getCause();
        int depth = 0;
        while (cause != null && depth++ < 5) {
            sw.write("\nCaused by: ");
            cause.printStackTrace(new PrintWriter(sw));
            cause = cause.getCause();
        }
        return sw.toString();
    }

    /** Escape a string for embedding in JSON */
    private static String esc(String s) {
        if (s == null) return "";
        // Cap at 4KB to avoid oversized payloads
        if (s.length() > 4096) s = s.substring(0, 4096) + "... (truncated)";
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }
}
