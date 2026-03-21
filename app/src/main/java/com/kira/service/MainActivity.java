package com.kira.service;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Bundle;
import android.hardware.Sensor;
import android.hardware.SensorEvent;
import android.hardware.SensorEventListener;
import android.hardware.SensorManager;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.text.InputType;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.HorizontalScrollView;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;

import org.json.JSONArray;
import org.json.JSONObject;

import rikka.shizuku.Shizuku;

import java.util.ArrayList;
import java.util.List;

public class MainActivity extends Activity
        implements SensorEventListener {

    // ── Catppuccin Mocha palette ──────────────────────────────────────────────
    private static final int D_BG          = 0xFF1E1E2E; // Base
    private static final int D_SURFACE     = 0xFF181825; // Mantle
    private static final int D_SURFACE2    = 0xFF181825; // Mantle
    private static final int D_BORDER      = 0xFF313244; // Surface0
    private static final int D_TEXT        = 0xFFCDD6F4; // Text
    private static final int D_TEXT2       = 0xFFBAC2DE; // Subtext1
    private static final int D_TEXT3       = 0xFF9399B2; // Overlay2
    private static final int D_NAV         = 0xFF181825; // Mantle
    private static final int D_INPUT_BG    = 0xFF313244; // Surface0
    private static final int D_USER_BUBBLE = 0xFF313244; // Surface0
    private static final int D_KIRA_BUBBLE = 0xFF1E1E2E; // Base
    private static final int D_TOOL_BG     = 0xFF1E2E1E; // tinted Base
    private static final int D_ERROR_BG    = 0xFF2E1E1E; // tinted Base
    private static final int D_CODE_BG     = 0xFF11111B; // Crust
    private static final int D_CODE_HDR    = 0xFF181825; // Mantle
    // Light theme (Catppuccin Latte)
    private static final int L_BG          = 0xFFEFF1F5; // Base
    private static final int L_SURFACE     = 0xFFFFFFFF;
    private static final int L_SURFACE2    = 0xFFE6E9EF; // Mantle
    private static final int L_BORDER      = 0xFFCCD0DA; // Surface0
    private static final int L_TEXT        = 0xFF4C4F69; // Text
    private static final int L_TEXT2       = 0xFF5C5F77; // Subtext1
    private static final int L_TEXT3       = 0xFF6C6F85; // Overlay2
    private static final int L_NAV         = 0xFFE6E9EF; // Mantle
    private static final int L_INPUT_BG    = 0xFFDCE0E8; // Surface1
    private static final int L_USER_BUBBLE = 0xFFDCE0E8; // Surface1
    private static final int L_KIRA_BUBBLE = 0xFFFFFFFF;
    private static final int L_TOOL_BG     = 0xFFE8F5E8;
    private static final int L_ERROR_BG    = 0xFFFFF0F0;
    private static final int L_CODE_BG     = 0xFFE6E9EF; // Mantle
    private static final int L_CODE_HDR    = 0xFFCCD0DA; // Surface0
    // Accent: Catppuccin Lavender
    private static final int ACCENT        = 0xFFB4BEFE; // Lavender

    // ── L10: Typography 3-tier system ────────────────────────────────────────
    // Primary:   CDD6F4 14sp regular   — main text
    // Secondary: 9399B2 11sp monospace — values, timestamps, hints
    // Accent:    B4BEFE 13sp bold      — labels, highlighted values
    private static final int TYP_PRIMARY   = 0xFFCDD6F4;
    private static final int TYP_SECONDARY = 0xFF9399B2;
    private static final int TYP_ACCENT    = 0xFFB4BEFE;
    private static final int TYP_PRIMARY_SP   = 14;
    private static final int TYP_SECONDARY_SP = 11;
    private static final int TYP_ACCENT_SP    = 13;
    // ── L10: 8dp grid spacing ────────────────────────────────────────────────
    private static final int GRID = 8; // all spacing = multiples of this
    private static final int ACCENT_DIM    = 0xFF2A2A40;

    // ── Live theme tokens from Rust getTheme() — updated in applyTheme() ─────
    private int T_BG           = D_BG;
    private int T_SURFACE      = D_SURFACE;
    private int T_SURFACE2     = D_SURFACE2;
    private int T_SURFACE_VAR  = D_INPUT_BG;
    private int T_SURFACE5     = D_BORDER;
    private int T_TEXT         = D_TEXT;
    private int T_TEXT2        = D_TEXT2;
    private int T_TEXT3        = D_TEXT3;
    private int T_ACCENT       = ACCENT;
    private int T_ON_ACCENT    = D_BG;
    private int T_SECONDARY    = 0xFFCBA6F7;
    private int T_TERTIARY     = 0xFFFAB387;
    private int T_SUCCESS      = 0xFFA6E3A1;
    private int T_ERROR        = 0xFFF38BA8;
    private int T_OUTLINE      = D_BORDER;

        private static final int SHIZUKU_CODE    = 1001;
    private static final int PERMISSION_CODE = 1002;

    private Handler uiHandler;
    private KiraAI  ai;
    private com.kira.service.ai.KiraAgent agent;
    private com.kira.service.ai.KiraChain chain;
    private KiraConfig cfg;
    private int currentTab = 0;

    // Fragments
    private View homeFragment, toolsFragment, historyFragment, settingsFragment;

    // Home
    private LinearLayout chatContainer;
    private com.kira.service.ui.GalaxyView galaxyView;
    private SensorManager sensorManager;
    private Sensor accelSensor;
    private ScrollView   chatScroll;
    private EditText     inputField;
    private TextView     sendBtn, headerSubtitle;
    private HorizontalScrollView suggestionsScroll;
    private LinearLayout suggestionsRow;

    // Conversation -- each turn is stored here for Claude-style resending
    private final List<ConvTurn> conversation = new ArrayList<>();

    // History
    private LinearLayout historyList;
    private TextView     historyCount;

    // Settings
    private TextView    apiKeyHint, visionHint, modelHint, baseUrlHint, tgTokenHint, tgIdHint;
    private TextView    maxStepsHint, heartbeatHint, personaHint, providerHint, skillsHint, checkpointsHint, auditHint, userNameHint, rustStatsHint, rustStatsContent;
    private View shizukuStatus;
    private TextView    shizukuStatusTitle, shizukuStatusIcon;
    private TextView    floatingToggle;
    private boolean     floatingActive = false;

    // Shizuku permission result listener (kept as field so we can remove it)
    private final Shizuku.OnRequestPermissionResultListener shizukuPermListener =
        (requestCode, grantResult) -> {
            if (grantResult == PackageManager.PERMISSION_GRANTED) {
                uiHandler.post(() -> {
                    android.widget.Toast.makeText(this,
                        "Shizuku active \u2014 god mode enabled!", android.widget.Toast.LENGTH_SHORT).show();
                    updateShizukuStatus();
                });
            } else {
                uiHandler.post(this::updateShizukuStatus);
            }
        };
    private TextView    memoryHint, memoryContent, clearHistoryBtn, historySettingHint;

    // Theme
    boolean isDarkTheme = true;  // auto-set in onCreate

    // Nav
    private TextView[]     navIcons, navTexts;
    private LinearLayout[] navItems;

    // -- Turn model ------------------------------------------------------------

    static class ConvTurn {
        String role;   // "user" | "kira" | "tool" | "error"
        String text;
        long   timestamp;
        ConvTurn(String role, String text) {
            this.role = role; this.text = text; this.timestamp = System.currentTimeMillis();
        }
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        // Skip setup if already configured
        com.kira.service.ai.KiraConfig cfgCheck = com.kira.service.ai.KiraConfig.load(this);
        if (!cfgCheck.setupDone && !getIntent().getBooleanExtra("skip_setup", false)) {
            startActivity(new android.content.Intent(this, SetupActivity.class));
            finish();
            return;
        }
        setContentView(R.layout.activity_main);
        uiHandler = new Handler(Looper.getMainLooper());
        cfg = KiraConfig.load(this);
        // Auto theme: follow system setting
        int uiMode = getResources().getConfiguration().uiMode & android.content.res.Configuration.UI_MODE_NIGHT_MASK;
        // Respect saved preference, else follow system
        boolean savedDark = getSharedPreferences("kira_theme", MODE_PRIVATE)
            .getBoolean("dark", uiMode == android.content.res.Configuration.UI_MODE_NIGHT_YES);
        isDarkTheme = savedDark;
        applyTheme();


        // Init accelerometer for star parallax
        sensorManager = (SensorManager) getSystemService(SENSOR_SERVICE);
        if (sensorManager != null)
            accelSensor = sensorManager.getDefaultSensor(Sensor.TYPE_ACCELEROMETER);
        // Session I: init AI objects off main thread (they're thin wrappers now)
        new Thread(() -> {
            ai    = new KiraAI(MainActivity.this);
            agent = new com.kira.service.ai.KiraAgent(MainActivity.this);
            chain = new com.kira.service.ai.KiraChain(MainActivity.this);
        }, "kira-init").start();
        initViews();
        showTab(0);

        // Session I: populate UI from Rust state after short delay
        uiHandler.postDelayed(() -> {
            new Thread(() -> {
                try {
                    java.net.HttpURLConnection c = (java.net.HttpURLConnection)
                        new java.net.URL("http://localhost:7070/ai/history").openConnection();
                    c.setConnectTimeout(1000); c.setReadTimeout(1000);
                    if (c.getResponseCode() == 200) {
                        java.io.BufferedReader br = new java.io.BufferedReader(
                            new java.io.InputStreamReader(c.getInputStream()));
                        StringBuilder sb = new StringBuilder(); String line;
                        while ((line = br.readLine()) != null) sb.append(line);
                        // History loaded — could restore bubbles here in future
                    }
                    c.disconnect();
                } catch (Exception ignored) {}
            }).start();
        }, 800);

        // If launched from CrashActivity — pre-fill input with crash context
        String crashPrompt = getIntent().getStringExtra("crash_prompt");
        if (crashPrompt != null) {
            uiHandler.postDelayed(() -> {
                if (inputField != null) inputField.setText(crashPrompt);
                addSystemNotice("Kira crashed. Paste the crash to ask for help.");
            }, 600);
        } else {
            // Show welcome message so user knows Kira is alive
            uiHandler.postDelayed(() -> {
                String welcome = cfg.userName != null && !cfg.userName.equals("User")
                    ? "Hi " + cfg.userName + "! I'm Kira. How can I help?"
                    : "Hi! I'm Kira, your AI agent. How can I help?";
                addSystemNotice(welcome);
            }, 500);
        }

        // Register Shizuku permission result listener before requesting
        try { Shizuku.addRequestPermissionResultListener(shizukuPermListener); }
        catch (Exception ignored) {}
        uiHandler.postDelayed(this::requestAllPermissions, 2000); // Session I: delay past first frame
        uiHandler.postDelayed(this::checkShizuku, 8000);   // 8s — let user see UI first
        uiHandler.postDelayed(this::checkAccessibility, 10000); // 10s

        // Start foreground service (also starts Rust HTTP server inside it)
        KiraForegroundService.start(this);
        // Safety: also attempt Rust server start from main thread in case service delays
        new Thread(() -> {
            try { Thread.sleep(200); } catch (Exception ignored) {}
            try { RustBridge.startServer(7070); }
            catch (Throwable ignored) {} // no-op if already running
        }, "kira-rust-init").start();
        // v43: init OTA engine (registers version with Rust, schedules checks)
        initOta();
        // Start galaxy animation polling
        animHandler.postDelayed(animPollRunnable, 1000);
        // OTA check (non-blocking, 3s delay)
    }

    @Override
    protected void onNewIntent(android.content.Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        // Handle crash_prompt from CrashActivity
        String crashPrompt = intent.getStringExtra("crash_prompt");
        if (crashPrompt != null && inputField != null) {
            uiHandler.postDelayed(() -> {
                if (inputField != null) {
                    inputField.setText(crashPrompt);
                    addSystemNotice("Kira crashed. Paste the crash to ask for help.");
                }
            }, 300);
        }
    }

    // -- Permissions -----------------------------------------------------------

    private void requestAllPermissions() {
        String[] perms = {
            Manifest.permission.RECORD_AUDIO, Manifest.permission.SEND_SMS,
            Manifest.permission.READ_SMS,     Manifest.permission.CALL_PHONE,
            Manifest.permission.READ_CONTACTS,Manifest.permission.READ_CALL_LOG,
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.READ_EXTERNAL_STORAGE,
            Manifest.permission.CAMERA,
        };
        List<String> needed = new ArrayList<>();
        for (String p : perms) {
            if (checkSelfPermission(p) != PackageManager.PERMISSION_GRANTED) needed.add(p);
        }
        if (!needed.isEmpty()) requestPermissions(needed.toArray(new String[0]), PERMISSION_CODE);

        if (android.os.Build.VERSION.SDK_INT >= 33) {
            if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, PERMISSION_CODE + 1);
            }
        }
    }

    private void checkShizuku() {
        try {
            if (!Shizuku.pingBinder()) {
                uiHandler.postDelayed(() -> showShizukuDialog(), 1500);
                return;
            }
            if (Shizuku.checkSelfPermission() != PackageManager.PERMISSION_GRANTED) {
                ShizukuShell.requestPermission(SHIZUKU_CODE);
            }
        } catch (Exception ignored) {}
    }

    private void showShizukuDialog() {
        showKiraDialogMulti("Enable Phone Control",
            "Kira uses Shizuku for ADB-level shell access.\n\n" +
            "1. Install Shizuku from Play Store\n" +
            "2. Open Shizuku \u2192 Start via Wireless Debugging\n" +
            "3. Return to Kira\n\n" +
            "Basic screen control works without Shizuku.",
            new String[]{"GET SHIZUKU", "ALREADY RUNNING", "SKIP"},
            new Runnable[]{
                () -> { try { startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=moe.shizuku.privileged.api"))); } catch (Exception e) { startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("https://shizuku.rikka.app"))); } },
                () -> checkShizuku(),
                null
            });
    }

    private void checkAccessibility() {
        if (KiraAccessibilityService.instance == null) {
            // Don't interrupt immediately — show after 10s so user can see the UI first
            uiHandler.postDelayed(() -> {
                if (KiraAccessibilityService.instance != null) return; // granted in the meantime
                showKiraDialogMulti("Enable Screen Control",
                    "For full autonomous control, enable Accessibility.\n\n" +
                    "Settings → Accessibility → Kira → Enable\n\n" +
                    "Basic chat works without it.",
                    new String[]{"OPEN SETTINGS", "LATER"},
                    new Runnable[]{
                        () -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)),
                        null
                    });
            }, 10_000); // 10 second delay — let user see the app first
        }
    }

    // -- View init -------------------------------------------------------------

    private void initViews() {
        FrameLayout frame = findViewById(R.id.contentFrame);
        homeFragment     = getLayoutInflater().inflate(R.layout.fragment_home,     frame, false);
        toolsFragment    = getLayoutInflater().inflate(R.layout.fragment_tools,    frame, false);
        historyFragment  = getLayoutInflater().inflate(R.layout.fragment_history,  frame, false);
        settingsFragment = getLayoutInflater().inflate(R.layout.fragment_settings, frame, false);
        frame.addView(homeFragment); frame.addView(toolsFragment);
        frame.addView(historyFragment); frame.addView(settingsFragment);

        // Nav
        LinearLayout[] navLayouts = {
            findViewById(R.id.navHome), findViewById(R.id.navAbilities),
            findViewById(R.id.navHistory), findViewById(R.id.navSettings)
        };
        navItems = navLayouts;
        navIcons = new TextView[4]; navTexts = new TextView[4];
        for (int i = 0; i < 4; i++) {
            navIcons[i] = (TextView) navLayouts[i].getChildAt(0);
            navTexts[i] = (TextView) navLayouts[i].getChildAt(1);
        }
        navLayouts[0].setOnClickListener(v -> showTab(0));
        navLayouts[1].setOnClickListener(v -> showTab(1));
        navLayouts[2].setOnClickListener(v -> showTab(2));
        navLayouts[3].setOnClickListener(v -> showTab(3));

        // Home
        chatContainer   = homeFragment.findViewById(R.id.chatContainer);
        chatScroll      = homeFragment.findViewById(R.id.chatScroll);
        inputField      = homeFragment.findViewById(R.id.inputField);
        sendBtn         = homeFragment.findViewById(R.id.sendBtn);
        headerSubtitle  = homeFragment.findViewById(R.id.headerSubtitle);
        suggestionsRow  = homeFragment.findViewById(R.id.suggestionsRow);
        suggestionsScroll = homeFragment.findViewById(R.id.suggestionsScroll);

        // Layer 3: Send button — press spring animation
        sendBtn.setOnTouchListener((v, ev) -> {
            if (ev.getAction() == android.view.MotionEvent.ACTION_DOWN) {
                sendBtn.animate().scaleX(0.88f).scaleY(0.88f).setDuration(80).start();
            } else if (ev.getAction() == android.view.MotionEvent.ACTION_UP ||
                       ev.getAction() == android.view.MotionEvent.ACTION_CANCEL) {
                sendBtn.animate()
                    .scaleX(1.05f).scaleY(1.05f).setDuration(120)
                    .withEndAction(() -> sendBtn.animate()
                        .scaleX(1f).scaleY(1f).setDuration(80).start())
                    .start();
            }
            return false;  // pass through to onClick
        });

        // Layer 3: Input field border animation on focus
        inputField.setOnFocusChangeListener((v, focused) -> {
            android.graphics.drawable.GradientDrawable fieldBg =
                new android.graphics.drawable.GradientDrawable();
            fieldBg.setColor(0xFF313244);  // Surface0
            fieldBg.setCornerRadius(dp(12));
            fieldBg.setStroke(dp(1), focused ? 0x99B4BEFE : 0x00000000); // Lavender 60% → none
            inputField.setBackground(fieldBg);
            inputField.setPadding(dp(12), dp(10), dp(12), dp(10));
        });

        // Layer 3: Send button pulse when input has text
        inputField.addTextChangedListener(new android.text.TextWatcher() {
            @Override public void beforeTextChanged(CharSequence s, int st, int c, int a) {}
            @Override public void onTextChanged(CharSequence s, int st, int b, int c) {
                boolean hasText = s.length() > 0;
                if (hasText) {
                    // Pulse glow: animate alpha 30→70%
                    android.animation.ObjectAnimator pulse =
                        android.animation.ObjectAnimator.ofFloat(sendBtn, "alpha", 0.85f, 1.0f);
                    pulse.setDuration(800);
                    pulse.setRepeatMode(android.animation.ValueAnimator.REVERSE);
                    pulse.setRepeatCount(android.animation.ValueAnimator.INFINITE);
                    sendBtn.setTag("pulse");
                    sendBtn.setTag(R.id.tag1, pulse);
                    pulse.start();
                } else {
                    // Stop pulse
                    Object p = sendBtn.getTag(R.id.tag1);
                    if (p instanceof android.animation.ObjectAnimator)
                        ((android.animation.ObjectAnimator) p).cancel();
                    sendBtn.setAlpha(1f);
                }
            }
            @Override public void afterTextChanged(android.text.Editable s) {}
        });

        // Layer 1: keyboard visibility → nav bar float
        final android.view.ViewTreeObserver.OnGlobalLayoutListener keyboardListener =
            new android.view.ViewTreeObserver.OnGlobalLayoutListener() {
            private boolean wasOpen = false;
            @Override public void onGlobalLayout() {
                android.graphics.Rect r = new android.graphics.Rect();
                getWindow().getDecorView().getWindowVisibleDisplayFrame(r);
                int screenH = getWindow().getDecorView().getHeight();
                boolean isOpen = (screenH - r.bottom) > screenH * 0.15;
                if (isOpen != wasOpen) {
                    wasOpen = isOpen;
                    View nav = findViewById(R.id.bottomNav);
                    if (nav != null) {
                        nav.animate()
                            .translationY(isOpen ? -dp(4) : 0f)
                            .scaleX(isOpen ? 0.97f : 1f)
                            .scaleY(isOpen ? 0.97f : 1f)
                            .setDuration(200)
                            .setInterpolator(new android.view.animation.OvershootInterpolator(1.8f))
                            .start();
                    }
                }
            }
        };
        getWindow().getDecorView().getViewTreeObserver()
            .addOnGlobalLayoutListener(keyboardListener);

        sendBtn.setOnClickListener(v -> {
            // K badge rotates 360° on send
            View kBadge = homeFragment.findViewWithTag("kBadge");
            if (kBadge != null) {
                kBadge.animate()
                    .rotationBy(360f)
                    .setDuration(600)
                    .setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator())
                    .start();
            }
            sendMessage();
        });
        inputField.setOnEditorActionListener((v, id, e) -> {
            if (id == android.view.inputmethod.EditorInfo.IME_ACTION_SEND) { sendMessage(); return true; }
            return false;
        });
        buildSuggestions();

        stopSubtitleCycle(); if (headerSubtitle != null) headerSubtitle.setText("ready · " + cfg.userName.toLowerCase());

        // History
        historyList  = historyFragment.findViewById(R.id.historyList);
        historyCount = historyFragment.findViewById(R.id.historyCount);

        // Settings
        apiKeyHint        = settingsFragment.findViewById(R.id.apiKeyHint);
        modelHint         = settingsFragment.findViewById(R.id.modelHint);
        baseUrlHint       = settingsFragment.findViewById(R.id.baseUrlHint);
        tgTokenHint       = settingsFragment.findViewById(R.id.tgTokenHint);
        tgIdHint          = settingsFragment.findViewById(R.id.tgIdHint);
        shizukuStatus = settingsFragment.findViewById(R.id.cardShizuku);
        shizukuStatusTitle= settingsFragment.findViewById(R.id.shizukuTitle);
        shizukuStatusIcon = settingsFragment.findViewById(R.id.shizukuIcon);
        floatingToggle    = settingsFragment.findViewById(R.id.floatingToggle);

        // Layer 5: Settings row tap flash (Lavender 15% overlay, 200ms)
        int[] settingsRows = {R.id.rowApiKey, R.id.rowModel, R.id.rowBaseUrl,
            R.id.rowTgToken, R.id.rowTgId, R.id.rowThemeToggle, R.id.cardShizuku,
            R.id.rowFloating, R.id.rowPersona, R.id.rowMaxSteps, R.id.rowHeartbeat,
            R.id.rowAuditLog, R.id.rowHistory, R.id.rowSkills, R.id.rowCheckpoints,
            R.id.rowRustStats, R.id.rowOta};
        for (int rid : settingsRows) {
            View row = settingsFragment.findViewById(rid);
            if (row == null) continue;
            row.setOnTouchListener((v2, ev) -> {
                if (ev.getAction() == android.view.MotionEvent.ACTION_DOWN) {
                    android.animation.ValueAnimator flash = android.animation.ValueAnimator
                        .ofArgb(0x00B4BEFE, 0x26B4BEFE, 0x00B4BEFE);
                    flash.setDuration(200);
                    flash.addUpdateListener(a -> {
                        int col = (int) a.getAnimatedValue();
                        row.setForeground(new android.graphics.drawable.ColorDrawable(col));
                    });
                    flash.start();
                    // Layer 5: log row tap to Rust for analytics
                    final String rowName = getResources().getResourceEntryName(rid);
                    // row tap analytics removed (was creating OkHttpClient per tap)
                }
                return false;
            });
        }

        settingsFragment.findViewById(R.id.rowApiKey).setOnClickListener(v ->
            editSetting("API Key", cfg.apiKey, false, val -> { cfg.apiKey = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.rowModel).setOnClickListener(v ->
            editSetting("Model", cfg.model, false, val -> { cfg.model = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.rowBaseUrl).setOnClickListener(v ->
            editSetting("Base URL", cfg.baseUrl, false, val -> { cfg.baseUrl = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.rowTgToken).setOnClickListener(v ->
            editSetting("Telegram Bot Token", cfg.tgToken, false, val -> { cfg.tgToken = val; cfg.save(this); updateSettingsUI(); restartTelegram(); }));
        settingsFragment.findViewById(R.id.rowTgId).setOnClickListener(v ->
            editSetting("Your Telegram ID", cfg.tgAllowed == 0 ? "" : String.valueOf(cfg.tgAllowed), true, val -> {
                try { cfg.tgAllowed = val.isEmpty() ? 0 : Long.parseLong(val.trim()); cfg.save(this); updateSettingsUI(); } catch (Exception ignored) {}
            }));
        visionHint = settingsFragment.findViewById(R.id.visionHint);
        View settingVision = settingsFragment.findViewById(R.id.rowVision);
        if (settingVision != null) settingVision.setOnClickListener(v ->
            editSetting("Vision Model", cfg.visionModel, false, val -> { cfg.visionModel = val; cfg.save(this); if (visionHint != null) visionHint.setText(val.isEmpty() ? "not set" : val); }));


        // Status cards
        View cardShizuku = settingsFragment.findViewById(R.id.cardShizuku);
        View cardAcc2    = settingsFragment.findViewById(R.id.cardAccessibility);
        View cardNotif   = settingsFragment.findViewById(R.id.cardNotifListener);
        if (cardShizuku != null) cardShizuku.setOnClickListener(v -> {
            boolean permOk   = ShizukuShell.isAvailable();
            boolean binderUp = ShizukuShell.isInstalled();
            boolean apkEx    = ShizukuShell.isApkInstalled(this);
            if (permOk) {
                // Already active \u2014 show status
                android.widget.Toast.makeText(this, "Shizuku god mode active \u2713", android.widget.Toast.LENGTH_SHORT).show();
            } else if (binderUp) {
                // Running but no permission \u2014 request it
                ShizukuShell.requestPermission(SHIZUKU_CODE);
            } else if (apkEx) {
                // Installed but not running \u2014 open Shizuku to start it
                try {
                    android.content.Intent i = getPackageManager().getLaunchIntentForPackage("moe.shizuku.privileged.api");
                    if (i != null) { i.addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK); startActivity(i); }
                    else android.widget.Toast.makeText(this, "Open Shizuku app and tap Start", android.widget.Toast.LENGTH_LONG).show();
                } catch (Exception e) {
                    android.widget.Toast.makeText(this, "Open Shizuku app and tap Start", android.widget.Toast.LENGTH_LONG).show();
                }
            } else {
                // Not installed \u2014 go to Play Store
                try { startActivity(new android.content.Intent(android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("market://details?id=moe.shizuku.privileged.api"))); }
                catch (Exception e) { startActivity(new android.content.Intent(android.content.Intent.ACTION_VIEW,
                    android.net.Uri.parse("https://shizuku.rikka.app"))); }
            }
        });
        if (cardAcc2  != null) cardAcc2.setOnClickListener(v -> startActivity(new android.content.Intent(android.provider.Settings.ACTION_ACCESSIBILITY_SETTINGS)));
        if (cardNotif != null) cardNotif.setOnClickListener(v -> startActivity(new android.content.Intent("android.settings.ACTION_NOTIFICATION_LISTENER_SETTINGS")));

        // New hint fields
        maxStepsHint    = settingsFragment.findViewById(R.id.maxStepsHint);
        heartbeatHint   = settingsFragment.findViewById(R.id.heartbeatHint);
        personaHint     = settingsFragment.findViewById(R.id.personaHint);
        providerHint    = settingsFragment.findViewById(R.id.providerHint);
        skillsHint      = settingsFragment.findViewById(R.id.skillsHint);
        checkpointsHint = settingsFragment.findViewById(R.id.checkpointsHint);
        auditHint       = settingsFragment.findViewById(R.id.auditHint);
        userNameHint    = settingsFragment.findViewById(R.id.userNameHint);
        rustStatsHint   = settingsFragment.findViewById(R.id.rustStatsHint);
        rustStatsContent= settingsFragment.findViewById(R.id.rustStatsContent);

        // Agent behavior rows
        View rowMaxSteps = settingsFragment.findViewById(R.id.rowMaxSteps);
        if (rowMaxSteps != null) rowMaxSteps.setOnClickListener(v ->
            editSetting("Max Agent Steps", String.valueOf(cfg.agentMaxSteps), false, val -> {
                try { cfg.agentMaxSteps = Integer.parseInt(val.trim()); cfg.save(MainActivity.this); if (maxStepsHint!=null) maxStepsHint.setText(val+" steps"); } catch (Exception ignored) {}
            }));
        View rowAutoApprove = settingsFragment.findViewById(R.id.rowAutoApprove);
        TextView autoTv = settingsFragment.findViewById(R.id.autoApproveToggle);
        if (autoTv != null) { autoTv.setText(cfg.agentAutoApprove?"ON":"OFF"); autoTv.setTextColor(cfg.agentAutoApprove?0xFFDC143C:0xFF666666); autoTv.setBackgroundColor(cfg.agentAutoApprove?0xFF1A0008:0xFF222222); }
        if (rowAutoApprove != null && autoTv != null) rowAutoApprove.setOnClickListener(v -> {
            cfg.agentAutoApprove = !cfg.agentAutoApprove; cfg.save(MainActivity.this);
            autoTv.setText(cfg.agentAutoApprove?"ON":"OFF"); autoTv.setTextColor(cfg.agentAutoApprove?0xFFDC143C:0xFF666666); autoTv.setBackgroundColor(cfg.agentAutoApprove?0xFF1A0008:0xFF222222);
        });
        View rowHeartbeat = settingsFragment.findViewById(R.id.rowHeartbeat);
        if (rowHeartbeat != null) rowHeartbeat.setOnClickListener(v ->
            editSetting("Heartbeat (min, 0=off)", String.valueOf(cfg.heartbeatInterval), false, val -> {
                try { cfg.heartbeatInterval = Integer.parseInt(val.trim()); cfg.save(MainActivity.this); if (heartbeatHint!=null) heartbeatHint.setText(cfg.heartbeatInterval==0?"disabled":cfg.heartbeatInterval+" min"); } catch (Exception ignored) {}
            }));
        View rowPersona = settingsFragment.findViewById(R.id.rowPersona);
        if (rowPersona != null) rowPersona.setOnClickListener(v ->
            editSetting("Persona (SOUL.md)", cfg.persona.isEmpty()?"You are Kira, a powerful Android AI agent.":cfg.persona, false, val -> {
                cfg.persona=val; cfg.save(MainActivity.this); com.kira.service.RustBridge.pushContextTurn("system","[SOUL] "+val); if (personaHint!=null) personaHint.setText(val.substring(0,Math.min(50,val.length())));
            }));
        View rowProvider = settingsFragment.findViewById(R.id.rowProvider);
        if (rowProvider != null) rowProvider.setOnClickListener(v -> showProviderPicker());
        View rowUserName = settingsFragment.findViewById(R.id.rowUserName);
        if (rowUserName != null) rowUserName.setOnClickListener(v ->
            editSetting("Your Name", cfg.userName, false, val -> { cfg.userName=val; cfg.save(MainActivity.this); if (userNameHint!=null) userNameHint.setText(val); }));

        // Theme toggle row (reuses rowFloating area \u2014 add after floating)
        View rowThemeToggle = settingsFragment.findViewById(R.id.rowThemeToggle);
        if (rowThemeToggle != null) rowThemeToggle.setOnClickListener(v -> toggleTheme());

        // OTA check row
        View rowOta = settingsFragment.findViewById(R.id.rowOta);
        if (rowOta != null) rowOta.setOnClickListener(v -> {
            if (otaUpdater == null) initOta();
            android.widget.Toast.makeText(this, "Checking for updates…", android.widget.Toast.LENGTH_SHORT).show();
            otaUpdater.checkForUpdate();
        });

        // Tools rows
        View rowSkills = settingsFragment.findViewById(R.id.rowSkills);
        if (rowSkills != null) rowSkills.setOnClickListener(v -> showInfoDialog("Available Skills", "Kira has 176+ built-in tools including:\nscreen control, shell commands, SMS, calls, web search, file management, notifications, scheduling, vision AI, and more.\n\nSkill details available via chat: ask \'list tools\'"));
        View rowCheckpoints = settingsFragment.findViewById(R.id.rowCheckpoints);
        if (rowCheckpoints != null) rowCheckpoints.setOnClickListener(v -> showInfoDialog("Checkpoints", new com.kira.service.ai.KiraCheckpoint(this).getAllJson()));
        View rowAuditLog = settingsFragment.findViewById(R.id.rowAuditLog);
        if (rowAuditLog != null) rowAuditLog.setOnClickListener(v -> showInfoDialog("Audit Log", "Audit log stored in conversation history.\nSee History tab for full interaction records."));

        // Rust stats
        View refreshBtn = settingsFragment.findViewById(R.id.rustRefreshBtn);
        View rowRustStats = settingsFragment.findViewById(R.id.rowRustStats);
        Runnable loadStats = () -> new Thread(() -> { String d=fetchRust("http://localhost:7070/health"); String fmt=d.replace(",\"","\n").replace("}","").replace("{","").replace("\"","").replace(":"," = "); uiHandler.post(() -> { if (rustStatsContent!=null){rustStatsContent.setText(fmt);rustStatsContent.setVisibility(android.view.View.VISIBLE);} if(rustStatsHint!=null)rustStatsHint.setText("online"); }); }).start();
        if (refreshBtn  != null) refreshBtn.setOnClickListener(v -> loadStats.run());
        if (rowRustStats != null) rowRustStats.setOnClickListener(v -> loadStats.run());

        // History row
        View rowHistory2 = settingsFragment.findViewById(R.id.rowHistory);
        if (rowHistory2 != null) rowHistory2.setOnClickListener(v -> showConfirmDialog("Clear all history?", () -> { new com.kira.service.ai.KiraMemory(this).clearHistory(); conversation.clear(); chatContainer.removeAllViews(); }));

        { View _acc = settingsFragment.findViewById(R.id.cardAccessibility);
          if (_acc != null) _acc.setOnClickListener(v -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))); }
        { View _flt = settingsFragment.findViewById(R.id.rowFloating);
          if (_flt != null) _flt.setOnClickListener(v -> toggleFloating()); }

        buildToolsList();
        updateSettingsUI();
        loadMemorySection();

        // Memory section wiring
        memoryHint        = settingsFragment.findViewById(R.id.memoryHint);
        memoryContent     = settingsFragment.findViewById(R.id.memoryContent);
        // clearHistoryBtn handled by rowHistory onClick
        historySettingHint= settingsFragment.findViewById(R.id.historySettingHint);
        TextView memoryClearBtn = settingsFragment.findViewById(R.id.memoryClearBtn);

        // rust stats row already handled by rowRustStats above
        if (memoryClearBtn != null) memoryClearBtn.setOnClickListener(v -> clearMemory());
        // clearHistory wired via rowHistory2 above
    }

    // -- Tab nav ---------------------------------------------------------------

    private void showTab(int tab) {
        currentTab = tab;
        if (homeFragment != null) homeFragment.setVisibility(tab == 0 ? View.VISIBLE : View.GONE);
        if (toolsFragment != null) toolsFragment.setVisibility(tab == 1 ? View.VISIBLE : View.GONE);
        if (historyFragment != null) historyFragment.setVisibility(tab == 2 ? View.VISIBLE : View.GONE);
        if (settingsFragment != null) settingsFragment.setVisibility(tab == 3 ? View.VISIBLE : View.GONE);
        for (int i = 0; i < 4; i++) {
            boolean on = i == tab;
            // Catppuccin: Lavender active, Overlay0 inactive
            int activeColor = 0xFFB4BEFE;  // Lavender
            int idleColor   = 0xFF6C7086;  // Overlay0
            navIcons[i].setTextColor(on ? activeColor : idleColor);
            navTexts[i].setTextColor(on ? activeColor : idleColor);
            navItems[i].setBackgroundColor(0x00000000); // transparent — aura is drawn by NeuralNavBar
            // Icon spring animation: grow on activate, shrink on deactivate
            if (navIcons[i] != null) {
                navIcons[i].animate()
                    .scaleX(on ? 1.18f : 1.0f)
                    .scaleY(on ? 1.18f : 1.0f)
                    .setDuration(on ? 250 : 200)
                    .setInterpolator(new android.view.animation.OvershootInterpolator(2.8f))
                    .start();
            }
        }
        if (tab == 2) refreshHistory();
        if (tab == 3) updateSettingsUI();
    }

    // -- Chat -- Claude-style ---------------------------------------------------

    private void sendMessage() {
        if (inputField == null) return;
        String text = inputField.getText().toString().trim();
        if (text.isEmpty()) return;
        sendMessage(text);
    }

    private void sendMessage(String text) {
        if (text.isEmpty()) return;
        if (inputField != null) inputField.setText("");
        if (suggestionsScroll != null) suggestionsScroll.setVisibility(View.GONE);

        // Agent mode: prefix with /agent or /auto
        if (text.startsWith("/kb ")) {
            String query = text.substring(4).trim();
            addSystemNotice("KB search: " + query + "\n(tip: ask Kira directly \u2014 say \'remember: ...\'  to store facts)");
            return;
        }
        if (text.equals("/events")) {
            addSystemNotice("Event log: " + new com.kira.service.ai.KiraMemory(this).listAll());
            return;
        }
        if (text.equals("/metrics")) {
            addSystemNotice("Memory: " + new com.kira.service.ai.KiraMemory(this).listAll());
            return;
        }
        if (text.equals("/budget")) {
            addSystemNotice("Budget tracking not available in this build.");
            return;
        }
        if (text.startsWith("/workflow ") || text.equals("/workflows")) {
            if (text.equals("/workflows")) {
                new Thread(() -> { String list = new com.kira.service.ai.KiraWorkflow(MainActivity.this).listJson(); uiHandler.post(() -> addSystemNotice("Workflows: " + list)); }).start();
                return;
            }
            String goal = new com.kira.service.ai.KiraWorkflow(this).buildGoal(text.substring(10).trim());
            runAgent(goal);
            return;
        }
        if (text.startsWith("/chain ")) {
            runChain(text.substring(7));
            return;
        }
        if (text.startsWith("/agent ") || text.startsWith("/auto ")) {
            String goal = text.replaceFirst("^/(?:agent|auto)\\s+", "");
            runAgent(goal);
            return;
        }

        ConvTurn userTurn = new ConvTurn("user", text);
        conversation.add(userTurn);
        addUserBubble(userTurn);
        // Layer 0: vortex ON — Kira is thinking
        new Thread(() -> { try { new okhttp3.OkHttpClient().newCall(
            new okhttp3.Request.Builder().url("http://localhost:7070/theme/thinking")
                .post(okhttp3.RequestBody.create("{\"active\":true}",
                    okhttp3.MediaType.parse("application/json"))).build()).execute();
        } catch (Exception ignored) {} }).start();
        // Layer 2: Pulse header border Lavender 27%→35%
        View hb = homeFragment != null ? homeFragment.findViewById(R.id.headerBorder) : null;
        if (hb != null) {
            android.animation.ObjectAnimator borderPulse =
                android.animation.ObjectAnimator.ofArgb(hb, "backgroundColor",
                    0x44B4BEFE, 0x59B4BEFE);
            borderPulse.setDuration(600);
            borderPulse.setRepeatMode(android.animation.ValueAnimator.REVERSE);
            borderPulse.setRepeatCount(android.animation.ValueAnimator.INFINITE);
            hb.setTag(R.id.tag2, borderPulse);
            borderPulse.start();
        }

        startSubtitleCycle(new String[]{"thinking...", "reasoning...", "processing...", "composing..."});
        if (sendBtn != null) sendBtn.setEnabled(false);

        // Thinking placeholder
        ConvTurn[] kiraTurn = {null};

        ai.chat(text, new KiraAI.Callback() {
            @Override public void onThinking() {
                uiHandler.post(() -> { if (kiraTurn[0] == null) { kiraTurn[0] = new ConvTurn("kira", "???"); addThinkingBubble(kiraTurn[0]); } });
            }
            @Override public void onTool(String name, String result) {
                ConvTurn toolTurn = new ConvTurn("tool", "? " + name + ": " + result.substring(0, Math.min(100, result.length())));
                conversation.add(toolTurn);
                uiHandler.post(() -> addToolBubble(toolTurn));
            }
            @Override public void onReply(String reply) {
                uiHandler.post(() -> {
                    if (kiraTurn[0] != null) {
                        kiraTurn[0].text = reply;
                        updateThinkingBubble(kiraTurn[0], reply);
                    } else {
                        kiraTurn[0] = new ConvTurn("kira", reply);
                        conversation.add(kiraTurn[0]);
                        addKiraBubble(kiraTurn[0]);
                    }
                    if (sendBtn != null) sendBtn.setEnabled(true);
                    stopSubtitleCycle(); if (headerSubtitle != null) headerSubtitle.setText("ready · " + cfg.userName.toLowerCase());
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    ConvTurn errTurn = new ConvTurn("error", error);
                    conversation.add(errTurn);
                    addErrorBubble(errTurn);
                    if (sendBtn != null) sendBtn.setEnabled(true);
                    if (headerSubtitle != null) headerSubtitle.setText("error");
                });
            }
        });
    }

    // -- Bubble builders -------------------------------------------------------

    /** Theme helper: returns dark or light value based on isDarkTheme flag */
    private int t(int dark, int light) { return isDarkTheme ? dark : light; }

        private void addUserBubble(ConvTurn turn) {
        LinearLayout wrap = new LinearLayout(this);
        wrap.setOrientation(LinearLayout.VERTICAL);
        wrap.setTag("user_" + turn.timestamp);
        LinearLayout.LayoutParams wp = new LinearLayout.LayoutParams(MATCH, WRAP);
        wp.setMargins(0, 0, 0, dp(12));
        wrap.setLayoutParams(wp);

        // Label row with Edit button
        LinearLayout labelRow = new LinearLayout(this);
        labelRow.setOrientation(LinearLayout.HORIZONTAL);
        labelRow.setGravity(Gravity.CENTER_VERTICAL);
        labelRow.setPadding(0, 0, 0, dp(3));

        TextView label = makeLabel("YOU");
        label.setTextColor(t(D_TEXT3, L_TEXT3));
        label.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

        // Edit button -- lets user edit and resend (like Claude's edit feature)
        TextView editBtn = new TextView(this);
        editBtn.setText("? edit");
        editBtn.setTextColor(t(D_TEXT3, L_TEXT3));
        editBtn.setTextSize(10);
        editBtn.setOnClickListener(v -> {
            inputField.setText(turn.text);
            inputField.setSelection(turn.text.length());
            inputField.requestFocus();
            // Remove all turns from this turn onward (Claude-style branching)
            int idx = conversation.indexOf(turn);
            if (idx >= 0) {
                while (conversation.size() > idx) conversation.remove(conversation.size() - 1);
                // Remove all bubbles from this turn onward
                rebuildChat();
            }
        });

        labelRow.addView(label);
        labelRow.addView(editBtn);

        TextView msg = new TextView(this);
        msg.setText(turn.text);
        msg.setTextColor(TYP_PRIMARY);  // L10: tier-1 text
        msg.setTextSize(14);
        // Surface0 bubble, rounded corners except bottom-right
        android.graphics.drawable.GradientDrawable userBg = new android.graphics.drawable.GradientDrawable();
        userBg.setColor(0xFF313244);  // Surface0
        userBg.setCornerRadii(new float[]{dp(8),dp(8), dp(8),dp(8), 0,0, dp(8),dp(8)});
        msg.setBackground(userBg);
        msg.setPadding(dp(14), dp(10), dp(14), dp(10));
        msg.setLineSpacing(dp(2), 1);
        msg.setTextIsSelectable(true);

        wrap.addView(labelRow);
        wrap.addView(msg);
        if (chatContainer != null) chatContainer.addView(wrap);
        // Spring in from right (Layer 2)
        wrap.setTranslationX(dp(40));
        wrap.setAlpha(0f);
        wrap.animate()
            .translationX(0f).alpha(1f)
            .setDuration(320)
            .setInterpolator(new android.view.animation.OvershootInterpolator(1.4f))
            .start();
        scrollToBottom();
    }

    private View thinkingView;
    private ConvTurn thinkingTurn;

    private void addThinkingBubble(ConvTurn turn) {
        thinkingTurn = turn;
        showTypingIndicator();  // Layer 2: sinusoidal dot animation
        thinkingView = typingIndicator;
    }

    private void updateThinkingBubble(ConvTurn turn, String reply) {
        if (thinkingView == null) {
            conversation.add(turn);
            addKiraBubble(turn);
            return;
        }
        // Replace the "???" with real content
        if (chatContainer != null) chatContainer.removeView(thinkingView);
        thinkingView = null;
        conversation.add(turn);
        addKiraBubble(turn);
    }

    private void removeThinkingBubble() {
        hideTypingIndicator();
        thinkingView = null;
    }

    private void addKiraBubble(ConvTurn turn) {
        LinearLayout wrap = new LinearLayout(this);
        wrap.setOrientation(LinearLayout.VERTICAL);
        wrap.setTag("kira_" + turn.timestamp);
        LinearLayout.LayoutParams wp = new LinearLayout.LayoutParams(MATCH, WRAP);
        wp.setMargins(0, 0, 0, dp(16));
        wrap.setLayoutParams(wp);

        // Header row: KIRA | copy | resend
        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(Gravity.CENTER_VERTICAL);
        header.setPadding(0, 0, 0, dp(3));

        TextView label = makeLabel("KIRA");
        label.setTextColor(0xFFDC143C);
        label.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

        TextView copyBtn = makeActionBtn("copy");
        copyBtn.setOnClickListener(v -> copyText(turn.text));

        TextView resendBtn = makeActionBtn("? resend");
        resendBtn.setOnClickListener(v -> { if (inputField!=null) { inputField.setText(turn.text); inputField.setSelection(turn.text.length()); inputField.requestFocus(); } });

        header.addView(label);
        header.addView(copyBtn);
        header.addView(resendBtn);

        // Check if reply contains code blocks
        boolean hasCode = turn.text.contains("```");

        if (hasCode) {
            // Split by code blocks and render mixed content
            wrap.addView(header);
            renderMixedContent(wrap, turn.text);
        } else {
            TextView msg = new TextView(this);
            msg.setText(turn.text);
            msg.setTextColor(0xFFeeeeee);
            msg.setTextSize(14);
            msg.setBackgroundColor(0x880e0e18);
            msg.setPadding(dp(14), dp(10), dp(14), dp(10));
            msg.setLineSpacing(dp(2), 1);
            msg.setTextIsSelectable(true);
            // Kira bubble: Base bg + Lavender left border
            android.graphics.drawable.GradientDrawable kiraBg =
                new android.graphics.drawable.GradientDrawable();
            kiraBg.setColor(0xFF1E1E2E);  // Base
            kiraBg.setCornerRadii(new float[]{0,0, dp(8),dp(8), dp(8),dp(8), dp(8),dp(8)});
            msg.setBackground(kiraBg);
            msg.setTextColor(0xFFCDD6F4);  // Text
            // Left Lavender bar wrapper
            LinearLayout kiraRow = new LinearLayout(this);
            kiraRow.setOrientation(LinearLayout.HORIZONTAL);
            View lavBar = new View(this);
            lavBar.setBackgroundColor(0xFFB4BEFE);
            kiraRow.addView(lavBar, new LinearLayout.LayoutParams(dp(3), LinearLayout.LayoutParams.MATCH_PARENT));
            kiraRow.addView(msg,   new LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1));
            wrap.addView(header);
            wrap.addView(kiraRow);
        }

        // L7: long-press context menu (Copy / Resend) spring from bubble center
        wrap.setOnLongClickListener(v -> {
            showBubbleContextMenu(wrap, turn.text);
            return true;
        });
        if (chatContainer != null) chatContainer.addView(wrap);
        // Spring in from left (Layer 2)
        wrap.setTranslationX(-dp(40));
        wrap.setAlpha(0f);
        wrap.animate()
            .translationX(0f).alpha(1f)
            .setDuration(320)
            .setInterpolator(new android.view.animation.OvershootInterpolator(1.4f))
            .start();
        // Scroll + burst
        if (chatScroll == null) return;
        chatScroll.post(() -> chatScroll.fullScroll(android.widget.ScrollView.FOCUS_DOWN));
        onKiraReplied();
    }

    /**
     * Renders text + code blocks like Claude:
     * - Plain text ? regular text view
     * - ```code``` ? dark terminal box with language label + Copy button
     */
    private void renderMixedContent(LinearLayout parent, String text) {
        String[] parts = text.split("```");
        boolean inCode = false;
        for (String part : parts) {
            if (!inCode) {
                if (!part.trim().isEmpty()) {
                    TextView tv = new TextView(this);
                    tv.setText(part.trim());
                    tv.setTextColor(t(D_TEXT, L_TEXT));
                    tv.setTextSize(14);
                    tv.setPadding(dp(14), dp(8), dp(14), dp(8));
                    tv.setLineSpacing(dp(2), 1);
                    tv.setTextIsSelectable(true);
                    parent.addView(tv);
                }
            } else {
                // Code block
                String code = part;
                String lang = "";
                // Extract language hint from first line
                int nl = code.indexOf('\n');
                if (nl >= 0 && nl < 20) {
                    lang = code.substring(0, nl).trim();
                    code = code.substring(nl + 1);
                }

                LinearLayout codeBlock = new LinearLayout(this);
                codeBlock.setOrientation(LinearLayout.VERTICAL);
                codeBlock.setBackgroundColor(t(D_CODE_BG, L_CODE_BG));
                LinearLayout.LayoutParams cbp = new LinearLayout.LayoutParams(MATCH, WRAP);
                cbp.setMargins(0, dp(4), 0, dp(4));
                codeBlock.setLayoutParams(cbp);

                // Code header: language + Copy
                LinearLayout codeHeader = new LinearLayout(this);
                codeHeader.setOrientation(LinearLayout.HORIZONTAL);
                codeHeader.setGravity(Gravity.CENTER_VERTICAL);
                codeHeader.setBackgroundColor(t(D_CODE_HDR, L_CODE_HDR));
                codeHeader.setPadding(dp(12), dp(6), dp(12), dp(6));

                TextView langLabel = new TextView(this);
                langLabel.setText(lang.isEmpty() ? "code" : lang);
                langLabel.setTextColor(t(0xFF8888AA, L_TEXT3));
                langLabel.setTextSize(11);
                langLabel.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

                final String finalCode = code.trim();
                TextView codeCopyBtn = new TextView(this);
                codeCopyBtn.setText("Copy");
                codeCopyBtn.setTextColor(0xFFDC143C);
                codeCopyBtn.setTextSize(11);
                codeCopyBtn.setOnClickListener(v -> {
                    copyText(finalCode);
                    codeCopyBtn.setText("Copied!");
                    uiHandler.postDelayed(() -> codeCopyBtn.setText("Copy"), 2000);
                });

                codeHeader.addView(langLabel);
                codeHeader.addView(codeCopyBtn);

                // Code body -- monospace, horizontally scrollable
                HorizontalScrollView hScroll = new HorizontalScrollView(this);
                hScroll.setHorizontalScrollBarEnabled(true);
                hScroll.setLayoutParams(new LinearLayout.LayoutParams(MATCH, WRAP));

                TextView codeTv = new TextView(this);
                codeTv.setText(finalCode);
                codeTv.setTextColor(t(0xFF00CC77, 0xFF004466));
                codeTv.setTextSize(12);
                codeTv.setTypeface(android.graphics.Typeface.MONOSPACE);
                codeTv.setPadding(dp(12), dp(10), dp(12), dp(10));
                codeTv.setTextIsSelectable(true);
                codeTv.setBackgroundColor(t(D_CODE_BG, L_CODE_BG));

                hScroll.addView(codeTv);
                codeBlock.addView(codeHeader);
                codeBlock.addView(hScroll);
                parent.addView(codeBlock);
            }
            inCode = !inCode;
        }
    }

    private void addToolBubble(ConvTurn turn) {
        TextView tv = new TextView(this);
        tv.setText(turn.text);
        tv.setTextColor(0xFF4a7a4a);
        tv.setTextSize(11);
        tv.setBackgroundColor(0x880d1a0d);
        tv.setPadding(dp(12), dp(5), dp(12), dp(5));
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(MATCH, WRAP);
        p.setMargins(0, dp(1), 0, dp(1));
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
        scrollToBottom();
    }

    private void addErrorBubble(ConvTurn turn) {
        LinearLayout wrap = new LinearLayout(this);
        wrap.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams wp = new LinearLayout.LayoutParams(MATCH, WRAP);
        wp.setMargins(0, 0, 0, dp(8));
        wrap.setLayoutParams(wp);

        TextView label = makeLabel("ERROR");
        label.setTextColor(0xFFDD3333);
        label.setPadding(0, 0, 0, dp(3));

        TextView msg = new TextView(this);
        msg.setText(turn.text);
        msg.setTextColor(0xFFff8888);
        msg.setTextSize(13);
        msg.setBackgroundColor(0xBB1a0808);
        msg.setPadding(dp(14), dp(10), dp(14), dp(10));
        msg.setTextIsSelectable(true);

        wrap.addView(label);
        wrap.addView(msg);
        if (chatContainer != null) chatContainer.addView(wrap);
        scrollToBottom();
    }

    private void rebuildChat() {
        chatContainer.removeAllViews();
        for (ConvTurn turn : conversation) {
            switch (turn.role) {
                case "user":  addUserBubble(turn); break;
                case "kira":  addKiraBubble(turn); break;
                case "tool":  addToolBubble(turn); break;
                case "error": addErrorBubble(turn); break;
            }
        }
    }



    private void runChain(String goal) {
        ConvTurn userTurn = new ConvTurn("user", "/chain " + goal);
        conversation.add(userTurn);
        addUserBubble(userTurn);
        if (headerSubtitle != null) headerSubtitle.setText("\uD83D\uDD17 ReAct chain...");
        if (sendBtn != null) sendBtn.setEnabled(false);
        addSystemNotice("\uD83E\uDDE0 ReAct mode: reason + act loop");

        chain.run(goal, 5, new com.kira.service.ai.KiraChain.ChainCallback() {
            @Override public void onStep(String thought) {
                uiHandler.post(() -> addSystemNotice("\uD83E\uDDE0 " + thought));
            }
            @Override public void onConclusion(String answer) {
                uiHandler.post(() -> {
                    ConvTurn t2 = new ConvTurn("kira", answer);
                    conversation.add(t2);
                    addKiraBubble(t2);
                    if (sendBtn != null) sendBtn.setEnabled(true);
                    if (headerSubtitle != null) headerSubtitle.setText("done.");
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    addSystemNotice("\u274C Chain error: " + error);
                    if (sendBtn != null) sendBtn.setEnabled(true);
                });
            }
        });
    }

    private void runAgent(String goal) {
        ConvTurn userTurn = new ConvTurn("user", "/agent " + goal);
        conversation.add(userTurn);
        addUserBubble(userTurn);
        if (headerSubtitle != null) headerSubtitle.setText("agent running...");
        if (sendBtn != null) sendBtn.setEnabled(false);

        addSystemNotice("Agent mode: planning task...");

        agent.execute(goal, new com.kira.service.ai.KiraAgent.AgentCallback() {
            @Override public void onPlan(String plan) {
                uiHandler.post(() -> addSystemNotice("Plan:\n" + plan));
            }
            @Override public void onStep(int step, String action, String result) {
                uiHandler.post(() -> addToolBubble(new ConvTurn("tool", "Step " + step + ": " + action + "\n-> " + result.substring(0, Math.min(100, result.length())))));
            }
            @Override public void onDone(String summary) {
                uiHandler.post(() -> {
                    ConvTurn turn = new ConvTurn("kira", summary);
                    conversation.add(turn);
                    addKiraBubble(turn);
                    if (sendBtn != null) sendBtn.setEnabled(true);
                    if (headerSubtitle != null) headerSubtitle.setText("done.");
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    addErrorBubble(new ConvTurn("error", error));
                    if (sendBtn != null) sendBtn.setEnabled(true);
                    if (headerSubtitle != null) headerSubtitle.setText("agent error");
                });
            }
        });
    }

    private void addSystemNotice(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextColor(t(0xFF8888AA, L_TEXT3));
        tv.setTextSize(12);
        tv.setPadding(dp(12), dp(6), dp(12), dp(6));
        tv.setBackgroundColor(0x88080810);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(MATCH, WRAP);
        p.setMargins(0, dp(2), 0, dp(2));
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
        scrollToBottom();
    }

    @Override
    public void onSensorChanged(SensorEvent event) {
        if (event.sensor.getType() != Sensor.TYPE_ACCELEROMETER) return;
        float ax = event.values[0]; // tilt left/right  (–10 to +10)
        float ay = event.values[1]; // tilt forward/back
        // Push to Rust for EMA smoothing
        RustBridge.updateTilt(ax, ay);
        // Pass normalised tilt directly to GalaxyView (–1 to +1 range)
        if (galaxyView != null) {
            galaxyView.setParallax(ax / 10f, ay / 10f);
        }
    }

    @Override
    public void onAccuracyChanged(Sensor s, int acc) {}

    private double parseJsonDouble(String json, String key) {
        try {
            int i = json.indexOf("\"" + key + "\":");
            if (i < 0) return 0;
            int s = i + key.length() + 3;
            int e = s;
            while (e < json.length() && "0123456789.-Ee".indexOf(json.charAt(e)) >= 0) e++;
            return Double.parseDouble(json.substring(s, e));
        } catch (Exception e2) { return 0; }
    }

    private String parseJsonString(String json, String key) {
        try {
            String needle = "\"" + key + "\":\"";
            int i = json.indexOf(needle);
            if (i < 0) return "";
            int s = i + needle.length();
            int e = json.indexOf("\"", s);
            return e > s ? json.substring(s, e) : "";
        } catch (Exception e2) { return ""; }
    }

    private float parseJsonFloat(String json, String key) {
        try {
            int i = json.indexOf("\"" + key + "\":");
            if (i < 0) return 0f;
            int start = i + key.length() + 3;
            int end = start;
            while (end < json.length() && "0123456789.-Ee".indexOf(json.charAt(end)) >= 0) end++;
            return Float.parseFloat(json.substring(start, end));
        } catch (Exception e) { return 0f; }
    }

    private void seedGalaxyFromRust() {
        // GalaxyView seeds deterministically — no action needed
    }

    // ── Galaxy animation polling (every 500ms) ─────────────────────────────
    private final android.os.Handler animHandler = new android.os.Handler(
        android.os.Looper.getMainLooper());
    private final Runnable animPollRunnable = new Runnable() {
        @Override public void run() {
            pollGalaxyAnim();
            animHandler.postDelayed(this, 500);
        }
    };

    // Shared OkHttpClient for galaxy poll - never recreate per call
    private final okhttp3.OkHttpClient animClient = new okhttp3.OkHttpClient.Builder()
        .connectTimeout(400, java.util.concurrent.TimeUnit.MILLISECONDS)
        .readTimeout(400, java.util.concurrent.TimeUnit.MILLISECONDS).build();

    private void pollGalaxyAnim() {
        if (galaxyView == null) return;
        new Thread(() -> {
            try {
                okhttp3.Response resp = animClient.newCall(
                    new okhttp3.Request.Builder()
                        .url("http://localhost:7070/layer0").get().build()).execute();
                if (resp.body() == null) return;
                String j = resp.body().string();
                float hueShift = parseJsonFloat(j, "hue_shift");
                float vortex   = parseJsonFloat(j, "vortex");
                float activity = parseJsonFloat(j, "activity");
                boolean thinking = j.contains("\"thinking\":true");
                uiHandler.post(() -> {
                    galaxyView.setAnimState(hueShift, vortex, activity, thinking);
                    View hb = homeFragment != null ? homeFragment.findViewById(R.id.headerBorder) : null;
                    if (hb != null && !thinking) {
                        int alpha = 0x44 + (int)(activity * 0x22);
                        hb.setBackgroundColor((alpha << 24) | 0x00B4BEFE);
                    }
                });
            } catch (Exception ignored) {}
        }).start();
    }

    /** Called by KiraAI when a response is fully received — triggers burst */
    public void onKiraReplied() {
        fireLightning(0); // L6: reply arc — burst triggered automatically by thinking→false
        // Stop header border pulse
        View hb = homeFragment != null ? homeFragment.findViewById(R.id.headerBorder) : null;
        if (hb != null) {
            Object p = hb.getTag(R.id.tag2);
            if (p instanceof android.animation.ObjectAnimator)
                ((android.animation.ObjectAnimator) p).cancel();
            hb.setBackgroundColor(0x44B4BEFE);
        }
        hideTypingIndicator();
        // Signal Rust: burst + stop thinking
        new Thread(() -> { try {
            new okhttp3.OkHttpClient().newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070/layer0/burst")
                    .post(okhttp3.RequestBody.create(new byte[0], null)).build()).execute();
            new okhttp3.OkHttpClient().newCall(
                new okhttp3.Request.Builder().url("http://localhost:7070/theme/thinking")
                    .post(okhttp3.RequestBody.create("{\"active\":false}",
                        okhttp3.MediaType.parse("application/json"))).build()).execute();
        } catch (Exception ignored) {} }).start();
    }

    private void showProviderPicker() {
        final String[][] PROVIDERS = {
            {"groq",       "Groq  llama-3.1-8b",              "https://api.groq.com/openai/v1",                          "llama-3.1-8b-instant"},
            {"openai",     "OpenAI  gpt-4o-mini",              "https://api.openai.com/v1",                               "gpt-4o-mini"},
            {"anthropic",  "Anthropic  claude-haiku",          "https://api.anthropic.com/v1",                            "claude-3-haiku-20240307"},
            {"gemini",     "Gemini  2.0 flash",                "https://generativelanguage.googleapis.com/v1beta/openai", "gemini-2.0-flash"},
            {"deepseek",   "DeepSeek  chat",                   "https://api.deepseek.com/v1",                             "deepseek-chat"},
            {"openrouter", "OpenRouter  auto",                 "https://openrouter.ai/api/v1",                            "openrouter/auto"},
            {"ollama",     "Ollama  local",                    "http://localhost:11434/v1",                                "llama3"},
            {"together",   "Together AI",                      "https://api.together.xyz/v1",                             "meta-llama/Llama-3-8b-chat-hf"},
            {"mistral",    "Mistral  small",                   "https://api.mistral.ai/v1",                               "mistral-small-latest"},
            {"cohere",     "Cohere  command-r",                "https://api.cohere.ai/v1",                                "command-r"},
            {"perplexity", "Perplexity  sonar",                "https://api.perplexity.ai",                               "llama-3.1-sonar-small-128k-online"},
            {"xai",        "xAI  Grok-2",                     "https://api.x.ai/v1",                                     "grok-2-latest"},
            {"cerebras",   "Cerebras  llama3.1",               "https://api.cerebras.ai/v1",                              "llama3.1-8b"},
            {"fireworks",  "Fireworks AI",                     "https://api.fireworks.ai/inference/v1",                   "accounts/fireworks/models/llama-v3p1-8b-instruct"},
            {"sambanova",  "SambaNova  llama3.1",              "https://api.sambanova.ai/v1",                             "Meta-Llama-3.1-8B-Instruct"},
            {"novita",     "Novita AI",                        "https://api.novita.ai/v3/openai",                         "llama-3.1-8b-instruct"},
            {"custom",     "Custom URL...",                    "",                                                         ""},
        };

        String[] displayNames = new String[PROVIDERS.length];
        for (int i = 0; i < PROVIDERS.length; i++) {
            String purl = PROVIDERS[i][2];
            boolean isActive = purl.equals(cfg.baseUrl) ||
                ("custom".equals(PROVIDERS[i][0]) && !isKnownProvider(cfg.baseUrl));
            // Show custom URL if currently set
            if ("custom".equals(PROVIDERS[i][0]) && !cfg.baseUrl.isEmpty() && !isKnownProvider(cfg.baseUrl)) {
                displayNames[i] = "Custom: " + cfg.baseUrl + (isActive ? " \u2713" : "");
            } else {
                displayNames[i] = PROVIDERS[i][1] + (isActive ? "  \u2713" : "");
            }
        }

        showProviderListDialog(displayNames, PROVIDERS);
    }

    @SuppressWarnings("unused")
    private void _providerDialogLambda(String[][] PROVIDERS, String[] displayNames) {
        // kept for reference \u2014 actual impl is showProviderListDialog
    }

    private void showProviderListDialog(String[] displayNames, String[][] PROVIDERS) {
        android.widget.FrameLayout overlay = new android.widget.FrameLayout(this);
        overlay.setBackgroundColor(0xCC000000);
        overlay.setLayoutParams(new android.widget.FrameLayout.LayoutParams(MATCH, MATCH));

        android.widget.LinearLayout card = new android.widget.LinearLayout(this);
        card.setOrientation(android.widget.LinearLayout.VERTICAL);
        android.graphics.drawable.GradientDrawable cardBg = new android.graphics.drawable.GradientDrawable();
        cardBg.setColor(isDarkTheme ? 0xFF0c0c18 : 0xFFf0f0f8);
        cardBg.setCornerRadius(dp(4));
        cardBg.setStroke(dp(1), isDarkTheme ? 0xFF1a1a2e : 0xFFddddee);
        card.setBackground(cardBg);
        android.widget.FrameLayout.LayoutParams cardLp = new android.widget.FrameLayout.LayoutParams(
            (int)(getResources().getDisplayMetrics().widthPixels * 0.9f),
            (int)(getResources().getDisplayMetrics().heightPixels * 0.75f));
        cardLp.gravity = android.view.Gravity.CENTER;
        card.setLayoutParams(cardLp);

        // Accent bar
        android.view.View bar = new android.view.View(this);
        bar.setBackgroundColor(0xFFDC143C);
        bar.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(2)));
        card.addView(bar);

        // Title
        android.widget.TextView ttv = new android.widget.TextView(this);
        ttv.setText("SELECT PROVIDER");
        ttv.setTextColor(t(D_TEXT, L_TEXT));
        ttv.setTextSize(13); ttv.setTypeface(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD);
        android.widget.LinearLayout.LayoutParams ttp = new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        ttp.setMargins(dp(16), dp(12), dp(16), dp(10)); ttv.setLayoutParams(ttp);
        card.addView(ttv);

        android.view.View sep = new android.view.View(this);
        sep.setBackgroundColor(t(D_BORDER, L_BORDER));
        sep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(sep);

        // Scrollable provider list
        android.widget.ScrollView sv = new android.widget.ScrollView(this);
        sv.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, 0, 1));
        android.widget.LinearLayout list = new android.widget.LinearLayout(this);
        list.setOrientation(android.widget.LinearLayout.VERTICAL);
        list.setPadding(0, dp(4), 0, dp(4));

        android.view.ViewGroup root = (android.view.ViewGroup) getWindow().getDecorView();
        root.addView(overlay);

        Runnable dismiss = () -> root.removeView(overlay);
        overlay.setOnClickListener(v -> dismiss.run());
        card.setOnClickListener(v -> {});

        for (int i = 0; i < displayNames.length; i++) {
            final int idx = i;
            final String[] prov = PROVIDERS[i];
            android.widget.LinearLayout row = new android.widget.LinearLayout(this);
            row.setOrientation(android.widget.LinearLayout.HORIZONTAL);
            row.setGravity(android.view.Gravity.CENTER_VERTICAL);
            row.setPadding(dp(16), dp(12), dp(16), dp(12));
            boolean isActive = displayNames[i].endsWith("  \u2713");
            row.setBackgroundColor(isActive ? (isDarkTheme ? 0x22DC143C : 0x11DC143C) : 0x00000000);
            row.setClickable(true); row.setFocusable(true);

            android.widget.TextView nameTV = new android.widget.TextView(this);
            nameTV.setText(displayNames[i]);
            nameTV.setTextColor(isActive ? 0xFFDC143C : (isDarkTheme ? 0xFFccccdd : 0xFF222233));
            nameTV.setTextSize(13);
            nameTV.setTypeface(android.graphics.Typeface.MONOSPACE);
            nameTV.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, WRAP, 1));

            row.addView(nameTV);
            if (isActive) {
                android.widget.TextView chk = new android.widget.TextView(this);
                chk.setText("\u2713"); chk.setTextColor(0xFFDC143C); chk.setTextSize(14);
                row.addView(chk);
            }
            row.setOnClickListener(v -> {
                dismiss.run();
                if ("custom".equals(prov[0])) { showCustomProviderDialog(); }
                else {
                    cfg.baseUrl = prov[2]; cfg.model = prov[3]; cfg.save(this);
                    try { RustBridge.setActiveProvider(prov[0]); } catch (Exception ignored) {}
                    updateSettingsUI();
                    android.widget.Toast.makeText(this, prov[1], android.widget.Toast.LENGTH_SHORT).show();
                }
            });

            // Separator
            android.view.View rowSep = new android.view.View(this);
            rowSep.setBackgroundColor(t(D_BORDER, L_BORDER));
            rowSep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));

            list.addView(row);
            list.addView(rowSep);
        }
        sv.addView(list);
        card.addView(sv);

        // Close button
        android.view.View closeSep = new android.view.View(this);
        closeSep.setBackgroundColor(t(D_BORDER, L_BORDER));
        closeSep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(closeSep);
        android.widget.TextView closeBtn = new android.widget.TextView(this);
        closeBtn.setText("CANCEL"); closeBtn.setTextColor(0xFF444466);
        closeBtn.setTextSize(11); closeBtn.setGravity(android.view.Gravity.CENTER);
        closeBtn.setTypeface(android.graphics.Typeface.MONOSPACE);
        closeBtn.setClickable(true); closeBtn.setFocusable(true);
        closeBtn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(48)));
        closeBtn.setOnClickListener(v -> dismiss.run());
        card.addView(closeBtn);
        overlay.addView(card);

    }

    private boolean isKnownProvider(String url) {
        if (url == null || url.isEmpty()) return false;
        String[] known = {
            "api.groq.com","api.openai.com","api.anthropic.com",
            "generativelanguage.googleapis.com","api.deepseek.com",
            "openrouter.ai","localhost:11434","api.together.xyz",
            "api.mistral.ai","api.cohere.ai","api.perplexity.ai",
            "api.x.ai","api.cerebras.ai","api.fireworks.ai",
            "api.sambanova.ai","api.novita.ai"
        };
        for (String k : known) if (url.contains(k)) return true;
        return false;
    }

    private void showCustomProviderDialog() {
        android.app.AlertDialog.Builder b = new android.app.AlertDialog.Builder(this);
        b.setTitle("Custom AI Provider");
        android.widget.LinearLayout layout = new android.widget.LinearLayout(this);
        layout.setOrientation(android.widget.LinearLayout.VERTICAL);
        layout.setPadding(dp(24), dp(16), dp(24), dp(8));
        layout.setBackgroundColor(0xFF0e0e18);

        android.widget.TextView urlLabel = new android.widget.TextView(this);
        urlLabel.setText("Base URL  (e.g. https://your-server/v1)");
        urlLabel.setTextColor(0xFF8888AA); urlLabel.setTextSize(11);
        layout.addView(urlLabel);

        android.widget.EditText urlInput = styledEditText(cfg.baseUrl, false);
        layout.addView(urlInput);

        android.widget.TextView modelLabel = new android.widget.TextView(this);
        modelLabel.setText("Model name");
        modelLabel.setTextColor(0xFF8888AA); modelLabel.setTextSize(11);
        android.widget.LinearLayout.LayoutParams mlp =
            new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        mlp.topMargin = dp(12);
        modelLabel.setLayoutParams(mlp);
        layout.addView(modelLabel);

        android.widget.EditText modelInput = styledEditText(cfg.model, false);
        layout.addView(modelInput);

        b.setView(layout);
        b.setPositiveButton("Save", (d, x) -> {
            String url   = urlInput.getText().toString().trim();
            String model = modelInput.getText().toString().trim();
            if (url.isEmpty()) { android.widget.Toast.makeText(this, "URL required", android.widget.Toast.LENGTH_SHORT).show(); return; }
            cfg.baseUrl = url;
            if (!model.isEmpty()) cfg.model = model;
            cfg.save(this);
            try { RustBridge.setCustomProvider(url, model); } catch (Exception ignored) {}
            updateSettingsUI();
            android.widget.Toast.makeText(this, "Custom provider saved", android.widget.Toast.LENGTH_SHORT).show();
        });
        b.setNegativeButton("Cancel", null);
        b.show();
    }

    private android.widget.EditText styledEditText(String current, boolean numeric) {
        android.widget.EditText et = new android.widget.EditText(this);
        et.setText(current);
        et.setTextColor(0xFFFFFFFF);
        et.setHintTextColor(0xFF555566);
        et.setTextSize(14);
        android.graphics.drawable.GradientDrawable bg = new android.graphics.drawable.GradientDrawable();
        bg.setColor(0xFF1A1A2E);
        bg.setCornerRadius(dp(6));
        bg.setStroke(dp(1), 0xFF2a2a44);
        et.setBackground(bg);
        et.setPadding(dp(12), dp(10), dp(12), dp(10));
        et.setInputType(numeric
            ? android.text.InputType.TYPE_CLASS_NUMBER
            : (android.text.InputType.TYPE_CLASS_TEXT | android.text.InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS));
        android.widget.LinearLayout.LayoutParams lp =
            new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        lp.topMargin = dp(4);
        et.setLayoutParams(lp);
        return et;
    }


    // fetchRust kept for any residual calls but Rust backend not required in v38
    private String fetchRust(String url) {
        try {
            okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                .connectTimeout(2, java.util.concurrent.TimeUnit.SECONDS)
                .readTimeout(3, java.util.concurrent.TimeUnit.SECONDS).build();
            okhttp3.Response resp = client.newCall(new okhttp3.Request.Builder().url(url).build()).execute();
            return resp.body() != null ? resp.body().string() : "(empty)";
        } catch(Exception e) { return "(unavailable)"; }
    }

    // \u2500\u2500 Theme \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Apply dark or light theme to the whole UI */
    // ── OTA Update (v43: Rust-backed) ─────────────────────────────────────────

    private KiraOtaUpdater otaUpdater;

    private void initOta() {
        otaUpdater = new KiraOtaUpdater(this);
        otaUpdater.init();
        otaUpdater.setCallback(new KiraOtaUpdater.OtaCallback() {
            @Override public void onCheckStart() {
                uiHandler.post(() -> {
                    if (rustStatsHint != null) rustStatsHint.setText("checking…");
                });
            }
            @Override public void onUpdateAvailable(String ver, String log, Runnable onInstall, Runnable onSkip) {
                uiHandler.post(() -> showKiraDialogMulti(
                    "Update Available",
                    ver + " is ready\n\n" + (log.length() > 280 ? log.substring(0, 280) + "…" : log),
                    new String[]{"INSTALL", "LATER", "SKIP"},
                    new Runnable[]{ onInstall, null, onSkip }
                ));
            }
            @Override public void onProgress(int pct, long done, long total) {
                uiHandler.post(() -> {
                    String mb = String.format("%.1f / %.1f MB", done/1048576.0, total/1048576.0);
                    if (rustStatsHint != null) rustStatsHint.setText("⬇ " + pct + "% · " + mb);
                });
            }
            @Override public void onInstalling(String method) {
                uiHandler.post(() -> {
                    String label = "shizuku".equals(method) ? "installing silently…"
                        : "package_installer".equals(method) ? "installing…" : "opening installer…";
                    if (rustStatsHint != null) rustStatsHint.setText(label);
                });
            }
            @Override public void onSuccess(String ver) {
                uiHandler.post(() -> showKiraDialogMulti(
                    "Update Installed",
                    "Kira " + ver + " installed.\nRestart to apply changes.",
                    new String[]{"RESTART", "LATER"},
                    new Runnable[]{
                        () -> {
                            Intent ri = getPackageManager().getLaunchIntentForPackage(getPackageName());
                            if (ri != null) {
                                ri.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP | Intent.FLAG_ACTIVITY_NEW_TASK);
                                startActivity(ri);
                            }
                            android.os.Process.killProcess(android.os.Process.myPid());
                        },
                        null
                    }
                ));
            }
            @Override public void onError(String msg) {
                uiHandler.post(() -> {
                    if (rustStatsHint != null) rustStatsHint.setText("update error");
                    Toast.makeText(MainActivity.this, "OTA: " + msg, Toast.LENGTH_LONG).show();
                });
            }
            @Override public void onUpToDate() {
                uiHandler.post(() -> {
                    Toast.makeText(MainActivity.this, "Already up to date ✓", Toast.LENGTH_SHORT).show();
                    if (rustStatsHint != null) rustStatsHint.setText("up to date ✓");
                });
            }
        });
        otaUpdater.scheduleChecks();
    }

    /** Load Catppuccin Mocha tokens from Rust — updates T_ fields */
    private void loadThemeTokens() {
        try {
            String json = RustBridge.getTheme();
            if (json == null || json.isEmpty()) return;
            org.json.JSONObject t = new org.json.JSONObject(json);
            T_BG          = (int) t.optLong("bg",         T_BG);
            T_SURFACE     = (int) t.optLong("surface",    T_SURFACE);
            T_SURFACE2    = (int) t.optLong("surface2",   T_SURFACE2);
            T_SURFACE_VAR = (int) t.optLong("surface_var",T_SURFACE_VAR);
            T_SURFACE5    = (int) t.optLong("surface5",   T_SURFACE5);
            T_TEXT        = (int) t.optLong("on_surface", T_TEXT);
            T_TEXT2       = (int) t.optLong("muted",      T_TEXT2);
            T_TEXT3       = (int) t.optLong("muted",      T_TEXT3);
            T_ACCENT      = (int) t.optLong("accent",     T_ACCENT);
            T_ON_ACCENT   = (int) t.optLong("on_accent",  T_ON_ACCENT);
            T_SECONDARY   = (int) t.optLong("secondary",  T_SECONDARY);
            T_TERTIARY    = (int) t.optLong("tertiary",   T_TERTIARY);
            T_SUCCESS     = (int) t.optLong("success",    T_SUCCESS);
            T_ERROR       = (int) t.optLong("error",      T_ERROR);
            T_OUTLINE     = (int) t.optLong("outline",    T_OUTLINE);
        } catch (Throwable e) {
            Log.w("KiraTheme", "loadThemeTokens: " + e.getMessage());
        }
    }

    private void applyTheme() {
        loadThemeTokens();
        // ── System chrome ───────────────────────────────────────────────────
        getWindow().setStatusBarColor(isDarkTheme ? D_BG : L_BG);
        getWindow().setNavigationBarColor(isDarkTheme ? D_NAV : L_NAV);
        if (android.os.Build.VERSION.SDK_INT >= 26) {
            int flags = getWindow().getDecorView().getSystemUiVisibility();
            if (!isDarkTheme) {
                flags |= android.view.View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
                flags |= android.view.View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
            } else {
                flags &= ~android.view.View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
                flags &= ~android.view.View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
            }
            getWindow().getDecorView().setSystemUiVisibility(flags);
        }

        // ── Nav bar ─────────────────────────────────────────────────────────
        View nav = findViewById(R.id.bottomNav);
        if (nav != null) {
            // Neural Nav Bar: Catppuccin Mantle floating island
            android.graphics.drawable.GradientDrawable navBg =
                new android.graphics.drawable.GradientDrawable();
            navBg.setShape(android.graphics.drawable.GradientDrawable.RECTANGLE);
            navBg.setCornerRadius(dp(24));
            navBg.setColor(isDarkTheme ? 0xF0181825 : 0xF0E6E9EF); // Mantle / Crust light
            navBg.setStroke(1, isDarkTheme ? 0x4DB4BEFE : 0x4D7C84BF); // Lavender top edge
            nav.setBackground(navBg);
            nav.setElevation(dp(8));
        }
        // Re-colour nav icons to correct active/inactive state
        for (int i = 0; i < 4; i++) {
            boolean on = i == currentTab;
            if (navIcons != null && navIcons[i] != null)
                navIcons[i].setTextColor(on ? 0xFFB4BEFE : (isDarkTheme ? 0xFF6C7086 : 0xFF888899));
            if (navTexts != null && navTexts[i] != null)
                navTexts[i].setTextColor(on ? 0xFFB4BEFE : (isDarkTheme ? 0xFF6C7086 : 0xFF888899));
        }

        // ── Fragment backgrounds ────────────────────────────────────────────
        // Dark: transparent so GalaxyView shows through
        // Light: solid warm white
        int fragBg = isDarkTheme ? 0x00000000 : L_BG;
        if (homeFragment    != null) homeFragment.setBackgroundColor(fragBg);
        if (settingsFragment!= null) settingsFragment.setBackgroundColor(fragBg);
        if (historyFragment != null) historyFragment.setBackgroundColor(fragBg);
        if (toolsFragment   != null) toolsFragment.setBackgroundColor(fragBg);

        // ── Chat input bar ──────────────────────────────────────────────────
        View inputBar = homeFragment != null ? homeFragment.findViewWithTag("inputBar") : null;
        if (inputBar != null) inputBar.setBackgroundColor(isDarkTheme ? 0xF0090913 : 0xF0F0F0FC);
        if (inputField != null) {
            inputField.setBackgroundColor(isDarkTheme ? D_INPUT_BG : L_INPUT_BG);
            inputField.setTextColor(isDarkTheme ? D_TEXT : L_TEXT);
            inputField.setHintTextColor(isDarkTheme ? D_TEXT3 : L_TEXT3);
        }

        // ── Header subtitle ─────────────────────────────────────────────────
        if (headerSubtitle != null)
            headerSubtitle.setTextColor(isDarkTheme ? D_TEXT3 : L_TEXT3);

        // ── Chat bubbles ────────────────────────────────────────────────────
        if (chatContainer != null) {
            for (int i = 0; i < chatContainer.getChildCount(); i++) {
                View child = chatContainer.getChildAt(i);
                if (!(child instanceof LinearLayout)) continue;
                Object tag = child.getTag();
                if (tag == null) continue;
                String ts = tag.toString();
                LinearLayout ll = (LinearLayout) child;
                if (ts.startsWith("user_") && ll.getChildCount() > 1) {
                    View msgV = ll.getChildAt(1);
                    if (msgV instanceof android.widget.TextView) {
                        msgV.setBackgroundColor(isDarkTheme ? D_USER_BUBBLE : L_USER_BUBBLE);
                        ((android.widget.TextView)msgV).setTextColor(isDarkTheme ? D_TEXT : L_TEXT);
                    }
                } else if (ts.startsWith("kira_") && ll.getChildCount() > 1) {
                    View msgV = ll.getChildAt(1);
                    if (msgV instanceof android.widget.TextView) {
                        msgV.setBackgroundColor(isDarkTheme ? D_KIRA_BUBBLE : L_KIRA_BUBBLE);
                        ((android.widget.TextView)msgV).setTextColor(isDarkTheme ? D_TEXT : L_TEXT);
                    }
                }
            }
        }

        // ── Settings hints ──────────────────────────────────────────────────
        if (settingsFragment != null) {
            int hintCol = isDarkTheme ? D_TEXT3 : L_TEXT3;
            int[] hintIds = { R.id.apiKeyHint, R.id.modelHint, R.id.baseUrlHint,
                R.id.tgTokenHint, R.id.tgIdHint, R.id.visionHint, R.id.maxStepsHint,
                R.id.heartbeatHint, R.id.personaHint, R.id.providerHint, R.id.skillsHint,
                R.id.checkpointsHint, R.id.auditHint, R.id.userNameHint,
                R.id.rustStatsHint, R.id.memoryHint, R.id.historySettingHint };
            for (int id : hintIds) {
                android.widget.TextView tv = settingsFragment.findViewById(id);
                if (tv != null) tv.setTextColor(hintCol);
            }
            if (rustStatsContent != null) {
                rustStatsContent.setTextColor(isDarkTheme ? 0xFF44AA44 : 0xFF226622);
                rustStatsContent.setBackgroundColor(isDarkTheme ? 0xFF030308 : 0xFFF4F8F4);
            }
            if (memoryContent != null) {
                memoryContent.setTextColor(isDarkTheme ? 0xFF44AA44 : 0xFF226622);
                memoryContent.setBackgroundColor(isDarkTheme ? 0xFF050508 : 0xFFF4F8F4);
            }
        }

        // ── Save preference ─────────────────────────────────────────────────
        getSharedPreferences("kira_theme", MODE_PRIVATE)
            .edit().putBoolean("dark", isDarkTheme).apply();
    }

    /** Toggle theme with Layer 5 camera-flash transition */
    private void toggleTheme() {
        flashThemeTransition();  // Layer 5: flash on/off
        uiHandler.postDelayed(() -> {
            isDarkTheme = !isDarkTheme;
            applyTheme();
            // Notify Rust of new theme
            final boolean dark = isDarkTheme;
            new Thread(() -> { try { new okhttp3.OkHttpClient().newCall(
                new okhttp3.Request.Builder()
                    .url("http://localhost:7070/theme/flash")
                    .post(okhttp3.RequestBody.create(
                        "{\"dark\":"+dark+"}", okhttp3.MediaType.parse("application/json")))
                    .build()).execute(); } catch (Exception ignored) {} }).start();
        }, 80);  // slight delay so flash precedes colour swap
    }

    // \u2500\u2500 Multi-button dialog \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /**
     * Kira dialog with N buttons (1-3). Labels array maps to actions array.
     * null action = dismiss only.
     */
    private void showKiraDialogMulti(String title, String msg, String[] labels, Runnable[] actions) {
        android.widget.FrameLayout overlay = new android.widget.FrameLayout(this);
        overlay.setBackgroundColor(0xCC000000);
        overlay.setLayoutParams(new android.widget.FrameLayout.LayoutParams(MATCH, MATCH));

        android.widget.LinearLayout card = new android.widget.LinearLayout(this);
        card.setOrientation(android.widget.LinearLayout.VERTICAL);
        int cardColor  = t(D_SURFACE, L_SURFACE);
        int borderColor= t(D_BORDER, L_BORDER);
        android.graphics.drawable.GradientDrawable cardBg = new android.graphics.drawable.GradientDrawable();
        cardBg.setColor(cardColor); cardBg.setCornerRadius(dp(4)); cardBg.setStroke(dp(1), borderColor);
        card.setBackground(cardBg);
        android.widget.FrameLayout.LayoutParams cardLp = new android.widget.FrameLayout.LayoutParams(
            (int)(getResources().getDisplayMetrics().widthPixels * 0.88f), WRAP);
        cardLp.gravity = android.view.Gravity.CENTER;
        card.setLayoutParams(cardLp);

        // Top accent
        android.view.View bar = new android.view.View(this);
        bar.setBackgroundColor(0xFFDC143C);
        bar.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(2)));
        card.addView(bar);

        // Title row
        android.widget.LinearLayout titleRow = new android.widget.LinearLayout(this);
        titleRow.setOrientation(android.widget.LinearLayout.HORIZONTAL);
        titleRow.setGravity(android.view.Gravity.CENTER_VERTICAL);
        titleRow.setPadding(dp(18), dp(14), dp(18), dp(10));
        android.widget.TextView titleTv = new android.widget.TextView(this);
        titleTv.setText(title);
        titleTv.setTextColor(t(D_TEXT, L_TEXT));
        titleTv.setTextSize(14); titleTv.setTypeface(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD);
        titleTv.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, WRAP, 1));
        android.widget.TextView kBadge = new android.widget.TextView(this);
        kBadge.setText("K"); kBadge.setTextColor(0x33DC143C); kBadge.setTextSize(20);
        kBadge.setTypeface(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD);
        titleRow.addView(titleTv); titleRow.addView(kBadge);
        card.addView(titleRow);

        android.view.View sep = new android.view.View(this);
        sep.setBackgroundColor(t(D_BORDER, L_BORDER));
        sep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(sep);

        // Message
        android.widget.TextView msgTv = new android.widget.TextView(this);
        msgTv.setText(msg);
        msgTv.setTextColor(t(D_TEXT2, L_TEXT2));
        msgTv.setTextSize(12); msgTv.setTypeface(android.graphics.Typeface.MONOSPACE);
        msgTv.setLineSpacing(dp(2), 1);
        android.widget.LinearLayout.LayoutParams msgLp = new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        msgLp.setMargins(dp(18), dp(12), dp(18), dp(16)); msgTv.setLayoutParams(msgLp);
        card.addView(msgTv);

        // Button row
        android.view.View btnSep = new android.view.View(this);
        btnSep.setBackgroundColor(t(D_BORDER, L_BORDER));
        btnSep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(btnSep);

        android.widget.LinearLayout btnRow = new android.widget.LinearLayout(this);
        btnRow.setOrientation(android.widget.LinearLayout.HORIZONTAL);
        btnRow.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(52)));

        android.view.ViewGroup root = (android.view.ViewGroup) getWindow().getDecorView();
        root.addView(overlay);
        Runnable dismiss = () -> root.removeView(overlay);
        overlay.setOnClickListener(v -> dismiss.run());
        card.setOnClickListener(v -> {});

        for (int i = 0; i < labels.length; i++) {
            final int idx = i;
            if (i > 0) {
                android.view.View bd = new android.view.View(this);
                bd.setBackgroundColor(t(D_BORDER, L_BORDER));
                bd.setLayoutParams(new android.widget.LinearLayout.LayoutParams(dp(1), MATCH));
                btnRow.addView(bd);
            }
            android.widget.TextView btn = new android.widget.TextView(this);
            btn.setText(labels[i]);
            // Last button = primary (crimson), others = muted
            boolean isPrimary = (i == 0);
            btn.setTextColor(isPrimary ? ACCENT : t(D_TEXT3, L_TEXT3));
            btn.setTextSize(11);
            btn.setTypeface(android.graphics.Typeface.MONOSPACE, isPrimary ? android.graphics.Typeface.BOLD : android.graphics.Typeface.NORMAL);
            btn.setGravity(android.view.Gravity.CENTER);
            btn.setClickable(true); btn.setFocusable(true);
            btn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, MATCH, 1));
            btn.setOnClickListener(v -> {
                dismiss.run();
                if (actions[idx] != null) actions[idx].run();
            });
            btnRow.addView(btn);
        }
        card.addView(btnRow);
        overlay.addView(card);
    }

    // \u2500\u2500 OTA Update \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500

    /** Custom Kira info dialog \u2014 no stock Android chrome */
    private void showInfoDialog(String title, String msg) {
        uiHandler.post(() -> showKiraDialog(title, msg.length() > 3000 ? msg.substring(0, 3000) + "\u2026" : msg, "OK", "CLOSE", null));
    }

    private void showConfirmDialog(String msg, Runnable action) {
        showKiraDialog("Confirm", msg, "YES", "CANCEL", action);
    }

    private void showKiraDialog(String title, String msg, String posLabel, String negLabel, Runnable posAction) {
        android.widget.FrameLayout overlay = new android.widget.FrameLayout(this);
        overlay.setBackgroundColor(0xBB000000);
        overlay.setLayoutParams(new android.widget.FrameLayout.LayoutParams(MATCH, MATCH));

        android.widget.LinearLayout card = new android.widget.LinearLayout(this);
        card.setOrientation(android.widget.LinearLayout.VERTICAL);
        android.graphics.drawable.GradientDrawable cardBg = new android.graphics.drawable.GradientDrawable();
        cardBg.setColor(0xFF0c0c18);
        cardBg.setCornerRadius(dp(4));
        cardBg.setStroke(dp(1), 0xFF1a1a2e);
        card.setBackground(cardBg);
        android.widget.FrameLayout.LayoutParams cardLp = new android.widget.FrameLayout.LayoutParams(
            (int)(getResources().getDisplayMetrics().widthPixels * 0.88f), WRAP);
        cardLp.gravity = android.view.Gravity.CENTER;
        card.setLayoutParams(cardLp);

        // Accent bar
        android.view.View bar = new android.view.View(this);
        bar.setBackgroundColor(0xFFDC143C);
        bar.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(2)));
        card.addView(bar);

        // Title
        android.widget.TextView titleTv = new android.widget.TextView(this);
        titleTv.setText(title);
        titleTv.setTextColor(0xFFFFFFFF);
        titleTv.setTextSize(14);
        titleTv.setTypeface(android.graphics.Typeface.MONOSPACE, android.graphics.Typeface.BOLD);
        android.widget.LinearLayout.LayoutParams ttLp = new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        ttLp.setMargins(dp(18), dp(14), dp(18), dp(8));
        titleTv.setLayoutParams(ttLp);
        card.addView(titleTv);

        // Separator
        android.view.View sep = new android.view.View(this);
        sep.setBackgroundColor(0xFF111122);
        sep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(sep);

        // Message (scrollable for long content)
        android.widget.ScrollView sv = new android.widget.ScrollView(this);
        android.widget.LinearLayout.LayoutParams svLp = new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        svLp.setMargins(0, 0, 0, 0);
        sv.setLayoutParams(svLp);
        // constrain via LayoutParams
        svLp.height = dp(300); svLp.weight = 0;

        android.widget.TextView msgTv = new android.widget.TextView(this);
        msgTv.setText(msg);
        msgTv.setTextColor(0xFF8888AA);
        msgTv.setTextSize(12);
        msgTv.setTypeface(android.graphics.Typeface.MONOSPACE);
        msgTv.setLineSpacing(dp(2), 1);
        android.widget.LinearLayout.LayoutParams msgLp = new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        msgLp.setMargins(dp(18), dp(12), dp(18), dp(14));
        msgTv.setLayoutParams(msgLp);
        sv.addView(msgTv);
        card.addView(sv);

        // Button row
        android.view.View btnSep = new android.view.View(this);
        btnSep.setBackgroundColor(0xFF0e0e1e);
        btnSep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(btnSep);

        android.widget.LinearLayout btnRow = new android.widget.LinearLayout(this);
        btnRow.setOrientation(android.widget.LinearLayout.HORIZONTAL);
        btnRow.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(48)));

        android.widget.TextView negBtn = new android.widget.TextView(this);
        negBtn.setText(negLabel);
        negBtn.setTextColor(0xFF333355);
        negBtn.setTextSize(11);
        negBtn.setTypeface(android.graphics.Typeface.MONOSPACE);
        negBtn.setGravity(android.view.Gravity.CENTER);
        negBtn.setClickable(true); negBtn.setFocusable(true);
        negBtn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, MATCH, 1));

        android.view.View bd = new android.view.View(this);
        bd.setBackgroundColor(0xFF0e0e1e);
        bd.setLayoutParams(new android.widget.LinearLayout.LayoutParams(dp(1), MATCH));

        android.widget.TextView posBtn = new android.widget.TextView(this);
        posBtn.setText(posLabel);
        posBtn.setTextColor(0xFFDC143C);
        posBtn.setTextSize(11);
        posBtn.setTypeface(null, android.graphics.Typeface.BOLD);
        posBtn.setTypeface(android.graphics.Typeface.MONOSPACE);
        posBtn.setGravity(android.view.Gravity.CENTER);
        posBtn.setClickable(true); posBtn.setFocusable(true);
        posBtn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, MATCH, 1));

        btnRow.addView(negBtn);
        btnRow.addView(bd);
        btnRow.addView(posBtn);
        card.addView(btnRow);

        overlay.addView(card);

        android.view.ViewGroup root = (android.view.ViewGroup) getWindow().getDecorView();
        root.addView(overlay);

        Runnable dismiss = () -> root.removeView(overlay);

        negBtn.setOnClickListener(v -> dismiss.run());
        overlay.setOnClickListener(v -> dismiss.run());
        card.setOnClickListener(v -> {});
        posBtn.setOnClickListener(v -> {
            dismiss.run();
            if (posAction != null) posAction.run();
        });
    }

    private void scrollToBottom() {
        chatScroll.post(() -> chatScroll.fullScroll(View.FOCUS_DOWN));
    }

    private void copyText(String text) {
        android.content.ClipboardManager cm = (android.content.ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        if (cm != null) cm.setPrimaryClip(android.content.ClipData.newPlainText("kira", text));
        Toast.makeText(this, "Copied", Toast.LENGTH_SHORT).show();
    }

    // -- Suggestions -----------------------------------------------------------

    /** Maps tool name to a natural-language example the user can send */
    // ═══════════════════════════════════════════════════════════
    // Layer 6 — Lightning Engine
    // Canvas overlay on root FrameLayout. Fires once per event.
    // ═══════════════════════════════════════════════════════════

    /** Draw and immediately decay a lightning event on the root overlay. */
    private void fireLightning(int type) {
        android.view.ViewGroup root = (android.view.ViewGroup)
            getWindow().getDecorView().getRootView();

        android.view.View bolt = new android.view.View(this) {
            private final android.graphics.Paint lp =
                new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
            private float alpha = 1f;
            private final long born = System.currentTimeMillis();
            private final android.graphics.Path path = new android.graphics.Path();
            private boolean built = false;

            @Override protected void onDraw(android.graphics.Canvas canvas) {
                long age = System.currentTimeMillis() - born;
                if (age > 250) { setVisibility(GONE); return; }
                alpha = 1f - age / 250f;
                int w = getWidth(), h = getHeight();
                if (!built && w > 0) {
                    built = true;
                    switch (type) {
                        case 0: { // Reply: branching arc from K badge area to center-bottom
                            float sx = dp(44), sy = dp(52);  // K badge position
                            float ex = w * 0.5f, ey = h * 0.65f;
                            path.moveTo(sx, sy);
                            // 3 jagged branches
                            float mx1 = sx + (ex-sx)*0.33f + 20, my1 = sy + (ey-sy)*0.33f - 30;
                            float mx2 = ex - (ex-sx)*0.33f - 20, my2 = ey - (ey-sy)*0.33f + 20;
                            path.lineTo(mx1, my1);
                            path.lineTo(mx1 + 30, my1 + 40); // branch 1
                            path.moveTo(mx1, my1);
                            path.lineTo(mx2, my2);
                            path.lineTo(mx2 - 25, my2 + 35); // branch 2
                            path.moveTo(mx2, my2);
                            path.lineTo(ex, ey);
                            lp.setStrokeWidth(dp(2));
                            lp.setColor(0xFFB4BEFE); // Lavender
                            break;
                        }
                        case 1: { // Macro: horizontal streak top 4dp
                            path.moveTo(0, dp(2));
                            path.lineTo(w, dp(2));
                            lp.setStrokeWidth(dp(4));
                            lp.setColor(0xFFFAB387); // Peach
                            break;
                        }
                        case 2: { // Shizuku: 4 radial lines from status dot
                            float cx = w - dp(22), cy = dp(26); // status dot position
                            for (int i = 0; i < 4; i++) {
                                double ang = i * Math.PI / 2;
                                path.moveTo(cx, cy);
                                path.lineTo(cx + (float)(Math.cos(ang) * dp(28)),
                                            cy + (float)(Math.sin(ang) * dp(28)));
                            }
                            lp.setStrokeWidth(dp(2));
                            lp.setColor(0xFFA6E3A1); // Green
                            break;
                        }
                    }
                    lp.setStyle(android.graphics.Paint.Style.STROKE);
                    lp.setStrokeCap(android.graphics.Paint.Cap.ROUND);
                    lp.setStrokeJoin(android.graphics.Paint.Join.ROUND);
                }
                lp.setAlpha((int)(alpha * 178)); // 70% max
                canvas.drawPath(path, lp);
                // 1-frame white flash at origin on first frame
                if (age < 32) {
                    android.graphics.Paint flash = new android.graphics.Paint();
                    flash.setColor(0xFFFFFFFF);
                    flash.setAlpha((int)(( 1f - age/32f) * 100));
                    canvas.drawCircle(type == 0 ? dp(44) : (type == 2 ? getWidth()-dp(22) : 0),
                                      type == 0 ? dp(52) : (type == 2 ? dp(26) : dp(2)), dp(8), flash);
                }
                postInvalidateDelayed(16);
            }

            private int dp(int v) {
                return Math.round(v * getResources().getDisplayMetrics().density);
            }
        };
        bolt.setLayoutParams(new android.view.ViewGroup.LayoutParams(
            android.view.ViewGroup.LayoutParams.MATCH_PARENT,
            android.view.ViewGroup.LayoutParams.MATCH_PARENT));
        bolt.setClickable(false); bolt.setFocusable(false);
        root.addView(bolt);
        // Auto-remove after animation
        uiHandler.postDelayed(() -> root.removeView(bolt), 300);
    }

    private String buildToolExample(String tool) {
        switch (tool) {
            case "open_app":          return "open YouTube";
            case "read_screen":       return "read my screen";
            case "tap_screen":        return "tap screen at 540 960";
            case "tap_text":          return "tap the button that says OK";
            case "type_text":         return "type Hello world";
            case "swipe_screen":      return "swipe up";
            case "get_notifications": return "show my notifications";
            case "send_sms":          return "send SMS to +1234567890 saying hello";
            case "make_call":         return "call +1234567890";
            case "web_search":        return "search the web for latest AI news";
            case "analyze_screen":    return "what is on my screen?";
            case "find_element":      return "find and tap the search icon";
            case "sh_run":            return "run shell command: pm list packages";
            case "sh_screenshot":     return "take a screenshot";
            case "remember":          return "remember my name is Imran";
            case "recall":            return "what do you remember about me?";
            case "battery_info":      return "what is my battery level?";
            case "list_files":        return "list files in /sdcard/Download";
            case "get_wifi_info":     return "show my WiFi info";
            case "watch_notif":       return "watch for notification containing payment";
            case "schedule_task":     return "in 5 minutes check battery";
            case "if_then":           return "if battery below 20% then notify me";
            case "repeat_task":       return "every 30 minutes check notifications";
            case "open_url":          return "open https://news.ycombinator.com";
            case "send_email":        return "send email to test@example.com subject hello";
            default: return "what can you do?";
        }
    }

    // ── Layer 2: Subtitle crossfade cycle ───────────────────────────────────
    private final android.os.Handler subtitleHandler =
        new android.os.Handler(android.os.Looper.getMainLooper());
    private Runnable subtitleRunnable;
    private int subtitleIdx = 0;

    private void startSubtitleCycle(String[] labels) {
        subtitleIdx = 0;
        subtitleHandler.removeCallbacks(subtitleRunnable);
        subtitleRunnable = new Runnable() {
            @Override public void run() {
                if (headerSubtitle == null) return;
                // Fade out → change → fade in
                headerSubtitle.animate().alpha(0f).setDuration(150)
                    .withEndAction(() -> {
                        subtitleIdx = (subtitleIdx + 1) % labels.length;
                        if (headerSubtitle != null) headerSubtitle.setText(labels[subtitleIdx]);
                        headerSubtitle.animate().alpha(1f).setDuration(150).start();
                    }).start();
                subtitleHandler.postDelayed(this, 1800);
            }
        };
        if (headerSubtitle != null) headerSubtitle.setText(labels[0]);
        subtitleHandler.postDelayed(subtitleRunnable, 1800);
    }

    private void stopSubtitleCycle() {
        subtitleHandler.removeCallbacks(subtitleRunnable);
    }

    // ── Layer 2: Animated typing indicator (three sine-wave dots) ────────
    private LinearLayout typingIndicator;
    private android.os.Handler typingHandler =
        new android.os.Handler(android.os.Looper.getMainLooper());

    private void showTypingIndicator() {
        if (typingIndicator != null) { chatContainer.removeView(typingIndicator); }
        typingIndicator = new LinearLayout(this);
        typingIndicator.setOrientation(LinearLayout.HORIZONTAL);
        typingIndicator.setGravity(android.view.Gravity.CENTER_VERTICAL);
        typingIndicator.setPadding(dp(16), dp(8), dp(16), dp(8));
        typingIndicator.setTag("typing_indicator");
        // 3 Lavender dots with staggered sine-wave bounce
        for (int i = 0; i < 3; i++) {
            View dot = new View(this);
            dot.setBackgroundColor(0xFFB4BEFE);  // Lavender
            LinearLayout.LayoutParams lp = new LinearLayout.LayoutParams(dp(6), dp(6));
            lp.setMargins(dp(3), 0, dp(3), 0);
            dot.setLayoutParams(lp);
            typingIndicator.addView(dot);
            // Stagger: 0ms, 120ms, 240ms
            final int delay = i * 120;
            uiHandler.postDelayed(() -> animateDot(dot), delay);
        }
        typingIndicator.setAlpha(0f);
        chatContainer.addView(typingIndicator);
        typingIndicator.animate().alpha(1f).setDuration(100).start();
        scrollToBottom();
    }

    private void animateDot(View dot) {
        dot.animate()
            .translationY(-dp(4))
            .setDuration(300)
            .setInterpolator(new android.view.animation.OvershootInterpolator(1.5f))
            .withEndAction(() -> dot.animate()
                .translationY(0)
                .setDuration(300)
                .withEndAction(() -> {
                    // Repeat if still visible
                    if (typingIndicator != null && typingIndicator.getParent() != null) {
                        uiHandler.postDelayed(() -> animateDot(dot), 200);
                    }
                }).start())
            .start();
    }

    private void hideTypingIndicator() {
        if (typingIndicator != null) {
            final LinearLayout ti = typingIndicator;
            typingIndicator = null;
            ti.animate().alpha(0f).setDuration(150)
                .withEndAction(() -> chatContainer.removeView(ti)).start();
        }
    }

    // ── Layer 5: Section header underline animation ─────────────────────────
    private void animateSectionHeaders() {
        if (settingsFragment == null) return;
        android.widget.ScrollView sv = settingsFragment.findViewById(R.id.settingsScroll);
        if (sv == null) return;
        // Find all TextViews with SectionHeader style (identified by their appearance)
        scanForHeaders((android.view.ViewGroup) sv.getChildAt(0));
    }

    private void scanForHeaders(android.view.ViewGroup group) {
        if (group == null) return;
        for (int i = 0; i < group.getChildCount(); i++) {
            View child = group.getChildAt(i);
            if (child instanceof TextView) {
                TextView tv = (TextView) child;
                // Section headers: Lavender, 10sp, bold, monospace
                if (tv.getTextColors().getDefaultColor() == 0xFFDC143C ||
                    tv.getTextColors().getDefaultColor() == 0xFFB4BEFE) {
                    // Add animated underline via ViewTreeObserver
                    attachScrollReveal(tv);
                }
            } else if (child instanceof android.view.ViewGroup) {
                scanForHeaders((android.view.ViewGroup) child);
            }
        }
    }

    private void attachScrollReveal(View v) {
        v.getViewTreeObserver().addOnGlobalLayoutListener(() -> {
            if (!v.isShown()) return;
            // Animate ScaleX from 0 to 1 only once
            if (v.getScaleX() < 1f) {
                v.setScaleX(0f);
                v.setPivotX(0f);
                v.animate().scaleX(1f).setDuration(400)
                    .setInterpolator(new android.view.animation.DecelerateInterpolator())
                    .start();
            }
        });
    }

    // ── Layer 5: Theme toggle camera-flash effect ─────────────────────────────
    private void flashThemeTransition() {
        View root = getWindow().getDecorView().getRootView();
        View flash = new View(this);
        flash.setBackgroundColor(isDarkTheme ? 0x33000000 : 0x33FFFFFF);
        flash.setLayoutParams(new android.view.ViewGroup.LayoutParams(
            android.view.ViewGroup.LayoutParams.MATCH_PARENT,
            android.view.ViewGroup.LayoutParams.MATCH_PARENT));
        if (root instanceof android.widget.FrameLayout) {
            android.widget.FrameLayout fl = (android.widget.FrameLayout) root;
            fl.addView(flash);
            flash.animate().alpha(0f).setDuration(300)
                .withEndAction(() -> fl.removeView(flash)).start();
        }
    }

    // ── Layer 5: CounterAnimator — animate number changes over 600ms EaseOut ──
    private final java.util.Map<Integer, String> counterLastValues = new java.util.HashMap<>();

    private void animateCounter(View parent, int viewId, String newValue) {
        if (parent == null) return;
        TextView tv = parent.findViewById(viewId);
        if (tv == null) return;
        String old = counterLastValues.get(viewId);
        if (newValue.equals(old)) return;
        counterLastValues.put(viewId, newValue);
        // Fade out → update → fade in (counter feel)
        tv.animate().alpha(0.4f).setDuration(150)
            .withEndAction(() -> {
                tv.setText(newValue);
                tv.animate().alpha(1f).setDuration(300)
                    .setInterpolator(new android.view.animation.DecelerateInterpolator())
                    .start();
            }).start();
    }

    // ── Layer 9 (via L5): God Mode Halo — Lavender border traces screen edge ──
    private android.animation.ObjectAnimator haloAnimator;
    private View haloView;

    private void applyGodModeHalo(boolean visible, int color, int revolutionMs) {
        // Halo is a fixed overlay view on the root window
        android.view.ViewGroup root = (android.view.ViewGroup)
            getWindow().getDecorView().getRootView();
        if (!visible) {
            if (haloView != null) {
                haloView.setVisibility(View.GONE);
                if (haloAnimator != null) haloAnimator.cancel();
            }
            return;
        }
        // Create halo view if needed
        if (haloView == null) {
            haloView = new View(this) {
                private final android.graphics.Paint haloPaint =
                    new android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG);
                private float rotation = 0f;

                @Override protected void onDraw(android.graphics.Canvas canvas) {
                    if (!visible) return;
                    int w = getWidth(), h = getHeight();
                    haloPaint.setColor(color);
                    haloPaint.setStyle(android.graphics.Paint.Style.STROKE);
                    haloPaint.setStrokeWidth(dp(2));
                    haloPaint.setAlpha(180);
                    android.graphics.RectF rect = new android.graphics.RectF(1, 1, w-1, h-1);
                    // Draw rotating arc: 30dp long, orbiting the screen perimeter
                    float perimeter = 2f * (w + h);
                    float arcFraction = dp(30) / perimeter * 360f;
                    canvas.drawArc(rect, rotation, arcFraction, false, haloPaint);
                    postInvalidateDelayed(16);
                }

                private int dp(int v) {
                    return Math.round(v * getResources().getDisplayMetrics().density);
                }
            };
            haloView.setLayoutParams(new android.view.ViewGroup.LayoutParams(
                android.view.ViewGroup.LayoutParams.MATCH_PARENT,
                android.view.ViewGroup.LayoutParams.MATCH_PARENT));
            haloView.setClickable(false);
            haloView.setFocusable(false);
            root.addView(haloView);
        }
        haloView.setVisibility(View.VISIBLE);
        // Rotate the arc: full revolution = revolutionMs
        if (haloAnimator != null) haloAnimator.cancel();
        haloAnimator = android.animation.ObjectAnimator.ofFloat(haloView, "rotation", 0f, 360f);
        haloAnimator.setDuration(revolutionMs > 0 ? revolutionMs : 4000);
        haloAnimator.setRepeatCount(android.animation.ValueAnimator.INFINITE);
        haloAnimator.setInterpolator(new android.view.animation.LinearInterpolator());
        haloAnimator.start();
        haloView.invalidate();
    }

    private void buildSuggestions() {
        String[][] s = {
            {"\uD83D\uDCF1 Open YouTube",    "Open YouTube"},
            {"\uD83D\uDD14 Notifications",   "Check notifications"},
            {"\uD83D\uDD0B Battery",         "Battery status"},
            {"\uD83D\uDCF8 Screenshot",      "Take screenshot"},
            {"\uD83C\uDF10 Search web",      "Search web for news"},
            {"\uD83D\uDDBC Read screen",     "Read my screen"},
            {"\uD83D\uDCEC SMS",             "Show recent SMS"},
            {"\u26A1 Running apps",          "Running apps"},
            {"\u26A1 Agent",                 "/agent open youtube"},
        };
        for (String[] item : s) {
            TextView chip = new TextView(this);
            chip.setText(item[0]);
            chip.setTextSize(11);
            chip.setTextColor(0xFF8888AA);
            chip.setTypeface(android.graphics.Typeface.MONOSPACE);
            android.graphics.drawable.GradientDrawable chipBg = new android.graphics.drawable.GradientDrawable();
            chipBg.setColor(0xAA080814);
            chipBg.setCornerRadius(dp(2));
            chipBg.setStroke(dp(1), 0xFF111130);
            chip.setBackground(chipBg);
            chip.setPadding(dp(10), dp(6), dp(10), dp(6));
            LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
            p.setMargins(0, 0, dp(8), 0);
            chip.setLayoutParams(p);
            chip.setOnClickListener(v -> { inputField.setText(item[1]); sendMessage(); });
            suggestionsRow.addView(chip);
        }
    }

    // -- Tools list ------------------------------------------------------------

    private void buildToolsList() {
        LinearLayout list = toolsFragment.findViewById(R.id.toolsList);
        Object[][] tools = {
            // Tap row = paste example. Long-press = send immediately.
            {"\uD83D\uDCF1","open_app","Open any app by name"},
            {"\uD83D\uDC41","read_screen {}","Read all visible text"},
            {"\uD83D\uDC46","tap_screen {x,y}","Tap coordinates"},
            {"\uD83D\uDD0D","tap_text {text}","Find and tap by text"},
            {"\u2328","type_text {text}","Type into focused field"},
            {"\uD83D\uDD14","get_notifications {}","All notifications"},
            {"\uD83D\uDCAC","send_sms {number,message}","Send SMS"},
            {"\uD83C\uDF10","web_search {query}","Search DuckDuckGo"},
            {"\uD83D\uDD2D","analyze_screen {question}","Vision AI"},
            {"\u26A1","sh_run {cmd}","Shell command (Shizuku)"},
            {"\uD83D\uDCF8","sh_screenshot {}","Screenshot"},
            {"\uD83E\uDDE0","remember {key,value}","Store fact"},
            {"\uD83D\uDD0B","battery_info {}","Battery level"},
            {"\uD83D\uDCC2","list_files {path}","List directory"},
            {"\uD83D\uDCF6","get_wifi_info {}","WiFi info"},
            {"\uD83D\uDD14","watch_notif {keyword,action}","Watch notifications"},
            {"\u23F0","schedule_task {task,minutes}","Schedule task"},
            {"\uD83D\uDD2D","find_element {description}","Vision: tap by description"},
        };
        for (Object[] t : tools) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            android.graphics.drawable.GradientDrawable rowBg = new android.graphics.drawable.GradientDrawable();
            rowBg.setColor(0xAA060610);
            rowBg.setCornerRadius(0);
            row.setBackground(rowBg);
            row.setPadding(dp(14), dp(12), dp(14), dp(12));
            // Left accent bar via start padding \u2014 fake left border
            row.setPadding(dp(12), dp(10), dp(14), dp(10));
            LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(MATCH, WRAP);
            rp.setMargins(0, 0, 0, dp(2));
            row.setLayoutParams(rp);
            row.setClickable(true); row.setFocusable(true);

            final String toolN = ((String)t[1]);
            final String toolEx = buildToolExample(toolN);
            row.setOnClickListener(v -> {
                showTab(0);
                if (inputField != null) {
                    inputField.setText(toolEx);
                    inputField.setSelection(toolEx.length());
                    inputField.requestFocus();
                }
            });
            row.setOnLongClickListener(v -> {
                showTab(0);
                if (inputField != null) { inputField.setText(toolEx); sendMessage(); }
                return true;
            });

            TextView icon = new TextView(this);
            icon.setText((String)t[0]); icon.setTextSize(20);
            icon.setWidth(dp(44)); icon.setGravity(Gravity.CENTER);

            LinearLayout info = new LinearLayout(this);
            info.setOrientation(LinearLayout.VERTICAL);
            info.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

            TextView name = new TextView(this); name.setText((String)t[1]);
            name.setTextColor(0xFFCCCCDD); name.setTextSize(12);
            name.setTypeface(android.graphics.Typeface.MONOSPACE);

            TextView desc = new TextView(this); desc.setText((String)t[2]);
            desc.setTextColor(0xFF8888AA); desc.setTextSize(12);

            info.addView(name); info.addView(desc);
            row.addView(icon); row.addView(info);
            // L7: cascade stagger 30ms per row
            final android.view.View fr = row;
            final int tidx = list.getChildCount();
            fr.setAlpha(0f); fr.setTranslationX(-dp(20));
            list.addView(fr);
            uiHandler.postDelayed(() -> fr.animate()
                .alpha(1f).translationX(0f)
                .setDuration(200)
                .setInterpolator(new android.view.animation.DecelerateInterpolator())
                .start(), tidx * 30L);
        }
    }

    // -- History -- Claude-style ------------------------------------------------

    private void refreshHistory() {
        historyList.removeAllViews();
        try {
            KiraMemory mem = new KiraMemory(this);
            JSONArray arr = mem.loadHistory();
            if (arr.length() == 0) {
                historyCount.setText("No conversations yet");
                return;
            }
            historyCount.setText(arr.length() + " conversations");

            for (int i = arr.length() - 1; i >= 0; i--) {
                JSONObject entry = arr.getJSONObject(i);
                String user = entry.getString("user");
                String kira = entry.getString("kira");
                long at = entry.optLong("at", 0);
                String timeStr = at > 0
                    ? new java.text.SimpleDateFormat("MMM d, HH:mm", java.util.Locale.getDefault()).format(new java.util.Date(at))
                    : "";

                LinearLayout card = new LinearLayout(this);
                card.setOrientation(LinearLayout.VERTICAL);
                android.graphics.drawable.GradientDrawable histBg = new android.graphics.drawable.GradientDrawable();
                histBg.setColor(0xAA060610);
                histBg.setStroke(dp(1), 0xFF0e0e22);
                card.setBackground(histBg);
                card.setPadding(dp(14), dp(12), dp(14), dp(12));
                LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(MATCH, WRAP);
                cp.setMargins(0, 0, 0, dp(8));
                card.setLayoutParams(cp);

                // Header: time + action buttons
                LinearLayout headerRow = new LinearLayout(this);
                headerRow.setOrientation(LinearLayout.HORIZONTAL);
                headerRow.setGravity(Gravity.CENTER_VERTICAL);
                headerRow.setPadding(0, 0, 0, dp(6));

                TextView timeTv = new TextView(this);
                timeTv.setText(timeStr);
                timeTv.setTextColor(0xFF555555);
                timeTv.setTextSize(11);
                timeTv.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

                // Copy Kira reply
                TextView copyKira = makeActionBtn("copy reply");
                copyKira.setOnClickListener(v -> copyText(kira));

                // Resend -- puts user message in input and sends
                TextView resendBtn = makeActionBtn("? resend");
                resendBtn.setTextColor(0xFFDC143C);
                resendBtn.setOnClickListener(v -> {
                    showTab(0);
                    inputField.setText(user);
                    sendMessage();
                });

                // Continue -- put in input field only (user can edit before sending)
                TextView continueBtn = makeActionBtn("continue");
                continueBtn.setOnClickListener(v -> {
                    showTab(0);
                    inputField.setText(user);
                    inputField.setSelection(user.length());
                });

                headerRow.addView(timeTv);
                headerRow.addView(copyKira);
                headerRow.addView(resendBtn);
                headerRow.addView(continueBtn);

                // User message
                TextView userTv = new TextView(this);
                userTv.setText(user.length() > 120 ? user.substring(0, 120) + "?" : user);
                userTv.setTextColor(0xFFdddddd);
                userTv.setTextSize(13);

                // Kira reply preview
                TextView kiraTv = new TextView(this);
                kiraTv.setText(kira.length() > 150 ? kira.substring(0, 150) + "?" : kira);
                kiraTv.setTextColor(t(D_TEXT2, L_TEXT2));
                kiraTv.setTextSize(12);
                kiraTv.setPadding(0, dp(4), 0, 0);

                // Tap to see full conversation
                card.setOnClickListener(v -> showFullDialog(user, kira, timeStr));

                card.addView(headerRow);
                card.addView(userTv);
                card.addView(kiraTv);
                // L7: cascade stagger — each card dealt 40ms later
                final LinearLayout fc = card;
                final int delay = (arr.length() - 1 - i) * 40;
                fc.setAlpha(0f);
                fc.setTranslationY(dp(16));
                historyList.addView(fc);
                uiHandler.postDelayed(() -> fc.animate()
                    .alpha(1f).translationY(0f)
                    .setDuration(260)
                    .setInterpolator(new android.view.animation.DecelerateInterpolator(1.5f))
                    .start(), delay);
            }
        } catch (Exception e) {
            historyCount.setText("error loading history");
        }
    }

    private void showFullDialog(String user, String kira, String time) {
        String preview = "YOU:\n" + user.substring(0, Math.min(user.length(), 200))
            + "\n\n------\n\nKIRA:\n" + kira.substring(0, Math.min(kira.length(), 300));
        showKiraDialogMulti(time, preview,
            new String[]{"RESEND", "COPY", "CLOSE"},
            new Runnable[]{
                () -> { showTab(0); inputField.setText(user); sendMessage(); },
                () -> copyText(kira),
                null
            });
    }

    // -- Settings --------------------------------------------------------------

    private void updateSettingsUI() {
        // Layer 5: section header underlines animate in via ScaleX
        animateSectionHeaders();
        // Poll Rust /settings/health for live stats
        new Thread(() -> {
            try {
                okhttp3.OkHttpClient cl = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(1, java.util.concurrent.TimeUnit.SECONDS)
                    .readTimeout(2, java.util.concurrent.TimeUnit.SECONDS).build();
                okhttp3.Response r = cl.newCall(new okhttp3.Request.Builder()
                    .url("http://localhost:7070/settings/health").get().build()).execute();
                if (r.body() == null) return;
                String j = r.body().string();
                long uptime    = (long) parseJsonDouble(j, "uptime_ms");
                long tools     = (long) parseJsonDouble(j, "tool_calls");
                float activity = (float) parseJsonDouble(j, "activity");
                int bpm        = (int)  parseJsonDouble(j, "pulse_bpm");
                int macros     = (int)  parseJsonDouble(j, "automation_count");
                int memEntries = (int)  parseJsonDouble(j, "memory_count");
                String model   = parseJsonString(j, "model");
                boolean apiSet = j.contains("\"api_key_set\":true");
                uiHandler.post(() -> {
                    // Update rustStatsHint and rustStatsContent if present
                    if (rustStatsHint != null) {
                        String uptimeStr = uptime < 60000 ? (uptime/1000)+"s"
                            : uptime < 3600000 ? (uptime/60000)+"m"
                            : (uptime/3600000)+"h";
                        rustStatsHint.setText("up " + uptimeStr
                            + " · " + tools + " calls"
                            + " · bpm " + bpm
                            + " · act " + String.format("%.0f%%", activity*100));
                    }
                    if (rustStatsContent != null) {
                        rustStatsContent.setText(
                            "automations: " + macros
                            + "  ·  memory: " + memEntries + " facts"
                            + "  ·  model: " + (model.length() > 24 ? model.substring(0,24)+"…" : model)
                            + (apiSet ? "  ·  key ✓" : "  ·  no key"));
                    }
                    // Layer 5: pulse rustStatsContent alpha with activity level
                    if (rustStatsContent != null && activity > 0.1f) {
                        rustStatsContent.animate()
                            .alpha(0.6f + activity * 0.4f)
                            .setDuration(400).start();
                    }
                });
            } catch (Exception ignored) {}
        }).start();

        // Layer 5: poll /settings/counters for CounterAnimator values
        new Thread(() -> {
            try {
                okhttp3.OkHttpClient cl = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(1, java.util.concurrent.TimeUnit.SECONDS)
                    .readTimeout(2, java.util.concurrent.TimeUnit.SECONDS).build();
                // Counters
                okhttp3.Response rc = cl.newCall(new okhttp3.Request.Builder()
                    .url("http://localhost:7070/settings/counters").get().build()).execute();
                if (rc.body() != null) {
                    String jc = rc.body().string();
                    long uptimeS   = (long) parseJsonDouble(jc, "uptime_s");
                    long toolCalls = (long) parseJsonDouble(jc, "tool_calls");
                    long memFacts  = (long) parseJsonDouble(jc, "memory_facts");
                    long macrosEn  = (long) parseJsonDouble(jc, "macros_enabled");
                    long macroRuns = (long) parseJsonDouble(jc, "macro_runs");
                    uiHandler.post(() -> {
                        // CounterAnimator: animate each counter value
                        animateCounter(settingsFragment, R.id.rustStatsHint,
                            "up " + uptimeS + "s · " + toolCalls + " calls");
                        if (memoryHint != null)
                            animateCounter(settingsFragment, R.id.memoryHint,
                                memFacts + " facts");
                        if (rustStatsContent != null)
                            rustStatsContent.setText(
                                macrosEn + " automations · " + macroRuns + " total runs");
                    });
                }
                // Halo state
                okhttp3.Response rh = cl.newCall(new okhttp3.Request.Builder()
                    .url("http://localhost:7070/settings/shizuku/halo").get().build()).execute();
                if (rh.body() != null) {
                    String jh = rh.body().string();
                    boolean haloVisible = jh.contains("\"visible\":true");
                    int haloColor = (int)(long) parseJsonDouble(jh, "color");
                    int revMs     = (int) parseJsonDouble(jh, "revolution_ms");
                    uiHandler.post(() -> applyGodModeHalo(haloVisible, haloColor, revMs));
                }
            } catch (Exception ignored) {}
        }).start();

        cfg = com.kira.service.ai.KiraConfig.load(this);
        if (apiKeyHint == null) return;
        apiKeyHint.setText(cfg.apiKey.isEmpty() ? "tap to set" :
            "\u25CF\u25CF\u25CF\u25CF" + cfg.apiKey.substring(Math.max(0, cfg.apiKey.length()-4)));
        modelHint.setText(cfg.model.isEmpty() ? "not set" : cfg.model);
        String urlDisplay = cfg.baseUrl.isEmpty() ? "not set" :
            cfg.baseUrl.replace("https://","").replace("http://","");
        if (urlDisplay.length() > 36) urlDisplay = urlDisplay.substring(0, 33) + "\u2026";
        baseUrlHint.setText(urlDisplay);
        tgTokenHint.setText(cfg.tgToken.isEmpty() ? "not configured" : "\u2713 configured");
        tgIdHint.setText(cfg.tgAllowed == 0 ? "0 = anyone" : String.valueOf(cfg.tgAllowed));
        if (visionHint != null) visionHint.setText(cfg.visionModel.isEmpty() ? "not set" : cfg.visionModel);
        if (providerHint != null) {
            String pu = cfg.baseUrl;
            String label;
            if      (pu.contains("groq.com"))          label = "Groq \u00B7 llama-3.1-8b";
            else if (pu.contains("openai.com"))         label = "OpenAI \u00B7 " + cfg.model;
            else if (pu.contains("anthropic.com"))      label = "Anthropic \u00B7 claude";
            else if (pu.contains("googleapis.com"))     label = "Gemini \u00B7 " + cfg.model;
            else if (pu.contains("deepseek.com"))       label = "DeepSeek";
            else if (pu.contains("openrouter.ai"))      label = "OpenRouter";
            else if (pu.contains("localhost"))          label = "Ollama (local)";
            else if (pu.contains("together.xyz"))       label = "Together AI";
            else if (pu.contains("mistral.ai"))         label = "Mistral";
            else if (pu.contains("cohere.ai"))          label = "Cohere";
            else if (pu.contains("perplexity.ai"))      label = "Perplexity";
            else if (pu.contains("x.ai"))               label = "xAI Grok";
            else if (pu.contains("cerebras.ai"))        label = "Cerebras";
            else if (pu.contains("fireworks.ai"))       label = "Fireworks AI";
            else if (pu.contains("sambanova.ai"))       label = "SambaNova";
            else if (pu.contains("novita.ai"))          label = "Novita AI";
            else if (!pu.isEmpty())                     label = "custom: " + urlDisplay;
            else                                        label = "not set";
            providerHint.setText(label);
        }
        updateShizukuStatus();
    }

    private void loadMemorySection() {
        try {
            KiraMemory mem = new KiraMemory(this);
            String all = mem.listAll();
            int count = all.isEmpty() || all.equals("(empty)") ? 0 : all.split("\n").length;
            JSONArray hist = mem.loadHistory();
            if (memoryHint != null) memoryHint.setText(count + " facts ? " + hist.length() + " conversations");
            if (historySettingHint != null) historySettingHint.setText(hist.length() + " conversations stored");
        } catch (Exception e) {
            if (memoryHint != null) memoryHint.setText("tap to view");
        }
    }

    private void toggleMemoryContent() {
        if (memoryContent == null) return;
        if (memoryContent.getVisibility() == View.GONE) {
            try {
                KiraMemory mem = new KiraMemory(this);
                String all = mem.listAll();
                memoryContent.setText(all.isEmpty() ? "(no facts stored yet)" : all);
            } catch (Exception e) { memoryContent.setText("error reading memory"); }
            memoryContent.setVisibility(View.VISIBLE);
        } else {
            memoryContent.setVisibility(View.GONE);
        }
    }

    private void clearMemory() {
        showKiraDialog("Clear Memory", "Delete all stored facts?\nConversation history is kept.",
            "CLEAR", "CANCEL", () -> {
                try { new KiraMemory(this).clearFacts(); loadMemorySection();
                    Toast.makeText(this,"Facts cleared",Toast.LENGTH_SHORT).show(); } catch (Exception e) {}
            });
    }

    private void clearHistory() {
        showKiraDialog("Clear History", "Delete all conversation history?",
            "CLEAR", "CANCEL", () -> {
                try { new KiraMemory(this).clearHistory(); loadMemorySection();
                    Toast.makeText(this,"History cleared",Toast.LENGTH_SHORT).show(); } catch (Exception e) {}
            });
    }

    private void updateShizukuStatus() {
        if (shizukuStatusTitle == null) return;
        boolean permOk    = ShizukuShell.isAvailable();        // binder alive + permission granted
        boolean binderUp  = ShizukuShell.isInstalled();        // binder alive (no permission yet)
        boolean apkExists = ShizukuShell.isApkInstalled(this); // APK installed on device

        String title; int color; String icon; int bg;
        if (permOk) {
            title = "Shizuku \u2713  god mode active";
            color = 0xFFDC143C; icon = "\u2713"; bg = 0xFF080f08;
        } else if (binderUp) {
            title = "Shizuku running  \u2014  tap to grant permission";
            color = 0xFFffaa00; icon = "!"; bg = 0xFF0f0c00;
        } else if (apkExists) {
            title = "Shizuku installed  \u2014  tap to start service";
            color = 0xFFffaa00; icon = "\u25B6"; bg = 0xFF0f0c00;
        } else {
            title = "Shizuku not installed  \u2014  tap to get it";
            color = 0xFF555566; icon = "\u2193"; bg = 0xFF0a0a14;
        }
        shizukuStatusTitle.setText(title);
        shizukuStatusTitle.setTextColor(color);
        shizukuStatusIcon.setText(icon);
        shizukuStatusIcon.setTextColor(color);
        if (shizukuStatus != null) shizukuStatus.setBackgroundColor(bg);

        // Layer 5: pulsing left-border on Shizuku card
        View border = settingsFragment != null ? settingsFragment.findViewById(R.id.shizukuBorder) : null;
        if (border != null) {
            int borderColor = permOk ? 0xFFB4BEFE    // Lavender — god mode
                            : binderUp ? 0xFFFAB387  // Peach — partial
                            : 0xFFF38BA8;            // Pink — offline
            border.setBackgroundColor(borderColor);
            Object existing = border.getTag(R.id.tag3);
            if (!(existing instanceof android.animation.ObjectAnimator) ||
                !((android.animation.ObjectAnimator)existing).isRunning()) {
                android.animation.ObjectAnimator pulse =
                    android.animation.ObjectAnimator.ofFloat(border, "alpha", 1f, 0.5f, 1f);
                pulse.setDuration(1500);
                pulse.setRepeatCount(android.animation.ValueAnimator.INFINITE);
                pulse.setInterpolator(new android.view.animation.AccelerateDecelerateInterpolator());
                border.setTag(R.id.tag3, pulse);
                pulse.start();
            }
        }

        // Sync to Rust state
        try { RustBridge.updateShizukuStatus(binderUp, permOk, ""); } catch (Exception ignored) {}
        // Layer 5: also poll /settings/shizuku for Rust-computed border color
        new Thread(() -> {
            try {
                okhttp3.OkHttpClient cl2 = new okhttp3.OkHttpClient.Builder()
                    .connectTimeout(1, java.util.concurrent.TimeUnit.SECONDS).build();
                okhttp3.Response r2 = cl2.newCall(new okhttp3.Request.Builder()
                    .url("http://localhost:7070/settings/shizuku").get().build()).execute();
                if (r2.body() == null) return;
                String j2 = r2.body().string();
                long borderColorL = (long) parseJsonDouble(j2, "border_color");
                int rustBorderColor = (int) borderColorL;
                uiHandler.post(() -> {
                    View border2 = settingsFragment != null
                        ? settingsFragment.findViewById(R.id.shizukuBorder) : null;
                    if (border2 != null && rustBorderColor != 0)
                        border2.setBackgroundColor(rustBorderColor);
                });
            } catch (Exception ignored2) {}
        }).start();
    }

    private void toggleFloating() {
        if (!Settings.canDrawOverlays(this)) {
            showKiraDialogMulti("Overlay Permission",
                "Kira needs 'Display over other apps'.\n\nSettings \u2192 Apps \u2192 Kira \u2192 Display over other apps \u2192 Enable",
                new String[]{"OPEN SETTINGS", "CANCEL"},
                new Runnable[]{
                    () -> startActivity(new Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + getPackageName()))),
                    null
                });
            return;
        }
        floatingActive = !floatingActive;
        if (floatingActive) {
            FloatingWindowService.start(this);
            floatingToggle.setText("ON");
            floatingToggle.setTextColor(0xFFDC143C);
        } else {
            FloatingWindowService.stop(this);
            floatingToggle.setText("OFF");
            floatingToggle.setTextColor(0xFF666666);
        }
    }

    private void restartTelegram() {
        if (KiraAccessibilityService.instance != null) {
            KiraAccessibilityService.instance.restartTelegram();
        }
        Toast.makeText(this, "Telegram config updated -- restarting bot", Toast.LENGTH_SHORT).show();
    }

    interface StringCallback { void onResult(String v); }

    /**
     * Modern Kira dialog \u2014 replaces stock Android AlertDialog.
     * Dark obsidian panel, crimson accent bar, styled EditText.
     */
    private void editSetting(String title, String current, boolean numeric, StringCallback cb) {
        // Full-screen dim overlay
        android.widget.FrameLayout overlay = new android.widget.FrameLayout(this);
        overlay.setBackgroundColor(0xBB000000);
        android.widget.FrameLayout.LayoutParams ovLp = new android.widget.FrameLayout.LayoutParams(MATCH, MATCH);
        overlay.setLayoutParams(ovLp);

        // Card panel
        android.widget.LinearLayout card = new android.widget.LinearLayout(this);
        card.setOrientation(android.widget.LinearLayout.VERTICAL);
        android.graphics.drawable.GradientDrawable cardBg = new android.graphics.drawable.GradientDrawable();
        cardBg.setColor(0xFF0c0c18);
        cardBg.setCornerRadius(dp(4));
        cardBg.setStroke(dp(1), 0xFF1a1a2e);
        card.setBackground(cardBg);

        android.widget.FrameLayout.LayoutParams cardLp = new android.widget.FrameLayout.LayoutParams(
            (int)(getResources().getDisplayMetrics().widthPixels * 0.88f), WRAP);
        cardLp.gravity = android.view.Gravity.CENTER;
        card.setLayoutParams(cardLp);

        // Top accent bar (crimson line)
        android.view.View accentBar = new android.view.View(this);
        accentBar.setBackgroundColor(0xFFDC143C);
        accentBar.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(2)));
        card.addView(accentBar);

        // Title row
        android.widget.LinearLayout titleRow = new android.widget.LinearLayout(this);
        titleRow.setOrientation(android.widget.LinearLayout.HORIZONTAL);
        titleRow.setGravity(android.view.Gravity.CENTER_VERTICAL);
        titleRow.setPadding(dp(20), dp(16), dp(20), dp(12));

        android.widget.TextView titleTv = new android.widget.TextView(this);
        titleTv.setText(title);
        titleTv.setTextColor(0xFFFFFFFF);
        titleTv.setTextSize(15);
        titleTv.setTypeface(android.graphics.Typeface.DEFAULT_BOLD);
        titleTv.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, WRAP, 1));
        titleRow.addView(titleTv);

        // Small K monogram
        android.widget.TextView kMono = new android.widget.TextView(this);
        kMono.setText("K");
        kMono.setTextColor(0x44DC143C);
        kMono.setTextSize(22);
        kMono.setTypeface(android.graphics.Typeface.DEFAULT_BOLD);
        titleRow.addView(kMono);
        card.addView(titleRow);

        // Separator
        android.view.View sep = new android.view.View(this);
        sep.setBackgroundColor(0xFF111122);
        sep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(sep);

        // Input field
        android.widget.EditText et = new android.widget.EditText(this);
        et.setText(current);
        et.setTextColor(0xFFFFFFFF);
        et.setHintTextColor(0xFF333355);
        et.setTextSize(15);
        et.setTypeface(android.graphics.Typeface.MONOSPACE);
        et.setSingleLine(!numeric);
        et.setMaxLines(numeric ? 1 : 3);
        android.graphics.drawable.GradientDrawable inputBg = new android.graphics.drawable.GradientDrawable();
        inputBg.setColor(0xFF080814);
        inputBg.setCornerRadius(dp(3));
        inputBg.setStroke(dp(1), 0xFF1e1e3a);
        et.setBackground(inputBg);
        et.setPadding(dp(14), dp(12), dp(14), dp(12));
        et.setInputType(numeric
            ? android.text.InputType.TYPE_CLASS_NUMBER
            : (android.text.InputType.TYPE_CLASS_TEXT | android.text.InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS));
        android.widget.LinearLayout.LayoutParams etLp =
            new android.widget.LinearLayout.LayoutParams(MATCH, WRAP);
        etLp.setMargins(dp(16), dp(14), dp(16), dp(16));
        et.setLayoutParams(etLp);
        card.addView(et);

        // Button row
        android.view.View btnSep = new android.view.View(this);
        btnSep.setBackgroundColor(0xFF0e0e1e);
        btnSep.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(1)));
        card.addView(btnSep);

        android.widget.LinearLayout btnRow = new android.widget.LinearLayout(this);
        btnRow.setOrientation(android.widget.LinearLayout.HORIZONTAL);
        btnRow.setLayoutParams(new android.widget.LinearLayout.LayoutParams(MATCH, dp(52)));

        android.widget.TextView cancelBtn = new android.widget.TextView(this);
        cancelBtn.setText("CANCEL");
        cancelBtn.setTextColor(0xFF444466);
        cancelBtn.setTextSize(12);
        cancelBtn.setTypeface(android.graphics.Typeface.MONOSPACE);
        cancelBtn.setGravity(android.view.Gravity.CENTER);
        cancelBtn.setClickable(true); cancelBtn.setFocusable(true);
        cancelBtn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, MATCH, 1));

        android.view.View btnDivider = new android.view.View(this);
        btnDivider.setBackgroundColor(0xFF0e0e1e);
        btnDivider.setLayoutParams(new android.widget.LinearLayout.LayoutParams(dp(1), MATCH));

        android.widget.TextView saveBtn = new android.widget.TextView(this);
        saveBtn.setText("SAVE");
        saveBtn.setTextColor(0xFFDC143C);
        saveBtn.setTextSize(12);
        saveBtn.setTypeface(android.graphics.Typeface.MONOSPACE);
        saveBtn.setTypeface(null, android.graphics.Typeface.BOLD);
        saveBtn.setGravity(android.view.Gravity.CENTER);
        saveBtn.setClickable(true); saveBtn.setFocusable(true);
        saveBtn.setLayoutParams(new android.widget.LinearLayout.LayoutParams(0, MATCH, 1));

        btnRow.addView(cancelBtn);
        btnRow.addView(btnDivider);
        btnRow.addView(saveBtn);
        card.addView(btnRow);

        overlay.addView(card);

        // Add overlay to window
        android.view.ViewGroup root = (android.view.ViewGroup) getWindow().getDecorView();
        root.addView(overlay);

        // Show keyboard
        et.requestFocus();
        uiHandler.postDelayed(() -> {
            android.view.inputmethod.InputMethodManager imm =
                (android.view.inputmethod.InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
            if (imm != null) imm.showSoftInput(et, android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT);
        }, 150);

        // Dismiss + callbacks
        Runnable dismiss = () -> {
            root.removeView(overlay);
            android.view.inputmethod.InputMethodManager imm =
                (android.view.inputmethod.InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
            if (imm != null) imm.hideSoftInputFromWindow(et.getWindowToken(), 0);
        };

        cancelBtn.setOnClickListener(v -> dismiss.run());
        overlay.setOnClickListener(v -> dismiss.run()); // tap outside = dismiss
        card.setOnClickListener(v -> {}); // consume clicks so card doesn't dismiss

        saveBtn.setOnClickListener(v -> {
            String val = et.getText().toString().trim();
            dismiss.run();
            cb.onResult(val);
        });
    }


    // Setup handled by SetupActivity

    // -- Helpers ---------------------------------------------------------------

    private TextView makeLabel(String text) {
        TextView tv = new TextView(this);
        tv.setText(text); tv.setTextSize(10);
        tv.setLetterSpacing(0.1f);
        return tv;
    }

    private TextView makeActionBtn(String text) {
        TextView tv = new TextView(this);
        tv.setText(text); tv.setTextColor(0xFF555555);
        tv.setTextSize(10); tv.setPadding(dp(6), 0, 0, 0);
        tv.setClickable(true); tv.setFocusable(true);
        return tv;
    }

    private int dp(int v) { return (int)(v * getResources().getDisplayMetrics().density); }

    private static final int MATCH = ViewGroup.LayoutParams.MATCH_PARENT;
    private static final int WRAP  = ViewGroup.LayoutParams.WRAP_CONTENT;

    @Override protected void onResume() {
        super.onResume();
        if (currentTab == 3) updateShizukuStatus();
    }

    // ── L7: Bubble context menu ─────────────────────────────────────────────
    private void showBubbleContextMenu(android.view.View anchor, String text) {
        // Remove any existing menu
        android.view.ViewGroup root = (android.view.ViewGroup) getWindow().getDecorView().getRootView();
        android.view.View old = root.findViewWithTag("bubble_ctx_menu");
        if (old != null) root.removeView(old);

        // Build menu card
        LinearLayout menu = new LinearLayout(this);
        menu.setTag("bubble_ctx_menu");
        menu.setOrientation(LinearLayout.HORIZONTAL);
        android.graphics.drawable.GradientDrawable mbg = new android.graphics.drawable.GradientDrawable();
        mbg.setColor(0xFF313244); mbg.setCornerRadius(dp(10));
        menu.setBackground(mbg);
        menu.setElevation(dp(12));
        menu.setPadding(dp(4), dp(4), dp(4), dp(4));

        String[][] actions = {{"Copy", "copy"}, {"Edit", "edit"}, {"Resend", "resend"}};
        for (String[] action : actions) {
            TextView btn = new TextView(this);
            btn.setText(action[0]);
            btn.setTextColor(0xFFCDD6F4);
            btn.setTextSize(13);
            btn.setPadding(dp(14), dp(10), dp(14), dp(10));
            btn.setClickable(true); btn.setFocusable(true);
            final String act = action[1];
            btn.setOnClickListener(v2 -> {
                root.removeView(menu);
                if ("copy".equals(act)) copyText(text);
                else if ("resend".equals(act) || "edit".equals(act)) {
                    inputField.setText(text);
                    inputField.setSelection(text.length());
                }
            });
            menu.addView(btn);
        }

        // Position above the anchor view
        int[] loc = new int[2]; anchor.getLocationInWindow(loc);
        android.widget.FrameLayout.LayoutParams mlp =
            new android.widget.FrameLayout.LayoutParams(
                android.view.ViewGroup.LayoutParams.WRAP_CONTENT,
                android.view.ViewGroup.LayoutParams.WRAP_CONTENT);
        mlp.leftMargin = loc[0] + dp(8);
        mlp.topMargin  = Math.max(0, loc[1] - dp(52));
        menu.setLayoutParams(mlp);

        // Spring in from anchor center
        menu.setScaleX(0f); menu.setScaleY(0f); menu.setAlpha(0f);
        root.addView(menu);
        menu.animate().scaleX(1f).scaleY(1f).alpha(1f)
            .setDuration(200)
            .setInterpolator(new android.view.animation.OvershootInterpolator(2f))
            .start();

        // Auto-dismiss after 4s
        uiHandler.postDelayed(() -> {
            if (menu.getParent() != null) {
                menu.animate().alpha(0f).setDuration(150)
                    .withEndAction(() -> root.removeView(menu)).start();
            }
        }, 4000);
    }

    /** Called by KiraWatcher/KiraHeartbeat when an automation macro fires */
    public void onMacroFired() {
        uiHandler.post(() -> fireLightning(1)); // L6: macro streak
    }

    /** Called when Shizuku permission is granted */
    public void onShizukuConnected() {
        uiHandler.post(() -> fireLightning(2)); // L6: shizuku radial burst
    }

    @Override protected void onStop() {
        super.onStop();
        animHandler.removeCallbacks(animPollRunnable);
        if (sensorManager != null) sensorManager.unregisterListener(this);
    }

    @Override protected void onDestroy() {
        super.onDestroy();
        try { Shizuku.removeRequestPermissionResultListener(shizukuPermListener); }
        catch (Exception ignored) {}
    }
}
