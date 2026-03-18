package com.kira.service;

import android.app.Service;
import android.util.Log;
import android.content.Context;
import android.content.Intent;
import android.graphics.PixelFormat;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import com.kira.service.ai.KiraAI;

/**
 * Floating window controller — like Rou Bao.
 *
 * Two views:
 *  1. Bubble — small draggable "K" pill, always on top
 *  2. Panel  — expanded input panel, also draggable, hides bubble while open
 */
public class FloatingWindowService extends Service {

    private WindowManager wm;
    private View bubbleView;
    private View panelView;
    private boolean expanded = false;
    private KiraAI ai;
    private Handler uiHandler;

    // Bubble layout params
    private WindowManager.LayoutParams bubbleLP;
    // Panel layout params
    private WindowManager.LayoutParams panelLP;

    public static void start(Context ctx) {
        ctx.startService(new Intent(ctx, FloatingWindowService.class));
    }

    public static void stop(Context ctx) {
        ctx.stopService(new Intent(ctx, FloatingWindowService.class));
    }

    @Override
    public void onCreate() {
        super.onCreate();
        wm = (WindowManager) getSystemService(WINDOW_SERVICE);
        uiHandler = new Handler(Looper.getMainLooper());
        ai = new KiraAI(this);
        buildBubble();
        buildPanel();
    }

    // ── Bubble ────────────────────────────────────────────────────────────────

    private void buildBubble() {
        TextView bubble = new TextView(this);
        bubble.setText("K");
        bubble.setTextColor(0xFF000000);
        bubble.setTextSize(16);
        bubble.setGravity(Gravity.CENTER);
        bubble.setTypeface(null, android.graphics.Typeface.BOLD);
        bubble.setBackgroundColor(0xFFff8c00);

        int dp56 = dp(56);
        bubbleLP = new WindowManager.LayoutParams(
            dp56, dp56,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE |
            WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        );
        bubbleLP.gravity = Gravity.TOP | Gravity.START;
        bubbleLP.x = 0;
        bubbleLP.y = 400;

        makeDraggable(bubble, bubbleLP, true);
        wm.addView(bubble, bubbleLP);
        bubbleView = bubble;
    }

    // ── Panel ─────────────────────────────────────────────────────────────────

    private void buildPanel() {
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setBackgroundColor(0xF01a1a1a);
        panel.setPadding(dp(12), dp(8), dp(12), dp(12));

        // Drag handle bar
        View dragBar = new View(this);
        dragBar.setBackgroundColor(0xFF333333);
        LinearLayout.LayoutParams dbp = new LinearLayout.LayoutParams(dp(40), dp(4));
        dbp.gravity = Gravity.CENTER_HORIZONTAL;
        dbp.setMargins(0, 0, 0, dp(8));
        dragBar.setLayoutParams(dbp);

        // Header row
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(Gravity.CENTER_VERTICAL);
        LinearLayout.LayoutParams hp = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        hp.setMargins(0, 0, 0, dp(8));
        header.setLayoutParams(hp);

        TextView title = new TextView(this);
        title.setText("Kira");
        title.setTextColor(0xFFff8c00);
        title.setTextSize(15);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setLayoutParams(new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));

        TextView closeBtn = new TextView(this);
        closeBtn.setText("✕");
        closeBtn.setTextColor(0xFF888888);
        closeBtn.setTextSize(18);
        closeBtn.setPadding(dp(8), 0, 0, 0);
        closeBtn.setOnClickListener(v -> collapsePanel());

        header.addView(title);
        header.addView(closeBtn);

        // Reply display
        TextView replyView = new TextView(this);
        replyView.setText("Ready. What do you need?");
        replyView.setTextColor(0xFFcccccc);
        replyView.setTextSize(13);
        replyView.setBackgroundColor(0xFF111111);
        replyView.setPadding(dp(10), dp(8), dp(10), dp(8));
        replyView.setMinLines(2);
        replyView.setMaxLines(6);
        replyView.setTextIsSelectable(true);
        LinearLayout.LayoutParams rvp = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        rvp.setMargins(0, 0, 0, dp(8));
        replyView.setLayoutParams(rvp);

