package com.kira.service.ui;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.RadialGradient;
import android.graphics.Shader;
import android.util.AttributeSet;
import android.view.View;

/**
 * Full-screen star field driven by Rust state.
 * Rust owns: star positions, parallax offsets, twinkle phases.
 * Java only calls canvas - zero UI logic here.
 *
 * Stars are rendered in 3 layers (depth) for parallax:
 *   Layer 0 - distant, tiny, slow
 *   Layer 1 - mid,     small, medium
 *   Layer 2 - near,    bright, fast
 *
 * Tilt from accelerometer -> Rust smooths with EMA -> Java reads offsets.
 */
public class GalaxyView extends View {

    // \u2500\u2500 Star data owned by Rust, mirrored here after each frame \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
    private static final int STAR_COUNT = 110;

    private final float[] sx   = new float[STAR_COUNT]; // normalised 0..1
    private final float[] sy   = new float[STAR_COUNT];
    private final float[] sr   = new float[STAR_COUNT]; // radius
    private final int[]   sl   = new int[STAR_COUNT];   // layer 0/1/2
    private final float[] sph  = new float[STAR_COUNT]; // twinkle phase

    // Parallax offsets from Rust (smoothed)
    private float parallaxX = 0f;
    private float parallaxY = 0f;

    // Nebula blobs (static, decorative)
    private static final float[][] NEBULAE = {
        {0.15f, 0.20f, 0.28f, 0xFFDC143C},  // crimson top-left
        {0.80f, 0.35f, 0.22f, 0xFF1a003a},  // deep violet right
        {0.45f, 0.70f, 0.18f, 0xFF0a0030},  // indigo mid-low
        {0.65f, 0.10f, 0.15f, 0xFF2a0020},  // dark crimson top
    };

    private final Paint starPaint  = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint glowPaint  = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint nebPaint   = new Paint(Paint.ANTI_ALIAS_FLAG);
    private long tickMs = 0L;
    private boolean seeded = false;

    public GalaxyView(Context c) { super(c); init(); }
    public GalaxyView(Context c, AttributeSet a) { super(c, a); init(); }

    private void init() {
        setLayerType(LAYER_TYPE_HARDWARE, null);
        starPaint.setStyle(Paint.Style.FILL);
        glowPaint.setStyle(Paint.Style.FILL);
        nebPaint.setStyle(Paint.Style.FILL);
    }

    /** Called by MainActivity on each sensor update with Rust-smoothed offsets */
    public void setParallax(float px, float py) {
        parallaxX = px;
        parallaxY = py;
        invalidate();
    }

    /** Seed star positions - called once from Rust JSON, or fallback random */
    public void seedStars(float[] x, float[] y, float[] r, int[] layer, float[] phase) {
        int n = Math.min(STAR_COUNT, x.length);
        System.arraycopy(x, 0, sx, 0, n);
        System.arraycopy(y, 0, sy, 0, n);
        System.arraycopy(r, 0, sr, 0, n);
        System.arraycopy(layer, 0, sl, 0, n);
        System.arraycopy(phase, 0, sph, 0, n);
        seeded = true;
        invalidate();
    }

    /** Fallback: generate deterministic stars if Rust not ready */
    private void ensureSeeded() {
        if (seeded) return;
        java.util.Random rng = new java.util.Random(0xCAFEBABEL);
        for (int i = 0; i < STAR_COUNT; i++) {
            sx[i] = rng.nextFloat();
            sy[i] = rng.nextFloat();
            int layer = i < 44 ? 0 : i < 77 ? 1 : 2;
            sl[i] = layer;
            sr[i] = layer == 0 ? 0.8f + rng.nextFloat() * 0.7f
                  : layer == 1 ? 1.2f + rng.nextFloat() * 1.0f
                  :              1.8f + rng.nextFloat() * 1.4f;
            sph[i] = rng.nextFloat() * 6.28318f;
        }
        seeded = true;
    }

    @Override
    protected void onDraw(Canvas canvas) {
        ensureSeeded();
        long now = System.currentTimeMillis();
        float t = (now % 8000) / 8000f; // 0..1 cycle for twinkle
        tickMs = now;

        int w = getWidth(), h = getHeight();
        if (w == 0 || h == 0) return;

        // \u2500\u2500 1. Deep space background \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        canvas.drawColor(0xFF050508);

        // \u2500\u2500 2. Nebula blobs (radial gradients) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        for (float[] neb : NEBULAE) {
            float nx = neb[0] * w, ny = neb[1] * h, nr = neb[2] * w;
            int col = (int) neb[3];
            int transparent = col & 0x00FFFFFF;
            RadialGradient rg = new RadialGradient(nx, ny, nr,
                new int[]{col | 0x22000000, col | 0x0A000000, transparent},
                new float[]{0f, 0.5f, 1f}, Shader.TileMode.CLAMP);
            nebPaint.setShader(rg);
            canvas.drawCircle(nx, ny, nr, nebPaint);
        }
        nebPaint.setShader(null);

        // \u2500\u2500 3. Stars with parallax layers \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        for (int i = 0; i < STAR_COUNT; i++) {
            int layer = sl[i];
            float mult = layer == 0 ? 0.3f : layer == 1 ? 0.65f : 1.0f;

            // Base position + parallax offset (layer-scaled)
            float ox = sx[i] * w + parallaxX * w * mult * 0.08f;
            float oy = sy[i] * h + parallaxY * h * mult * 0.08f;
            // Wrap around edges
            ox = ((ox % w) + w) % w;
            oy = ((oy % h) + h) % h;

            // Twinkle: sine wave per star with phase offset
            float twinkle = 0.4f + 0.6f * (float)(0.5 + 0.5 * Math.sin(
                t * 6.28318f * 2 + sph[i]));

            float radius = sr[i];

            if (layer == 2) {
                // Near stars: crimson glow halo
                int glowAlpha = (int)(twinkle * 55);
                int glowColor = (glowAlpha << 24) | 0xDC143C;
                RadialGradient rg = new RadialGradient(ox, oy, radius * 3.5f,
                    new int[]{glowColor, 0x00DC143C},
                    new float[]{0f, 1f}, Shader.TileMode.CLAMP);
                glowPaint.setShader(rg);
                canvas.drawCircle(ox, oy, radius * 3.5f, glowPaint);
                glowPaint.setShader(null);
                // Core
                int alpha = (int)(twinkle * 255);
                starPaint.setColor((alpha << 24) | 0xFFCCCC);
                canvas.drawCircle(ox, oy, radius * twinkle, starPaint);
            } else if (layer == 1) {
                // Mid stars: white-blue
                int alpha = (int)(twinkle * 200);
                starPaint.setColor((alpha << 24) | 0xAABBFF);
                canvas.drawCircle(ox, oy, radius * (0.7f + 0.3f * twinkle), starPaint);
            } else {
                // Distant: dim white
                int alpha = (int)(twinkle * 130);
                starPaint.setColor((alpha << 24) | 0xCCCCDD);
                canvas.drawCircle(ox, oy, radius, starPaint);
            }
        }

        // \u2500\u2500 4. Schedule next frame (60fps) \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500
        postInvalidateDelayed(16);
    }
}
