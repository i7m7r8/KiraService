package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;

import java.util.Random;

/**
 * StarFieldView — pure moving star field, Catppuccin Mocha palette.
 * No nebulae. No galaxy branding. Just stars.
 * Colors: Crust bg, Text/Lavender/Mauve/Sapphire star tints.
 */
public class GalaxyView extends View {

    private static final int STAR_COUNT = 80;

    private final float[] sx  = new float[STAR_COUNT]; // 0..1 normalized
    private final float[] sy  = new float[STAR_COUNT]; // 0..1 normalized
    private final float[] sr  = new float[STAR_COUNT]; // radius px
    private final float[] sv  = new float[STAR_COUNT]; // vertical drift speed
    private final float[] sph = new float[STAR_COUNT]; // twinkle phase 0..2π
    private final int[]   sc  = new int[STAR_COUNT];   // color

    // Catppuccin Mocha star colors (varied opacity)
    private static final int[] STAR_COLORS = {
        0xCCCDD6F4, // Text
        0xAAB4BEFE, // Lavender
        0x99CBA6F7, // Mauve
        0xBB89DCEB, // Sky
        0xAA74C7EC, // Sapphire
        0xFF7F849C, // Overlay1 (dim)
        0xFFBAC2DE, // Subtext1
    };

    private float parallaxX = 0f;
    private float parallaxY = 0f;
    private long lastTick = 0L;
    private boolean seeded = false;

    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);

    public GalaxyView(Context c) { super(c); init(); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); init(); }

    private void init() {
        setLayerType(LAYER_TYPE_HARDWARE, null);
        paint.setStyle(Paint.Style.FILL);
    }

    public void setParallax(float px, float py) {
        parallaxX = px; parallaxY = py;
    }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0 && !seeded) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xCAFE_BEBE_L);
        for (int i = 0; i < STAR_COUNT; i++) {
            sx[i]  = rng.nextFloat();
            sy[i]  = rng.nextFloat();
            sr[i]  = 0.6f + rng.nextFloat() * 1.8f;
            sv[i]  = 0.00004f + rng.nextFloat() * 0.00012f; // normalized/ms
            sph[i] = rng.nextFloat() * (float)(Math.PI * 2);
            sc[i]  = STAR_COLORS[rng.nextInt(STAR_COLORS.length)];
        }
        seeded = true;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        int w = getWidth(), h = getHeight();
        if (w == 0 || h == 0) return;
        if (!seeded) seed(w, h);

        long now = System.currentTimeMillis();
        float dt = lastTick == 0 ? 16f : Math.min(now - lastTick, 50f);
        lastTick = now;
        float t = now * 0.001f;

        // Background — Catppuccin Crust
        canvas.drawColor(0xFF11111B);

        float px = parallaxX * w * 0.04f;
        float py = parallaxY * h * 0.04f;

        for (int i = 0; i < STAR_COUNT; i++) {
            // Drift upward slowly, wrap around
            sy[i] -= sv[i] * dt;
            if (sy[i] < 0f) { sy[i] = 1f; sx[i] = (float)(Math.random()); }

            float x = sx[i] * w + px;
            float y = sy[i] * h + py;

            // Twinkle — modulate alpha
            float twinkle = 0.65f + 0.35f * (float)Math.sin(t * 1.4f + sph[i]);
            int base = sc[i];
            int alpha = (int)(((base >>> 24) & 0xFF) * twinkle);
            int color = (alpha << 24) | (base & 0x00FFFFFF);

            paint.setColor(color);
            canvas.drawCircle(x, y, sr[i], paint);

            // Soft glow on larger stars
            if (sr[i] > 1.5f) {
                paint.setColor((alpha / 4 << 24) | (base & 0x00FFFFFF));
                canvas.drawCircle(x, y, sr[i] * 2.2f, paint);
            }
        }

        postInvalidateDelayed(32); // ~30fps
    }
}
