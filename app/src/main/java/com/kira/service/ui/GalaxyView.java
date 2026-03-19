package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RadialGradient;
import android.graphics.Shader;
import android.util.AttributeSet;
import android.view.View;
import java.util.Random;

/**
 * Layer 0 — The Living Canvas.
 *
 * Three depth layers of stars with gyroscope parallax (0.3×, 0.8×, 1.5×).
 * Chromatic hue pulse synchronized to Rust engine uptime.
 * Vortex drift when Kira is thinking, burst explosion on reply.
 */
public class GalaxyView extends View {

    // ── Star counts per depth layer ────────────────────────────────────────
    private static final int NEAR_COUNT = 25;   // fast parallax, large
    private static final int MID_COUNT  = 50;   // medium
    private static final int FAR_COUNT  = 80;   // barely moves, tiny
    private static final int TOTAL      = NEAR_COUNT + MID_COUNT + FAR_COUNT;

    // Star positions (normalised 0-1), radii, base hues
    private final float[] sx      = new float[TOTAL];
    private final float[] sy      = new float[TOTAL];
    private final float[] sr      = new float[TOTAL];
    private final float[] sHue    = new float[TOTAL]; // base hue 0-360
    private final float[] sAlpha  = new float[TOTAL]; // base alpha 0-1
    private final int[]   sLayer  = new int[TOTAL];   // 0=far,1=mid,2=near

    // Parallax multipliers per layer
    private static final float[] PARALLAX = {0.3f, 0.8f, 1.5f};

    // Current smoothed parallax offset from accelerometer (in px)
    private float parallaxX = 0f;
    private float parallaxY = 0f;

    // Chromatic pulse phase 0-1 (driven by Rust uptime, period 3s)
    private float pulsePhase = 0f;
    private float pulseBpm   = 60f;

    // Vortex state: intensity 0-1 (thinking) → stars drift toward center
    private float vortexIntensity = 0f;

    // Burst state: 0 = idle, >0 = burst in progress (counts down)
    private float burstStrength   = 0f;
    private static final float BURST_DECAY = 0.035f;

    // Per-star animated offsets (vortex + burst)
    private final float[] driftX  = new float[TOTAL];
    private final float[] driftY  = new float[TOTAL];

