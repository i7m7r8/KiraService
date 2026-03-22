package com.kira.service;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationManager;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.graphics.Typeface;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.*;

/**
 * CrashActivity — runs in :crash process (survives main app death).
 *
 * Features:
 *  - Shows instantly after any crash, even first launch crashes
 *  - Pulls crash data from: Intent extras → SharedPrefs → Rust /crash/log endpoint
 *  - Displays full colored stack trace (kira frames in Lavender, errors in Pink)
 *  - History tab: all stored crashes (up to 50, from Rust log)
 *  - Copy, Restart, Ask Kira to Fix, Share buttons
 *  - Clears notification on open
 */
public class CrashActivity extends Activity {

    // Catppuccin Mocha
    private static final int BG     = 0xFF11111B;  // Crust
    private static final int CARD   = 0xFF1E1E2E;  // Base
    private static final int HEADER = 0xFF181825;  // Mantle
    private static final int RED    = 0xFFF38BA8;  // Pink
    private static final int PEACH  = 0xFFFAB387;  // Peach
    private static final int LAV    = 0xFFB4BEFE;  // Lavender
    private static final int TEXT   = 0xFFCDD6F4;  // Text
    private static final int MUTED  = 0xFF6C7086;  // Overlay0
    private static final int GREEN  = 0xFFA6E3A1;  // Green
    private static final int YELLOW = 0xFFF9E2AF;  // Yellow
    private static final int SURFACE= 0xFF313244;  // Surface0

    private FrameLayout root;
    private LinearLayout currentTabView;
    private String currentTrace, currentThread, currentTime;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Clear the crash notification since we are now showing the crash
        clearCrashNotification();

        // Pull crash data: intent → SharedPrefs → fallback
        String trace  = getIntent().getStringExtra("trace");
        long   ts     = getIntent().getLongExtra("ts", 0);
        String thread = getIntent().getStringExtra("thread");
        String message= getIntent().getStringExtra("message");

