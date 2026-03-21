package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;
import java.util.Random;

/**
 * Static Catppuccin Mocha star field.
 * NO animation loop — drawn once, stays static.
 */
public class GalaxyView extends View {

    private static final int STAR_COUNT = 90;
    private final float[] sx = new float[STAR_COUNT];
    private final float[] sy = new float[STAR_COUNT];
    private final float[] sr = new float[STAR_COUNT];
    private final int[]   sc = new int[STAR_COUNT];
    private boolean seeded = false;
    private final Paint p = new Paint(Paint.ANTI_ALIAS_FLAG);

    // Catppuccin Mocha: Text, Lavender, Mauve, Sky, Subtext1, Overlay1
    private static final int[] PALETTE = {
        0xCCCDD6F4, 0x99B4BEFE, 0x77CBA6F7,
        0x8889DCEB, 0xAABAC2DE, 0x88A6ADC8,
    };

    public GalaxyView(Context c) { super(c); p.setStyle(Paint.Style.FILL); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); p.setStyle(Paint.Style.FILL); }

    public void setParallax(float px, float py) { /* static — no-op */ }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xB4BEFEL);
        for (int i = 0; i < STAR_COUNT; i++) {
            sx[i] = rng.nextFloat() * w;
            sy[i] = rng.nextFloat() * h;
            sr[i] = 0.4f + rng.nextFloat() * 1.6f;
            sc[i] = PALETTE[rng.nextInt(PALETTE.length)];
        }
        seeded = true;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        canvas.drawColor(0xFF1E1E2E); // Catppuccin Base
        int w = getWidth(), h = getHeight();
        if (!seeded && w > 0 && h > 0) seed(w, h);
        if (!seeded) return;
        for (int i = 0; i < STAR_COUNT; i++) {
            p.setColor(sc[i]);
            canvas.drawCircle(sx[i], sy[i], sr[i], p);
        }
        // NO postInvalidateDelayed — static only
    }
}