    private boolean seeded = false;
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);

    // Catppuccin Mocha base hues for star colours
    // Lavender=240°, Mauve=267°, Sky=200°, Sapphire=212°, Text=230°, Subtext=220°
    private static final float[] BASE_HUES = { 240f, 267f, 200f, 212f, 230f, 220f };

    public GalaxyView(Context c) { super(c); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); }

    // ── Public API ────────────────────────────────────────────────────────

    /** Called by MainActivity on each sensor event with Rust-smoothed values */
    public void setParallax(float px, float py) {
        // Low-pass filter to prevent jitter
        parallaxX = parallaxX * 0.7f + px * 0.3f;
        parallaxY = parallaxY * 0.7f + py * 0.3f;
    }

    /** Called by MainActivity every 500ms from /theme/anim poll */
    public void setAnimState(float phase, float bpm, float activity, boolean thinking) {
        pulsePhase      = phase;
        pulseBpm        = bpm;
        vortexIntensity = thinking ? Math.min(vortexIntensity + 0.08f, 1.0f)
                                   : Math.max(vortexIntensity - 0.05f, 0.0f);
        postInvalidate();
    }

    /** Trigger burst explosion — called when Kira finishes responding */
    public void triggerBurst() {
        burstStrength = 1.0f;
        // Reset drift so stars spring outward from current position
        for (int i = 0; i < TOTAL; i++) {
            driftX[i] = 0f;
            driftY[i] = 0f;
        }
        postInvalidate();
    }

    // ── Seeding ───────────────────────────────────────────────────────────

    @Override
    protected void onSizeChanged(int w, int h, int ow, int oh) {
        if (w > 0 && h > 0) seed(w, h);
    }

    private void seed(int w, int h) {
        Random rng = new Random(0xB4BEFEL); // deterministic
        for (int i = 0; i < TOTAL; i++) {
            sx[i]    = rng.nextFloat();
            sy[i]    = rng.nextFloat();
            sHue[i]  = BASE_HUES[rng.nextInt(BASE_HUES.length)] + (rng.nextFloat() * 20f - 10f);
            sAlpha[i]= 0.4f + rng.nextFloat() * 0.6f;
            // Assign layer and radius
            if (i < FAR_COUNT) {
                sLayer[i] = 0; // far
                sr[i] = 0.4f + rng.nextFloat() * 0.6f;
            } else if (i < FAR_COUNT + MID_COUNT) {
                sLayer[i] = 1; // mid
                sr[i] = 0.7f + rng.nextFloat() * 1.0f;
            } else {
                sLayer[i] = 2; // near
                sr[i] = 1.2f + rng.nextFloat() * 1.8f;
            }
            driftX[i] = 0f;
            driftY[i] = 0f;
        }
        seeded = true;
    }

    // ── Drawing ───────────────────────────────────────────────────────────

    @Override
    protected void onDraw(Canvas canvas) {
        int w = getWidth(), h = getHeight();
        if (!seeded && w > 0 && h > 0) seed(w, h);
        if (!seeded) return;

        // Background: Catppuccin Base #1E1E2E
        canvas.drawColor(0xFF1E1E2E);

        float cx = w * 0.5f;
        float cy = h * 0.5f;

        // Chromatic pulse: hue shift ±12° in a sine wave driven by pulsePhase
        float hueShift = (float)(Math.sin(pulsePhase * 2.0 * Math.PI) * 12.0);

        // Burst: decay this frame
        if (burstStrength > 0) {
            burstStrength = Math.max(0f, burstStrength - BURST_DECAY);
        }

        for (int i = 0; i < TOTAL; i++) {
            int layer = sLayer[i];
            float pMult = PARALLAX[layer];

            // Base position in pixels
            float bx = sx[i] * w;
            float by = sy[i] * h;

            // Parallax offset (accelerometer tilt)
            float ox = parallaxX * pMult * 18f; // max ±18px for near layer
            float oy = parallaxY * pMult * 18f;

            // Vortex: drift toward center proportional to layer depth + phase
            if (vortexIntensity > 0) {
                float toCx = cx - bx;
                float toCy = cy - by;
                float vSpeed = vortexIntensity * 0.004f * (layer + 1);
                // Slight tangential component for spiral effect
                float tang = 0.0015f * vortexIntensity;
                driftX[i] += toCx * vSpeed - toCy * tang;
                driftY[i] += toCy * vSpeed + toCx * tang;
                // Dampen drift so stars don't all collapse
                driftX[i] *= 0.95f;
                driftY[i] *= 0.95f;
            } else {
                // Return to origin when not thinking
                driftX[i] *= 0.92f;
                driftY[i] *= 0.92f;
            }

            // Burst: push stars outward from center
            if (burstStrength > 0) {
                float fromCx = bx - cx;
                float fromCy = by - cy;
                float dist = (float)Math.sqrt(fromCx * fromCx + fromCy * fromCy);
                if (dist > 1f) {
                    float burst = burstStrength * 28f * pMult / dist;
                    driftX[i] += fromCx * burst * 0.012f;
                    driftY[i] += fromCy * burst * 0.012f;
                }
            }

            float fx = bx + ox + driftX[i];
            float fy = by + oy + driftY[i];

            // Wrap around screen edges
            fx = ((fx % w) + w) % w;
            fy = ((fy % h) + h) % h;

            // Animated hue + alpha (twinkle via phase offset per star)
            float starPhaseOffset = sx[i] * 0.4f + sy[i] * 0.3f; // unique per star
            float twinklePhase = (pulsePhase + starPhaseOffset) % 1.0f;
            float twinkle = 0.7f + (float)(Math.sin(twinklePhase * 2.0 * Math.PI)) * 0.3f;

            float finalHue  = ((sHue[i] + hueShift) % 360f + 360f) % 360f;
            float finalAlpha= sAlpha[i] * twinkle;

            // Near-layer stars get a subtle glow on burst
            float radius = sr[i];
            if (layer == 2 && burstStrength > 0.3f) {
                radius += burstStrength * 1.5f;
                finalAlpha = Math.min(1f, finalAlpha + burstStrength * 0.4f);
            }

            paint.setColor(hsvToArgb(finalHue, 0.35f, 0.95f, finalAlpha));
            canvas.drawCircle(fx, fy, radius, paint);
        }

        // Schedule next frame — use variable rate: fast when animating, slow when idle
        boolean animating = vortexIntensity > 0.01f || burstStrength > 0.01f
                         || Math.abs(parallaxX) > 0.01f || Math.abs(parallaxY) > 0.01f;
        postInvalidateDelayed(animating ? 16L : 500L); // 60fps when live, 2fps idle
    }

    // ── Colour helpers ────────────────────────────────────────────────────

    /** Convert HSV + alpha to ARGB int */
    private static int hsvToArgb(float h, float s, float v, float a) {
        float[] hsv = { h, s, v };
        int rgb = android.graphics.Color.HSVToColor(hsv);
        int alpha = Math.min(255, Math.max(0, (int)(a * 255)));
        return (alpha << 24) | (rgb & 0x00FFFFFF);
    }
}
