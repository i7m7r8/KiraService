package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;
import java.util.Random;

/**
 * GalaxyView — Catppuccin Mocha star field.
 * Static by default. Animates only when Rust signals activity.
 * NO continuous postInvalidateDelayed loop — driven externally.
 */
public class GalaxyView extends View {

    private static final int NEAR = 25, MID = 50, FAR = 80, TOTAL = 155;
    private static final float[] PARALLAX = {0.3f, 0.8f, 1.5f};

    private final float[] sx     = new float[TOTAL];
    private final float[] sy     = new float[TOTAL];
    private final float[] sr     = new float[TOTAL];
    private final float[] sHue   = new float[TOTAL];
    private final float[] sAlpha = new float[TOTAL];
    private final int[]   sLayer = new int[TOTAL];
    private final float[] driftX = new float[TOTAL];
    private final float[] driftY = new float[TOTAL];

    private float parallaxX = 0f, parallaxY = 0f;
    private float rustHueShift = 0f;
    private float rustVortex   = 0f;
    private float burstStrength = 0f;
    private boolean seeded = false;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);

    private static final float[] BASE_HUES = {240f, 267f, 200f, 212f, 230f, 220f};

    public GalaxyView(Context c) { super(c); paint.setStyle(Paint.Style.FILL); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); paint.setStyle(Paint.Style.FILL); }

    private long lastInvalidateMs = 0;

    public void setParallax(float px, float py) {
        parallaxX = parallaxX * 0.7f + px * 0.3f;
        parallaxY = parallaxY * 0.7f + py * 0.3f;
        // Throttle redraws to 30fps max — sensor fires at up to 50Hz
        long now = System.currentTimeMillis();
        if (now - lastInvalidateMs > 32) {
            lastInvalidateMs = now;
            postInvalidate();
        }
    }

    public void triggerBurst() {
        burstStrength = 1.0f;
        for (int i = 0; i < TOTAL; i++) { driftX[i] = 0f; driftY[i] = 0f; }
        postInvalidate();
    }

    public void setAnimState(float hueShift, float vortex, float activity, boolean thinking) {
        rustHueShift = hueShift;
        rustVortex   = thinking ? vortex : Math.max(0f, vortex * 0.3f);
        postInvalidate(); // one redraw per poll cycle (every 500ms)
    }

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xB4BEFEL);
        for (int i = 0; i < TOTAL; i++) {
            sx[i]     = rng.nextFloat();
            sy[i]     = rng.nextFloat();
            sHue[i]   = BASE_HUES[rng.nextInt(BASE_HUES.length)] + rng.nextFloat() * 20f - 10f;
            sAlpha[i] = 0.4f + rng.nextFloat() * 0.6f;
            if      (i < FAR)       { sLayer[i] = 0; sr[i] = 0.4f + rng.nextFloat() * 0.6f; }
            else if (i < FAR + MID) { sLayer[i] = 1; sr[i] = 0.7f + rng.nextFloat() * 1.0f; }
            else                    { sLayer[i] = 2; sr[i] = 1.2f + rng.nextFloat() * 1.8f; }
            driftX[i] = driftY[i] = 0f;
        }
        seeded = true;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        int w = getWidth(), h = getHeight();
        if (!seeded && w > 0) seed(w, h);
        if (!seeded) return;

        canvas.drawColor(0xFF1E1E2E); // Catppuccin Base

        float cx = w * 0.5f, cy = h * 0.5f;
        if (burstStrength > 0) burstStrength = Math.max(0f, burstStrength - 0.04f);

        for (int i = 0; i < TOTAL; i++) {
            int   layer = sLayer[i];
            float pMult = PARALLAX[layer];
            float bx    = sx[i] * w;
            float by    = sy[i] * h;

            float ox = parallaxX * pMult * 18f;
            float oy = parallaxY * pMult * 18f;

            if (rustVortex > 0.01f) {
                float toCx = (cx - bx) * rustVortex * 0.005f * (layer + 1);
                float toCy = (cy - by) * rustVortex * 0.005f * (layer + 1);
                float tang = rustVortex * 0.002f;
                driftX[i] += toCx - (by - cy) * tang;
                driftY[i] += toCy + (bx - cx) * tang;
                driftX[i] *= 0.94f; driftY[i] *= 0.94f;
            } else {
                driftX[i] *= 0.90f; driftY[i] *= 0.90f;
            }

            if (burstStrength > 0.01f) {
                float fromCx = bx - cx, fromCy = by - cy;
                float dist = (float) Math.sqrt(fromCx * fromCx + fromCy * fromCy);
                if (dist > 1f) {
                    float f = burstStrength * 30f * pMult / dist;
                    driftX[i] += fromCx * f * 0.015f;
                    driftY[i] += fromCy * f * 0.015f;
                }
            }

            float fx = ((bx + ox + driftX[i]) % w + w) % w;
            float fy = ((by + oy + driftY[i]) % h + h) % h;

            float phase    = (sx[i] * 0.4f + sy[i] * 0.3f);
            float twinkle  = 0.7f + (float)(Math.sin(phase * 6.28 + rustHueShift * 0.05)) * 0.3f;
            float finalHue = ((sHue[i] + rustHueShift) % 360f + 360f) % 360f;
            float finalAlpha = sAlpha[i] * twinkle;
            float radius = sr[i];

            if (layer == 2 && burstStrength > 0.3f) {
                radius     += burstStrength * 1.5f;
                finalAlpha  = Math.min(1f, finalAlpha + burstStrength * 0.4f);
            }

            paint.setColor(hsvToArgb(finalHue, 0.35f, 0.95f, finalAlpha));
            canvas.drawCircle(fx, fy, radius, paint);
        }

        // CRITICAL: NO postInvalidateDelayed here.
        // Only redraw when setAnimState / triggerBurst / setParallax is called.
    }

    private static int hsvToArgb(float h, float s, float v, float a) {
        float[] hsv = {h, s, v};
        int rgb = android.graphics.Color.HSVToColor(hsv);
        int alpha = Math.min(255, Math.max(0, (int)(a * 255)));
        return (alpha << 24) | (rgb & 0x00FFFFFF);
    }
}
