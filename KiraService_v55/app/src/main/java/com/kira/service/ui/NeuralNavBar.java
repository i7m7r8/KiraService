package com.kira.service.ui;

import android.animation.Animator;
import android.animation.AnimatorListenerAdapter;
import android.animation.ObjectAnimator;
import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.RadialGradient;
import android.graphics.RectF;
import android.graphics.Shader;
import android.graphics.drawable.GradientDrawable;
import android.util.AttributeSet;
import android.view.MotionEvent;
import android.view.View;
import android.view.animation.OvershootInterpolator;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

/**
 * Layer 1 — The Neural Nav Bar.
 *
 * Floating island: 88% width, 72dp, 24dp corners, Catppuccin Mantle 94% opacity.
 * 1px top edge in Lavender (#B4BEFE) at 30% opacity.
 *
 * Active tab: icon grows 22→26sp with OvershootInterpolator (tension 2.8),
 *             radial Lavender aura blooms beneath (0→32dp in 180ms).
 * Inactive tab: spring back with same curve.
 *
 * Geometric glyphs: ⬡ Chat, ⠿ Tools, ☰ Log, ⧉ System.
 * Keyboard: bar floats up 4dp + 0.97 scale when keyboard opens.
 */
public class NeuralNavBar extends FrameLayout {

    // ── Catppuccin Mocha colours ──────────────────────────────────────────
    private static final int MANTLE    = 0xF0181825; // 94% opacity Mantle
    private static final int LAVENDER  = 0xFFB4BEFE;
    private static final int LAV_DIM   = 0x4DB4BEFE; // 30% Lavender border
    private static final int LAV_AURA  = 0x66B4BEFE; // 40% aura center
    private static final int TEXT_MUTED= 0xFF6C7086; // Overlay0 — inactive
    private static final int TEXT_ACT  = 0xFFB4BEFE; // Lavender — active

    // Tab glyphs and labels
    private static final String[] GLYPHS  = {"⬡", "⠿", "☰", "⧉"};
    private static final String[] LABELS  = {"Chat", "Tools", "Log", "System"};

    private int currentTab = 0;
    private final TabView[] tabs = new TabView[4];
    private OnTabSelectedListener listener;

    // Aura paint (drawn on canvas)
    private final Paint auraPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF auraRect  = new RectF();
    private final float[] auraRadius = new float[4]; // per tab
    private final ValueAnimator[] auraAnim = new ValueAnimator[4];

    public interface OnTabSelectedListener {
        void onTabSelected(int index);
    }

    public NeuralNavBar(Context ctx) { super(ctx); init(ctx); }
    public NeuralNavBar(Context ctx, AttributeSet a) { super(ctx, a); init(ctx); }

    public void setOnTabSelectedListener(OnTabSelectedListener l) { listener = l; }

    private void init(Context ctx) {
        setWillNotDraw(false); // We draw the aura ourselves

        // Pill background — Mantle with Lavender top border
        GradientDrawable bg = new GradientDrawable();
        bg.setShape(GradientDrawable.RECTANGLE);
        bg.setCornerRadius(dp(24));
        bg.setColor(MANTLE);
        bg.setStroke(dp(1), LAV_DIM);
        setBackground(bg);

        // Shadow for floating effect
        setElevation(dp(8));

        // Build tab views
        LinearLayout row = new LinearLayout(ctx);
        row.setOrientation(LinearLayout.HORIZONTAL);
        LinearLayout.LayoutParams rowLp = new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.MATCH_PARENT);
        addView(row, rowLp);

