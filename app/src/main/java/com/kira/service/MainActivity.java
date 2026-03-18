package com.kira.service;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.HorizontalScrollView;
import android.widget.TextView;
import android.widget.Toast;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;

import org.json.JSONArray;
import org.json.JSONObject;

import rikka.shizuku.Shizuku;

public class MainActivity extends Activity {

    private static final int SHIZUKU_REQUEST_CODE = 1001;
    private static final int PERMISSION_REQUEST_CODE = 1002;

    private Handler uiHandler;
    private KiraAI ai;
    private KiraConfig cfg;
    private int currentTab = 0;

    // Fragment views
    private View homeFragment, toolsFragment, historyFragment, settingsFragment;

    // Home tab
    private LinearLayout chatContainer;
    private ScrollView chatScroll;
    private EditText inputField;
    private TextView sendBtn, headerSubtitle;
    private LinearLayout suggestionsRow;
    private HorizontalScrollView suggestionsScroll;

    // History
    private LinearLayout historyList;
    private TextView historyCount;

    // Settings
    private TextView apiKeyHint, modelHint, baseUrlHint, tgTokenHint, tgIdHint;
    private LinearLayout shizukuStatus;
    private TextView floatingToggle;
    private boolean floatingActive = false;
    private TextView shizukuStatusTitle, shizukuStatusIcon;

    // Nav
    private TextView[] navIcons, navTexts;
    private LinearLayout[] navItems;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        uiHandler = new Handler(Looper.getMainLooper());
        cfg = KiraConfig.load(this);

        if (!cfg.setupDone) {
            showFirstSetup();
            return;
        }

        ai = new KiraAI(this);
        initViews();
        showTab(0);

