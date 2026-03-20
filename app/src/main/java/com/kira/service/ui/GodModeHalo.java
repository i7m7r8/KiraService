package com.kira.service.ui;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RectF;
import android.util.AttributeSet;
import android.view.View;
import android.view.animation.LinearInterpolator;

/**
 * Layer 9 — God Mode Halo.
 *
 * When Shizuku is fully active: a 2dp Lavender border traces the entire
 * screen edge. A 30dp-long rotating highlight orbits the perimeter at
 * 4 seconds per revolution — like a radar sweep.
 *
 * When Shizuku is absent: halo is invisible (alpha = 0).
 *
 * Drawn as a full-screen transparent overlay (last child of root FrameLayout).
 */
public class GodModeHalo extends View {

    // Catppuccin Mocha
    private static final int  LAVENDER     = 0xFFB4BEFE;
    private static final int  LAVENDER_DIM = 0x33B4BEFE;  // 20% for base ring

    private final Paint borderPaint   = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint sweepPaint    = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF bounds        = new RectF();

    private float sweepAngle = 0f;   // 0–360, current highlight position
    private boolean active   = false;
    private ValueAnimator animator;

    private static final float SWEEP_LENGTH = 30f;   // degrees
    private static final float BORDER_DP    = 2f;

    public GodModeHalo(Context c) { super(c); init(); }
    public GodModeHalo(Context c, AttributeSet a) { super(c, a); init(); }

    private void init() {
        setWillNotDraw(false);
        setClickable(false);
        setFocusable(false);

        borderPaint.setStyle(Paint.Style.STROKE);
        borderPaint.setStrokeWidth(dp(BORDER_DP));
        borderPaint.setColor(LAVENDER_DIM);

        sweepPaint.setStyle(Paint.Style.STROKE);
        sweepPaint.setStrokeWidth(dp(BORDER_DP + 1));
        sweepPaint.setColor(LAVENDER);
        sweepPaint.setStrokeCap(Paint.Cap.ROUND);

        // 4 second revolution, continuous
        animator = ValueAnimator.ofFloat(0f, 360f);
        animator.setDuration(4000);
        animator.setRepeatCount(ValueAnimator.INFINITE);
        animator.setInterpolator(new LinearInterpolator());
        animator.addUpdateListener(a -> {
            sweepAngle = (float) a.getAnimatedValue();
            if (active) invalidate();
        });
    }

    /** Call when Shizuku permission state changes */
    public void setGodModeActive(boolean godMode) {
        if (godMode == active) return;
        active = godMode;
        if (godMode) {
            animate().alpha(1f).setDuration(400).start();
            animator.start();
        } else {
            animate().alpha(0f).setDuration(600)
                .withEndAction(() -> { animator.pause(); invalidate(); }).start();
        }
    }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        float half = dp(BORDER_DP) / 2f;
        bounds.set(half, half, w - half, h - half);
    }

    @Override
    protected void onDraw(Canvas canvas) {
        if (!active && getAlpha() < 0.01f) return;

        float w = getWidth(), h = getHeight();
        float bd = dp(BORDER_DP);

        // ── Base perimeter ring (dim) ─────────────────────────────────────
        // Draw as 4 lines (top, right, bottom, left)
        borderPaint.setColor(LAVENDER_DIM);
        borderPaint.setStrokeWidth(bd);
        canvas.drawRect(bounds, borderPaint);

        // ── Sweeping highlight ────────────────────────────────────────────
        // Convert sweep angle to a position along perimeter
        // Perimeter: top (0°→90°), right (90°→180°), bottom (180°→270°), left (270°→360°)
        float perimeter = 2 * (w + h);
        float pos = (sweepAngle / 360f) * perimeter;

        // Draw highlight segment: 30dp long arc along the perimeter
        float sweepLenPx = dp(SWEEP_LENGTH);
        drawPerimeterSegment(canvas, pos, sweepLenPx, w, h, bd);
    }

    /**
     * Draw a highlight segment starting at 'startPos' along the perimeter.
     * Top edge (left→right), Right edge (top→bottom), Bottom edge (right→left), Left edge (bottom→top).
     */
    private void drawPerimeterSegment(Canvas canvas, float startPos, float length, float w, float h, float bd) {
        float drawn = 0f;
        float pos   = startPos;

        sweepPaint.setStrokeWidth(bd + dp(1));
        sweepPaint.setColor(LAVENDER);

        while (drawn < length) {
            float remaining = length - drawn;
            float perimeter  = 2 * (w + h);
            pos = ((pos % perimeter) + perimeter) % perimeter;

            if (pos < w) {
                // Top edge left→right
                float segLen = Math.min(remaining, w - pos);
                canvas.drawLine(pos + bd, bd, pos + segLen + bd, bd, sweepPaint);
                drawn += segLen; pos += segLen;
            } else if (pos < w + h) {
                // Right edge top→bottom
                float rp = pos - w;
                float segLen = Math.min(remaining, h - rp);
                canvas.drawLine(w - bd, rp + bd, w - bd, rp + segLen + bd, sweepPaint);
                drawn += segLen; pos += segLen;
            } else if (pos < 2 * w + h) {
                // Bottom edge right→left
                float bp = pos - (w + h);
                float segLen = Math.min(remaining, w - bp);
                canvas.drawLine(w - bp - bd, h - bd, w - bp - segLen - bd, h - bd, sweepPaint);
                drawn += segLen; pos += segLen;
            } else {
                // Left edge bottom→top
                float lp = pos - (2 * w + h);
                float segLen = Math.min(remaining, h - lp);
                canvas.drawLine(bd, h - lp - bd, bd, h - lp - segLen - bd, sweepPaint);
                drawn += segLen; pos += segLen;
            }
            if (drawn < length && pos >= 2 * (w + h)) pos = 0f; // wrap
            else break;
        }
    }

    private float dp(float v) {
        return v * getResources().getDisplayMetrics().density;
    }
    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
