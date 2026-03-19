package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;
import java.util.Random;

/**
 * GalaxyView — static star field, Catppuccin Mocha palette.
 * NO continuous animation (removed postInvalidateDelayed).
 * Draws once, stays static. Parallax updates on sensor events.
 */
public class GalaxyView extends View {

    private static final int STAR_COUNT = 80;

    private final float[] sx  = new float[STAR_COUNT];
    private final float[] sy  = new float[STAR_COUNT];
    private final float[] sr  = new float[STAR_COUNT];
    private final int[]   sc  = new int[STAR_COUNT];

    private boolean seeded = false;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);

    // Catppuccin Mocha star tints
    private static final int[] COLORS = {
        0xCCCDD6F4, // Text
        0xAAB4BEFE, // Lavender
        0x88CBA6F7, // Mauve
        0xBB89DCEB, // Sky
        0xAABAC2DE, // Subtext1
        0x99A6ADC8, // Subtext0
    };

    public GalaxyView(Context c) { super(c); paint.setStyle(Paint.Style.FILL); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); paint.setStyle(Paint.Style.FILL); }

    // Called by MainActivity on sensor — just triggers a redraw
    public void setParallax(float px, float py) { invalidate(); }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xCAFEBABEL);
        for (int i = 0; i < STAR_COUNT; i++) {
            sx[i] = rng.nextFloat() * w;
            sy[i] = rng.nextFloat() * h;
            sr[i] = 0.5f + rng.nextFloat() * 1.8f;
            sc[i] = COLORS[rng.nextInt(COLORS.length)];
        }
        seeded = true;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        // Catppuccin Crust background
        canvas.drawColor(0xFF11111B);

        int w = getWidth(), h = getHeight();
        if (!seeded && w > 0 && h > 0) seed(w, h);
        if (!seeded) return;

        for (int i = 0; i < STAR_COUNT; i++) {
            paint.setColor(sc[i]);
            canvas.drawCircle(sx[i], sy[i], sr[i], paint);
        }
        // NO postInvalidateDelayed — static render only
    }
}
