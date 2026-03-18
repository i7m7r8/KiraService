package com.kira.service;

import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.graphics.PixelFormat;
import android.os.IBinder;
import android.view.Gravity;
import android.view.LayoutInflater;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;

/**
 * Floating window controller — like Rou Bao's floating UI.
 * Shows a small draggable bubble on screen.
 * Tap to expand input, send to Kira, see reply inline.
 */
public class FloatingWindowService extends Service {

    private WindowManager windowManager;
    private View bubbleView;
    private View expandedView;
    private KiraAI ai;
    private boolean expanded = false;

    private WindowManager.LayoutParams bubbleParams;
    private WindowManager.LayoutParams expandedParams;

    public static void start(Context ctx) {
        ctx.startService(new Intent(ctx, FloatingWindowService.class));
    }

    public static void stop(Context ctx) {
        ctx.stopService(new Intent(ctx, FloatingWindowService.class));
    }

    @Override
    public void onCreate() {
        super.onCreate();
        windowManager = (WindowManager) getSystemService(WINDOW_SERVICE);
        ai = new KiraAI(this);
        createBubble();
        createExpandedView();
    }

    private void createBubble() {
        // Small orange "K" bubble
        bubbleView = new TextView(this);
        ((TextView) bubbleView).setText("K");
        ((TextView) bubbleView).setTextColor(0xFF000000);
        ((TextView) bubbleView).setTextSize(18);
        ((TextView) bubbleView).setGravity(android.view.Gravity.CENTER);
        ((TextView) bubbleView).setTypeface(null, android.graphics.Typeface.BOLD);
        bubbleView.setBackgroundColor(0xFFff8c00);
        bubbleView.setPadding(0, 0, 0, 0);

        bubbleParams = new WindowManager.LayoutParams(
            56 * (int) getResources().getDisplayMetrics().density,
            56 * (int) getResources().getDisplayMetrics().density,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE,
            PixelFormat.TRANSLUCENT
        );
        bubbleParams.gravity = Gravity.TOP | Gravity.START;
        bubbleParams.x = 0;
        bubbleParams.y = 300;

        setupBubbleDrag();
        windowManager.addView(bubbleView, bubbleParams);
    }

    private void createExpandedView() {
        // Expanded input panel
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setBackgroundColor(0xFF1a1a1a);
        panel.setPadding(12, 12, 12, 12);

        // Header
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(android.view.Gravity.CENTER_VERTICAL);
        header.setPadding(0, 0, 0, 8);

        TextView title = new TextView(this);
        title.setText("Kira");
        title.setTextColor(0xFFff8c00);
        title.setTextSize(16);
        title.setTypeface(null, android.graphics.Typeface.BOLD);
        title.setLayoutParams(new LinearLayout.LayoutParams(0, android.view.ViewGroup.LayoutParams.WRAP_CONTENT, 1));

        TextView closeBtn = new TextView(this);
        closeBtn.setText("✕");
        closeBtn.setTextColor(0xFF666666);
        closeBtn.setTextSize(16);
        closeBtn.setOnClickListener(v -> collapsePanel());

        header.addView(title);
        header.addView(closeBtn);

        // Reply area
        TextView replyArea = new TextView(this);
        replyArea.setId(android.view.View.generateViewId());
        replyArea.setText("Ready. What do you need?");
        replyArea.setTextColor(0xFFcccccc);
        replyArea.setTextSize(13);
        replyArea.setBackgroundColor(0xFF111111);
        replyArea.setPadding(10, 8, 10, 8);
        replyArea.setMinLines(2);
        replyArea.setMaxLines(5);
        LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(
            android.view.ViewGroup.LayoutParams.MATCH_PARENT,
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT);
        rp.setMargins(0, 0, 0, 8);
        replyArea.setLayoutParams(rp);

        // Input row
        LinearLayout inputRow = new LinearLayout(this);
        inputRow.setOrientation(LinearLayout.HORIZONTAL);
        inputRow.setGravity(android.view.Gravity.CENTER_VERTICAL);

        EditText input = new EditText(this);
        input.setHint("Ask Kira...");
        input.setTextColor(0xFFffffff);
        input.setTextSize(13);
        input.setBackgroundColor(0xFF2a2a2a);
        input.setPadding(10, 8, 10, 8);
        input.setSingleLine(true);
        input.setLayoutParams(new LinearLayout.LayoutParams(0,
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT, 1));

        TextView sendBtn = new TextView(this);
        sendBtn.setText("▶");
        sendBtn.setTextColor(0xFF000000);
        sendBtn.setBackgroundColor(0xFFff8c00);
        sendBtn.setTextSize(14);
        sendBtn.setGravity(android.view.Gravity.CENTER);
        sendBtn.setPadding(12, 8, 12, 8);
        sendBtn.setLayoutParams(new LinearLayout.LayoutParams(
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT,
            android.view.ViewGroup.LayoutParams.WRAP_CONTENT));

        sendBtn.setOnClickListener(v -> {
            String text = input.getText().toString().trim();
            if (text.isEmpty()) return;
            input.setText("");
            replyArea.setText("thinking...");
            replyArea.setTextColor(0xFF666666);

            ai.chat(text, new KiraAI.Callback() {
                @Override public void onThinking() {}
                @Override public void onTool(String name, String result) {
                    android.os.Handler h = new android.os.Handler(android.os.Looper.getMainLooper());
                    h.post(() -> replyArea.setText("⚡ " + name + "..."));
                }
                @Override public void onReply(String reply) {
                    android.os.Handler h = new android.os.Handler(android.os.Looper.getMainLooper());
                    h.post(() -> {
                        replyArea.setText(reply);
                        replyArea.setTextColor(0xFFcccccc);
                    });
                }
                @Override public void onError(String error) {
                    android.os.Handler h = new android.os.Handler(android.os.Looper.getMainLooper());
                    h.post(() -> {
                        replyArea.setText("error: " + error);
                        replyArea.setTextColor(0xFFff6666);
                    });
                }
            });
        });

        inputRow.addView(input);
        inputRow.addView(sendBtn);

        // Quick action chips
        LinearLayout chips = new LinearLayout(this);
        chips.setOrientation(LinearLayout.HORIZONTAL);
        chips.setPadding(0, 6, 0, 0);

        String[] quickActions = {"Screen", "Notifs", "Battery", "Screenshot"};
        for (String action : quickActions) {
            TextView chip = new TextView(this);
            chip.setText(action);
            chip.setTextColor(0xFF888888);
            chip.setBackgroundColor(0xFF2a2a2a);
            chip.setTextSize(10);
            chip.setPadding(8, 4, 8, 4);
            LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(
                android.view.ViewGroup.LayoutParams.WRAP_CONTENT,
                android.view.ViewGroup.LayoutParams.WRAP_CONTENT);
            cp.setMargins(0, 0, 6, 0);
            chip.setLayoutParams(cp);
            chip.setOnClickListener(v -> {
                input.setText(action.toLowerCase());
                sendBtn.performClick();
            });
            chips.addView(chip);
        }

        panel.addView(header);
        panel.addView(replyArea);
        panel.addView(inputRow);
        panel.addView(chips);

        expandedView = panel;

        int screenWidth = getResources().getDisplayMetrics().widthPixels;
        expandedParams = new WindowManager.LayoutParams(
            (int)(screenWidth * 0.85f),
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL,
            PixelFormat.TRANSLUCENT
        );
        expandedParams.gravity = Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL;
        expandedParams.y = 100;
    }

