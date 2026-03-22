package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.util.AttributeSet;
import android.view.View;
import java.util.Random;

/**
 * Layer 0 — The Living Canvas.
 *
 * Driven entirely by Rust /layer0 endpoint (polled every 500ms by MainActivity).
 * Three depth layers: FAR(80) 0.3×, MID(50) 0.8×, NEAR(25) 1.5× parallax.
 * Chromatic hue pulse from Rust hue_shift (±12°, 3s sine).
 * Vortex (stars drift inward) driven by Rust vortex field.
 * Burst explosion when Rust thinking flips false after being true.
 */
public class GalaxyView extends View {

    private static final int NEAR_COUNT = 25;
    private static final int MID_COUNT  = 50;
    private static final int FAR_COUNT  = 80;
    private static final int TOTAL      = NEAR_COUNT + MID_COUNT + FAR_COUNT;

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

    // Rust-driven state (set via setAnimState)
    private float rustHueShift    = 0f;   // ±12 degrees from Rust
    private float rustVortex      = 0f;   // 0-1 vortex intensity from Rust
    private float burstStrength   = 0f;   // decays each frame
    private boolean wasThinking   = false;
    private static final float BURST_DECAY = 0.04f;

    private boolean seeded = false;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);

    // Catppuccin Mocha base hues
    private static final float[] BASE_HUES = {240f, 267f, 200f, 212f, 230f, 220f};

    public GalaxyView(Context c) { super(c); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); }

    // ── Public API ─────────────────────────────────────────────────────────

    public void setParallax(float px, float py) {
        parallaxX = parallaxX * 0.7f + px * 0.3f;
        parallaxY = parallaxY * 0.7f + py * 0.3f;
    }

    /**
     * Called by MainActivity every 500ms with data from Rust /layer0.
     * @param hueShift  ±12 degrees from Rust sine oscillator
     * @param vortex    0-1 vortex intensity
     * @param activity  0-1 activity level (drives pulse speed)
     * @param thinking  true = vortex on, false = if was thinking, trigger burst
     */
    /** Called directly by MainActivity when Kira finishes replying */
    public void triggerBurst() {
        burstStrength = 1.0f;
        for (int i = 0; i < TOTAL; i++) { driftX[i] = 0f; driftY[i] = 0f; }
        postInvalidate();
    }

    public void setAnimState(float hueShift, float vortex, float activity, boolean thinking) {
        // Detect thinking → false transition → trigger burst
        if (wasThinking && !thinking) {
            burstStrength = 1.0f;
        }
        wasThinking   = thinking;
        rustHueShift  = hueShift;
        // Vortex: ramp toward target smoothly
        float targetVortex = thinking ? vortex : Math.max(0f, vortex * 0.3f);
        rustVortex = rustVortex * 0.85f + targetVortex * 0.15f;
        postInvalidate();
    }

    // ── Seeding ────────────────────────────────────────────────────────────

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xB4BEFEL);
        for (int i = 0; i < TOTAL; i++) {
            sx[i]    = rng.nextFloat();
            sy[i]    = rng.nextFloat();
            sHue[i]  = BASE_HUES[rng.nextInt(BASE_HUES.length)] + rng.nextFloat() * 20f - 10f;
            sAlpha[i]= 0.4f + rng.nextFloat() * 0.6f;
            if      (i < FAR_COUNT)              { sLayer[i] = 0; sr[i] = 0.4f + rng.nextFloat() * 0.6f; }
            else if (i < FAR_COUNT + MID_COUNT)  { sLayer[i] = 1; sr[i] = 0.7f + rng.nextFloat() * 1.0f; }
            else                                 { sLayer[i] = 2; sr[i] = 1.2f + rng.nextFloat() * 1.8f; }
            driftX[i] = driftY[i] = 0f;
        }
        seeded = true;
    }

    // ── Drawing ────────────────────────────────────────────────────────────

    @Override
    protected void onDraw(Canvas canvas) {
        int w = getWidth(), h = getHeight();
        if (!seeded && w > 0) seed(w, h);
        if (!seeded) return;

        canvas.drawColor(0xFF1E1E2E); // Catppuccin Base

        float cx = w * 0.5f, cy = h * 0.5f;
        if (burstStrength > 0) burstStrength = Math.max(0f, burstStrength - BURST_DECAY);

        for (int i = 0; i < TOTAL; i++) {
            int   layer  = sLayer[i];
            float pMult  = PARALLAX[layer];

            float bx = sx[i] * w;
            float by = sy[i] * h;

            // Parallax from accelerometer
            float ox = parallaxX * pMult * 18f;
            float oy = parallaxY * pMult * 18f;

            // Vortex: spiral toward center — Rust drives intensity
            if (rustVortex > 0.01f) {
                float toCx  = (cx - bx) * rustVortex * 0.005f * (layer + 1);
                float toCy  = (cy - by) * rustVortex * 0.005f * (layer + 1);
                float tang  = rustVortex * 0.002f;
                driftX[i]  += toCx - (by - cy) * tang;
                driftY[i]  += toCy + (bx - cx) * tang;
                driftX[i]  *= 0.94f;
                driftY[i]  *= 0.94f;
            } else {
                driftX[i] *= 0.90f;
                driftY[i] *= 0.90f;
            }

            // Burst: push outward from center
            if (burstStrength > 0.01f) {
                float fromCx = bx - cx, fromCy = by - cy;
                float dist   = (float) Math.sqrt(fromCx * fromCx + fromCy * fromCy);
                if (dist > 1f) {
                    float f = burstStrength * 30f * pMult / dist;
                    driftX[i] += fromCx * f * 0.015f;
                    driftY[i] += fromCy * f * 0.015f;
                }
            }

            float fx = ((bx + ox + driftX[i]) % w + w) % w;
            float fy = ((by + oy + driftY[i]) % h + h) % h;

            // Per-star twinkle phase offset
            float twinklePhase = (sx[i] * 0.4f + sy[i] * 0.3f);
            float twinkle = 0.7f + (float)(Math.sin(twinklePhase * 6.28 + rustHueShift * 0.05)) * 0.3f;

            // Hue from Rust (direct, no local calculation)
            float finalHue   = ((sHue[i] + rustHueShift) % 360f + 360f) % 360f;
            float finalAlpha = sAlpha[i] * twinkle;
            float radius     = sr[i];

            if (layer == 2 && burstStrength > 0.3f) {
                radius     += burstStrength * 1.5f;
                finalAlpha  = Math.min(1f, finalAlpha + burstStrength * 0.4f);
            }

            paint.setColor(hsvToArgb(finalHue, 0.35f, 0.95f, finalAlpha));
            canvas.drawCircle(fx, fy, radius, paint);
        }

        boolean animating = rustVortex > 0.01f || burstStrength > 0.01f
                         || Math.abs(parallaxX) > 0.005f || Math.abs(parallaxY) > 0.005f;
        postInvalidateDelayed(animating ? 16L : 500L);
    }

    private static int hsvToArgb(float h, float s, float v, float a) {
        float[] hsv = {h, s, v};
        int rgb = android.graphics.Color.HSVToColor(hsv);
        int alpha = Math.min(255, Math.max(0, (int)(a * 255)));
        return (alpha << 24) | (rgb & 0x00FFFFFF);
    }
}