        // Input row
        LinearLayout inputRow = new LinearLayout(this);
        inputRow.setOrientation(LinearLayout.HORIZONTAL);
        inputRow.setGravity(Gravity.CENTER_VERTICAL);
        LinearLayout.LayoutParams irp = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT);
        irp.setMargins(0, 0, 0, dp(6));
        inputRow.setLayoutParams(irp);

        EditText inputField = new EditText(this);
        inputField.setHint("Ask Kira anything...");
        inputField.setTextColor(0xFFffffff);
        inputField.setHintTextColor(0xFF555555);
        inputField.setTextSize(13);
        inputField.setBackgroundColor(0xFF2a2a2a);
        inputField.setPadding(dp(10), dp(8), dp(10), dp(8));
        inputField.setSingleLine(true);
        inputField.setLayoutParams(new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));

        TextView sendBtn = new TextView(this);
        sendBtn.setText("▶");
        sendBtn.setTextColor(0xFF000000);
        sendBtn.setBackgroundColor(0xFFff8c00);
        sendBtn.setTextSize(14);
        sendBtn.setGravity(Gravity.CENTER);
        int p = dp(10);
        sendBtn.setPadding(p, dp(8), p, dp(8));

        sendBtn.setOnClickListener(v -> {
            String text = inputField.getText().toString().trim();
            if (text.isEmpty()) return;
            inputField.setText("");
            replyView.setText("thinking...");
            replyView.setTextColor(0xFF555555);

            ai.chat(text, new KiraAI.Callback() {
                @Override public void onThinking() {}
                @Override public void onTool(String name, String result) {
                    uiHandler.post(() -> replyView.setText("⚡ " + name + "…"));
                }
                @Override public void onReply(String reply) {
                    uiHandler.post(() -> {
                        replyView.setText(reply);
                        replyView.setTextColor(0xFFcccccc);
                    });
                }
                @Override public void onError(String error) {
                    uiHandler.post(() -> {
                        replyView.setText("❌ " + error);
                        replyView.setTextColor(0xFFff6666);
                    });
                }
            });
        });

        inputRow.addView(inputField);
        inputRow.addView(sendBtn);

        // Quick chips
        LinearLayout chips = new LinearLayout(this);
        chips.setOrientation(LinearLayout.HORIZONTAL);

        String[][] quickCmds = {
            {"📱", "Read screen"},
            {"🔔", "Notifications"},
            {"🔋", "Battery"},
            {"📸", "Screenshot"},
            {"⚡", "Running apps"},
        };

        for (String[] cmd : quickCmds) {
            TextView chip = new TextView(this);
            chip.setText(cmd[0] + " " + cmd[1]);
            chip.setTextColor(0xFF888888);
            chip.setBackgroundColor(0xFF2a2a2a);
            chip.setTextSize(10);
            int cp2 = dp(6);
            chip.setPadding(cp2, dp(3), cp2, dp(3));
            LinearLayout.LayoutParams cpp = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            cpp.setMargins(0, 0, dp(4), 0);
            chip.setLayoutParams(cpp);
            chip.setOnClickListener(v -> {
                inputField.setText(cmd[1]);
                sendBtn.performClick();
            });
            chips.addView(chip);
        }

        panel.addView(dragBar);
        panel.addView(header);
        panel.addView(replyView);
        panel.addView(inputRow);
        panel.addView(chips);

        int screenWidth = getResources().getDisplayMetrics().widthPixels;
        panelLP = new WindowManager.LayoutParams(
            (int)(screenWidth * 0.9f),
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL |
            WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        );
        panelLP.gravity = Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL;
        panelLP.y = dp(80);

        // Make panel draggable via the drag bar
        makeDraggable(dragBar, panelLP, false);
        panelView = panel;
    }

    // ── Drag logic ────────────────────────────────────────────────────────────

    private void makeDraggable(View view, WindowManager.LayoutParams lp, boolean isBubble) {
        final int[] ix = {0}, iy = {0};
        final float[] tx = {0}, ty = {0};
        final long[] downTime = {0};

        view.setOnTouchListener((v, event) -> {
            switch (event.getAction()) {
                case MotionEvent.ACTION_DOWN:
                    ix[0] = lp.x; iy[0] = lp.y;
                    tx[0] = event.getRawX(); ty[0] = event.getRawY();
                    downTime[0] = System.currentTimeMillis();
                    return true;

                case MotionEvent.ACTION_MOVE:
                    lp.x = ix[0] + (int)(event.getRawX() - tx[0]);
                    lp.y = iy[0] + (int)(event.getRawY() - ty[0]);
                    try {
                        View target = isBubble ? bubbleView : panelView;
                        if (target != null && target.isAttachedToWindow()) {
                            wm.updateViewLayout(target, lp);
                        }
                    } catch (Exception ignored) {}
                    return true;

                case MotionEvent.ACTION_UP:
                    long elapsed = System.currentTimeMillis() - downTime[0];
                    float dx = Math.abs(event.getRawX() - tx[0]);
                    float dy = Math.abs(event.getRawY() - ty[0]);
                    if (isBubble && elapsed < 250 && dx < 12 && dy < 12) {
                        // Tap on bubble = toggle panel
                        if (expanded) collapsePanel(); else expandPanel();
                    }
                    return true;
            }
            return false;
        });
    }

    // ── Show / Hide ───────────────────────────────────────────────────────────

    private void expandPanel() {
        if (expanded) return;
        try {
            // Hide bubble while panel is open
            bubbleView.setVisibility(View.GONE);
            wm.addView(panelView, panelLP);
            expanded = true;
        } catch (Exception e) {
            Log.e("FloatingWin", "expandPanel: " + e.getMessage());
        }
    }

    private void collapsePanel() {
        if (!expanded) return;
        try {
            wm.removeView(panelView);
            expanded = false;
        } catch (Exception ignored) {}
        // Restore bubble
        if (bubbleView != null) bubbleView.setVisibility(View.VISIBLE);
    }

    @Override
    public void onDestroy() {
        collapsePanel();
        try { if (bubbleView != null) wm.removeView(bubbleView); } catch (Exception ignored) {}
        super.onDestroy();
    }

    @Override public IBinder onBind(Intent intent) { return null; }

    private int dp(int dp) {
        return (int)(dp * getResources().getDisplayMetrics().density);
    }
}
