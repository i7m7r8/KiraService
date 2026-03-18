package com.kira.service;

import android.animation.Animator;
import android.animation.AnimatorListenerAdapter;
import android.animation.ObjectAnimator;
import android.animation.AnimatorSet;
import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.view.animation.DecelerateInterpolator;
import android.view.animation.OvershootInterpolator;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

import com.kira.service.ai.KiraConfig;

/**
 * Multi-page animated setup experience.
 *
 * Pages:
 *   0 - Welcome: Kira logo + motivational quote + brief description
 *   1 - API Key:  enter key, provider selection chips
 *   2 - Name:     personalize experience
 *   3 - Model:    model + base URL
 *   4 - Telegram: optional bot integration
 *   5 - Ready:    final animation + launch
 *
 * Quotes fetched from Rust /kb endpoint (pre-seeded) or local fallback.
 * Each page slides in from right with alpha fade.
 */
public class SetupActivity extends Activity {

    private static final int TOTAL_PAGES = 6;

    // Motivational quotes shown on welcome page (cycling)
    private static final String[] QUOTES = {
        "The best way to predict the future is to create it.",
        "Intelligence is the ability to adapt to change.",
        "Automation is not about replacing humans. It is about elevating them.",
        "The machine does not isolate man from the great problems of nature but plunges him more deeply into them.",
        "Any sufficiently advanced technology is indistinguishable from magic.",
        "We are the first generation to build gods, and the last generation to be free of them.",
        "The real problem is not whether machines think but whether men do.",
        "You have power over your mind, not outside events. Realize this and you will find strength.",
        "Do not wait to strike till the iron is hot, but make it hot by striking.",
        "The secret of getting ahead is getting started.",
    };

    private FrameLayout pageContainer;
    private LinearLayout dotsRow;
    private TextView nextBtn, skipBtn;
    private View[] dots;
    private int currentPage = 0;
    private View currentView;
    private KiraConfig cfg;
    private Handler handler = new Handler(Looper.getMainLooper());

