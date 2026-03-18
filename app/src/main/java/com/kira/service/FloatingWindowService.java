package com.kira.service;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.Service;
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
import android.util.Log;

import com.kira.service.ai.KiraAI;

/**
 * Floating overlay window.
 * - Bubble: small draggable K pill, tap to open panel
 * - Panel: full input panel, ENTIRE HEADER is draggable
 * Fixed: correct WindowManager flags for drag + focus
 */
public class FloatingWindowService extends Service {

    private static final String TAG = "FloatingWin";

    private WindowManager wm;
    private View bubbleView;
    private LinearLayout panelView;
    private boolean panelOpen = false;
    private KiraAI ai;
    private Handler handler;

    private WindowManager.LayoutParams bubbleLP;
    private WindowManager.LayoutParams panelLP;

    public static void start(Context ctx) { ctx.startService(new Intent(ctx, FloatingWindowService.class)); }
    public static void stop(Context ctx)  { ctx.stopService(new Intent(ctx, FloatingWindowService.class)); }

    @Override
    public IBinder onBind(Intent i) { return null; }

    @Override
    public void onCreate() {
        super.onCreate();
        wm      = (WindowManager) getSystemService(WINDOW_SERVICE);
        handler = new Handler(Looper.getMainLooper());
        ai      = new KiraAI(this);
        startForegroundCompat();
        buildBubble();
    }

    @Override
    public void onDestroy() {
        removeSafely(bubbleView);
        removeSafely(panelView);
        super.onDestroy();
    }

