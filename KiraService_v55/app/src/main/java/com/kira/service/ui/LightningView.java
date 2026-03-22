package com.kira.service.ui;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.Path;
import android.util.AttributeSet;
import android.view.View;
import java.util.ArrayList;
import java.util.List;
import java.util.Random;

/**
 * Layer 6 — Lightning Engine.
 *
 * Drawn as a transparent overlay on the root FrameLayout.
 * Three trigger types:
 *   KIRA_REPLY   — branching arc from K badge to last message bubble (Lavender, 80ms)
 *   MACRO_FIRE   — horizontal streak across screen top 4dp (Peach, 150ms)
 *   SYSTEM_CONNECT — circular burst from status dot (Green radial lines, 200ms)
 *
 * Each event fires ONCE — never loops.
 */
public class LightningView extends View {

    public enum Event { KIRA_REPLY, MACRO_FIRE, SYSTEM_CONNECT }

    private static final int  LAVENDER = 0xFFB4BEFE;
    private static final int  PEACH    = 0xFFFAB387;
    private static final int  GREEN    = 0xFFA6E3A1;

    private static class Bolt {
        Path   path;
        int    color;
        float  alpha;        // current alpha (0–1)
        long   startMs;
        long   durationMs;
        boolean done;
    }

    private final List<Bolt> bolts = new ArrayList<>();
    private final Paint      paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Random     rng   = new Random();
    private ValueAnimator    ticker;

    // Anchor points set by MainActivity
    private float kBadgeX, kBadgeY;        // centre of K badge
    private float lastBubbleX, lastBubbleY;// top-left of last Kira bubble
    private float statusDotX, statusDotY;  // centre of status dot

    public LightningView(Context c) { super(c); init(); }
    public LightningView(Context c, AttributeSet a) { super(c, a); init(); }

    private void init() {
        setWillNotDraw(false);
        setClickable(false);
        setFocusable(false);
        // Ticker: 60fps while bolts are live
        ticker = ValueAnimator.ofFloat(0f, 1f);
        ticker.setDuration(10000);
        ticker.setRepeatCount(ValueAnimator.INFINITE);
        ticker.addUpdateListener(a -> {
            if (!bolts.isEmpty()) {
                pruneFinished();
                invalidate();
            }
        });
        ticker.start();
    }

    // ── Public API ────────────────────────────────────────────────────────

    public void setKBadgeAnchor(float x, float y)     { kBadgeX = x; kBadgeY = y; }
    public void setLastBubbleAnchor(float x, float y) { lastBubbleX = x; lastBubbleY = y; }
    public void setStatusDotAnchor(float x, float y)  { statusDotX = x; statusDotY = y; }

    public void fire(Event event) {
        long now = System.currentTimeMillis();
        switch (event) {
            case KIRA_REPLY:
                // 3 branches Lavender arc K badge → last bubble
                for (int b = 0; b < 3; b++) {
                    Bolt bolt = new Bolt();
                    bolt.path      = buildBranching(kBadgeX, kBadgeY, lastBubbleX, lastBubbleY, 3 - b);
                    bolt.color     = LAVENDER;
                    bolt.alpha     = 1f;
                    bolt.startMs   = now + b * 12L;
                    bolt.durationMs= 80;
                    bolts.add(bolt);
                }
                break;
            case MACRO_FIRE:
                // Horizontal Peach streak across top 4dp
                Bolt streak = new Bolt();
                streak.path      = buildHStreak(0, dp(2), getWidth(), dp(2));
                streak.color     = PEACH;
                streak.alpha     = 1f;
                streak.startMs   = now;
                streak.durationMs= 150;
                bolts.add(streak);
                break;
            case SYSTEM_CONNECT:
                // 4 radial Green lines expanding from status dot
                for (int r = 0; r < 4; r++) {
                    Bolt ray = new Bolt();
                    double angle = r * Math.PI / 2.0;
                    float ex = statusDotX + (float)(Math.cos(angle) * dp(32));
                    float ey = statusDotY + (float)(Math.sin(angle) * dp(32));
                    ray.path      = buildBranching(statusDotX, statusDotY, ex, ey, 1);
                    ray.color     = GREEN;
                    ray.alpha     = 1f;
                    ray.startMs   = now + r * 20L;
                    ray.durationMs= 200;
                    bolts.add(ray);
                }
                break;
        }
        invalidate();
    }

    // ── Drawing ───────────────────────────────────────────────────────────

    @Override
    protected void onDraw(Canvas canvas) {
        long now = System.currentTimeMillis();
        paint.setStyle(Paint.Style.STROKE);
        paint.setStrokeCap(Paint.Cap.ROUND);

        for (Bolt b : bolts) {
            if (b.done) continue;
            long elapsed = now - b.startMs;
            if (elapsed < 0) continue;
            float t = Math.min(1f, (float) elapsed / b.durationMs);
            // Fade: appear fast, die slowly
            float alpha = t < 0.2f ? t / 0.2f : 1f - ((t - 0.2f) / 0.8f);
            paint.setStrokeWidth(dp(1.5f));
            int a = Math.max(0, Math.min(255, (int)(alpha * 200)));
            paint.setColor((a << 24) | (b.color & 0x00FFFFFF));
            canvas.drawPath(b.path, paint);
            if (t >= 1f) b.done = true;
        }
    }

    // ── Path builders ─────────────────────────────────────────────────────

    /** Branching lightning path from (x1,y1) to (x2,y2) with 'branches' sub-forks */
    private Path buildBranching(float x1, float y1, float x2, float y2, int branches) {
        Path path = new Path();
        path.moveTo(x1, y1);
        float dx = x2 - x1, dy = y2 - y1;
        int segments = 6 + rng.nextInt(4);
        float cx = x1, cy = y1;
        for (int i = 1; i <= segments; i++) {
            float progress = (float) i / segments;
            float nx = x1 + dx * progress + (rng.nextFloat() - 0.5f) * dp(16) * (1f - progress);
            float ny = y1 + dy * progress + (rng.nextFloat() - 0.5f) * dp(16) * (1f - progress);
            path.lineTo(nx, ny);
            // Branch point at ~40% of the bolt
            if (branches > 0 && i == segments / 2) {
                Path sub = buildBranching(cx, cy,
                    nx + (rng.nextFloat() - 0.5f) * dp(30),
                    ny + dy * 0.3f + (rng.nextFloat() - 0.5f) * dp(20), 0);
                path.addPath(sub);
                path.moveTo(nx, ny);
            }
            cx = nx; cy = ny;
        }
        path.lineTo(x2, y2);
        return path;
    }

    /** Horizontal streak left → right */
    private Path buildHStreak(float x1, float y, float x2, float y2) {
        Path path = new Path();
        path.moveTo(x1, y);
        int segs = 12;
        for (int i = 1; i <= segs; i++) {
            float px = x1 + (x2 - x1) * i / segs;
            float py = y + (rng.nextFloat() - 0.5f) * dp(3);
            path.lineTo(px, py);
        }
        return path;
    }

    private void pruneFinished() {
        bolts.removeIf(b -> b.done);
    }

    private float dp(float v) {
        return v * getResources().getDisplayMetrics().density;
    }
    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
}
