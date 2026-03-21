package com.kira.service;

import android.animation.Animator;
import android.animation.AnimatorListenerAdapter;
import android.animation.ObjectAnimator;
import android.animation.AnimatorSet;
import android.animation.ValueAnimator;
import android.app.Activity;
import android.content.Context;
import android.content.Intent;
import android.graphics.Canvas;
import android.graphics.Color;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.text.TextWatcher;
import android.text.Editable;
import android.view.Gravity;
import android.view.View;
import android.view.animation.DecelerateInterpolator;
import android.view.animation.OvershootInterpolator;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import com.kira.service.ai.KiraConfig;
import android.util.Log;

import java.util.Random;

/**
 * KiraService v38 \u2014 Setup wizard.
 * Catppuccin Mocha UI + animated star field (tilts with accelerometer) +
 * custom AI provider support.
 */
public class SetupActivity extends Activity implements SensorEventListener {

    private static final int TOTAL_PAGES = 6;

    private static final String[] QUOTES = {
        "The best way to predict the future is to create it.",
        "Intelligence is the ability to adapt to change.",
        "Automation elevates the human; it never replaces the soul.",
        "Any sufficiently advanced technology is indistinguishable from magic.",
        "We build the gods. We choose what they remember.",
        "The real problem is not whether machines think but whether men do.",
        "You have power over your mind \u2014 not outside events.",
        "Do not wait to strike till the iron is hot; make it hot by striking.",
        "The secret of getting ahead is getting started.",
        "Build something worthy of the future you imagine.",
    };

    // Catppuccin Mocha palette
    private static final int C_BG         = 0xFF1E1E2E; // Base
    private static final int C_CARD       = 0xFF181825; // Mantle
    private static final int C_SURFACE    = 0xFF313244; // Surface0
    private static final int C_SURFACE2   = 0xFF45475A; // Surface1
    private static final int C_ACCENT     = 0xFFB4BEFE; // Lavender
    private static final int C_ACCENT2    = 0xFFCBA6F7; // Mauve
    private static final int C_ACCENT_DIM = 0xFF2A2A40;
    private static final int C_TEXT       = 0xFFCDD6F4; // Text
    private static final int C_MUTED      = 0xFF9399B2; // Overlay2
    private static final int C_HINT       = 0xFF45475A; // Surface1
    private static final int C_SUCCESS    = 0xFFA6E3A1; // Green
    private static final int C_ERROR      = 0xFFF38BA8; // Pink
    private static final int C_PEACH      = 0xFFFAB387; // Peach

    private StarFieldView starField;
    private FrameLayout pageContainer;
    private LinearLayout dotsRow;
    private TextView nextBtn, skipBtn;
    private View[] dots;
    private int currentPage = 0;
    private View currentView;
    private KiraConfig cfg;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private SensorManager sensorManager;
    private Sensor accelerometer;

    private String apiKey = "", baseUrl = "", model = "", name = "", tgToken = "", tgId = "";
    private int quoteIndex = 0;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Skip setup if already done
        if (com.kira.service.ai.KiraConfig.load(this).setupDone) {
            startActivity(new android.content.Intent(this, MainActivity.class));
            finish(); return;
        }

        FrameLayout root = new FrameLayout(this);
        root.setBackgroundColor(C_BG);

