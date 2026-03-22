package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.util.AttributeSet;

/**
 * TextView with crimson neon glow effect.
 * Draws the text multiple times with decreasing alpha and increasing blur
 * to simulate a neon light bloom - pure Java canvas, no shader plugins.
 */
public class NeonTextView extends android.widget.TextView {

    private static final int GLOW_COLOR = 0xFFDC143C;
    private final Paint glowPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private boolean neonEnabled = true;

    public NeonTextView(Context c) { super(c); initNeon(); }
    public NeonTextView(Context c, AttributeSet a) { super(a == null ? c : c, a); initNeon(); }
    public NeonTextView(Context c, AttributeSet a, int s) { super(c, a, s); initNeon(); }

    private void initNeon() {
        setShadowLayer(12f, 0f, 0f, GLOW_COLOR);
    }

    public void setNeonEnabled(boolean e) {
        neonEnabled = e;
        if (e) setShadowLayer(12f, 0f, 0f, GLOW_COLOR);
        else   setShadowLayer(0f, 0f, 0f, Color.TRANSPARENT);
        invalidate();
    }
}
