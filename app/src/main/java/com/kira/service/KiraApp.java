package com.kira.service;

import android.app.AlarmManager;
import android.app.Application;
import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.SystemClock;
import android.util.Log;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.text.SimpleDateFormat;
import java.util.Date;
import java.util.Locale;

public class KiraApp extends Application {

    static final String PREFS_CRASH   = "kira_crash";
    static final String KEY_TRACE     = "last_trace";
    static final String KEY_TS        = "last_crash_ts";
    static final String KEY_THREAD    = "last_crash_thread";
    static final String CHANNEL_CRASH = "kira_crash";
    static final String CHANNEL_OTA   = "kira_ota";

    private static final String TAG = "KiraApp";

    @Override
    public void onCreate() {
        super.onCreate();
        createChannels();
        installCrashHandler();
    }

    private void createChannels() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return;
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (nm == null) return;

        // Crash channel — HIGH importance so head-up notification appears
        NotificationChannel crash = new NotificationChannel(
            CHANNEL_CRASH, "Kira Crash Reporter", NotificationManager.IMPORTANCE_HIGH);
        crash.setDescription("Shows when Kira crashes");
        crash.enableVibration(true);
        nm.createNotificationChannel(crash);

        // OTA channel
        NotificationChannel ota = new NotificationChannel(
            CHANNEL_OTA, "Kira Updates", NotificationManager.IMPORTANCE_DEFAULT);
        ota.setDescription("Update notifications");
        nm.createNotificationChannel(ota);
    }

    private void installCrashHandler() {
        Thread.UncaughtExceptionHandler defaultHandler =
            Thread.getDefaultUncaughtExceptionHandler();

        Thread.setDefaultUncaughtExceptionHandler((thread, throwable) -> {
            try {
                String trace   = stackTrace(throwable);
                long   ts      = System.currentTimeMillis();
                String message = throwable.toString();
                String tname   = thread.getName();

                // 1. SharedPrefs — synchronous commit before anything else
                getSharedPreferences(PREFS_CRASH, MODE_PRIVATE).edit()
                    .putString(KEY_TRACE,  trace)
                    .putLong  (KEY_TS,     ts)
                    .putString(KEY_THREAD, tname)
                    .commit();

                // 2. Rust JNI — best effort
                try { RustBridge.logCrash(tname, message, trace, ts); }
                catch (Throwable ignored) {}

                // 3. Show crash notification (works from any context, survives process death)
                showCrashNotification(trace, ts, tname, message);

                // 4. Schedule CrashActivity via AlarmManager — fires 600ms after we die.
                //    This is the ONLY reliable way to start an activity after process death.
                scheduleCrashActivity(trace, ts, tname, message);

                // 5. Brief pause so notification posts before process dies
                try { Thread.sleep(500); } catch (Throwable ignored) {}

            } catch (Throwable t) {
                Log.e(TAG, "Crash handler error: " + t);
            } finally {
                if (defaultHandler != null)
                    defaultHandler.uncaughtException(thread, throwable);
            }
        });
    }

    /**
     * Schedule CrashActivity via AlarmManager.
     * AlarmManager fires even after the process that scheduled it dies —
     * this is the standard pattern for reliable post-crash UI.
     */
    private void scheduleCrashActivity(String trace, long ts, String thread, String message) {
        try {
            Intent intent = new Intent(getApplicationContext(), CrashActivity.class);
            intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            intent.putExtra("trace",   trace.length() > 8000 ? trace.substring(0, 8000) : trace);
            intent.putExtra("ts",      ts);
            intent.putExtra("thread",  thread);
            intent.putExtra("message", message);

            PendingIntent pi = PendingIntent.getActivity(
                getApplicationContext(), 0xDEAD, intent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

            AlarmManager am = (AlarmManager) getSystemService(ALARM_SERVICE);
            if (am != null) {
                // Fire 600ms from now — process will be dead by then, alarm still fires
                am.setExact(AlarmManager.ELAPSED_REALTIME_WAKEUP,
                    SystemClock.elapsedRealtime() + 600, pi);
            }
        } catch (Throwable t) {
            Log.e(TAG, "scheduleCrashActivity failed: " + t);
        }
    }

    private void showCrashNotification(String trace, long ts, String thread, String message) {
        try {
            Context ctx = getApplicationContext();

            Intent openIntent = new Intent(ctx, CrashActivity.class);
            openIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
            openIntent.putExtra("trace",   trace.length() > 8000 ? trace.substring(0, 8000) : trace);
            openIntent.putExtra("ts",      ts);
            openIntent.putExtra("thread",  thread);
            openIntent.putExtra("message", message);

            PendingIntent pi = PendingIntent.getActivity(ctx, 0xCA5E, openIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);

            String shortMsg = message.length() > 100 ? message.substring(0, 100) + "…" : message;

            Notification.Builder nb = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
                ? new Notification.Builder(ctx, CHANNEL_CRASH)
                : new Notification.Builder(ctx);

            // Do NOT set PRIORITY_MAX — channel importance handles priority on API 26+
            nb.setSmallIcon(android.R.drawable.ic_dialog_alert)
              .setContentTitle("💀 Kira crashed — tap to view")
              .setContentText(shortMsg)
              .setStyle(new Notification.BigTextStyle()
                  .bigText("Thread: " + thread + "\n" + shortMsg))
              .setContentIntent(pi)
              .setAutoCancel(true)
              .setOngoing(false)
              .setColor(0xFFF38BA8);

            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
                nb.setPriority(Notification.PRIORITY_MAX);
            }

            NotificationManager nm =
                (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm != null) nm.notify(0xCA5E, nb.build());
        } catch (Throwable t) {
            Log.e(TAG, "showCrashNotification failed: " + t);
        }
    }

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

    static String esc(String s) {
        if (s == null) return "";
        if (s.length() > 4096) s = s.substring(0, 4096) + "...(truncated)";
        return s.replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("\n", "\\n")
                .replace("\r", "\\r")
                .replace("\t", "\\t");
    }
}