    private void setupBubbleDrag() {
        final int[] initialX = {0};
        final int[] initialY = {0};
        final float[] initialTouchX = {0};
        final float[] initialTouchY = {0};
        final long[] touchDownTime = {0};

        bubbleView.setOnTouchListener((v, event) -> {
            switch (event.getAction()) {
                case MotionEvent.ACTION_DOWN:
                    initialX[0] = bubbleParams.x;
                    initialY[0] = bubbleParams.y;
                    initialTouchX[0] = event.getRawX();
                    initialTouchY[0] = event.getRawY();
                    touchDownTime[0] = System.currentTimeMillis();
                    return true;

                case MotionEvent.ACTION_MOVE:
                    bubbleParams.x = initialX[0] + (int)(event.getRawX() - initialTouchX[0]);
                    bubbleParams.y = initialY[0] + (int)(event.getRawY() - initialTouchY[0]);
                    windowManager.updateViewLayout(bubbleView, bubbleParams);
                    return true;

                case MotionEvent.ACTION_UP:
                    long elapsed = System.currentTimeMillis() - touchDownTime[0];
                    float dx = Math.abs(event.getRawX() - initialTouchX[0]);
                    float dy = Math.abs(event.getRawY() - initialTouchY[0]);
                    if (elapsed < 200 && dx < 10 && dy < 10) {
                        // Tap - toggle expanded panel
                        togglePanel();
                    }
                    return true;
            }
            return false;
        });
    }

    private void togglePanel() {
        if (expanded) {
            collapsePanel();
        } else {
            expandPanel();
        }
    }

    private void expandPanel() {
        if (!expanded) {
            try {
                windowManager.addView(expandedView, expandedParams);
                expanded = true;
                // Update bubble params to allow focus when expanded
                bubbleParams.flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE;
                windowManager.updateViewLayout(bubbleView, bubbleParams);
            } catch (Exception e) {
                // View already added
            }
        }
    }

    private void collapsePanel() {
        if (expanded) {
            try {
                windowManager.removeView(expandedView);
                expanded = false;
            } catch (Exception ignored) {}
        }
    }

    @Override
    public void onDestroy() {
        collapsePanel();
        if (bubbleView != null) {
            try { windowManager.removeView(bubbleView); } catch (Exception ignored) {}
        }
        super.onDestroy();
    }

    @Override
    public IBinder onBind(Intent intent) { return null; }
}
