package com.kira.service;

import android.app.Activity;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.Intent;
import android.graphics.Typeface;
import android.os.Bundle;
import android.view.Gravity;
import android.view.ViewGroup;
import android.widget.*;

/**
 * Standalone crash reporter — runs in :crash process.
 * Survives main app death. Shows full stack trace + restart/copy buttons.
 */
public class CrashActivity extends Activity {

    private static final int BG      = 0xFF11111B;
    private static final int CARD    = 0xFF1E1E2E;
    private static final int RED     = 0xFFF38BA8;
    private static final int PEACH   = 0xFFFAB387;
    private static final int LAV     = 0xFFB4BEFE;
    private static final int TEXT    = 0xFFCDD6F4;
    private static final int MUTED   = 0xFF6C7086;
    private static final int GREEN   = 0xFFA6E3A1;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        // Read crash data — from intent or SharedPrefs fallback
        String trace  = getIntent().getStringExtra("trace");
        long   ts     = getIntent().getLongExtra("ts", 0);
        String thread = getIntent().getStringExtra("thread");

        if (trace == null || trace.isEmpty()) {
            android.content.SharedPreferences p =
                getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE);
            trace  = p.getString(KiraApp.KEY_TRACE,  "No crash data found");
            ts     = p.getLong  (KiraApp.KEY_TS,     0);
            thread = p.getString(KiraApp.KEY_THREAD, "unknown");
        }

        final String fTrace  = trace;
        final String fThread = thread != null ? thread : "unknown";
        final String fTime   = ts > 0
            ? new java.text.SimpleDateFormat("yyyy-MM-dd HH:mm:ss",
                java.util.Locale.getDefault()).format(new java.util.Date(ts))
            : "unknown time";

        // ── UI ────────────────────────────────────────────────────────────
        ScrollView scroll = new ScrollView(this);
        scroll.setBackgroundColor(BG);
        setContentView(scroll);

        LinearLayout col = new LinearLayout(this);
        col.setOrientation(LinearLayout.VERTICAL);
        col.setPadding(dp(16), dp(48), dp(16), dp(32));
        scroll.addView(col, new ScrollView.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT));

        // Header
        addText(col, "💀  Kira Crashed", 22, RED, Typeface.BOLD, Gravity.CENTER, 0, dp(4));
        addText(col, "Thread: " + fThread + "  ·  " + fTime,
            11, MUTED, Typeface.NORMAL, Gravity.CENTER, 0, dp(16));

        // Stack trace card
        LinearLayout card = new LinearLayout(this);
        card.setOrientation(LinearLayout.VERTICAL);
        card.setBackgroundColor(CARD);
        card.setPadding(dp(12), dp(12), dp(12), dp(12));
        lp(card, 0, dp(16));
        col.addView(card);

        addText(card, "STACK TRACE", 10, PEACH, Typeface.NORMAL, Gravity.LEFT, 0, dp(8));

        HorizontalScrollView hScroll = new HorizontalScrollView(this);
        hScroll.setLayoutParams(new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT));
        TextView traceView = new TextView(this);
        traceView.setText(colorTrace(fTrace));
        traceView.setTextSize(10);
        traceView.setTypeface(Typeface.MONOSPACE);
        traceView.setLineSpacing(dp(2), 1f);
        traceView.setTextIsSelectable(true);
        traceView.setPadding(0, 0, dp(16), 0);
        hScroll.addView(traceView);
        card.addView(hScroll);

        // Buttons
        LinearLayout btns = new LinearLayout(this);
        btns.setOrientation(LinearLayout.HORIZONTAL);
        btns.setGravity(Gravity.CENTER);
        lp(btns, 0, dp(16));
        col.addView(btns);

        // Copy
        TextView copyBtn = makeBtn("📋 Copy");
        copyBtn.setBackgroundColor(LAV);
        copyBtn.setTextColor(0xFF1E1E2E);
        copyBtn.setOnClickListener(v -> {
            ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("crash", fTrace));
            Toast.makeText(this, "Copied", Toast.LENGTH_SHORT).show();
        });
        btns.addView(copyBtn);
        addSpacer(btns, dp(12));

        // Restart
        TextView restartBtn = makeBtn("🔄 Restart");
        restartBtn.setBackgroundColor(GREEN);
        restartBtn.setTextColor(0xFF1E1E2E);
        restartBtn.setOnClickListener(v -> {
            Intent i = getPackageManager().getLaunchIntentForPackage(getPackageName());
            if (i != null) {
                i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
                startActivity(i);
            }
            finishAndRemoveTask();
        });
        btns.addView(restartBtn);
        addSpacer(btns, dp(12));

        // Clear
        TextView clearBtn = makeBtn("🗑 Clear");
        clearBtn.setBackgroundColor(PEACH);
        clearBtn.setTextColor(0xFF1E1E2E);
        clearBtn.setOnClickListener(v -> {
            getSharedPreferences(KiraApp.PREFS_CRASH, MODE_PRIVATE).edit().clear().apply();
            Toast.makeText(this, "Cleared", Toast.LENGTH_SHORT).show();
            finishAndRemoveTask();
        });
        btns.addView(clearBtn);

        // Ask Kira
        TextView askBtn = makeBtn("🤖 Ask Kira to Fix");
        askBtn.setBackgroundColor(RED);
        askBtn.setTextColor(0xFF1E1E2E);
        LinearLayout.LayoutParams askLp = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        askLp.setMargins(0, dp(8), 0, 0);
        askBtn.setLayoutParams(askLp);
        String prompt = "I crashed. Fix this:\n" + fTrace.substring(0, Math.min(800, fTrace.length()));
        askBtn.setOnClickListener(v -> {
            ClipboardManager cm = (ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(ClipData.newPlainText("crash_prompt", prompt));
            Intent i = new Intent(this, MainActivity.class);
            i.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TASK | Intent.FLAG_ACTIVITY_NEW_TASK);
            i.putExtra("crash_prompt", prompt);
            startActivity(i);
            finishAndRemoveTask();
        });
        col.addView(askBtn);
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    private android.text.SpannableString colorTrace(String trace) {
        android.text.SpannableStringBuilder sb = new android.text.SpannableStringBuilder(trace);
        String[] lines = trace.split("\n");
        int pos = 0;
        for (String line : lines) {
            int end = pos + line.length();
            if (end > sb.length()) break;
            int color = TEXT;
            if (line.contains("com.kira.service")) color = LAV;
            else if (line.startsWith("Caused by") || line.contains("Error") || line.contains("Exception")) color = RED;
            else if (line.startsWith("\tat ")) color = MUTED;
            sb.setSpan(new android.text.style.ForegroundColorSpan(color),
                pos, end, android.text.Spanned.SPAN_EXCLUSIVE_EXCLUSIVE);
            pos = end + 1;
        }
        return new android.text.SpannableString(sb);
    }

    private TextView makeBtn(String label) {
        TextView btn = new TextView(this);
        btn.setText(label);
        btn.setTextSize(13);
        btn.setTypeface(null, Typeface.BOLD);
        btn.setGravity(Gravity.CENTER);
        btn.setPadding(dp(16), dp(10), dp(16), dp(10));
        android.graphics.drawable.GradientDrawable bg =
            new android.graphics.drawable.GradientDrawable();
        bg.setCornerRadius(dp(10));
        btn.setBackground(bg);
        btn.setClickable(true);
        btn.setFocusable(true);
        return btn;
    }

    private void addText(LinearLayout parent, String text, int sp, int color,
                         int style, int gravity, int topMargin, int bottomMargin) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextSize(sp);
        tv.setTextColor(color);
        tv.setTypeface(null, style);
        tv.setGravity(gravity);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        lp.setMargins(0, topMargin, 0, bottomMargin);
        tv.setLayoutParams(lp);
        parent.addView(tv);
    }

    private void lp(android.view.View v, int top, int bottom) {
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        p.setMargins(0, top, 0, bottom);
        v.setLayoutParams(p);
    }

    private void addSpacer(LinearLayout parent, int width) {
        android.view.View space = new android.view.View(this);
        parent.addView(space, new LinearLayout.LayoutParams(width, 1));
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
