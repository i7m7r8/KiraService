package com.kira.service;

import android.app.Application;
import android.content.Intent;
import android.util.Log;
import java.io.PrintWriter;
import java.io.StringWriter;

public class KiraApp extends Application {

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

                // 1. Persist to SharedPrefs
                getSharedPreferences(PREFS_CRASH, MODE_PRIVATE).edit()
                    .putString(KEY_TRACE,  trace)
                    .putLong  (KEY_TS,     ts)
                    .putString(KEY_THREAD, thread.getName())
                    .commit();

                // 2. Launch standalone crash reporter (separate process)
                try {
                    Intent intent = new Intent(getApplicationContext(), CrashActivity.class);
                    intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK
                                  | Intent.FLAG_ACTIVITY_CLEAR_TASK);
                    intent.putExtra("trace",  trace);
                    intent.putExtra("ts",     ts);
                    intent.putExtra("thread", thread.getName());
                    startActivity(intent);
                } catch (Throwable ignored) {}

            } catch (Throwable t) {
                Log.e("KiraApp", "Crash handler failed: " + t);
            } finally {
                if (defaultHandler != null)
                    defaultHandler.uncaughtException(thread, throwable);
            }
        });
    }

    static String stackTrace(Throwable t) {
        StringWriter sw = new StringWriter(4096);
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
}