        starField = new StarFieldView(this);
        root.addView(starField, new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT));

        pageContainer = new FrameLayout(this);
        pageContainer.setId(R.id.setupPageContainer);
        // Leave 140dp at the bottom for the nav buttons so they're never covered
        FrameLayout.LayoutParams pcLp = new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT);
        pcLp.bottomMargin = dp(140);
        root.addView(pageContainer, pcLp);

        dotsRow = new LinearLayout(this);
        dotsRow.setId(R.id.setupDots);
        dotsRow.setOrientation(LinearLayout.HORIZONTAL);
        dotsRow.setGravity(Gravity.CENTER);
        FrameLayout.LayoutParams dlp = new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT);
        dlp.gravity = Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL;
        dlp.bottomMargin = dp(120);
        root.addView(dotsRow, dlp);

        LinearLayout btnRow = new LinearLayout(this);
        btnRow.setOrientation(LinearLayout.HORIZONTAL);
        btnRow.setGravity(Gravity.CENTER_VERTICAL);
        btnRow.setPadding(dp(24), 0, dp(24), dp(40));
        FrameLayout.LayoutParams brlp = new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT, dp(92));
        brlp.gravity = Gravity.BOTTOM;
        root.addView(btnRow, brlp);

        skipBtn = new TextView(this);
        skipBtn.setId(R.id.setupSkipBtn);
        skipBtn.setText("Skip \u2192");
        skipBtn.setTextColor(0xFFAAAAAA);  // brighter than C_MUTED so it's always readable
        skipBtn.setTextSize(15);
        skipBtn.setGravity(Gravity.CENTER_VERTICAL | Gravity.START);
        skipBtn.setClickable(true);
        skipBtn.setFocusable(true);
        // Ensure tap target is large enough
        btnRow.addView(skipBtn, new LinearLayout.LayoutParams(0, dp(52), 1));

        nextBtn = new TextView(this);
        nextBtn.setId(R.id.setupNextBtn);
        nextBtn.setText("Get Started");
        nextBtn.setTextColor(C_TEXT);
        nextBtn.setTextSize(15);
        nextBtn.setTypeface(null, android.graphics.Typeface.BOLD);
        nextBtn.setGravity(Gravity.CENTER);
        // Layer 4: ripple from exact tap point
        android.graphics.drawable.GradientDrawable nextBg = new android.graphics.drawable.GradientDrawable();
        nextBg.setColor(C_ACCENT);
        nextBg.setCornerRadius(dp(12));
        android.content.res.ColorStateList ripple =
            android.content.res.ColorStateList.valueOf(0x33000000);
        android.graphics.drawable.RippleDrawable nextRipple =
            new android.graphics.drawable.RippleDrawable(ripple, nextBg, null);
        nextBtn.setBackground(nextRipple);
        nextBtn.setClickable(true);
        nextBtn.setFocusable(true);
        nextBtn.setPadding(dp(32), 0, dp(32), 0);
        btnRow.addView(nextBtn, new LinearLayout.LayoutParams(dp(170), dp(52)));

        setContentView(root);

        cfg = KiraConfig.load(this);
        buildDots();
        showPage(0, true);

        nextBtn.setOnClickListener(v -> advance());
        skipBtn.setOnClickListener(v -> {
            cfg.setupDone = true;
            cfg.save(this);
            startActivity(new Intent(this, MainActivity.class));
            finish();
        });
        handler.postDelayed(this::cycleQuote, 4000);

        sensorManager = (SensorManager) getSystemService(Context.SENSOR_SERVICE);
        if (sensorManager != null)
            accelerometer = sensorManager.getDefaultSensor(Sensor.TYPE_ACCELEROMETER);
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (accelerometer != null && sensorManager != null)
            sensorManager.registerListener(this, accelerometer, SensorManager.SENSOR_DELAY_GAME);
    }

    @Override
    protected void onPause() {
        super.onPause();
        if (sensorManager != null) sensorManager.unregisterListener(this);
    }

    @Override
    public void onSensorChanged(SensorEvent e) {
        if (e.sensor.getType() == Sensor.TYPE_ACCELEROMETER) {
            starField.onTilt(e.values[0], e.values[1]);
            // v38: Rust smooths the parallax; Java still draws independently
            // but Rust state is available for /theme/tilt endpoint
            try { RustBridge.updateTilt(e.values[0], e.values[1]); }
            catch (UnsatisfiedLinkError ignored) {}
        }
    }

    @Override public void onAccuracyChanged(Sensor s, int a) {}

    // \u2500\u2500 Star field \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    static class StarFieldView extends View {
        private static final int N = 110;
        private final float[] x = new float[N], y = new float[N], sz = new float[N],
                               br = new float[N], ph = new float[N];
        private final int[] clr = new int[N];
        private final android.graphics.Paint p = new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
        private float tiltX, tiltY, offX, offY;
        private final long t0 = System.currentTimeMillis();

        StarFieldView(Context ctx) {
            super(ctx);
            Random rnd = new Random(7);
            int[] palette = {0xFFCDD6F4, 0xFFB4BEFE, 0xFFCBA6F7, 0xFF89DCEB, 0xFF74C7EC, 0xFF9399B2};
            for (int i = 0; i < N; i++) {
                x[i] = rnd.nextFloat(); y[i] = rnd.nextFloat();
                sz[i] = 0.7f + rnd.nextFloat() * 2.2f;
                br[i] = 0.3f + rnd.nextFloat() * 0.7f;
                ph[i] = rnd.nextFloat() * 6.28f;
                clr[i] = palette[rnd.nextInt(palette.length)];
            }
            ValueAnimator va = ValueAnimator.ofFloat(0f, 1f);
            va.setDuration(60); va.setRepeatCount(ValueAnimator.INFINITE);
            va.addUpdateListener(a -> invalidate());
            va.start();
        }

        void onTilt(float ax, float ay) { tiltX = ax; tiltY = ay; }

        @Override
        protected void onDraw(Canvas c) {
            int w = getWidth(), h = getHeight();
            if (w == 0) return;
            offX += (-tiltX * 0.013f - offX) * 0.07f;
            offY += ( tiltY * 0.013f - offY) * 0.07f;
            float t = (System.currentTimeMillis() - t0) / 1000f;
            for (int i = 0; i < N; i++) {
                float fx = (x[i] + offX * (sz[i] / 3f) + 1.5f) % 1f;
                float fy = (y[i] + offY * (sz[i] / 3f) + 1.5f) % 1f;
                float tw = 0.55f + 0.45f * (float)Math.sin(t * 1.2f + ph[i]);
                int alpha = (int)(br[i] * tw * 210);
                p.setColor((clr[i] & 0x00FFFFFF) | (alpha << 24));
                float px = fx * w, py = fy * h;
                c.drawCircle(px, py, sz[i] * 0.5f, p);
                if (sz[i] > 2f) {
                    p.setColor((clr[i] & 0x00FFFFFF) | (Math.max(0, alpha - 170) << 24));
                    c.drawCircle(px, py, sz[i] * 2.5f, p);
                }
            }
        }
    }

    // \u2500\u2500 Pages \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    private View buildPage(int page) {
        switch (page) {
            case 0:  return buildWelcomePage();
            case 1:  return buildApiKeyPage();
            case 2:  return buildNamePage();
            case 3:  return buildModelPage();
            case 4:  return buildTelegramPage();
            default: return buildReadyPage();
        }
    }

    private View buildWelcomePage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();

        TextView logo = new TextView(this);
        logo.setText("K");
        logo.setTextSize(80);
        logo.setTextColor(C_ACCENT);
        logo.setTypeface(null, android.graphics.Typeface.BOLD);
        logo.setGravity(Gravity.CENTER);
        logo.setBackgroundColor(C_ACCENT_DIM);
        logo.setShadowLayer(dp(18), 0, 0, C_ACCENT2);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(110), dp(110));
        lp.gravity = Gravity.CENTER_HORIZONTAL;
        lp.setMargins(0, 0, 0, dp(20));
        logo.setLayoutParams(lp);
        root.addView(logo);
        // Layer 4: K logo breath animation (scale 1.0→1.06→1.0, 3s)
        logo.post(() -> startLogoBreathe(logo));

        TextView title = bigText("Meet Kira");
        title.setShadowLayer(dp(10), 0, 0, C_ACCENT);
        root.addView(title);

        TextView sub = new TextView(this);
        sub.setText("Your autonomous AI agent for Android.\nNo root. No Termux. Just intelligence.");
        sub.setTextColor(C_MUTED);
        sub.setTextSize(15);
        sub.setGravity(Gravity.CENTER);
        sub.setLineSpacing(0, 1.4f);
        LinearLayout.LayoutParams sp = new LinearLayout.LayoutParams(MATCH, WRAP);
        sp.setMargins(dp(32), 0, dp(32), dp(36));
        sub.setLayoutParams(sp);
        root.addView(sub);

        // Quote card
        LinearLayout qcard = new LinearLayout(this);
        qcard.setOrientation(LinearLayout.HORIZONTAL);
        qcard.setBackgroundColor(C_CARD);
        qcard.setPadding(dp(16), dp(18), dp(20), dp(18));
        LinearLayout.LayoutParams qp = new LinearLayout.LayoutParams(MATCH, WRAP);
        qp.setMargins(dp(24), 0, dp(24), dp(28));
        qcard.setLayoutParams(qp);

        View bar = new View(this);
        bar.setBackgroundColor(C_ACCENT);
        qcard.addView(bar, new LinearLayout.LayoutParams(dp(3), dp(56)));

        TextView qt = new TextView(this);
        qt.setText(QUOTES[quoteIndex]);
        qt.setTextColor(0xFF9999BB);
        qt.setTextSize(13);
        qt.setLineSpacing(0, 1.5f);
        qt.setTag("quoteText");
        LinearLayout.LayoutParams qtlp = new LinearLayout.LayoutParams(0, WRAP, 1);
        qtlp.setMargins(dp(14), 0, 0, 0);
        qt.setLayoutParams(qtlp);
        qcard.addView(qt);
        root.addView(qcard);

        // Feature chips
        LinearLayout chips = new LinearLayout(this);
        chips.setOrientation(LinearLayout.HORIZONTAL);
        chips.setGravity(Gravity.CENTER);
        for (String[] chip : new String[][]{
                {"\uD83E\uDD16", "AI Agent"}, {"\uD83D\uDD2D", "Vision"},
                {"\u26A1", "Rust Core"},      {"\uD83D\uDD14", "Telegram"}}) {
            TextView tv = new TextView(this);
            tv.setText(chip[0] + " " + chip[1]);
            tv.setTextColor(C_MUTED);
            tv.setTextSize(11);
            tv.setBackgroundColor(C_CARD);
            tv.setPadding(dp(10), dp(7), dp(10), dp(7));
            LinearLayout.LayoutParams tp = new LinearLayout.LayoutParams(WRAP, WRAP);
            tp.setMargins(dp(4), 0, dp(4), 0);
            tv.setLayoutParams(tp);
            chips.addView(tv);
        }
        root.addView(chips);
        sv.addView(root);
        return sv;
    }

    private View buildApiKeyPage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83D\uDD11"));
        root.addView(bigText("AI Provider & Key"));
        root.addView(hint("Enter your API key. Choose a provider\nor add a custom endpoint."));

        EditText keyInput = inputField("sk-... or gsk-...", false);
        keyInput.setTag("apiKeyInput");
        if (!cfg.apiKey.isEmpty())
            keyInput.setText("*".repeat(Math.min(16, cfg.apiKey.length())));
        root.addView(keyInput);

        root.addView(sectionLabel("Quick-select provider:"));

        // Session J: providers loaded from Rust /setup/providers
        // Falls back to hardcoded list if Rust server not yet running
        String[][] providers = loadProvidersFromRust();
        if (providers == null || providers.length == 0) {
            providers = new String[][]{
                {"Groq (free)",    "https://api.groq.com/openai/v1"},
                {"OpenAI",         "https://api.openai.com/v1"},
                {"Anthropic",      "https://api.anthropic.com/v1"},
                {"Together AI",    "https://api.together.xyz/v1"},
                {"OpenRouter",     "https://openrouter.ai/api/v1"},
                {"Custom",         ""}
            };
        }

        // Custom URL field \u2014 hidden until Custom is tapped
        EditText customUrl = inputField("https://your-server/v1", false);
        customUrl.setTag("customUrlInput");
        customUrl.setVisibility(View.GONE);
        customUrl.addTextChangedListener(new TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int st, int c, int a) {}
            @Override public void onTextChanged(CharSequence s, int st, int b, int c) {
                baseUrl = s.toString().trim();
                // v38: live-update Rust custom provider URL
                if (!baseUrl.isEmpty()) {
                    try { RustBridge.setCustomProvider(baseUrl, ""); } catch (UnsatisfiedLinkError ignored) {}
                }
            }
            @Override public void afterTextChanged(Editable s) {}
        });

        LinearLayout chipsRow = new LinearLayout(this);
        chipsRow.setOrientation(LinearLayout.HORIZONTAL);
        chipsRow.setPadding(dp(24), 0, dp(24), 0);

        TextView[] chipViews = new TextView[providers.length];
        for (int i = 0; i < providers.length; i++) {
            final String label = providers[i][0];
            final String url   = providers[i][1];
            final boolean isCustom = url.equals("custom");
            TextView tv = new TextView(this);
            tv.setText(label);
            tv.setTextSize(12);
            tv.setTextColor(C_MUTED);
            tv.setBackgroundColor(C_CARD);
            tv.setPadding(dp(12), dp(8), dp(12), dp(8));
            LinearLayout.LayoutParams clp2 = new LinearLayout.LayoutParams(WRAP, WRAP);
            clp2.setMargins(0, 0, dp(8), 0);
            tv.setLayoutParams(clp2);
            tv.setClickable(true);
            chipViews[i] = tv;
            final int fi = i;
            // Layer 4: shimmer on idle chips (explore affordance)
            handler.postDelayed(() -> startShimmer(tv), fi * 200L);
            tv.setOnClickListener(v -> {
                // Stop shimmer when selected
                for (TextView c2 : chipViews) {
                    stopShimmer(c2);
                    c2.setBackgroundColor(C_CARD);
                    c2.setTextColor(C_MUTED);
                }
                tv.setBackgroundColor(C_ACCENT_DIM);
                tv.setTextColor(C_ACCENT2);
                // Layer 4: consumed scale animation on chip
                tv.animate().scaleX(0.92f).scaleY(0.92f).setDuration(60)
                    .withEndAction(() -> tv.animate().scaleX(1f).scaleY(1f)
                        .setInterpolator(new OvershootInterpolator(2f)).setDuration(180).start())
                    .start();
                if (isCustom) {
                    customUrl.setVisibility(View.VISIBLE);
                    baseUrl = customUrl.getText().toString().trim();
                    // v38: register in Rust provider registry immediately
                    if (!baseUrl.isEmpty()) {
                        try { RustBridge.setCustomProvider(baseUrl, ""); } catch (UnsatisfiedLinkError ignored) {}
                    }
                } else {
                    customUrl.setVisibility(View.GONE);
                    baseUrl = url;
                    // v38: switch active provider in Rust
                    try { RustBridge.setActiveProvider(url); } catch (UnsatisfiedLinkError ignored) {}
                }
            });
            chipsRow.addView(tv);
        }

        HorizontalScrollView hscroll = new HorizontalScrollView(this);
        hscroll.setHorizontalScrollBarEnabled(false);
        LinearLayout.LayoutParams hslp = new LinearLayout.LayoutParams(MATCH, WRAP);
        hslp.setMargins(0, dp(4), 0, dp(12));
        hscroll.setLayoutParams(hslp);
        hscroll.addView(chipsRow);
        root.addView(hscroll);
        root.addView(customUrl);
        sv.addView(root);
        return sv;
    }

    private View buildNamePage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83D\uDC64"));
        root.addView(bigText("What's your name?"));
        root.addView(hint("Kira will use this to personalise\nyour experience."));
        EditText input = inputField("Your name\u2026", false);
        input.setTag("nameInput");
        if (!cfg.userName.isEmpty() && !cfg.userName.equals("User"))
            input.setText(cfg.userName);
        root.addView(input);

        LinearLayout tip = new LinearLayout(this);
        tip.setBackgroundColor(C_CARD);
        tip.setPadding(dp(16), dp(14), dp(16), dp(14));
        LinearLayout.LayoutParams tlp = new LinearLayout.LayoutParams(MATCH, WRAP);
        tlp.setMargins(dp(24), dp(8), dp(24), 0);
        tip.setLayoutParams(tlp);
        TextView tipIcon = new TextView(this);
        tipIcon.setText("\uD83D\uDCA1");
        tipIcon.setTextSize(16);
        tipIcon.setPadding(0, 0, dp(10), 0);
        TextView tipText = new TextView(this);
        tipText.setText("Say \"remember my name is \u2026\" and Kira will never forget it across sessions.");
        tipText.setTextColor(0xFF445533);
        tipText.setTextSize(12);
        tipText.setLineSpacing(0, 1.4f);
        tipText.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));
        tip.addView(tipIcon);
        tip.addView(tipText);
        root.addView(tip);
        sv.addView(root);
        return sv;
    }

    private View buildModelPage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83E\uDDE0"));
        root.addView(bigText("Choose Your Model"));
        root.addView(hint("The AI model Kira will use.\nFast models respond in milliseconds."));

        String[][] models = {
            {"llama-3.1-8b-instant",    "Groq \u00B7 ultrafast, free"},
            {"llama-3.3-70b-versatile", "Groq \u00B7 smarter, free"},
            {"gpt-4o-mini",             "OpenAI \u00B7 balanced"},
            {"claude-3-haiku-20240307", "Anthropic \u00B7 precise"},
            {"gemini-2.0-flash",        "Google \u00B7 multimodal"},
            {"deepseek-chat",           "DeepSeek \u00B7 powerful"},
            {"mistral-7b-instruct",     "Mistral \u00B7 lean"},
            {"openrouter/auto",         "OpenRouter \u00B7 auto-route"},
        };

        LinearLayout modelList = new LinearLayout(this);
        modelList.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams mlp = new LinearLayout.LayoutParams(MATCH, WRAP);
        mlp.setMargins(dp(24), dp(8), dp(24), 0);
        modelList.setLayoutParams(mlp);

        for (int mi = 0; mi < models.length; mi++) {
            final String mName = models[mi][0];
            final String mDesc = models[mi][1];
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setBackgroundColor(C_CARD);
            row.setPadding(dp(16), dp(14), dp(16), dp(14));
            row.setGravity(Gravity.CENTER_VERTICAL);
            row.setClickable(true);
            row.setFocusable(true);
            LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(MATCH, WRAP);
            rp.setMargins(0, 0, 0, dp(2));
            row.setLayoutParams(rp);

            View dot = new View(this);
            dot.setBackgroundColor(C_HINT);
            dot.setTag("dot");
            LinearLayout.LayoutParams dlp2 = new LinearLayout.LayoutParams(dp(8), dp(8));
            dlp2.gravity = Gravity.CENTER_VERTICAL;
            dlp2.setMargins(0, 0, dp(12), 0);
            dot.setLayoutParams(dlp2);

            TextView mn = new TextView(this);
            mn.setText(mName);
            mn.setTextColor(C_TEXT);
            mn.setTextSize(13);
            mn.setTypeface(android.graphics.Typeface.MONOSPACE);
            mn.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));
            mn.setTag("name");

            TextView md = new TextView(this);
            md.setText(mDesc);
            md.setTextColor(C_MUTED);
            md.setTextSize(11);

            row.addView(dot); row.addView(mn); row.addView(md);

            row.setOnClickListener(v -> {
                model = mName;
                for (int j = 0; j < modelList.getChildCount(); j++) {
                    LinearLayout r2 = (LinearLayout) modelList.getChildAt(j);
                    r2.setBackgroundColor(C_CARD);
                    View d2 = r2.findViewWithTag("dot");
                    TextView n2 = r2.findViewWithTag("name");
                    if (d2 != null) d2.setBackgroundColor(C_HINT);
                    if (n2 != null) n2.setTextColor(C_TEXT);
                }
                row.setBackgroundColor(C_ACCENT_DIM);
                dot.setBackgroundColor(C_ACCENT);
                mn.setTextColor(C_ACCENT2);
            });
            modelList.addView(row);
        }

        android.widget.ScrollView innerSv = new android.widget.ScrollView(this);
        innerSv.setVerticalScrollBarEnabled(false);
        innerSv.setLayoutParams(new LinearLayout.LayoutParams(MATCH, dp(280)));
        innerSv.addView(modelList);
        root.addView(innerSv);
        sv.addView(root);
        return sv;
    }

    private View buildTelegramPage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\u2708"));
        root.addView(bigText("Telegram Bot"));
        root.addView(hint("Control Kira remotely.\nOptional \u2014 you can skip this."));
        root.addView(sectionLabel("Bot Token  \u00B7  get from @BotFather"));
        EditText tgIn = inputField("123456:ABC-DEF\u2026", false);
        tgIn.setTag("tgInput");
        if (!cfg.tgToken.isEmpty())
            tgIn.setText(cfg.tgToken.substring(0, Math.min(10, cfg.tgToken.length())) + "\u2026");
        root.addView(tgIn);
        root.addView(sectionLabel("Your Telegram ID  \u00B7  get from @userinfobot"));
        EditText tgIdIn = inputField("0 = anyone can use", true);
        tgIdIn.setTag("tgIdInput");
        if (cfg.tgAllowed > 0) tgIdIn.setText(String.valueOf(cfg.tgAllowed));
        root.addView(tgIdIn);

        LinearLayout cmdsCard = new LinearLayout(this);
        cmdsCard.setOrientation(LinearLayout.VERTICAL);
        cmdsCard.setBackgroundColor(C_CARD);
        cmdsCard.setPadding(dp(16), dp(14), dp(16), dp(14));
        LinearLayout.LayoutParams ccp = new LinearLayout.LayoutParams(MATCH, WRAP);
        ccp.setMargins(dp(24), dp(8), dp(24), 0);
        cmdsCard.setLayoutParams(ccp);
        for (String[] cmd : new String[][]{
                {"/run <task>",    "Execute any task"},
                {"/chain <goal>",  "ReAct autonomous agent"},
                {"/agent <goal>",  "Step-by-step planner"},
                {"/status",        "System health"},
                {"/screen",        "Screenshot"}}) {
            LinearLayout row = new LinearLayout(this);
            row.setPadding(0, dp(4), 0, dp(4));
            TextView c1 = new TextView(this); c1.setText(cmd[0]);
            c1.setTextColor(C_ACCENT2); c1.setTextSize(12);
            c1.setTypeface(android.graphics.Typeface.MONOSPACE);
            c1.setLayoutParams(new LinearLayout.LayoutParams(dp(150), WRAP));
            TextView c2 = new TextView(this); c2.setText(cmd[1]);
            c2.setTextColor(C_MUTED); c2.setTextSize(12);
            row.addView(c1); row.addView(c2);
            cmdsCard.addView(row);
        }
        root.addView(cmdsCard);
        sv.addView(root);
        return sv;
    }

    private View buildReadyPage() {
        ScrollView sv = sv();
        LinearLayout root = pageRoot();
        root.setGravity(Gravity.CENTER);

        TextView check = new TextView(this);
        check.setText("\u2714");
        check.setTextSize(72);
        check.setTextColor(C_ACCENT);
        check.setGravity(Gravity.CENTER);
        check.setTag("readyCheck");
        check.setShadowLayer(dp(24), 0, 0, C_ACCENT2);
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(dp(110), dp(110));
        cp.gravity = Gravity.CENTER_HORIZONTAL;
        cp.setMargins(0, 0, 0, dp(24));
        check.setLayoutParams(cp);
        root.addView(check);

        TextView title = bigText("Kira is Ready.");
        title.setShadowLayer(dp(10), 0, 0, C_ACCENT);
        root.addView(title);
        root.addView(hint("Your autonomous AI agent is configured.\nStart chatting or use /agent to begin."));

        LinearLayout statsRow = new LinearLayout(this);
        statsRow.setOrientation(LinearLayout.HORIZONTAL);
        statsRow.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams srp = new LinearLayout.LayoutParams(MATCH, WRAP);
        srp.setMargins(dp(24), dp(32), dp(24), 0);
        statsRow.setLayoutParams(srp);
        for (String[] stat : new String[][]{{"176","Tools"},{"8+","Providers"},{"\u221E","Memory"}}) {
            LinearLayout card = new LinearLayout(this);
            card.setOrientation(LinearLayout.VERTICAL);
            card.setGravity(Gravity.CENTER);
            card.setBackgroundColor(C_CARD);
            card.setPadding(dp(20), dp(16), dp(20), dp(16));
            LinearLayout.LayoutParams scp = new LinearLayout.LayoutParams(0, WRAP, 1);
            scp.setMargins(dp(4), 0, dp(4), 0);
            card.setLayoutParams(scp);
            TextView num = new TextView(this);
            num.setText(stat[0]); num.setTextColor(C_ACCENT); num.setTextSize(24);
            num.setTypeface(null, android.graphics.Typeface.BOLD); num.setGravity(Gravity.CENTER);
            num.setShadowLayer(dp(8), 0, 0, C_ACCENT);
            TextView lbl = new TextView(this);
            lbl.setText(stat[1]); lbl.setTextColor(C_MUTED); lbl.setTextSize(11);
            lbl.setGravity(Gravity.CENTER);
            card.addView(num); card.addView(lbl);
            statsRow.addView(card);
        }
        root.addView(statsRow);
        sv.addView(root);
        return sv;
    }

    // \u2500\u2500 Navigation \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    private void advance() {
        collectCurrentPage();
        if (currentPage >= TOTAL_PAGES - 1) { saveAndLaunch(); return; }
        View next = buildPage(currentPage + 1);
        next.setAlpha(0); next.setTranslationX(dp(60));
        pageContainer.addView(next);
        AnimatorSet anim = new AnimatorSet();
        anim.setDuration(320);
        anim.setInterpolator(new DecelerateInterpolator(1.5f));
        View prev = currentView;
        anim.playTogether(
            ObjectAnimator.ofFloat(prev, "alpha", 1f, 0f),
            ObjectAnimator.ofFloat(prev, "translationX", 0, -dp(60)),
            ObjectAnimator.ofFloat(next, "alpha", 0f, 1f),
            ObjectAnimator.ofFloat(next, "translationX", dp(60), 0)
        );
        anim.addListener(new AnimatorListenerAdapter() {
            @Override public void onAnimationEnd(Animator a) { pageContainer.removeView(prev); }
        });
        anim.start();
        currentPage++; currentView = next; updateDots(); updateNextBtn();
        handler.postDelayed(() -> {
            AnimatorSet b = new AnimatorSet();
            b.playTogether(
                ObjectAnimator.ofFloat(nextBtn, "scaleX", 1f, 1.07f, 1f),
                ObjectAnimator.ofFloat(nextBtn, "scaleY", 1f, 1.07f, 1f));
            b.setDuration(300); b.setInterpolator(new OvershootInterpolator(3f)); b.start();
        }, 200);
    }

    private void collectCurrentPage() {
        switch (currentPage) {
            case 1:
                EditText ak = pageContainer.findViewWithTag("apiKeyInput");
                if (ak != null && !ak.getText().toString().startsWith("*"))
                    apiKey = ak.getText().toString().trim();
                EditText cu = pageContainer.findViewWithTag("customUrlInput");
                if (cu != null && cu.getVisibility() == View.VISIBLE && !cu.getText().toString().isEmpty())
                    baseUrl = cu.getText().toString().trim();
                break;
            case 2:
                EditText nm = pageContainer.findViewWithTag("nameInput");
                if (nm != null && !nm.getText().toString().isEmpty())
                    name = nm.getText().toString().trim();
                break;
            case 4:
                EditText tg = pageContainer.findViewWithTag("tgInput");
                EditText tid = pageContainer.findViewWithTag("tgIdInput");
                if (tg != null && !tg.getText().toString().contains("\u2026"))
                    tgToken = tg.getText().toString().trim();
                if (tid != null) tgId = tid.getText().toString().trim();
                break;
        }
        // v38: push page state to Rust after collecting local fields
        try {
            long tgIdLong = 0;
            try { tgIdLong = tgId.isEmpty() ? 0 : Long.parseLong(tgId); } catch (Exception ignored) {}
            RustBridge.updateSetupPage(
                currentPage, apiKey, baseUrl, model, name, tgToken, tgIdLong
            );
        } catch (UnsatisfiedLinkError ignored) {}
    }

    private void saveAndLaunch() {
        if (!apiKey.isEmpty())  cfg.apiKey   = apiKey;
        if (!baseUrl.isEmpty()) cfg.baseUrl  = baseUrl;
        if (!model.isEmpty())   cfg.model    = model;
        if (!name.isEmpty())    cfg.userName = name;
        if (!tgToken.isEmpty()) cfg.tgToken  = tgToken;
        if (!tgId.isEmpty()) {
            try { cfg.tgAllowed = Long.parseLong(tgId); } catch (Exception ignored) {}
        }
        cfg.setupDone = true;
        cfg.save(this);
        // v38: mark setup done in Rust state
        View check = currentView.findViewWithTag("readyCheck");
        if (check != null) {
            AnimatorSet pop = new AnimatorSet();
            pop.playTogether(
                ObjectAnimator.ofFloat(check, "scaleX", 0.5f, 1.3f, 1f),
                ObjectAnimator.ofFloat(check, "scaleY", 0.5f, 1.3f, 1f),
                ObjectAnimator.ofFloat(check, "alpha",  0f, 1f));
            pop.setDuration(500); pop.setInterpolator(new OvershootInterpolator(2f)); pop.start();
        }
        handler.postDelayed(() -> {
            startActivity(new Intent(this, MainActivity.class));
            overridePendingTransition(android.R.anim.fade_in, android.R.anim.fade_out);
            finish();
        }, 800);
    }

    /** Session J: Load provider list from Rust /setup/providers */
    private String[][] loadProvidersFromRust() {
        try {
            java.net.HttpURLConnection c = (java.net.HttpURLConnection)
                new java.net.URL("http://localhost:7070/setup/providers").openConnection();
            c.setConnectTimeout(1500); c.setReadTimeout(1500);
            if (c.getResponseCode() != 200) return null;
            java.io.BufferedReader br = new java.io.BufferedReader(
                new java.io.InputStreamReader(c.getInputStream()));
            StringBuilder sb = new StringBuilder(); String line;
            while ((line = br.readLine()) != null) sb.append(line);
            c.disconnect();
            String json = sb.toString().trim();
            // Parse JSON array of {name, base_url} objects
            java.util.List<String[]> result = new java.util.ArrayList<>();
            int pos = 0;
            while ((pos = json.indexOf("\"name\":", pos)) >= 0) {
                pos += 7;
                int ns = json.indexOf('"', pos) + 1;
                int ne = json.indexOf('"', ns);
                String name = json.substring(ns, ne);
                int us = json.indexOf("\"base_url\":\"", pos) + 12;
                int ue = json.indexOf('"', us);
                String url  = json.substring(us, ue);
                result.add(new String[]{name, url});
                pos = ue;
            }
            return result.isEmpty() ? null : result.toArray(new String[0][]);
        } catch (Exception e) {
            return null; // fallback to hardcoded list
        }
    }

    private void showPage(int page, boolean initial) {
        currentPage = page;
        View v = buildPage(page);
        if (initial) {
            v.setAlpha(0);
            pageContainer.addView(v);
            currentView = v;
            ObjectAnimator.ofFloat(v, "alpha", 0f, 1f).setDuration(600).start();
        } else {
            pageContainer.addView(v);
            currentView = v;
        }
        updateDots(); updateNextBtn();
    }

    // ── Layer 4: Progress dots — active becomes 22dp oval, animated width ─────
    private void animateDotWidth(View dot, int fromDp, int toDp) {
        android.animation.ValueAnimator wa = android.animation.ValueAnimator.ofInt(dp(fromDp), dp(toDp));
        wa.setDuration(200);
        wa.setInterpolator(new android.view.animation.DecelerateInterpolator());
        wa.addUpdateListener(a -> {
            LinearLayout.LayoutParams lp2 = (LinearLayout.LayoutParams) dot.getLayoutParams();
            lp2.width = (int) a.getAnimatedValue();
            dot.setLayoutParams(lp2);
        });
        wa.start();
    }

    // ── Layer 4: Input field focus → animated border (2px Surface1 → 2px Lavender) ─
    private void animateFieldFocus(android.widget.EditText field, boolean focused) {
        android.graphics.drawable.GradientDrawable bg = new android.graphics.drawable.GradientDrawable();
        bg.setColor(0xFF1A1A2E);
        bg.setCornerRadius(dp(8));
        bg.setStroke(dp(2), focused ? C_ACCENT : 0xFF333355);
        field.setBackground(bg);
        field.setPadding(dp(16), dp(14), dp(16), dp(14));
    }

    // ── Layer 4: Shimmer pass on provider chip ────────────────────────────────
    private void startShimmer(View v) {
        android.animation.ObjectAnimator shimmer = android.animation.ObjectAnimator
            .ofFloat(v, "alpha", 1f, 0.6f, 1f);
        shimmer.setDuration(1800);
        shimmer.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        shimmer.setInterpolator(new android.view.animation.LinearInterpolator());
        v.setTag(R.id.tag3, shimmer);
        shimmer.start();
    }

    private void stopShimmer(View v) {
        Object s = v.getTag(R.id.tag3);
        if (s instanceof android.animation.ObjectAnimator)
            ((android.animation.ObjectAnimator) s).cancel();
        v.setAlpha(1f);
    }

    // ── Layer 4: K logo breath (scale 1.0→1.06→1.0, 3s, corona glow pulse) ──
    private void startLogoBreathe(TextView logo) {
        android.animation.AnimatorSet breath = new android.animation.AnimatorSet();
        android.animation.ObjectAnimator scaleX =
            android.animation.ObjectAnimator.ofFloat(logo, "scaleX", 1.0f, 1.06f, 1.0f);
        android.animation.ObjectAnimator scaleY =
            android.animation.ObjectAnimator.ofFloat(logo, "scaleY", 1.0f, 1.06f, 1.0f);
        // Shadow radius pulsed via alpha proxy
        android.animation.ObjectAnimator glow =
            android.animation.ObjectAnimator.ofFloat(logo, "alpha", 0.85f, 1.0f, 0.85f);
        scaleX.setDuration(3000); scaleX.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        scaleY.setDuration(3000); scaleY.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        glow.setDuration(3000);   glow.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        scaleX.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
        scaleY.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
        glow.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
        breath.playTogether(scaleX, scaleY, glow);
        breath.start();
        logo.setTag(R.id.tag3, breath);
    }

    // ── Layer 4: Next button label crossfade ──────────────────────────────────
    private void crossfadeNextLabel(String newLabel) {
        nextBtn.animate().alpha(0f).setDuration(150)
            .withEndAction(() -> {
                nextBtn.setText(newLabel);
                nextBtn.animate().alpha(1f).setDuration(150).start();
            }).start();
    }

    // ── Layer 4: Slide page transition (300ms EaseOut cubic) ─────────────────
    // Already in advance() — enhanced with simultaneous slide
    // (existing advance() already does translateX slide, we just tighten timing)

    private void buildDots() {
        dots = new View[TOTAL_PAGES];
        for (int i = 0; i < TOTAL_PAGES; i++) {
            View dot = new View(this);
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(8), dp(8));
            lp.setMargins(dp(4), 0, dp(4), 0);
            dot.setLayoutParams(lp);
            dot.setBackgroundColor(i == 0 ? C_ACCENT : C_HINT);
            dots[i] = dot;
            dotsRow.addView(dot);
        }
    }

    private void updateDots() {
        for (int i = 0; i < dots.length; i++) {
            boolean active = i == currentPage;
            // Animated width: 8dp circle → 22dp oval for active (Layer 4 spec)
            int targetW = active ? 22 : 8;
            LinearLayout.LayoutParams lp = (LinearLayout.LayoutParams) dots[i].getLayoutParams();
            if (lp.width != dp(targetW)) {
                animateDotWidth(dots[i], lp.width == dp(22) ? 22 : 8, targetW);
            }
            // Color: Lavender active, Surface1 inactive
            android.graphics.drawable.GradientDrawable dotBg =
                new android.graphics.drawable.GradientDrawable();
            dotBg.setShape(android.graphics.drawable.GradientDrawable.RECTANGLE);
            dotBg.setCornerRadius(dp(4));
            dotBg.setColor(active ? C_ACCENT : C_HINT);
            dots[i].setBackground(dotBg);
        }
    }

    private void updateNextBtn() {
        String label;
        if (currentPage == TOTAL_PAGES - 1) {
            label = "Launch Kira ✓";
            skipBtn.setVisibility(View.GONE);
        } else if (currentPage == 0) {
            label = "Get Started";
            skipBtn.setVisibility(View.VISIBLE);
        } else {
            label = "Next  →";
            skipBtn.setVisibility(View.VISIBLE);
        }
        // Layer 4: crossfade label instead of text snap
        if (!label.equals(nextBtn.getText().toString())) {
            crossfadeNextLabel(label);
        }
    }

    private void cycleQuote() {
        if (currentPage != 0) { handler.postDelayed(this::cycleQuote, 4000); return; }
        quoteIndex = (quoteIndex + 1) % QUOTES.length;
        TextView qt = (currentView != null) ? currentView.findViewWithTag("quoteText") : null;
        if (qt != null) {
            ObjectAnimator out = ObjectAnimator.ofFloat(qt, "alpha", 1f, 0f);
            out.setDuration(350);
            out.addListener(new AnimatorListenerAdapter() {
                @Override public void onAnimationEnd(Animator a) {
                    qt.setText(QUOTES[quoteIndex]);
                    ObjectAnimator.ofFloat(qt, "alpha", 0f, 1f).setDuration(350).start();
                }
            });
            out.start();
        }
        handler.postDelayed(this::cycleQuote, 4000);
    }

    // \u2500\u2500 View helpers \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    private static final int MATCH = LinearLayout.LayoutParams.MATCH_PARENT;
    private static final int WRAP  = LinearLayout.LayoutParams.WRAP_CONTENT;

    private ScrollView sv() {
        ScrollView sv = new ScrollView(this);
        sv.setLayoutParams(new FrameLayout.LayoutParams(MATCH, MATCH));
        sv.setVerticalScrollBarEnabled(false);
        sv.setFillViewport(true);
        return sv;
    }

    private LinearLayout pageRoot() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setLayoutParams(new ScrollView.LayoutParams(MATCH, WRAP));
        root.setPadding(0, dp(80), 0, dp(160));
        root.setGravity(Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        return root;
    }

    private TextView pageIcon(String emoji) {
        TextView tv = new TextView(this);
        tv.setText(emoji); tv.setTextSize(50); tv.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(0, 0, 0, dp(16)); tv.setLayoutParams(lp);
        return tv;
    }

    private TextView bigText(String text) {
        TextView tv = new TextView(this);
        tv.setText(text); tv.setTextColor(C_TEXT); tv.setTextSize(30);
        tv.setTypeface(null, android.graphics.Typeface.BOLD); tv.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(8)); tv.setLayoutParams(lp);
        return tv;
    }

    private TextView hint(String text) {
        TextView tv = new TextView(this);
        tv.setText(text); tv.setTextColor(C_MUTED); tv.setTextSize(14);
        tv.setGravity(Gravity.CENTER); tv.setLineSpacing(0, 1.4f);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(32), 0, dp(32), dp(28)); tv.setLayoutParams(lp);
        return tv;
    }

    private TextView sectionLabel(String text) {
        TextView tv = new TextView(this);
        tv.setText(text); tv.setTextColor(C_MUTED); tv.setTextSize(11);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(6)); tv.setLayoutParams(lp);
        return tv;
    }

    private EditText inputField(String hintText, boolean numeric) {
        EditText et = new EditText(this);
        et.setHint(hintText);
        // Readable hint: medium grey, clearly visible on dark background
        et.setHintTextColor(0xFF8888AA);
        // White text on dark card \u2014 maximum readability
        et.setTextColor(0xFFFFFFFF);
        et.setTextSize(16);
        // Solid dark card background with a subtle left accent border via padding
        android.graphics.drawable.GradientDrawable bg = new android.graphics.drawable.GradientDrawable();
        bg.setColor(0xFF1A1A2E);          // dark navy \u2014 clearly distinct from page bg
        bg.setCornerRadius(dp(8));
        bg.setStroke(dp(2), 0xFF333355); // Lavender on focus via animateFieldFocus
        et.setBackground(bg);
        et.setPadding(dp(16), dp(14), dp(16), dp(14));
        // Layer 4: animated focus border Surface1 → Lavender
        et.setOnFocusChangeListener((v, focused) -> animateFieldFocus(et, focused));
        et.setInputType(numeric
            ? InputType.TYPE_CLASS_NUMBER
            : (InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS));
        // NO setShadowLayer \u2014 it bleeds over the hint text making it unreadable
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(16));
        et.setLayoutParams(lp);
        return et;
    }

    private int dp(int dp) {
        return (int)(dp * getResources().getDisplayMetrics().density);
    }
}