    // \u2500\u2500 Foreground notification \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private void startForegroundCompat() {
        String chId = "kira_float";
        NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        if (nm != null) nm.createNotificationChannel(
            new NotificationChannel(chId, "Kira Floating", NotificationManager.IMPORTANCE_MIN));
        startForeground(3, new Notification.Builder(this, chId)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentTitle("Kira overlay active").build());
    }

    // \u2500\u2500 Bubble \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private void buildBubble() {
        TextView bubble = new TextView(this);
        // K with orange glow
        bubble.setText("K");
        bubble.setTextColor(0xFF000000);
        bubble.setTextSize(18);
        bubble.setGravity(Gravity.CENTER);
        bubble.setTypeface(null, android.graphics.Typeface.BOLD);
        bubble.setBackgroundColor(0xFFff8c00);

        int sz = dp(56);
        // KEY FIX: FLAG_NOT_FOCUSABLE for drag, FLAG_NOT_TOUCH_MODAL to allow touches outside
        bubbleLP = new WindowManager.LayoutParams(
            sz, sz,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
            | WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
            | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        );
        bubbleLP.gravity = Gravity.TOP | Gravity.START;
        bubbleLP.x = 20;
        bubbleLP.y = 400;

        setDraggable(bubble, bubbleLP, () -> {
            // on tap: toggle panel
            if (panelOpen) closePanel(); else openPanel();
        });

        wm.addView(bubble, bubbleLP);
        bubbleView = bubble;
    }

    // \u2500\u2500 Panel \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private void buildPanel() {
        LinearLayout panel = new LinearLayout(this);
        panel.setOrientation(LinearLayout.VERTICAL);
        panel.setBackgroundColor(0xF21a1a1a);

        int panelW = dp(300);
        int panelH = WindowManager.LayoutParams.WRAP_CONTENT;

        // KEY FIX: Panel needs FLAG_NOT_TOUCH_MODAL but NOT FLAG_NOT_FOCUSABLE
        // so the EditText can receive keyboard input
        panelLP = new WindowManager.LayoutParams(
            panelW, panelH,
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY,
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL
            | WindowManager.LayoutParams.FLAG_WATCH_OUTSIDE_TOUCH
            | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT
        );
        panelLP.gravity = Gravity.TOP | Gravity.START;
        panelLP.x = dp(10);
        panelLP.y = 300;
        panelLP.softInputMode = android.view.WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE;

        // \u2500\u2500 Drag handle (entire header row is draggable) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(Gravity.CENTER_VERTICAL);
        header.setBackgroundColor(0xFF111111);
        header.setPadding(dp(12), dp(10), dp(12), dp(10));

        // Drag indicator dots
        TextView dragDots = new TextView(this);
        dragDots.setText("\u2630"); // trigram = hamburger
        dragDots.setTextColor(0xFF555555);
        dragDots.setTextSize(14);
        dragDots.setPadding(0, 0, dp(10), 0);

        TextView kiraTitle = new TextView(this);
        kiraTitle.setText("Kira");
        kiraTitle.setTextColor(0xFFff8c00);
        kiraTitle.setTextSize(14);
        kiraTitle.setTypeface(null, android.graphics.Typeface.BOLD);
        kiraTitle.setLayoutParams(new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));

        TextView closeBtn = new TextView(this);
        closeBtn.setText("\u2715"); // X
        closeBtn.setTextColor(0xFF666666);
        closeBtn.setTextSize(16);
        closeBtn.setPadding(dp(8), dp(4), 0, dp(4));
        closeBtn.setOnClickListener(v -> closePanel());

        header.addView(dragDots);
        header.addView(kiraTitle);
        header.addView(closeBtn);

        // KEY FIX: Make entire HEADER draggable (not just a small drag bar)
        setDraggable(header, panelLP, null);

        // \u2500\u2500 Response area \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        TextView responseView = new TextView(this);
        responseView.setTextColor(0xFFcccccc);
        responseView.setTextSize(13);
        responseView.setPadding(dp(12), dp(8), dp(12), dp(4));
        responseView.setMaxLines(6);
        responseView.setEllipsize(android.text.TextUtils.TruncateAt.END);
        responseView.setText("How can I help?");

        // \u2500\u2500 Quick chips row \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        android.widget.HorizontalScrollView chipsScroll = new android.widget.HorizontalScrollView(this);
        chipsScroll.setScrollbars(0);
        LinearLayout chipsRow = new LinearLayout(this);
        chipsRow.setOrientation(LinearLayout.HORIZONTAL);
        chipsRow.setPadding(dp(8), dp(4), dp(8), dp(4));

        String[][] chips = {
            {"\uD83D\uDCF1", "read_screen"},
            {"\uD83D\uDD14", "notifications"},
            {"\uD83D\uDD0B", "battery"},
            {"\u26A1", "/agent "},
        };
        for (String[] chip : chips) {
            TextView tv = new TextView(this);
            tv.setText(chip[0]);
            tv.setTextSize(16);
            tv.setBackgroundColor(0xFF222222);
            tv.setPadding(dp(10), dp(6), dp(10), dp(6));
            LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT);
            cp.setMargins(0, 0, dp(6), 0);
            tv.setLayoutParams(cp);
            final String cmd = chip[1];
            tv.setOnClickListener(v -> sendCommand(cmd, responseView));
            chipsRow.addView(tv);
        }
        chipsScroll.addView(chipsRow);

        // \u2500\u2500 Input row \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        LinearLayout inputRow = new LinearLayout(this);
        inputRow.setOrientation(LinearLayout.HORIZONTAL);
        inputRow.setPadding(dp(8), dp(4), dp(8), dp(10));
        inputRow.setGravity(Gravity.CENTER_VERTICAL);

        EditText input = new EditText(this);
        input.setHint("Ask Kira...");
        input.setHintTextColor(0xFF555555);
        input.setTextColor(0xFFffffff);
        input.setTextSize(13);
        input.setBackgroundColor(0xFF222222);
        input.setPadding(dp(10), dp(8), dp(10), dp(8));
        input.setSingleLine(true);
        LinearLayout.LayoutParams ip = new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1);
        ip.setMargins(0, 0, dp(6), 0);
        input.setLayoutParams(ip);

        TextView sendBtn = new TextView(this);
        sendBtn.setText("\u25B6"); // play
        sendBtn.setTextColor(0xFF000000);
        sendBtn.setTextSize(14);
        sendBtn.setGravity(Gravity.CENTER);
        sendBtn.setBackgroundColor(0xFFff8c00);
        sendBtn.setPadding(dp(12), dp(8), dp(12), dp(8));
        sendBtn.setOnClickListener(v -> {
            String q = input.getText().toString().trim();
            if (!q.isEmpty()) {
                input.setText("");
                sendCommand(q, responseView);
            }
        });

        inputRow.addView(input);
        inputRow.addView(sendBtn);

        panel.addView(header);
        panel.addView(responseView);
        panel.addView(chipsScroll);
        panel.addView(inputRow);

        panelView = panel;
    }

    // \u2500\u2500 Open / Close \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private void openPanel() {
        if (panelOpen) return;
        if (panelView == null) buildPanel();
        try {
            bubbleView.setVisibility(View.GONE);
            if (!panelView.isAttachedToWindow()) {
                wm.addView(panelView, panelLP);
            }
            panelOpen = true;
        } catch (Exception e) { Log.e(TAG, "openPanel: " + e); }
    }

    private void closePanel() {
        if (!panelOpen) return;
        try {
            removeSafely(panelView);
            bubbleView.setVisibility(View.VISIBLE);
            panelOpen = false;
        } catch (Exception e) { Log.e(TAG, "closePanel: " + e); }
    }

    // \u2500\u2500 Command sender \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private void sendCommand(String cmd, TextView responseView) {
        responseView.setText("\u23F3 thinking..."); // hourglass
        responseView.setTextColor(0xFF555555);
        ai.chat(cmd, new KiraAI.Callback() {
            @Override public void onThinking() {
                handler.post(() -> responseView.setText("\u23F3 ..."));
            }
            @Override public void onTool(String name, String result) {
                handler.post(() -> responseView.setText("\u26A1 " + name));
            }
            @Override public void onReply(String reply) {
                handler.post(() -> {
                    responseView.setText(reply.substring(0, Math.min(200, reply.length())));
                    responseView.setTextColor(0xFFcccccc);
                });
            }
            @Override public void onError(String err) {
                handler.post(() -> { responseView.setText("err: " + err); responseView.setTextColor(0xFFcc4444); });
            }
        });
    }

    // \u2500\u2500 Drag logic (works for both bubble and panel header) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    /**
     * KEY FIX: Use raw screen coords (getRawX/getRawY) not view-relative coords.
     * Both bubble and panel use this same method.
     * onTap callback fires only if movement < 15dp (it was a tap, not a drag).
     */
    private void setDraggable(View view, WindowManager.LayoutParams lp, Runnable onTap) {
        final float[] downRawX = {0}, downRawY = {0};
        final int[]   downLpX  = {0}, downLpY  = {0};
        final boolean[] dragged = {false};

        view.setOnTouchListener((v, event) -> {
            switch (event.getActionMasked()) {
                case MotionEvent.ACTION_DOWN:
                    downRawX[0] = event.getRawX();
                    downRawY[0] = event.getRawY();
                    downLpX[0]  = lp.x;
                    downLpY[0]  = lp.y;
                    dragged[0]  = false;
                    return true;

                case MotionEvent.ACTION_MOVE:
                    float dx = event.getRawX() - downRawX[0];
                    float dy = event.getRawY() - downRawY[0];
                    if (Math.abs(dx) > dp(3) || Math.abs(dy) > dp(3)) {
                        dragged[0] = true;
                    }
                    if (dragged[0]) {
                        lp.x = downLpX[0] + (int) dx;
                        lp.y = downLpY[0] + (int) dy;
                        View target = (lp == bubbleLP) ? bubbleView : panelView;
                        if (target != null && target.isAttachedToWindow()) {
                            try { wm.updateViewLayout(target, lp); }
                            catch (Exception ignored) {}
                        }
                    }
                    return true;

                case MotionEvent.ACTION_UP:
                    if (!dragged[0] && onTap != null) {
                        onTap.run();
                    }
                    return true;
            }
            return false;
        });
    }

    private void removeSafely(View v) {
        try { if (v != null && v.isAttachedToWindow()) wm.removeView(v); }
        catch (Exception ignored) {}
    }

    private int dp(int dp) {
        return (int)(dp * getResources().getDisplayMetrics().density);
    }
}
