package com.kira.service;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.graphics.Typeface;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

/**
 * CrashActivity — shown instead of system "App stopped" dialog.
 *
 * Displays:
 *   - Crash timestamp + thread name
 *   - Full stack trace (monospace, scrollable)
 *   - Copy / Restart / Report buttons
 *   - Polls Rust /crash/log to show crash history
 */
public class CrashActivity extends Activity {

    private static final int BG      = 0xFF0D0D14;
    private static final int CARD    = 0xFF181825;
    private static final int RED     = 0xFFF38BA8;   // Catppuccin Pink
    private static final int PEACH   = 0xFFFAB387;
    private static final int LAV     = 0xFFB4BEFE;
    private static final int TEXT    = 0xFFCDD6F4;
    private static final int MUTED   = 0xFF6C7086;
    private static final int GREEN   = 0xFFA6E3A1;
    private static final int SURFACE = 0xFF313244;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Get crash data from intent or SharedPreferences fallback
        String trace  = getIntent().getStringExtra("trace");
        long   ts     = getIntent().getLongExtra("ts", 0);
        String thread = getIntent().getStringExtra("thread");

        if (trace == null || trace.isEmpty()) {
            // Fallback: read from prefs (e.g. second launch after crash)
            android.content.SharedPreferences p =
                getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE);
            trace  = p.getString(KiraApp.KEY_TRACE,  "No trace available");
            ts     = p.getLong  (KiraApp.KEY_TS,     0);
            thread = p.getString(KiraApp.KEY_THREAD, "unknown");
        }

        final String finalTrace  = trace;
        final long   finalTs     = ts;
        final String finalThread = thread;

        // ── Build UI ─────────────────────────────────────────────────────────
        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(BG);
        setContentView(root);

        ScrollView scroll = new ScrollView(this);
        scroll.setLayoutParams(new FrameLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT));
        scroll.setVerticalScrollBarEnabled(false);
        root.addView(scroll);

        LinearLayout col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(dp(16), dp(48), dp(16), dp(32));
        scroll.addView(col, new ScrollView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT));

        // ── Header ────────────────────────────────────────────────────────────
        TextView skull = new TextView(this);
        skull.setText("💀");
        skull.setTextSize(48);
        skull.setGravity(Gravity.CENTER);
        skull.setPadding(0, 0, 0, dp(8));
        col.addView(skull, matchWrap());

        TextView title = new TextView(this);
        title.setText("Kira crashed");
        title.setTextColor(RED);
        title.setTextSize(22);
        title.setTypeface(null, Typeface.BOLD);
        title.setGravity(Gravity.CENTER);
        col.addView(title, matchWrap());

        // Timestamp + thread
        String timeStr = ts > 0
            ? new java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss",
                java.util.Locale.getDefault()).format(new java.util.Date(ts))
            : "unknown time";
        TextView meta = new TextView(this);
        meta.setText("Thread: " + (thread != null ? thread : "?") + "  ·  " + timeStr);
        meta.setTextColor(MUTED);
        meta.setTextSize(11);
        meta.setTypeface(Typeface.MONOSPACE);
        meta.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams metaLp = matchWrap();
        metaLp.setMargins(0, dp(4), 0, dp(16));
        meta.setLayoutParams(metaLp);
        col.addView(meta);

        // ── Stack trace card ─────────────────────────────────────────────────
        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setBackgroundColor(CARD);
        card.setPadding(dp(12), dp(12), dp(12), dp(12));
        LinearLayout.LayoutParams cardLp = matchWrap();
        cardLp.setMargins(0, 0, 0, dp(16));
        card.setLayoutParams(cardLp);

        TextView cardHeader = new TextView(this);
        cardHeader.setText("STACK TRACE");
        cardHeader.setTextColor(PEACH);
        cardHeader.setTextSize(10);
        cardHeader.setTypeface(Typeface.MONOSPACE);
        cardHeader.setPadding(0, 0, 0, dp(8));
        card.addView(cardHeader, matchWrap());

        HorizontalScrollView hScroll = new HorizontalScrollView(this);
        hScroll.setHorizontalScrollBarEnabled(true);
        hScroll.setLayoutParams(new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT));

        TextView traceView = new TextView(this);
        traceView.setText(finalTrace);
        traceView.setTextColor(0xFFCDD6F4);
        traceView.setTextSize(10);
        traceView.setTypeface(Typeface.MONOSPACE);
        traceView.setLineSpacing(dp(2), 1f);
        traceView.setTextIsSelectable(true);
        // Colour-code key lines
        colourTrace(traceView, finalTrace);
        hScroll.addView(traceView);
        card.addView(hScroll);
        col.addView(card);

        // ── Rust crash log ────────────────────────────────────────────────────
        TextView historyHeader = new TextView(this);
        historyHeader.setText("CRASH HISTORY  (from Rust)");
        historyHeader.setTextColor(PEACH);
        historyHeader.setTextSize(10);
        historyHeader.setTypeface(Typeface.MONOSPACE);
        LinearLayout.LayoutParams hhLp = matchWrap();
        hhLp.setMargins(0, 0, 0, dp(4));
        historyHeader.setLayoutParams(hhLp);
        col.addView(historyHeader);

        TextView historyView = new TextView(this);
        historyView.setText("loading…");
        historyView.setTextColor(MUTED);
        historyView.setTextSize(10);
        historyView.setTypeface(Typeface.MONOSPACE);
        historyView.setBackgroundColor(CARD);
        historyView.setPadding(dp(12), dp(10), dp(12), dp(10));
        LinearLayout.LayoutParams hvLp = matchWrap();
        hvLp.setMargins(0, 0, 0, dp(20));
        historyView.setLayoutParams(hvLp);
        col.addView(historyView);

        // Load crash history from Rust in background
        new Thread(() -> {
            try {
                okhttp3.OkHttpClient cl = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(2, java.util.concurrent.TimeUnit.SECONDS)
                    .readTimeout(3, java.util.concurrent.TimeUnit.SECONDS).build();
                okhttp3.Response r = cl.newCall(
                    new okhttp3.Request.Builder()
                        .url("http://localhost:7070/crash/log").get().build()).execute();
                if (r.body() != null) {
                    String json = r.body().string();
                    runOnUiThread(() -> {
                        historyView.setText(formatCrashLog(json));
                        historyView.setTextColor(TEXT);
                    });
                }
            } catch (Exception e) {
                runOnUiThread(() -> historyView.setText("Rust server not running\n(" + e.getMessage() + ")"));
            }
        }).start();

        // ── Action buttons ────────────────────────────────────────────────────
        LinearLayout btnRow = new LinearLayout(this);
        btnRow.setOrientation(LinearLayout.HORIZONTAL);
        btnRow.setGravity(Gravity.CENTER);
        btnRow.setPadding(0, 0, 0, dp(16));
        col.addView(btnRow, matchWrap());

        // Copy button
        View copyBtn = makeBtn("📋 Copy", LAV);
        copyBtn.setOnClickListener(v -> {
            ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(
                ClipData.newPlainText("crash", finalTrace));
            Toast.makeText(this, "Copied to clipboard", Toast.LENGTH_SHORT).show();
        });
        btnRow.addView(copyBtn);

        View space1 = new View(this);
        btnRow.addView(space1, new LinearLayout.LayoutParams(dp(12), 1));

        // Restart button
        View restartBtn = makeBtn("🔄 Restart", GREEN);
        restartBtn.setOnClickListener(v -> {
            Intent i = getPackageManager().getLaunchIntentForPackage(getPackageName());
            if (i != null) {
                i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(i);
            }
            finishAndRemoveTask();
        });
        btnRow.addView(restartBtn);

        View space2 = new View(this);
        btnRow.addView(space2, new LinearLayout.LayoutParams(dp(12), 1));

        // Clear crashes button
        View clearBtn = makeBtn("🗑 Clear", PEACH);
        clearBtn.setOnClickListener(v -> {
            getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE).edit().clear().apply();
            new Thread(() -> {
                try {
                    new okhttp3.OkHttpClient().newCall(new okhttp3.Request.Builder()
                        .url("http://localhost:7070/crash/clear")
                        .post(okhttp3.RequestBody.create(new byte[0], null)).build()).execute();
                } catch (Exception ignored) {}
            }).start();
            historyView.setText("cleared");
        });
        btnRow.addView(clearBtn);

        // ── Send to Kira button ───────────────────────────────────────────────
        View reportBtn = makeBtn("🤖 Ask Kira to fix this", RED);
        LinearLayout.LayoutParams rbLp = matchWrap();
        rbLp.setMargins(0, dp(8), 0, 0);
        reportBtn.setLayoutParams(rbLp);
        reportBtn.setOnClickListener(v -> {
            // Put crash context in clipboard then open MainActivity with it
            String prompt = "I crashed. Fix this:\n" + finalTrace.substring(
                0, Math.min(800, finalTrace.length()));
            ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("crash_prompt", prompt));
            Intent i = new Intent(this, MainActivity.class);
            i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
            i.putExtra("crash_prompt", prompt);
            startActivity(i);
            finishAndRemoveTask();
        });
        col.addView(reportBtn);
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    private void colourTrace(TextView tv, String trace) {
        if (trace == null) return;
        android.text.SpannableStringBuilder sb = new android.text.SpannableStringBuilder(trace);
        // Highlight "com.kira" lines in Lavender, "Caused by" in Red
        String[] lines = trace.split("\n");
        int pos = 0;
        for (String line : lines) {
            int end = pos + line.length();
            if (line.contains("com.kira.service")) {
                sb.setSpan(new android.text.style.ForegroundColorSpan(LAV),
                    pos, end, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
            } else if (line.startsWith("Caused by") || line.startsWith("Exception")
                    || line.contains("Error")) {
                sb.setSpan(new android.text.style.ForegroundColorSpan(RED),
                    pos, end, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
            } else if (line.startsWith("\tat ")) {
                sb.setSpan(new android.text.style.ForegroundColorSpan(MUTED),
                    pos, end, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
            }
            pos = end + 1;
        }
        tv.setText(sb);
    }

    private String formatCrashLog(String json) {
        // Parse array of crash objects from Rust
        try {
            org.json.JSONArray arr = new org.json.JSONArray(json);
            if (arr.length() == 0) return "No previous crashes recorded.";
            StringBuilder sb = new StringBuilder();
            for (int i = arr.length() - 1; i >= 0; i--) {
                org.json.JSONObject o = arr.getJSONObject(i);
                long ts = o.optLong("ts", 0);
                String thr = o.optString("thread", "?");
                String msg = o.optString("message", "");
                String dt = ts > 0
                    ? new java.text.SimpleDateFormat("MM-dd HH:mm:ss",
                        java.util.Locale.getDefault()).format(new java.util.Date(ts))
                    : "?";
                sb.append("▸ ").append(dt).append("  [").append(thr).append("]\n");
                if (!msg.isEmpty()) sb.append("  ").append(msg).append("\n");
                sb.append("\n");
            }
            return sb.toString().trim();
        } catch (Exception e) {
            return json; // raw fallback
        }
    }

    private View makeBtn(String label, int color) {
        TextView btn = new TextView(this);
        btn.setText(label);
        btn.setTextColor(0xFF1E1E2E);
        btn.setTextSize(13);
        btn.setTypeface(null, Typeface.BOLD);
        btn.setGravity(Gravity.CENTER);
        btn.setPadding(dp(16), dp(10), dp(16), dp(10));
        android.graphics.drawable.GradientDrawable bg =
            new android.graphics.drawable.GradientDrawable();
        bg.setColor(color);
        bg.setCornerRadius(dp(10));
        btn.setBackground(bg);
        btn.setClickable(true); btn.setFocusable(true);
        return btn;
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT);
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