        // Request permissions on startup
        requestAllPermissions();
        checkShizuku();
    }

    // ── Permissions ───────────────────────────────────────────────────────────

    private void requestAllPermissions() {
        String[] permissions = {
            Manifest.permission.RECORD_AUDIO,
            Manifest.permission.SEND_SMS,
            Manifest.permission.READ_SMS,
            Manifest.permission.CALL_PHONE,
            Manifest.permission.READ_CONTACTS,
            Manifest.permission.READ_CALL_LOG,
            Manifest.permission.ACCESS_FINE_LOCATION,
            Manifest.permission.READ_EXTERNAL_STORAGE,
        };
        java.util.List<String> needed = new java.util.ArrayList<>();
        for (String p : permissions) {
            if (checkSelfPermission(p) != PackageManager.PERMISSION_GRANTED) needed.add(p);
        }
        if (!needed.isEmpty()) {
            requestPermissions(needed.toArray(new String[0]), PERMISSION_REQUEST_CODE);
        }

        // Overlay permission
        if (!Settings.canDrawOverlays(this)) {
            // Don't force — just note it
        }

        // Notification permission (Android 13+)
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
                requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS}, PERMISSION_REQUEST_CODE + 1);
            }
        }
    }

    private void checkShizuku() {
        try {
            if (!Shizuku.pingBinder()) {
                // Shizuku not installed/running — show guidance
                uiHandler.postDelayed(() -> showShizukuDialog(), 1000);
                return;
            }
            if (Shizuku.checkSelfPermission() != PackageManager.PERMISSION_GRANTED) {
                Shizuku.addRequestPermissionResultListener(this::onShizukuPermissionResult);
                Shizuku.requestPermission(SHIZUKU_REQUEST_CODE);
            }
        } catch (Exception e) {
            // Shizuku not available
        }
    }

    private void onShizukuPermissionResult(int requestCode, int grantResult) {
        if (requestCode == SHIZUKU_REQUEST_CODE) {
            if (grantResult == PackageManager.PERMISSION_GRANTED) {
                uiHandler.post(() -> {
                    Toast.makeText(this, "Shizuku permission granted — god mode active", Toast.LENGTH_SHORT).show();
                    updateShizukuStatus();
                });
            } else {
                uiHandler.post(() -> Toast.makeText(this, "Shizuku permission denied", Toast.LENGTH_SHORT).show());
            }
        }
    }

    private void showShizukuDialog() {
        new AlertDialog.Builder(this)
            .setTitle("Enable Full Phone Control")
            .setMessage("Kira uses Shizuku for ADB-level phone control (install apps, run commands, etc).\n\n1. Install Shizuku from Play Store\n2. Open Shizuku → Start via Wireless Debugging\n3. Come back to Kira\n\nWithout Shizuku, basic screen control still works via Accessibility Service.")
            .setPositiveButton("Get Shizuku", (d, w) -> {
                try {
                    startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=moe.shizuku.privileged.api")));
                } catch (Exception e) {
                    startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("https://shizuku.rikka.app")));
                }
            })
            .setNeutralButton("Already Have It", (d, w) -> checkShizuku())
            .setNegativeButton("Skip", null)
            .show();
    }

    private void checkAccessibility() {
        if (KiraAccessibilityService.instance == null) {
            new AlertDialog.Builder(this)
                .setTitle("Enable Accessibility Service")
                .setMessage("Kira needs Accessibility Service to read and control your screen.\n\nSettings → Accessibility → Installed Services → Kira → Enable")
                .setPositiveButton("Open Settings", (d, w) -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)))
                .setNegativeButton("Later", null)
                .show();
        }
    }

    // ── View setup ────────────────────────────────────────────────────────────

    private void initViews() {
        FrameLayout frame = findViewById(R.id.contentFrame);

        homeFragment     = getLayoutInflater().inflate(R.layout.fragment_home,     frame, false);
        toolsFragment    = getLayoutInflater().inflate(R.layout.fragment_tools,    frame, false);
        historyFragment  = getLayoutInflater().inflate(R.layout.fragment_history,  frame, false);
        settingsFragment = getLayoutInflater().inflate(R.layout.fragment_settings, frame, false);

        frame.addView(homeFragment);
        frame.addView(toolsFragment);
        frame.addView(historyFragment);
        frame.addView(settingsFragment);

        // Nav
        LinearLayout navHome     = findViewById(R.id.navHome);
        LinearLayout navAbilities= findViewById(R.id.navAbilities);
        LinearLayout navHistory  = findViewById(R.id.navHistory);
        LinearLayout navSettings = findViewById(R.id.navSettings);

        navItems = new LinearLayout[]{navHome, navAbilities, navHistory, navSettings};
        navIcons = new TextView[4]; navTexts = new TextView[4];
        for (int i = 0; i < 4; i++) {
            navIcons[i] = (TextView) navItems[i].getChildAt(0);
            navTexts[i] = (TextView) navItems[i].getChildAt(1);
        }

        navHome.setOnClickListener(v -> showTab(0));
        navAbilities.setOnClickListener(v -> showTab(1));
        navHistory.setOnClickListener(v -> showTab(2));
        navSettings.setOnClickListener(v -> showTab(3));

        // Home
        chatContainer   = homeFragment.findViewById(R.id.chatContainer);
        chatScroll      = homeFragment.findViewById(R.id.chatScroll);
        inputField      = homeFragment.findViewById(R.id.inputField);
        sendBtn         = homeFragment.findViewById(R.id.sendBtn);
        headerSubtitle  = homeFragment.findViewById(R.id.headerSubtitle);
        suggestionsRow  = homeFragment.findViewById(R.id.suggestionsRow);
        suggestionsScroll = homeFragment.findViewById(R.id.suggestionsScroll);

        sendBtn.setOnClickListener(v -> sendMessage());
        inputField.setOnEditorActionListener((v, id, e) -> { if (id == android.view.inputmethod.EditorInfo.IME_ACTION_SEND) { sendMessage(); return true; } return false; });
        addSuggestions();

        // Check accessibility after small delay
        uiHandler.postDelayed(this::checkAccessibility, 2000);

        // History
        historyList  = historyFragment.findViewById(R.id.historyList);
        historyCount = historyFragment.findViewById(R.id.historyCount);

        // Settings
        apiKeyHint        = settingsFragment.findViewById(R.id.apiKeyHint);
        modelHint         = settingsFragment.findViewById(R.id.modelHint);
        baseUrlHint       = settingsFragment.findViewById(R.id.baseUrlHint);
        tgTokenHint       = settingsFragment.findViewById(R.id.tgTokenHint);
        tgIdHint          = settingsFragment.findViewById(R.id.tgIdHint);
        shizukuStatus     = settingsFragment.findViewById(R.id.shizukuStatus);
        shizukuStatusTitle= settingsFragment.findViewById(R.id.shizukuTitle);
        shizukuStatusIcon = settingsFragment.findViewById(R.id.shizukuIcon);

        settingsFragment.findViewById(R.id.settingApiKey).setOnClickListener(v ->
            editSetting("API Key", cfg.apiKey, false, val -> { cfg.apiKey = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingModel).setOnClickListener(v ->
            editSetting("Model", cfg.model, false, val -> { cfg.model = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingBaseUrl).setOnClickListener(v ->
            editSetting("Base URL", cfg.baseUrl, false, val -> { cfg.baseUrl = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingTgToken).setOnClickListener(v ->
            editSetting("Telegram Bot Token", cfg.tgToken, false, val -> { cfg.tgToken = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingTgId).setOnClickListener(v ->
            editSetting("Your Telegram ID", cfg.tgAllowed == 0 ? "" : String.valueOf(cfg.tgAllowed), true, val -> {
                try { cfg.tgAllowed = val.isEmpty() ? 0 : Long.parseLong(val); cfg.save(this); updateSettingsUI(); } catch (Exception ignored) {}
            }));

        // Shizuku status card is clickable directly
        shizukuStatus.setOnClickListener(v -> checkShizuku());

        // Floating window toggle
        floatingToggle = settingsFragment.findViewById(R.id.floatingToggle);
        settingsFragment.findViewById(R.id.settingFloating).setOnClickListener(v -> toggleFloating());

        buildToolsList();
        updateSettingsUI();

        String name = cfg.userName.isEmpty() ? "there" : cfg.userName.toLowerCase();
        headerSubtitle.setText("ready, " + name + ". tell me what to do.");
    }

    // ── Tab navigation ────────────────────────────────────────────────────────

    private void showTab(int tab) {
        currentTab = tab;
        homeFragment.setVisibility(tab == 0 ? View.VISIBLE : View.GONE);
        toolsFragment.setVisibility(tab == 1 ? View.VISIBLE : View.GONE);
        historyFragment.setVisibility(tab == 2 ? View.VISIBLE : View.GONE);
        settingsFragment.setVisibility(tab == 3 ? View.VISIBLE : View.GONE);
        for (int i = 0; i < 4; i++) {
            boolean active = i == tab;
            navIcons[i].setTextColor(active ? 0xFFFF8C00 : 0xFF666666);
            navTexts[i].setTextColor(active ? 0xFFFF8C00 : 0xFF666666);
            navItems[i].setBackgroundColor(active ? 0xFF1f1a0f : 0x00000000);
        }
        if (tab == 2) refreshHistory();
        if (tab == 3) updateSettingsUI();
    }

    // ── Chat ──────────────────────────────────────────────────────────────────

    private void sendMessage() {
        sendMessage(inputField.getText().toString().trim());
    }

    private void sendMessage(String text) {
        if (text.isEmpty()) return;
        inputField.setText("");
        suggestionsScroll.setVisibility(View.GONE);

        addUserBubble(text);
        sendBtn.setEnabled(false);
        headerSubtitle.setText("thinking...");

        ai.chat(text, new KiraAI.Callback() {
            @Override public void onThinking() {
                uiHandler.post(() -> addThinkingBubble());
            }
            @Override public void onTool(String name, String result) {
                uiHandler.post(() -> addToolBubble(name, result));
            }
            @Override public void onReply(String reply) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    addKiraBubble(reply);
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("ready, " + cfg.userName.toLowerCase() + ".");
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    addErrorBubble(error);
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("error");
                });
            }
        });
    }

    private void addUserBubble(String text) {
        LinearLayout container = new LinearLayout(this);
        container.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 0, 0, 12);
        container.setLayoutParams(p);

        TextView label = makeLabel("YOU");
        label.setTextColor(0xFF888888);

        TextView msg = new TextView(this);
        msg.setText(text);
        msg.setTextColor(0xFFdddddd);
        msg.setTextSize(14);
        msg.setBackgroundColor(0xFF2a2a2a);
        msg.setPadding(16, 12, 16, 12);
        msg.setLineSpacing(2, 1);

        container.addView(label);
        container.addView(msg);
        chatContainer.addView(container);
        scrollToBottom();
    }

    private TextView thinkingBubble;

    private void addThinkingBubble() {
        removeThinkingBubble();
        thinkingBubble = new TextView(this);
        thinkingBubble.setText("KIRA  ···");
        thinkingBubble.setTextColor(0xFF555555);
        thinkingBubble.setTextSize(12);
        thinkingBubble.setPadding(0, 4, 0, 4);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 0, 0, 4);
        thinkingBubble.setLayoutParams(p);
        chatContainer.addView(thinkingBubble);
        scrollToBottom();
    }

    private void removeThinkingBubble() {
        if (thinkingBubble != null && thinkingBubble.getParent() != null) {
            chatContainer.removeView(thinkingBubble);
            thinkingBubble = null;
        }
    }

    private void addKiraBubble(String text) {
        LinearLayout container = new LinearLayout(this);
        container.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 0, 0, 12);
        container.setLayoutParams(p);

        LinearLayout header = new LinearLayout(this);
        header.setOrientation(LinearLayout.HORIZONTAL);
        header.setGravity(android.view.Gravity.CENTER_VERTICAL);
        header.setPadding(0, 0, 0, 4);

        TextView label = makeLabel("KIRA");
        label.setTextColor(0xFFff8c00);
        header.addView(label);

        // Copy button
        TextView copyBtn = new TextView(this);
        copyBtn.setText("  copy");
        copyBtn.setTextColor(0xFF555555);
        copyBtn.setTextSize(10);
        copyBtn.setOnClickListener(v -> {
            android.content.ClipboardManager cm = (android.content.ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
            if (cm != null) cm.setPrimaryClip(android.content.ClipData.newPlainText("kira", text));
            Toast.makeText(this, "copied", Toast.LENGTH_SHORT).show();
        });
        header.addView(copyBtn);

        // Resend button
        TextView resendBtn = new TextView(this);
        resendBtn.setText("  ↑ resend");
        resendBtn.setTextColor(0xFF555555);
        resendBtn.setTextSize(10);
        resendBtn.setOnClickListener(v -> { inputField.setText(text); inputField.setSelection(text.length()); });
        header.addView(resendBtn);

        TextView msg = new TextView(this);
        msg.setText(text);
        msg.setTextColor(0xFFeeeeee);
        msg.setTextSize(14);
        msg.setBackgroundColor(0xFF1e1e1e);
        msg.setPadding(16, 12, 16, 12);
        msg.setLineSpacing(2, 1);
        msg.setTextIsSelectable(true);

        container.addView(header);
        container.addView(msg);
        chatContainer.addView(container);
    }

    private void addToolBubble(String name, String result) {
        String display = result.length() > 120 ? result.substring(0, 120) + "…" : result;
        TextView tv = new TextView(this);
        tv.setText("⚡ " + name + "\n" + display);
        tv.setTextColor(0xFF556655);
        tv.setTextSize(11);
        tv.setPadding(12, 6, 12, 6);
        tv.setBackgroundColor(0xFF0f1a0f);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 2, 0, 2);
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
        scrollToBottom();
    }

    private void addErrorBubble(String error) {
        LinearLayout container = new LinearLayout(this);
        container.setOrientation(LinearLayout.VERTICAL);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 0, 0, 8);
        container.setLayoutParams(p);

        TextView label = makeLabel("ERROR");
        label.setTextColor(0xFFcc4444);

        TextView msg = new TextView(this);
        msg.setText(error);
        msg.setTextColor(0xFFff8888);
        msg.setTextSize(13);
        msg.setBackgroundColor(0xFF2a1010);
        msg.setPadding(16, 10, 16, 10);

        container.addView(label);
        container.addView(msg);
        chatContainer.addView(container);
        scrollToBottom();
    }

    private void addSystemBubble(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextColor(0xFFff8c00);
        tv.setBackgroundColor(0xFF1a1200);
        tv.setTextSize(12);
        tv.setPadding(16, 10, 16, 10);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
        p.setMargins(0, 0, 0, 8);
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
    }

    private TextView makeLabel(String text) {
        TextView tv = new TextView(this);
        tv.setText(text);
        tv.setTextSize(10);
        tv.setLetterSpacing(0.1f);
        tv.setPadding(0, 0, 0, 3);
        return tv;
    }

    private void scrollToBottom() {
        chatScroll.post(() -> chatScroll.fullScroll(View.FOCUS_DOWN));
    }

    private static final int WRAP = ViewGroup.LayoutParams.MATCH_PARENT;

    // ── Suggestions ───────────────────────────────────────────────────────────

    private void addSuggestions() {
        String[][] suggestions = {
            {"📱", "Open YouTube"},
            {"🔔", "Check notifications"},
            {"🔋", "Battery status"},
            {"📸", "Take screenshot"},
            {"🌐", "Search web for news"},
            {"📋", "Read my screen"},
            {"📶", "WiFi info"},
            {"💬", "List recent SMS"},
        };
        for (String[] s : suggestions) {
            TextView chip = new TextView(this);
            chip.setText(s[0] + " " + s[1]);
            chip.setTextSize(12);
            chip.setTextColor(0xFFcccccc);
            chip.setBackgroundColor(0xFF222222);
            chip.setPadding(14, 8, 14, 8);
            LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT);
            p.setMargins(0, 0, 8, 0);
            chip.setLayoutParams(p);
            chip.setOnClickListener(v -> { inputField.setText(s[1]); sendMessage(); });
            suggestionsRow.addView(chip);
        }
    }

    private static final int WRAP_CONTENT = ViewGroup.LayoutParams.WRAP_CONTENT;

    // ── Tools list ────────────────────────────────────────────────────────────

    private void buildToolsList() {
        LinearLayout list = toolsFragment.findViewById(R.id.toolsList);
        Object[][] tools = {
            {"📱", "open_app", "Open any app by name (youtube, whatsapp, etc.) or package name", "open_app {\"package\": \"youtube\"}"},
            {"👁", "read_screen", "Read all visible text on current screen", "read_screen {}"},
            {"👆", "tap_screen", "Tap any screen coordinate", "tap_screen {\"x\": 540, \"y\": 960}"},
            {"🔍", "tap_text", "Find element by text and tap it", "tap_text {\"text\": \"Send\"}"},
            {"⌨", "type_text", "Type text into focused field", "type_text {\"text\": \"hello world\"}"},
            {"🔔", "get_notifications", "Get all recent notifications", "get_notifications {}"},
            {"💬", "send_sms", "Send SMS message", "send_sms {\"number\": \"+1234\", \"message\": \"hi\"}"},
            {"🔍", "web_search", "Search DuckDuckGo", "web_search {\"query\": \"weather today\"}"},
            {"⚡", "sh_run", "Run any shell command via Shizuku", "sh_run {\"cmd\": \"pm list packages\"}"},
            {"📸", "sh_screenshot", "Take and save screenshot", "sh_screenshot {}"},
            {"🧠", "remember", "Store a fact permanently", "remember {\"key\": \"home\", \"value\": \"Dhaka\"}"},
            {"🔋", "battery_info", "Get battery level and status", "battery_info {}"},
            {"📂", "list_files", "List files in directory", "list_files {\"path\": \"/sdcard\"}"},
            {"🌐", "http_get", "HTTP GET any URL", "http_get {\"url\": \"https://api.example.com\"}"},
        };

        for (Object[] tool : tools) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setBackgroundColor(0xFF1a1a1a);
            row.setPadding(16, 14, 16, 14);
            LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(WRAP, WRAP_CONTENT);
            rp.setMargins(0, 0, 0, 2);
            row.setLayoutParams(rp);
            row.setClickable(true);
            row.setFocusable(true);

            final String example = (String) tool[3];
            row.setOnClickListener(v -> {
                // Parse and insert example into input
                try {
                    String toolName = (String) tool[1];
                    String jsonArgs = example.substring(example.indexOf("{"));
                    org.json.JSONObject args = new org.json.JSONObject(jsonArgs);
                    // Build natural language prompt
                    inputField.setText("use " + toolName + " with " + jsonArgs);
                    showTab(0);
                } catch (Exception e) {
                    inputField.setText((String) tool[1]);
                    showTab(0);
                }
            });

            TextView icon = new TextView(this);
            icon.setText((String) tool[0]);
            icon.setTextSize(22);
            icon.setWidth(52);
            icon.setGravity(Gravity.CENTER);

            LinearLayout info = new LinearLayout(this);
            info.setOrientation(LinearLayout.VERTICAL);
            info.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP_CONTENT, 1));

            TextView name = new TextView(this);
            name.setText((String) tool[1]);
            name.setTextColor(0xFFffffff);
            name.setTextSize(14);

            TextView desc = new TextView(this);
            desc.setText((String) tool[2]);
            desc.setTextColor(0xFF888888);
            desc.setTextSize(12);

            TextView ex = new TextView(this);
            ex.setText((String) tool[3]);
            ex.setTextColor(0xFF444444);
            ex.setTextSize(10);
            ex.setPadding(0, 2, 0, 0);

            info.addView(name);
            info.addView(desc);
            info.addView(ex);

            TextView arrow = new TextView(this);
            arrow.setText("›");
            arrow.setTextColor(0xFF444444);
            arrow.setTextSize(20);
            arrow.setGravity(Gravity.CENTER);
            arrow.setPadding(8, 0, 0, 0);

            row.addView(icon);
            row.addView(info);
            row.addView(arrow);
            list.addView(row);
        }
    }

    // ── History ───────────────────────────────────────────────────────────────

    private void refreshHistory() {
        historyList.removeAllViews();
        try {
            KiraMemory mem = new KiraMemory(this);
            JSONArray arr = mem.loadHistory();
            if (arr.length() == 0) {
                historyCount.setText("No conversations yet");
                TextView empty = new TextView(this);
                empty.setText("Your conversation history will appear here.\nEach exchange with Kira is saved.");
                empty.setTextColor(0xFF555555);
                empty.setTextSize(14);
                empty.setPadding(0, 24, 0, 0);
                empty.setGravity(Gravity.CENTER);
                historyList.addView(empty);
                return;
            }
            historyCount.setText(arr.length() + " conversations");

            for (int i = arr.length() - 1; i >= 0; i--) {
                JSONObject entry = arr.getJSONObject(i);
                String user = entry.getString("user");
                String kira = entry.getString("kira");
                long at = entry.optLong("at", 0);

                LinearLayout card = new LinearLayout(this);
                card.setOrientation(LinearLayout.VERTICAL);
                card.setBackgroundColor(0xFF1a1a1a);
                card.setPadding(16, 14, 16, 14);
                LinearLayout.LayoutParams cp = new LinearLayout.LayoutParams(WRAP, WRAP_CONTENT);
                cp.setMargins(0, 0, 0, 8);
                card.setLayoutParams(cp);
                card.setClickable(true);
                card.setFocusable(true);

                // Time
                String timeStr = at > 0 ? new java.text.SimpleDateFormat("MMM d, HH:mm", java.util.Locale.getDefault()).format(new java.util.Date(at)) : "";

                LinearLayout headerRow = new LinearLayout(this);
                headerRow.setOrientation(LinearLayout.HORIZONTAL);
                headerRow.setPadding(0, 0, 0, 6);

                TextView timeTv = new TextView(this);
                timeTv.setText(timeStr);
                timeTv.setTextColor(0xFF555555);
                timeTv.setTextSize(11);
                timeTv.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP_CONTENT, 1));
                headerRow.addView(timeTv);

                // Resend button
                TextView resendBtn = new TextView(this);
                resendBtn.setText("↑ resend");
                resendBtn.setTextColor(0xFFff8c00);
                resendBtn.setTextSize(11);
                resendBtn.setPadding(8, 0, 0, 0);
                final String userMsg = user;
                resendBtn.setOnClickListener(v -> {
                    inputField.setText(userMsg);
                    showTab(0);
                    sendMessage();
                });
                headerRow.addView(resendBtn);

                TextView userTv = new TextView(this);
                userTv.setText(user.length() > 100 ? user.substring(0, 100) + "…" : user);
                userTv.setTextColor(0xFFdddddd);
                userTv.setTextSize(13);

                TextView kiraTv = new TextView(this);
                kiraTv.setText(kira.length() > 120 ? kira.substring(0, 120) + "…" : kira);
                kiraTv.setTextColor(0xFF888888);
                kiraTv.setTextSize(12);
                kiraTv.setPadding(0, 4, 0, 0);

                // Tap to expand
                card.setOnClickListener(v -> showFullConversation(user, kira, timeStr));

                card.addView(headerRow);
                card.addView(userTv);
                card.addView(kiraTv);
                historyList.addView(card);
            }
        } catch (Exception e) {
            historyCount.setText("error loading history");
        }
    }

    private void showFullConversation(String user, String kira, String time) {
        new AlertDialog.Builder(this)
            .setTitle(time)
            .setMessage("YOU:\n" + user + "\n\nKIRA:\n" + kira)
            .setPositiveButton("Resend", (d, w) -> { inputField.setText(user); showTab(0); sendMessage(); })
            .setNeutralButton("Copy Kira's reply", (d, w) -> {
                android.content.ClipboardManager cm = (android.content.ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
                if (cm != null) cm.setPrimaryClip(android.content.ClipData.newPlainText("kira", kira));
                Toast.makeText(this, "copied", Toast.LENGTH_SHORT).show();
            })
            .setNegativeButton("Close", null)
            .show();
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    private void updateSettingsUI() {
        cfg = KiraConfig.load(this);
        if (apiKeyHint == null) return;
        apiKeyHint.setText(cfg.apiKey.isEmpty() ? "Not set — tap to add" :
            "••••" + cfg.apiKey.substring(Math.max(0, cfg.apiKey.length() - 4)));
        modelHint.setText(cfg.model.isEmpty() ? "Not set" : cfg.model);
        baseUrlHint.setText(cfg.baseUrl.isEmpty() ? "Not set" : cfg.baseUrl);
        tgTokenHint.setText(cfg.tgToken.isEmpty() ? "Not set" : "Configured");
        tgIdHint.setText(cfg.tgAllowed == 0 ? "Not set" : String.valueOf(cfg.tgAllowed));
        updateShizukuStatus();
    }

    private void toggleFloating() {
        if (!android.provider.Settings.canDrawOverlays(this)) {
            new android.app.AlertDialog.Builder(this)
                .setTitle("Overlay Permission Required")
                .setMessage("Kira needs 'Display over other apps' permission for the floating window.\n\nSettings → Apps → Kira → Display over other apps → Allow")
                .setPositiveButton("Open Settings", (d, w) -> {
                    Intent i = new Intent(android.provider.Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                        android.net.Uri.parse("package:" + getPackageName()));
                    startActivity(i);
                })
                .setNegativeButton("Cancel", null)
                .show();
            return;
        }
        floatingActive = !floatingActive;
        if (floatingActive) {
            FloatingWindowService.start(this);
            floatingToggle.setText("ON");
            floatingToggle.setTextColor(0xFFff8c00);
            floatingToggle.setBackgroundColor(0xFF2a1500);
        } else {
            FloatingWindowService.stop(this);
            floatingToggle.setText("OFF");
            floatingToggle.setTextColor(0xFF666666);
            floatingToggle.setBackgroundColor(0xFF2a2a2a);
        }
    }

    private void updateShizukuStatus() {
        if (shizukuStatusTitle == null) return;
        boolean ok = ShizukuShell.isAvailable();
        boolean installed = ShizukuShell.isInstalled();
        String title = ok ? "Shizuku Connected — God Mode Active" : (installed ? "Shizuku Running — Permission Needed" : "Shizuku Not Running");
        int color = ok ? 0xFF00cc66 : (installed ? 0xFFffaa00 : 0xFFcc4444);
        shizukuStatusTitle.setText(title);
        shizukuStatusTitle.setTextColor(color);
        shizukuStatusIcon.setText(ok ? "✓" : (installed ? "!" : "✗"));
        shizukuStatusIcon.setTextColor(color);
        shizukuStatus.setBackgroundColor(ok ? 0xFF0a1a0a : (installed ? 0xFF1a1200 : 0xFF1a0a0a));
        if (installed && !ok) {
            shizukuStatus.setOnClickListener(v -> checkShizuku());

        // Floating window toggle
        floatingToggle = settingsFragment.findViewById(R.id.floatingToggle);
        settingsFragment.findViewById(R.id.settingFloating).setOnClickListener(v -> toggleFloating());
        }
    }

    interface StringCallback { void onResult(String value); }

    private void editSetting(String title, String current, boolean numeric, StringCallback cb) {
        AlertDialog.Builder builder = new AlertDialog.Builder(this);
        builder.setTitle(title);
        EditText input = new EditText(this);
        input.setText(current);
        input.setTextColor(0xFFffffff);
        input.setHintTextColor(0xFF555555);
        if (numeric) input.setInputType(InputType.TYPE_CLASS_NUMBER);
        LinearLayout wrapper = new LinearLayout(this);
        wrapper.setPadding(48, 16, 48, 0);
        wrapper.addView(input);
        builder.setView(wrapper);
        builder.setPositiveButton("Save", (d, w) -> cb.onResult(input.getText().toString().trim()));
        builder.setNegativeButton("Cancel", null);
        builder.show();
    }

    // ── First run setup ───────────────────────────────────────────────────────

    private void showFirstSetup() {
        setContentView(R.layout.activity_setup);
        EditText nameField    = findViewById(R.id.setupName);
        EditText apiKeyField  = findViewById(R.id.setupApiKey);
        EditText baseUrlField = findViewById(R.id.setupBaseUrl);
        EditText modelField   = findViewById(R.id.setupModel);
        EditText tgField      = findViewById(R.id.setupTgToken);
        EditText tgIdField    = findViewById(R.id.setupTgId);
        android.widget.Button saveBtn = findViewById(R.id.setupSave);

        baseUrlField.setText("https://api.groq.com/openai/v1");
        modelField.setText("llama-3.1-8b-instant");

        saveBtn.setOnClickListener(v -> {
            KiraConfig c = new KiraConfig();
            c.userName  = nameField.getText().toString().trim();
            c.apiKey    = apiKeyField.getText().toString().trim();
            c.baseUrl   = baseUrlField.getText().toString().trim();
            c.model     = modelField.getText().toString().trim();
            c.tgToken   = tgField.getText().toString().trim();
            String id   = tgIdField.getText().toString().trim();
            c.tgAllowed = id.isEmpty() ? 0 : Long.parseLong(id);
            c.setupDone = true;
            if (c.userName.isEmpty()) { nameField.setError("required"); return; }
            if (c.apiKey.isEmpty())   { apiKeyField.setError("required"); return; }
            c.save(this);
            recreate();
        });
    }

    @Override
    protected void onResume() {
        super.onResume();
        if (currentTab == 3) updateShizukuStatus();
    }
}