    // Collected values
    private String apiKey = "", baseUrl = "", model = "", name = "", tgToken = "", tgId = "";
    private int quoteIndex = 0;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_setup);

        cfg          = KiraConfig.load(this);
        pageContainer= findViewById(R.id.setupPageContainer);
        dotsRow      = findViewById(R.id.setupDots);
        nextBtn      = findViewById(R.id.setupNextBtn);
        skipBtn      = findViewById(R.id.setupSkipBtn);

        buildDots();
        showPage(0, true);

        nextBtn.setOnClickListener(v -> advance());
        skipBtn.setOnClickListener(v -> finish());

        // Auto-cycle quote every 4 seconds on welcome page
        handler.postDelayed(this::cycleQuote, 4000);
    }

    // ?? Page building ?????????????????????????????????????????????????????????

    private View buildPage(int page) {
        switch (page) {
            case 0: return buildWelcomePage();
            case 1: return buildApiKeyPage();
            case 2: return buildNamePage();
            case 3: return buildModelPage();
            case 4: return buildTelegramPage();
            case 5: return buildReadyPage();
            default: return buildWelcomePage();
        }
    }

    private View buildWelcomePage() {
        LinearLayout root = pageRoot();

        // Logo
        TextView logo = new TextView(this);
        logo.setText("K");
        logo.setTextSize(72);
        logo.setTextColor(0xFFff8c00);
        logo.setTypeface(null, android.graphics.Typeface.BOLD);
        logo.setGravity(Gravity.CENTER);
        logo.setTag("logo");
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(100), dp(100));
        lp.gravity = Gravity.CENTER_HORIZONTAL;
        lp.setMargins(0, 0, 0, dp(24));
        logo.setLayoutParams(lp);
        logo.setBackgroundColor(0xFF141414);

        // Title
        TextView title = bigText("Meet Kira");
        title.setTag("title");

        // Subtitle
        TextView sub = new TextView(this);
        sub.setText("Your autonomous AI agent for Android.\nNo root. No Termux. Just intelligence.");
        sub.setTextColor(0xFF666666);
        sub.setTextSize(15);
        sub.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams sp = new LinearLayout.LayoutParams(MATCH, WRAP);
        sp.setMargins(dp(32), 0, dp(32), dp(48));
        sub.setLayoutParams(sp);

        // Quote card
        LinearLayout quoteCard = new LinearLayout(this);
        quoteCard.setOrientation(LinearLayout.VERTICAL);
        quoteCard.setBackgroundColor(0xFF111111);
        quoteCard.setPadding(dp(24), dp(20), dp(24), dp(20));
        LinearLayout.LayoutParams qp = new LinearLayout.LayoutParams(MATCH, WRAP);
        qp.setMargins(dp(24), 0, dp(24), 0);
        quoteCard.setLayoutParams(qp);

        TextView quoteBar = new TextView(this);
        quoteBar.setText("\u201C");
        quoteBar.setTextColor(0xFFff8c00);
        quoteBar.setTextSize(32);
        quoteBar.setTag("quoteBar");
        quoteCard.addView(quoteBar);

        TextView quoteText = new TextView(this);
        quoteText.setText(QUOTES[quoteIndex]);
        quoteText.setTextColor(0xFFaaaaaa);
        quoteText.setTextSize(14);
        quoteText.setLineSpacing(0, 1.4f);
        quoteText.setTag("quoteText");
        quoteCard.addView(quoteText);

        // Feature chips
        LinearLayout chips = new LinearLayout(this);
        chips.setOrientation(LinearLayout.HORIZONTAL);
        chips.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(MATCH, WRAP);
        cp.setMargins(0, dp(32), 0, 0);
        chips.setLayoutParams(cp);

        for (String[] chip : new String[][]{
            {"\uD83E\uDD16", "AI Agent"},
            {"\uD83D\uDD2D", "Vision"},
            {"\u26A1", "Rust Core"},
            {"\uD83D\uDD14", "Telegram"},
        }) {
            TextView tv = new TextView(this);
            tv.setText(chip[0] + " " + chip[1]);
            tv.setTextColor(0xFF888888);
            tv.setTextSize(11);
            tv.setBackgroundColor(0xFF1a1a1a);
            tv.setPadding(dp(10), dp(6), dp(10), dp(6));
            LinearLayout.LayoutParams tp = new LinearLayout.LayoutParams(WRAP, WRAP);
            tp.setMargins(dp(4), 0, dp(4), 0);
            tv.setLayoutParams(tp);
            chips.addView(tv);
        }

        root.addView(logo);
        root.addView(title);
        root.addView(sub);
        root.addView(quoteCard);
        root.addView(chips);
        return root;
    }

    private View buildApiKeyPage() {
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83D\uDD11"));
        root.addView(bigText("API Key"));
        root.addView(hint("Enter your key from Groq, OpenAI, Anthropic\nor any OpenAI-compatible provider."));

        EditText input = inputField("sk-... or gsk-...", false);
        input.setTag("apiKeyInput");
        if (!cfg.apiKey.isEmpty()) input.setText("*".repeat(Math.min(16, cfg.apiKey.length())));
        root.addView(input);

        root.addView(sectionLabel("Quick select provider:"));

        LinearLayout chips = new LinearLayout(this);
        chips.setOrientation(LinearLayout.HORIZONTAL);
        chips.setPadding(dp(24), 0, dp(24), 0);
        chips.setTag("providerChips");

        String[][] providers = {
            {"Groq (free)", "https://api.groq.com/openai/v1"},
            {"OpenAI", "https://api.openai.com/v1"},
            {"Anthropic", "https://api.anthropic.com/v1"},
            {"Gemini", "https://generativelanguage.googleapis.com/v1beta/openai"},
            {"Local", "http://localhost:11434/v1"},
        };
        for (String[] p : providers) {
            TextView tv = new TextView(this);
            tv.setText(p[0]);
            tv.setTextSize(12);
            tv.setTextColor(0xFF888888);
            tv.setBackgroundColor(0xFF1a1a1a);
            tv.setPadding(dp(10), dp(8), dp(10), dp(8));
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(WRAP, WRAP);
            lp.setMargins(0, 0, dp(8), 0);
            tv.setLayoutParams(lp);
            tv.setClickable(true);
            final String url = p[1];
            tv.setOnClickListener(v -> {
                baseUrl = url;
                tv.setBackgroundColor(0xFF2a1a00);
                tv.setTextColor(0xFFff8c00);
            });
            chips.addView(tv);
        }

        android.widget.HorizontalScrollView scroll = new android.widget.HorizontalScrollView(this);
        scroll.setHorizontalScrollBarEnabled(false);
        LinearLayout.LayoutParams slp = new LinearLayout.LayoutParams(MATCH, WRAP);
        slp.setMargins(0, dp(8), 0, 0);
        scroll.setLayoutParams(slp);
        scroll.addView(chips);
        root.addView(scroll);

        return root;
    }

    private View buildNamePage() {
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83D\uDC64"));
        root.addView(bigText("What's your name?"));
        root.addView(hint("Kira will use this to personalize\nyour experience."));

        EditText input = inputField("Imran", false);
        input.setTag("nameInput");
        if (!cfg.userName.isEmpty() && !cfg.userName.equals("User")) input.setText(cfg.userName);
        root.addView(input);

        TextView note = new TextView(this);
        note.setText("\uD83D\uDCA1 Tip: Say \"remember my name is Imran\" and Kira will\nnever forget it across sessions.");
        note.setTextColor(0xFF445544);
        note.setTextSize(12);
        note.setLineSpacing(0, 1.4f);
        LinearLayout.LayoutParams np = new LinearLayout.LayoutParams(MATCH, WRAP);
        np.setMargins(dp(24), dp(24), dp(24), 0);
        note.setLayoutParams(np);
        root.addView(note);

        return root;
    }

    private View buildModelPage() {
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\uD83E\uDDE0"));
        root.addView(bigText("Choose Your Model"));
        root.addView(hint("The AI model Kira will use.\nFast models respond in milliseconds."));

        String[][] models = {
            {"llama-3.1-8b-instant", "Groq - ultrafast, free"},
            {"llama-3.3-70b-versatile", "Groq - smarter, free"},
            {"gpt-4o-mini", "OpenAI - balanced"},
            {"claude-3-haiku-20240307", "Anthropic - precise"},
            {"gemini-2.0-flash", "Google - multimodal"},
            {"deepseek-chat", "DeepSeek - powerful"},
        };

        LinearLayout modelList = new LinearLayout(this);
        modelList.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams mlp = new LinearLayout.LayoutParams(MATCH, WRAP);
        mlp.setMargins(dp(24), dp(8), dp(24), 0);
        modelList.setLayoutParams(mlp);

        for (String[] m : models) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setBackgroundColor(0xFF141414);
            row.setPadding(dp(16), dp(14), dp(16), dp(14));
            row.setGravity(Gravity.CENTER_VERTICAL);
            row.setClickable(true);
            row.setFocusable(true);
            LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(MATCH, WRAP);
            rp.setMargins(0, 0, 0, dp(2));
            row.setLayoutParams(rp);

            TextView mName = new TextView(this);
            mName.setText(m[0]);
            mName.setTextColor(0xFFdddddd);
            mName.setTextSize(13);
            mName.setTypeface(null, android.graphics.Typeface.BOLD);
            mName.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));
            mName.setFontFeatureSettings("monospace");

            TextView mDesc = new TextView(this);
            mDesc.setText(m[1]);
            mDesc.setTextColor(0xFF555555);
            mDesc.setTextSize(11);

            row.addView(mName);
            row.addView(mDesc);

            final String modelName = m[0];
            row.setOnClickListener(v -> {
                model = modelName;
                for (int i = 0; i < modelList.getChildCount(); i++) {
                    modelList.getChildAt(i).setBackgroundColor(0xFF141414);
                }
                row.setBackgroundColor(0xFF2a1a00);
                mName.setTextColor(0xFFff8c00);
            });

            modelList.addView(row);
        }

        android.widget.ScrollView sv = new android.widget.ScrollView(this);
        sv.setVerticalScrollBarEnabled(false);
        LinearLayout.LayoutParams svlp = new LinearLayout.LayoutParams(MATCH, dp(280));
        sv.setLayoutParams(svlp);
        sv.addView(modelList);
        root.addView(sv);

        return root;
    }

    private View buildTelegramPage() {
        LinearLayout root = pageRoot();
        root.addView(pageIcon("\u2708"));
        root.addView(bigText("Telegram Bot (Optional)"));
        root.addView(hint("Control Kira remotely.\nSend commands, get alerts, trigger agents."));

        root.addView(sectionLabel("Bot Token  /  get from @BotFather"));
        EditText tgInput = inputField("123456:ABC-DEF...", false);
        tgInput.setTag("tgInput");
        if (!cfg.tgToken.isEmpty()) tgInput.setText(cfg.tgToken.substring(0, Math.min(10, cfg.tgToken.length())) + "...");
        root.addView(tgInput);

        root.addView(sectionLabel("Your Telegram ID  /  get from @userinfobot"));
        EditText tgIdInput = inputField("0 = anyone can use", true);
        tgIdInput.setTag("tgIdInput");
        if (cfg.tgAllowed > 0) tgIdInput.setText(String.valueOf(cfg.tgAllowed));
        root.addView(tgIdInput);

        // Commands cheatsheet
        TextView cmds = new TextView(this);
        cmds.setText(
            "Commands:\n" +
            "\uD83D\uDCEC /run <cmd>      - execute any task\n" +
            "\uD83D\uDD17 /chain <goal>   - ReAct autonomous agent\n" +
            "\uD83E\uDDE0 /agent <goal>   - step-by-step planner\n" +
            "\uD83D\uDCCA /status         - system health\n" +
            "\uD83D\uDCF7 /screen         - screenshot"
        );
        cmds.setTextColor(0xFF445544);
        cmds.setTextSize(12);
        cmds.setLineSpacing(0, 1.5f);
        cmds.setTypeface(android.graphics.Typeface.MONOSPACE);
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(MATCH, WRAP);
        cp.setMargins(dp(24), dp(20), dp(24), 0);
        cmds.setLayoutParams(cp);
        cmds.setBackgroundColor(0xFF0d1a0d);
        cmds.setPadding(dp(16), dp(16), dp(16), dp(16));
        root.addView(cmds);

        return root;
    }

    private View buildReadyPage() {
        LinearLayout root = pageRoot();
        root.setGravity(Gravity.CENTER);

        // Animated checkmark
        TextView check = new TextView(this);
        check.setText("\u2714");
        check.setTextSize(64);
        check.setTextColor(0xFFff8c00);
        check.setGravity(Gravity.CENTER);
        check.setTag("readyCheck");
        LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(dp(100), dp(100));
        cp.gravity = Gravity.CENTER_HORIZONTAL;
        cp.setMargins(0, 0, 0, dp(24));
        check.setLayoutParams(cp);

        TextView title = bigText("Kira is Ready.");
        title.setTag("readyTitle");

        TextView sub = hint("Your autonomous AI agent is configured.\nStart chatting or use /agent to begin.");

        // Stats preview
        LinearLayout statsRow = new LinearLayout(this);
        statsRow.setOrientation(LinearLayout.HORIZONTAL);
        statsRow.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams srp = new LinearLayout.LayoutParams(MATCH, WRAP);
        srp.setMargins(dp(24), dp(32), dp(24), 0);
        statsRow.setLayoutParams(srp);

        for (String[] stat : new String[][] {
            {"176", "Tools"},
            {"17", "Providers"},
            {"7070", "Rust Port"},
        }) {
            LinearLayout card = new LinearLayout(this);
            card.setOrientation(LinearLayout.VERTICAL);
            card.setGravity(Gravity.CENTER);
            card.setBackgroundColor(0xFF141414);
            card.setPadding(dp(20), dp(16), dp(20), dp(16));
            LinearLayout.LayoutParams scp = new LinearLayout.LayoutParams(0, WRAP, 1);
            scp.setMargins(dp(4), 0, dp(4), 0);
            card.setLayoutParams(scp);

            TextView num = new TextView(this);
            num.setText(stat[0]);
            num.setTextColor(0xFFff8c00);
            num.setTextSize(22);
            num.setTypeface(null, android.graphics.Typeface.BOLD);
            num.setGravity(Gravity.CENTER);

            TextView label = new TextView(this);
            label.setText(stat[1]);
            label.setTextColor(0xFF555555);
            label.setTextSize(11);
            label.setGravity(Gravity.CENTER);

            card.addView(num);
            card.addView(label);
            statsRow.addView(card);
        }

        root.addView(check);
        root.addView(title);
        root.addView(sub);
        root.addView(statsRow);
        return root;
    }

    // ?? Navigation ????????????????????????????????????????????????????????????

    private void advance() {
        collectCurrentPage();

        if (currentPage >= TOTAL_PAGES - 1) {
            saveAndLaunch();
            return;
        }

        View nextView = buildPage(currentPage + 1);
        nextView.setAlpha(0);
        nextView.setTranslationX(dp(60));
        pageContainer.addView(nextView);

        AnimatorSet anim = new AnimatorSet();
        anim.setDuration(320);
        anim.setInterpolator(new DecelerateInterpolator(1.5f));

        View prev = currentView;
        anim.playTogether(
            ObjectAnimator.ofFloat(prev,     "alpha",        1f, 0f),
            ObjectAnimator.ofFloat(prev,     "translationX", 0,  -dp(60)),
            ObjectAnimator.ofFloat(nextView, "alpha",        0f, 1f),
            ObjectAnimator.ofFloat(nextView, "translationX", dp(60), 0)
        );
        anim.addListener(new AnimatorListenerAdapter() {
            @Override public void onAnimationEnd(Animator a) {
                pageContainer.removeView(prev);
            }
        });
        anim.start();

        currentPage++;
        currentView = nextView;
        updateDots();
        updateNextBtn();

        // Bounce animation on next button after transition
        handler.postDelayed(() -> {
            ObjectAnimator bounce = ObjectAnimator.ofFloat(nextBtn, "scaleX", 1f, 1.06f, 1f);
            bounce.setDuration(300);
            bounce.setInterpolator(new OvershootInterpolator(3f));
            ObjectAnimator bounceY = ObjectAnimator.ofFloat(nextBtn, "scaleY", 1f, 1.06f, 1f);
            bounceY.setDuration(300);
            bounceY.setInterpolator(new OvershootInterpolator(3f));
            AnimatorSet bounceSet = new AnimatorSet(); bounceSet.playTogether(bounce, bounceY); bounceSet.start();
        }, 200);
    }

    // ?? Helpers ????????????????????????????????????????????????????????????????

    private void collectCurrentPage() {
        switch (currentPage) {
            case 1:
                EditText ak = pageContainer.findViewWithTag("apiKeyInput");
                if (ak != null && !ak.getText().toString().startsWith("*")) apiKey = ak.getText().toString().trim();
                break;
            case 2:
                EditText nm = pageContainer.findViewWithTag("nameInput");
                if (nm != null && !nm.getText().toString().isEmpty()) name = nm.getText().toString().trim();
                break;
            case 4:
                EditText tg = pageContainer.findViewWithTag("tgInput");
                EditText tid = pageContainer.findViewWithTag("tgIdInput");
                if (tg != null && !tg.getText().toString().contains("...")) tgToken = tg.getText().toString().trim();
                if (tid != null) tgId = tid.getText().toString().trim();
                break;
        }
    }

    private void saveAndLaunch() {
        if (!apiKey.isEmpty())   cfg.apiKey    = apiKey;
        if (!baseUrl.isEmpty())  cfg.baseUrl   = baseUrl;
        if (!model.isEmpty())    cfg.model     = model;
        if (!name.isEmpty())     cfg.userName  = name;
        if (!tgToken.isEmpty())  cfg.tgToken   = tgToken;
        if (!tgId.isEmpty()) {
            try { cfg.tgAllowed = Long.parseLong(tgId); } catch (Exception ignored) {}
        }
        cfg.setupDone = true;
        cfg.save(this);

        // Final scale + fade animation on checkmark
        View check = currentView.findViewWithTag("readyCheck");
        if (check != null) {
            AnimatorSet pop = new AnimatorSet();
            pop.playTogether(
                ObjectAnimator.ofFloat(check, "scaleX", 0.5f, 1.3f, 1f),
                ObjectAnimator.ofFloat(check, "scaleY", 0.5f, 1.3f, 1f),
                ObjectAnimator.ofFloat(check, "alpha",  0f,   1f)
            );
            pop.setDuration(500);
            pop.setInterpolator(new OvershootInterpolator(2f));
            pop.start();
        }

        handler.postDelayed(() -> {
            startActivity(new Intent(this, MainActivity.class));
            overridePendingTransition(android.R.anim.fade_in, android.R.anim.fade_out);
            finish();
        }, 800);
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
        updateDots();
        updateNextBtn();
    }

    private void buildDots() {
        dots = new View[TOTAL_PAGES];
        for (int i = 0; i < TOTAL_PAGES; i++) {
            View dot = new View(this);
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(8), dp(8));
            lp.setMargins(dp(4), 0, dp(4), 0);
            dot.setLayoutParams(lp);
            dot.setBackgroundColor(i == 0 ? 0xFFff8c00 : 0xFF333333);
            dots[i] = dot;
            dotsRow.addView(dot);
        }
    }

    private void updateDots() {
        for (int i = 0; i < dots.length; i++) {
            dots[i].setBackgroundColor(i == currentPage ? 0xFFff8c00 : 0xFF333333);
            LinearLayout.LayoutParams lp = (LinearLayout.LayoutParams) dots[i].getLayoutParams();
            lp.width  = dp(i == currentPage ? 20 : 8);
            lp.height = dp(8);
            dots[i].setLayoutParams(lp);
        }
    }

    private void updateNextBtn() {
        if (currentPage == TOTAL_PAGES - 1) {
            nextBtn.setText("Launch Kira");
            skipBtn.setVisibility(View.GONE);
        } else if (currentPage == 0) {
            nextBtn.setText("Get Started");
            skipBtn.setVisibility(View.VISIBLE);
        } else {
            nextBtn.setText("Next");
            skipBtn.setVisibility(View.VISIBLE);
        }
    }

    private void cycleQuote() {
        if (currentPage != 0) return;
        quoteIndex = (quoteIndex + 1) % QUOTES.length;
        TextView qText = (currentView != null) ? (TextView) currentView.findViewWithTag("quoteText") : null;
        if (qText != null) {
            ObjectAnimator fadeOut = ObjectAnimator.ofFloat(qText, "alpha", 1f, 0f);
            fadeOut.setDuration(400);
            fadeOut.addListener(new AnimatorListenerAdapter() {
                @Override public void onAnimationEnd(Animator a) {
                    qText.setText(QUOTES[quoteIndex]);
                    ObjectAnimator.ofFloat(qText, "alpha", 0f, 1f).setDuration(400).start();
                }
            });
            fadeOut.start();
        }
        handler.postDelayed(this::cycleQuote, 4000);
    }

    // ?? View helpers ??????????????????????????????????????????????????????????

    private static final int MATCH = LinearLayout.LayoutParams.MATCH_PARENT;
    private static final int WRAP  = LinearLayout.LayoutParams.WRAP_CONTENT;

    private LinearLayout pageRoot() {
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setLayoutParams(new FrameLayout.LayoutParams(MATCH, MATCH));
        root.setPadding(0, dp(80), 0, dp(160));
        root.setGravity(Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        return root;
    }

    private TextView pageIcon(String emoji) {
        TextView tv = new TextView(this);
        tv.setText(emoji);
        tv.setTextSize(48);
        tv.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(0, 0, 0, dp(16));
        tv.setLayoutParams(lp);
        return tv;
    }

    private TextView bigText(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextColor(0xFFffffff);
        tv.setTextSize(28);
        tv.setTypeface(null, android.graphics.Typeface.BOLD);
        tv.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(8));
        tv.setLayoutParams(lp);
        return tv;
    }

    private TextView hint(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextColor(0xFF666666);
        tv.setTextSize(14);
        tv.setGravity(Gravity.CENTER);
        tv.setLineSpacing(0, 1.4f);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(32), 0, dp(32), dp(28));
        tv.setLayoutParams(lp);
        return tv;
    }

    private TextView sectionLabel(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextColor(0xFF555555);
        tv.setTextSize(11);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(6));
        tv.setLayoutParams(lp);
        return tv;
    }

    private EditText inputField(String hint, boolean numeric) {
        EditText et = new EditText(this);
        et.setHint(hint);
        et.setHintTextColor(0xFF444444);
        et.setTextColor(0xFFffffff);
        et.setTextSize(15);
        et.setBackgroundColor(0xFF141414);
        et.setPadding(dp(20), dp(16), dp(20), dp(16));
        et.setInputType(numeric ? InputType.TYPE_CLASS_NUMBER : InputType.TYPE_CLASS_TEXT);
        LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(MATCH, WRAP);
        lp.setMargins(dp(24), 0, dp(24), dp(16));
        et.setLayoutParams(lp);
        return et;
    }

    private int dp(int dp) {
        return (int)(dp * getResources().getDisplayMetrics().density);
    }
}