        for (int i = 0; i < 4; i++) {
            final int idx = i;
            TabView tab = new TabView(ctx, GLYPHS[i], LABELS[i]);
            tab.setActive(i == 0);
            tab.setOnClickListener(v -> selectTab(idx));
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(
                0, LinearLayout.LayoutParams.MATCH_PARENT, 1f);
            row.addView(tab, lp);
            tabs[i] = tab;
            auraRadius[i] = (i == 0) ? dp(32) : 0f;
        }
    }

    public void selectTab(int idx) {
        if (idx == currentTab) return;
        int prev = currentTab;
        currentTab = idx;

        // Animate icons
        if (tabs[prev] != null) tabs[prev].animateDeactivate();
        if (tabs[idx]  != null) tabs[idx].animateActivate();

        // Animate auras
        animateAura(prev, dp(32), 0f);
        animateAura(idx,  0f, dp(32));

        if (listener != null) listener.onTabSelected(idx);
        invalidate();
    }

    private void animateAura(int tabIdx, float from, float to) {
        if (auraAnim[tabIdx] != null) auraAnim[tabIdx].cancel();
        ValueAnimator anim = ValueAnimator.ofFloat(from, to);
        anim.setDuration(180);
        anim.setInterpolator(new OvershootInterpolator(2.8f));
        final int fi = tabIdx;
        anim.addUpdateListener(a -> {
            auraRadius[fi] = (float) a.getAnimatedValue();
            invalidate();
        });
        anim.start();
        auraAnim[tabIdx] = anim;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        // Draw radial auras under each active-ish tab
        int w = getWidth();
        int tabW = w / 4;
        for (int i = 0; i < 4; i++) {
            if (auraRadius[i] < 1f) continue;
            float cx = tabW * i + tabW * 0.5f;
            float cy = getHeight() * 0.5f;
            RadialGradient grad = new RadialGradient(
                cx, cy, auraRadius[i],
                new int[]{LAV_AURA, 0x00B4BEFE},
                new float[]{0f, 1f},
                Shader.TileMode.CLAMP);
            auraPaint.setShader(grad);
            canvas.drawCircle(cx, cy, auraRadius[i], auraPaint);
        }
    }

    /** Keyboard open/close: float up 4dp + 0.97 scale */
    public void onKeyboardVisible(boolean visible) {
        float targetY     = visible ? -dp(4) : 0f;
        float targetScale = visible ? 0.97f  : 1.0f;
        animate()
            .translationY(targetY)
            .scaleX(targetScale).scaleY(targetScale)
            .setDuration(200)
            .setInterpolator(new OvershootInterpolator(1.8f))
            .start();
    }

    // ── TabView ───────────────────────────────────────────────────────────

    private class TabView extends LinearLayout {
        private final TextView glyph;
        private final TextView label;

        TabView(Context ctx, String glyphStr, String labelStr) {
            super(ctx);
            setOrientation(LinearLayout.VERTICAL);
            setGravity(android.view.Gravity.CENTER);
            setPadding(0, dp(6), 0, dp(6));

            glyph = new TextView(ctx);
            glyph.setText(glyphStr);
            glyph.setTextSize(22);
            glyph.setGravity(android.view.Gravity.CENTER);
            glyph.setTextColor(TEXT_MUTED);
            addView(glyph, new LayoutParams(
                LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT));

            label = new TextView(ctx);
            label.setText(labelStr);
            label.setTextSize(9);
            label.setGravity(android.view.Gravity.CENTER);
            label.setTextColor(TEXT_MUTED);
            label.setTypeface(android.graphics.Typeface.MONOSPACE);
            android.view.ViewGroup.MarginLayoutParams lp =
                new LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.WRAP_CONTENT);
            addView(label, lp);
        }

        void setActive(boolean active) {
            glyph.setTextColor(active ? TEXT_ACT : TEXT_MUTED);
            glyph.setTextSize(active ? 26 : 22);
            label.setTextColor(active ? TEXT_ACT : TEXT_MUTED);
        }

        void animateActivate() {
            // Grow glyph with overshoot
            ObjectAnimator textSize = ObjectAnimator.ofFloat(glyph, "textSize", 22f, 26f);
            textSize.setDuration(250);
            textSize.setInterpolator(new OvershootInterpolator(2.8f));
            textSize.start();
            glyph.animate().scaleX(1.0f).scaleY(1.0f)
                .setDuration(250)
                .setInterpolator(new OvershootInterpolator(2.8f)).start();
            // Colour to Lavender
            glyph.setTextColor(TEXT_ACT);
            label.setTextColor(TEXT_ACT);
        }

        void animateDeactivate() {
            ObjectAnimator textSize = ObjectAnimator.ofFloat(glyph, "textSize", 26f, 22f);
            textSize.setDuration(200);
            textSize.setInterpolator(new OvershootInterpolator(2.0f));
            textSize.start();
            glyph.setTextColor(TEXT_MUTED);
            label.setTextColor(TEXT_MUTED);
        }
    }

    private int dp(int v) {
        return Math.round(v * getResources().getDisplayMetrics().density);
    }
    private float dp(float v) {
        return v * getResources().getDisplayMetrics().density;
    }
}