        if (trace == null || trace.isEmpty()) {
            SharedPreferences p = getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE);
            trace   = p.getString(KiraApp.KEY_TRACE,  "(no trace stored)");
            ts      = p.getLong  (KiraApp.KEY_TS,     0);
            thread  = p.getString(KiraApp.KEY_THREAD, "unknown");
        }

        currentTrace  = trace  != null ? trace  : "(no trace)";
        currentThread = thread != null ? thread : "unknown";
        currentTime   = ts > 0
            ? new java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss",
                java.util.Locale.getDefault()).format(new java.util.Date(ts))
            : "unknown time";

        buildUI();
    }

    private void buildUI() {
        root = new FrameLayout(this);
        root.setBackgroundColor(BG);
        setContentView(root);

        // ── Tab bar: CRASH | HISTORY ────────────────────────────────────────
        LinearLayout tabs = new LinearLayout(this);
        tabs.setOrientation(LinearLayout.HORIZONTAL);
        tabs.setBackgroundColor(HEADER);
        FrameLayout.LayoutParams tabLp = new FrameLayout.LayoutParams(MATCH, dp(52));
        tabLp.gravity = Gravity.TOP;
        root.addView(tabs, tabLp);

        // Header title
        LinearLayout titleRow = new LinearLayout(this);
        titleRow.setOrientation(LinearLayout.VERTICAL);
        titleRow.setGravity(Gravity.CENTER_VERTICAL);
        titleRow.setPadding(dp(16), 0, 0, 0);
        titleRow.setLayoutParams(new LinearLayout.LayoutParams(0, MATCH, 1));

        TextView titleTv = new TextView(this);
        titleTv.setText("\uD83D\uDCA5  Kira Crashed");
        titleTv.setTextColor(RED);
        titleTv.setTextSize(14);
        titleTv.setTypeface(null, Typeface.BOLD);
        titleRow.addView(titleTv);

        TextView subtitleTv = new TextView(this);
        subtitleTv.setText(currentTime + "  \u00B7  " + currentThread);
        subtitleTv.setTextColor(MUTED);
        subtitleTv.setTextSize(10);
        subtitleTv.setTypeface(Typeface.MONOSPACE);
        titleRow.addView(subtitleTv);
        tabs.addView(titleRow);

        // Tab buttons
        TextView crashTab   = makeTabBtn("TRACE");
        TextView historyTab = makeTabBtn("HISTORY");
        crashTab.setTextColor(LAV);
        crashTab.setBackgroundColor(SURFACE);
        tabs.addView(crashTab);
        tabs.addView(historyTab);

        // ── Content area ─────────────────────────────────────────────────────
        FrameLayout.LayoutParams contentLp = new FrameLayout.LayoutParams(MATCH, MATCH);
        contentLp.topMargin = dp(52);
        contentLp.bottomMargin = dp(56);

        FrameLayout contentFrame = new FrameLayout(this);
        root.addView(contentFrame, contentLp);

        // ── Button bar ────────────────────────────────────────────────────────
        LinearLayout btnBar = new LinearLayout(this);
        btnBar.setOrientation(LinearLayout.HORIZONTAL);
        btnBar.setBackgroundColor(HEADER);
        btnBar.setPadding(dp(12), dp(8), dp(12), dp(8));
        btnBar.setGravity(Gravity.CENTER_VERTICAL);
        FrameLayout.LayoutParams btnBarLp = new FrameLayout.LayoutParams(MATCH, dp(56));
        btnBarLp.gravity = Gravity.BOTTOM;
        root.addView(btnBar, btnBarLp);

        addBarBtn(btnBar, "\uD83D\uDCCB Copy",    LAV,   () -> copyTrace());
        addBarBtn(btnBar, "\uD83D\uDD04 Restart", GREEN, () -> restart());
        addBarBtn(btnBar, "\uD83E\uDD16 Fix",     RED,   () -> askKira());
        addBarBtn(btnBar, "\uD83D\uDDD1 Clear",   PEACH, () -> clearAndClose());

        // Show crash trace by default
        showCrashTab(contentFrame);

        crashTab.setOnClickListener(v -> {
            crashTab.setTextColor(LAV); crashTab.setBackgroundColor(SURFACE);
            historyTab.setTextColor(MUTED); historyTab.setBackgroundColor(0x00000000);
            showCrashTab(contentFrame);
        });
        historyTab.setOnClickListener(v -> {
            historyTab.setTextColor(LAV); historyTab.setBackgroundColor(SURFACE);
            crashTab.setTextColor(MUTED); crashTab.setBackgroundColor(0x00000000);
            showHistoryTab(contentFrame);
        });
    }

    // ── Crash trace tab ───────────────────────────────────────────────────────

    private void showCrashTab(FrameLayout frame) {
        frame.removeAllViews();

        ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(BG);
        frame.addView(scroll, new FrameLayout.LayoutParams(MATCH, MATCH));

        LinearLayout col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(dp(12), dp(12), dp(12), dp(12));
        scroll.addView(col, new ScrollView.LayoutParams(MATCH, WRAP));

        // Error summary card
        LinearLayout summaryCard = new LinearLayout(this);
        summaryCard.setOrientation(LinearLayout.VERTICAL);
        summaryCard.setBackgroundColor(0xFF2E1A1F); // dark pink tint
        summaryCard.setPadding(dp(14), dp(12), dp(14), dp(12));
        lp(summaryCard, 0, dp(12));
        col.addView(summaryCard);

        // First line of trace (the exception type + message)
        String firstLine = currentTrace.contains("\n")
            ? currentTrace.substring(0, currentTrace.indexOf("\n"))
            : currentTrace;
        if (firstLine.length() > 120) firstLine = firstLine.substring(0, 120) + "\u2026";

        addText(summaryCard, "\u2715 Exception", 10, RED, Typeface.BOLD, Gravity.LEFT, 0, dp(4));
        addText(summaryCard, firstLine, 11, YELLOW, Typeface.NORMAL, Gravity.LEFT, 0, 0);
        addText(summaryCard, "Thread: " + currentThread, 10, MUTED, Typeface.MONOSPACE, Gravity.LEFT, dp(6), 0);

        // Full stack trace card
        LinearLayout traceCard = new LinearLayout(this);
        traceCard.setOrientation(LinearLayout.VERTICAL);
        traceCard.setBackgroundColor(CARD);
        traceCard.setPadding(dp(10), dp(10), dp(10), dp(10));
        lp(traceCard, 0, 0);
        col.addView(traceCard);

        addText(traceCard, "STACK TRACE", 9, PEACH, Typeface.BOLD, Gravity.LEFT, 0, dp(8));

        HorizontalScrollView hs = new HorizontalScrollView(this);
        hs.setLayoutParams(new LinearLayout.LayoutParams(MATCH, WRAP));
        TextView traceView = new TextView(this);
        traceView.setText(colorTrace(currentTrace));
        traceView.setTextSize(9);
        traceView.setTypeface(Typeface.MONOSPACE);
        traceView.setLineSpacing(dp(1), 1f);
        traceView.setTextIsSelectable(true);
        traceView.setPadding(0, 0, dp(20), 0);
        hs.addView(traceView);
        traceCard.addView(hs);
    }

    // ── History tab ───────────────────────────────────────────────────────────

    private void showHistoryTab(FrameLayout frame) {
        frame.removeAllViews();

        ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(BG);
        frame.addView(scroll, new FrameLayout.LayoutParams(MATCH, MATCH));

        LinearLayout col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(dp(12), dp(12), dp(12), dp(12));
        scroll.addView(col, new ScrollView.LayoutParams(MATCH, WRAP));

        addText(col, "CRASH HISTORY", 10, PEACH, Typeface.BOLD, Gravity.LEFT, 0, dp(12));

        // Load from SharedPrefs (always available, even if Rust is dead)
        SharedPreferences p = getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE);
        String lastTrace  = p.getString(KiraApp.KEY_TRACE, null);
        long   lastTs     = p.getLong(KiraApp.KEY_TS, 0);
        String lastThread = p.getString(KiraApp.KEY_THREAD, "unknown");

        if (lastTrace != null && lastTs > 0) {
            String time = new java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss",
                java.util.Locale.getDefault()).format(new java.util.Date(lastTs));
            addCrashCard(col, lastTrace, time, lastThread);
        }

        // Try Rust /crash/log endpoint (non-blocking)
        TextView loadingTv = new TextView(this);
        loadingTv.setText("Loading Rust crash log\u2026");
        loadingTv.setTextColor(MUTED);
        loadingTv.setTextSize(11);
        loadingTv.setTypeface(Typeface.MONOSPACE);
        lp(loadingTv, dp(8), 0);
        col.addView(loadingTv);

        new Thread(() -> {
            try {
                okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(2, java.util.concurrent.TimeUnit.SECONDS)
                    .readTimeout(3, java.util.concurrent.TimeUnit.SECONDS).build();
                okhttp3.Response resp = client.newCall(
                    new okhttp3.Request.Builder()
                        .url("http://localhost:7070/crash/log")
                        .build()).execute();
                if (resp.body() == null) throw new Exception("empty body");
                String json = resp.body().string();

                new Handler(Looper.getMainLooper()).post(() -> {
                    col.removeView(loadingTv);
                    try {
                        org.json.JSONObject root = new org.json.JSONObject(json);
                        org.json.JSONArray crashes = root.getJSONArray("crashes");
                        if (crashes.length() == 0) {
                            addText(col, "No crashes in Rust log \u2713", 11, GREEN,
                                Typeface.MONOSPACE, Gravity.LEFT, 0, 0);
                            return;
                        }
                        addText(col, crashes.length() + " entries in Rust log", 10,
                            MUTED, Typeface.MONOSPACE, Gravity.LEFT, 0, dp(8));
                        // Show newest first
                        for (int i = crashes.length()-1; i >= 0; i--) {
                            org.json.JSONObject c = crashes.getJSONObject(i);
                            String ts2    = c.optString("ts_str", "");
                            String thread2= c.optString("thread", "");
                            String trace2 = c.optString("trace",  c.optString("message",""));
                            addCrashCard(col, trace2, ts2, thread2);
                        }
                    } catch (Exception e) {
                        addText(col, "Parse error: " + e.getMessage(), 10, RED,
                            Typeface.MONOSPACE, Gravity.LEFT, 0, 0);
                    }
                });
            } catch (Throwable e) {
                new Handler(Looper.getMainLooper()).post(() -> {
                    col.removeView(loadingTv);
                    addText(col, "Rust engine offline (crash log unavailable)", 10, MUTED,
                        Typeface.MONOSPACE, Gravity.LEFT, 0, 0);
                });
            }
        }).start();
    }

    private void addCrashCard(LinearLayout parent, String trace, String time, String thread) {
        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setBackgroundColor(CARD);
        card.setPadding(dp(12), dp(10), dp(12), dp(10));
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(MATCH, WRAP);
        cp.setMargins(0, 0, 0, dp(10));
        card.setLayoutParams(cp);
        card.setClickable(true);
        card.setFocusable(true);

        // Header row
        LinearLayout hrow = new LinearLayout(this);
        hrow.setOrientation(LinearLayout.HORIZONTAL);
        hrow.setGravity(Gravity.CENTER_VERTICAL);

        TextView timeTv = new TextView(this);
        timeTv.setText(time);
        timeTv.setTextColor(YELLOW);
        timeTv.setTextSize(10);
        timeTv.setTypeface(Typeface.MONOSPACE);
        timeTv.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));
        hrow.addView(timeTv);

        TextView threadTv = new TextView(this);
        threadTv.setText(thread);
        threadTv.setTextColor(MUTED);
        threadTv.setTextSize(9);
        threadTv.setTypeface(Typeface.MONOSPACE);
        hrow.addView(threadTv);
        card.addView(hrow);

        // First error line
        String first = trace.contains("\n") ? trace.substring(0, trace.indexOf("\n")) : trace;
        if (first.length() > 100) first = first.substring(0, 100) + "\u2026";
        addText(card, first, 10, RED, Typeface.MONOSPACE, Gravity.LEFT, dp(6), dp(4));

        // Expand on tap
        final boolean[] expanded = {false};
        final String finalTrace = trace;
        card.setOnClickListener(v -> {
            expanded[0] = !expanded[0];
            if (expanded[0]) {
                HorizontalScrollView hs = new HorizontalScrollView(this);
                hs.setTag("expanded");
                TextView tv = new TextView(this);
                tv.setText(colorTrace(finalTrace));
                tv.setTextSize(9);
                tv.setTypeface(Typeface.MONOSPACE);
                tv.setLineSpacing(dp(1), 1f);
                tv.setTextIsSelectable(true);
                hs.addView(tv);
                card.addView(hs);
            } else {
                View ex = card.findViewWithTag("expanded");
                if (ex != null) card.removeView(ex);
            }
        });

        parent.addView(card);
    }

    // ── Button actions ────────────────────────────────────────────────────────

    private void copyTrace() {
        ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("crash", currentTrace));
        Toast.makeText(this, "Stack trace copied", Toast.LENGTH_SHORT).show();
    }

    private void restart() {
        Intent i = getPackageManager().getLaunchIntentForPackage(getPackageName());
        if (i != null) {
            i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
            startActivity(i);
        }
        finishAndRemoveTask();
    }

    private void askKira() {
        String prompt = "Kira crashed. Please analyze this stack trace and explain what went wrong and how to fix it:\n\n"
            + currentTrace.substring(0, Math.min(1200, currentTrace.length()));
        ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("crash_prompt", prompt));
        Intent i = new Intent(this, MainActivity.class);
        i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
        i.putExtra("crash_prompt", prompt);
        startActivity(i);
        finishAndRemoveTask();
    }

    private void clearAndClose() {
        getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE).edit().clear().apply();
        // Also clear Rust log
        new Thread(() -> {
            try {
                new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder()
                    .url("http://localhost:7070/crash/clear")
                    .post(okhttp3.RequestBody.create("{}", okhttp3.MediaType.parse("application/json")))
                    .build()).execute();
            } catch (Throwable ignored) {}
        }).start();
        Toast.makeText(this, "Crash log cleared", Toast.LENGTH_SHORT).show();
        finishAndRemoveTask();
    }

    private void clearCrashNotification() {
        try {
            NotificationManager nm =
                (NotificationManager) getSystemService(Context.NOTIFICATION_SERVICE);
            if (nm != null) nm.cancel(0xCA5E);
        } catch (Throwable ignored) {}
    }

    // ── Colored trace ─────────────────────────────────────────────────────────

    private android.text.SpannableString colorTrace(String trace) {
        android.text.SpannableStringBuilder sb =
            new android.text.SpannableStringBuilder(trace);
        String[] lines = trace.split("\n");
        int pos = 0;
        for (String line : lines) {
            if (pos >= sb.length()) break;
            int end = Math.min(pos + line.length(), sb.length());
            int color = TEXT;
            if (line.contains("com.kira.service"))                                   color = LAV;
            else if (line.startsWith("Caused by") || line.contains("Exception")
                  || line.contains("Error:") || line.contains("FATAL"))              color = RED;
            else if (line.contains("NullPointer") || line.contains("ClassCast"))     color = YELLOW;
            else if (line.startsWith("\tat "))                                       color = MUTED;
            sb.setSpan(new android.text.style.ForegroundColorSpan(color),
                pos, end, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
            pos = end + 1;
        }
        return new android.text.SpannableString(sb);
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    private TextView makeTabBtn(String label) {
        TextView tv = new TextView(this);
        tv.setText(label);
        tv.setTextSize(11);
        tv.setTypeface(Typeface.MONOSPACE, Typeface.BOLD);
        tv.setTextColor(MUTED);
        tv.setGravity(Gravity.CENTER);
        tv.setPadding(dp(16), 0, dp(16), 0);
        tv.setLayoutParams(new LinearLayout.LayoutParams(WRAP, MATCH));
        tv.setClickable(true);
        tv.setFocusable(true);
        return tv;
    }

    private void addBarBtn(LinearLayout bar, String label, int color, Runnable action) {
        TextView btn = new TextView(this);
        btn.setText(label);
        btn.setTextSize(11);
        btn.setTypeface(null, Typeface.BOLD);
        btn.setTextColor(0xFF1E1E2E);
        btn.setGravity(Gravity.CENTER);
        btn.setPadding(dp(10), dp(6), dp(10), dp(6));
        btn.setBackgroundColor(color);
        btn.setClickable(true);
        btn.setFocusable(true);
        android.graphics.drawable.GradientDrawable bg =
            new android.graphics.drawable.GradientDrawable();
        bg.setColor(color);
        bg.setCornerRadius(dp(8));
        btn.setBackground(bg);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(0, WRAP, 1);
        lp.setMargins(dp(3), 0, dp(3), 0);
        btn.setLayoutParams(lp);
        btn.setOnClickListener(v -> action.run());
        bar.addView(btn);
    }

    private void addText(ViewGroup parent, String text, int sp, int color,
                         Typeface typeface, int gravity, int topMargin, int bottomMargin) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextSize(sp);
        tv.setTextColor(color);
        if (typeface == Typeface.MONOSPACE) {
            tv.setTypeface(Typeface.MONOSPACE);
        } else if (typeface != null) {
            tv.setTypeface(typeface);
        }
        tv.setGravity(gravity);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(0, topMargin, 0, bottomMargin);
        tv.setLayoutParams(lp);
        parent.addView(tv);
    }

    private void lp(View v, int top, int bottom) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(MATCH, WRAP);
        p.setMargins(0, top, 0, bottom);
        v.setLayoutParams(p);
    }

    private static final int MATCH = ViewGroup.LayoutParams.MATCH_PARENT;
    private static final int WRAP  = ViewGroup.LayoutParams.WRAP_CONTENT;

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
