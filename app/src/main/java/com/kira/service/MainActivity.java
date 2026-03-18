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

public class MainActivity extends Activity {

    private static final int SHIZUKU_CODE    = 1001;
    private static final int PERMISSION_CODE = 1002;

    private Handler uiHandler;
    private KiraAI  ai;
    private KiraConfig cfg;
    private int currentTab = 0;

    // Fragments
    private View homeFragment, toolsFragment, historyFragment, settingsFragment;

    // Home
    private LinearLayout chatContainer;
    private ScrollView   chatScroll;
    private EditText     inputField;
    private TextView     sendBtn, headerSubtitle;
    private HorizontalScrollView suggestionsScroll;
    private LinearLayout suggestionsRow;

    // Conversation — each turn is stored here for Claude-style resending
    private final List<ConvTurn> conversation = new ArrayList<>();

    // History
    private LinearLayout historyList;
    private TextView     historyCount;

    // Settings
    private TextView    apiKeyHint, modelHint, baseUrlHint, tgTokenHint, tgIdHint;
    private LinearLayout shizukuStatus;
    private TextView    shizukuStatusTitle, shizukuStatusIcon;
    private TextView    floatingToggle;
    private boolean     floatingActive = false;

    // Nav
    private TextView[]     navIcons, navTexts;
    private LinearLayout[] navItems;

    // ── Turn model ────────────────────────────────────────────────────────────

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
        setContentView(R.layout.activity_main);
        uiHandler = new Handler(Looper.getMainLooper());
        cfg = KiraConfig.load(this);

        if (!cfg.setupDone) { showFirstSetup(); return; }

        ai = new KiraAI(this);
        initViews();
        showTab(0);

        requestAllPermissions();
        uiHandler.postDelayed(this::checkShizuku, 500);
        uiHandler.postDelayed(this::checkAccessibility, 2000);

        // Start foreground service to keep Telegram alive
        KiraForegroundService.start(this);
    }

    // ── Permissions ───────────────────────────────────────────────────────────

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
                Shizuku.addRequestPermissionResultListener((code, result) -> {
                    if (result == PackageManager.PERMISSION_GRANTED) {
                        uiHandler.post(() -> {
                            Toast.makeText(this, "✅ Shizuku — god mode active!", Toast.LENGTH_SHORT).show();
                            updateShizukuStatus();
                        });
                    }
                });
                Shizuku.requestPermission(SHIZUKU_CODE);
            }
        } catch (Exception ignored) {}
    }

    private void showShizukuDialog() {
        new AlertDialog.Builder(this)
            .setTitle("Enable Full Phone Control")
            .setMessage("Kira uses Shizuku for ADB-level control (install apps, run shell commands, grant permissions).\n\n1. Install Shizuku from Play Store\n2. Open Shizuku → Start via Wireless Debugging\n3. Return to Kira\n\nBasic screen control still works without Shizuku.")
            .setPositiveButton("Get Shizuku", (d, w) -> {
                try { startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=moe.shizuku.privileged.api"))); }
                catch (Exception e) { startActivity(new Intent(Intent.ACTION_VIEW, Uri.parse("https://shizuku.rikka.app"))); }
            })
            .setNeutralButton("Already Running", (d, w) -> checkShizuku())
            .setNegativeButton("Skip", null).show();
    }

    private void checkAccessibility() {
        if (KiraAccessibilityService.instance == null) {
            new AlertDialog.Builder(this)
                .setTitle("Enable Accessibility Service")
                .setMessage("Kira needs Accessibility Service to read and control your screen.\n\nSettings → Accessibility → Kira → Enable")
                .setPositiveButton("Open Settings", (d, w) -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)))
                .setNegativeButton("Later", null).show();
        }
    }

    // ── View init ─────────────────────────────────────────────────────────────

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

        sendBtn.setOnClickListener(v -> sendMessage());
        inputField.setOnEditorActionListener((v, id, e) -> {
            if (id == android.view.inputmethod.EditorInfo.IME_ACTION_SEND) { sendMessage(); return true; }
            return false;
        });
        buildSuggestions();

        headerSubtitle.setText("ready, " + cfg.userName.toLowerCase() + ".");

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
        floatingToggle    = settingsFragment.findViewById(R.id.floatingToggle);

        settingsFragment.findViewById(R.id.settingApiKey).setOnClickListener(v ->
            editSetting("API Key", cfg.apiKey, false, val -> { cfg.apiKey = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingModel).setOnClickListener(v ->
            editSetting("Model", cfg.model, false, val -> { cfg.model = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingBaseUrl).setOnClickListener(v ->
            editSetting("Base URL", cfg.baseUrl, false, val -> { cfg.baseUrl = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingTgToken).setOnClickListener(v ->
            editSetting("Telegram Bot Token", cfg.tgToken, false, val -> { cfg.tgToken = val; cfg.save(this); updateSettingsUI(); restartTelegram(); }));
        settingsFragment.findViewById(R.id.settingTgId).setOnClickListener(v ->
            editSetting("Your Telegram ID", cfg.tgAllowed == 0 ? "" : String.valueOf(cfg.tgAllowed), true, val -> {
                try { cfg.tgAllowed = val.isEmpty() ? 0 : Long.parseLong(val.trim()); cfg.save(this); updateSettingsUI(); } catch (Exception ignored) {}
            }));
        settingsFragment.findViewById(R.id.settingAccessibility).setOnClickListener(v ->
            startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)));
        shizukuStatus.setOnClickListener(v -> checkShizuku());
        settingsFragment.findViewById(R.id.settingFloating).setOnClickListener(v -> toggleFloating());

        buildToolsList();
        updateSettingsUI();
    }

    // ── Tab nav ───────────────────────────────────────────────────────────────

    private void showTab(int tab) {
        currentTab = tab;
        homeFragment.setVisibility(tab == 0 ? View.VISIBLE : View.GONE);
        toolsFragment.setVisibility(tab == 1 ? View.VISIBLE : View.GONE);
        historyFragment.setVisibility(tab == 2 ? View.VISIBLE : View.GONE);
        settingsFragment.setVisibility(tab == 3 ? View.VISIBLE : View.GONE);
        for (int i = 0; i < 4; i++) {
            boolean on = i == tab;
            navIcons[i].setTextColor(on ? 0xFFFF8C00 : 0xFF666666);
            navTexts[i].setTextColor(on ? 0xFFFF8C00 : 0xFF666666);
            navItems[i].setBackgroundColor(on ? 0xFF1f1a0f : 0x00000000);
        }
        if (tab == 2) refreshHistory();
        if (tab == 3) updateSettingsUI();
    }

    // ── Chat — Claude-style ───────────────────────────────────────────────────

    private void sendMessage() {
        String text = inputField.getText().toString().trim();
        if (text.isEmpty()) return;
        sendMessage(text);
    }

    private void sendMessage(String text) {
        if (text.isEmpty()) return;
        inputField.setText("");
        suggestionsScroll.setVisibility(View.GONE);

        ConvTurn userTurn = new ConvTurn("user", text);
        conversation.add(userTurn);
        addUserBubble(userTurn);

        headerSubtitle.setText("thinking...");
        sendBtn.setEnabled(false);

        // Thinking placeholder
        ConvTurn[] kiraTurn = {null};

        ai.chat(text, new KiraAI.Callback() {
            @Override public void onThinking() {
                uiHandler.post(() -> { if (kiraTurn[0] == null) { kiraTurn[0] = new ConvTurn("kira", "···"); addThinkingBubble(kiraTurn[0]); } });
            }
            @Override public void onTool(String name, String result) {
                ConvTurn toolTurn = new ConvTurn("tool", "⚡ " + name + ": " + result.substring(0, Math.min(100, result.length())));
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
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("ready, " + cfg.userName.toLowerCase() + ".");
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    ConvTurn errTurn = new ConvTurn("error", error);
                    conversation.add(errTurn);
                    addErrorBubble(errTurn);
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("error");
                });
            }
        });
    }

    // ── Bubble builders ───────────────────────────────────────────────────────

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
        label.setTextColor(0xFF777777);
        label.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

        // Edit button — lets user edit and resend (like Claude's edit feature)
        TextView editBtn = new TextView(this);
        editBtn.setText("✏ edit");
        editBtn.setTextColor(0xFF555555);
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
        msg.setTextColor(0xFFdddddd);
        msg.setTextSize(14);
        msg.setBackgroundColor(0xFF2a2a2a);
        msg.setPadding(dp(14), dp(10), dp(14), dp(10));
        msg.setLineSpacing(dp(2), 1);
        msg.setTextIsSelectable(true);

        wrap.addView(labelRow);
        wrap.addView(msg);
        chatContainer.addView(wrap);
        scrollToBottom();
    }

    private View thinkingView;
    private ConvTurn thinkingTurn;

    private void addThinkingBubble(ConvTurn turn) {
        thinkingTurn = turn;
        LinearLayout wrap = new LinearLayout(this);
        wrap.setOrientation(LinearLayout.VERTICAL);
        wrap.setTag("thinking");
        LinearLayout.LayoutParams wp = new LinearLayout.LayoutParams(MATCH, WRAP);
        wp.setMargins(0, 0, 0, dp(4));
        wrap.setLayoutParams(wp);

        TextView label = makeLabel("KIRA");
        label.setTextColor(0xFFff8c00);
        label.setPadding(0, 0, 0, dp(3));

        TextView msg = new TextView(this);
        msg.setText("···");
        msg.setTextColor(0xFF555555);
        msg.setTextSize(14);
        msg.setTag("thinking_msg");

        wrap.addView(label);
        wrap.addView(msg);
        chatContainer.addView(wrap);
        thinkingView = wrap;
        scrollToBottom();
    }

    private void updateThinkingBubble(ConvTurn turn, String reply) {
        if (thinkingView == null) {
            conversation.add(turn);
            addKiraBubble(turn);
            return;
        }
        // Replace the "···" with real content
        chatContainer.removeView(thinkingView);
        thinkingView = null;
        conversation.add(turn);
        addKiraBubble(turn);
    }

    private void removeThinkingBubble() {
        if (thinkingView != null) {
            chatContainer.removeView(thinkingView);
            thinkingView = null;
        }
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
        label.setTextColor(0xFFff8c00);
        label.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

        TextView copyBtn = makeActionBtn("copy");
        copyBtn.setOnClickListener(v -> copyText(turn.text));

        TextView resendBtn = makeActionBtn("↑ resend");
        resendBtn.setOnClickListener(v -> { inputField.setText(turn.text); inputField.setSelection(turn.text.length()); });

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
            msg.setBackgroundColor(0xFF1e1e1e);
            msg.setPadding(dp(14), dp(10), dp(14), dp(10));
            msg.setLineSpacing(dp(2), 1);
            msg.setTextIsSelectable(true);
            wrap.addView(header);
            wrap.addView(msg);
        }

        chatContainer.addView(wrap);
    }

    /**
     * Renders text + code blocks like Claude:
     * - Plain text → regular text view
     * - ```code``` → dark terminal box with language label + Copy button
     */
    private void renderMixedContent(LinearLayout parent, String text) {
        String[] parts = text.split("```");
        boolean inCode = false;
        for (String part : parts) {
            if (!inCode) {
                if (!part.trim().isEmpty()) {
                    TextView tv = new TextView(this);
                    tv.setText(part.trim());
                    tv.setTextColor(0xFFeeeeee);
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
                codeBlock.setBackgroundColor(0xFF0d1117);
                LinearLayout.LayoutParams cbp = new LinearLayout.LayoutParams(MATCH, WRAP);
                cbp.setMargins(0, dp(4), 0, dp(4));
                codeBlock.setLayoutParams(cbp);

                // Code header: language + Copy
                LinearLayout codeHeader = new LinearLayout(this);
                codeHeader.setOrientation(LinearLayout.HORIZONTAL);
                codeHeader.setGravity(Gravity.CENTER_VERTICAL);
                codeHeader.setBackgroundColor(0xFF1a1a2e);
                codeHeader.setPadding(dp(12), dp(6), dp(12), dp(6));

                TextView langLabel = new TextView(this);
                langLabel.setText(lang.isEmpty() ? "code" : lang);
                langLabel.setTextColor(0xFF888888);
                langLabel.setTextSize(11);
                langLabel.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

                final String finalCode = code.trim();
                TextView codeCopyBtn = new TextView(this);
                codeCopyBtn.setText("Copy");
                codeCopyBtn.setTextColor(0xFFff8c00);
                codeCopyBtn.setTextSize(11);
                codeCopyBtn.setOnClickListener(v -> {
                    copyText(finalCode);
                    codeCopyBtn.setText("Copied!");
                    uiHandler.postDelayed(() -> codeCopyBtn.setText("Copy"), 2000);
                });

                codeHeader.addView(langLabel);
                codeHeader.addView(codeCopyBtn);

                // Code body — monospace, horizontally scrollable
                HorizontalScrollView hScroll = new HorizontalScrollView(this);
                hScroll.setHorizontalScrollBarEnabled(true);
                hScroll.setLayoutParams(new LinearLayout.LayoutParams(MATCH, WRAP));

                TextView codeTv = new TextView(this);
                codeTv.setText(finalCode);
                codeTv.setTextColor(0xFF00ff88);
                codeTv.setTextSize(12);
                codeTv.setTypeface(android.graphics.Typeface.MONOSPACE);
                codeTv.setPadding(dp(12), dp(10), dp(12), dp(10));
                codeTv.setTextIsSelectable(true);
                codeTv.setBackgroundColor(0xFF0d1117);

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
        tv.setBackgroundColor(0xFF0d1a0d);
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
        label.setTextColor(0xFFcc4444);
        label.setPadding(0, 0, 0, dp(3));

        TextView msg = new TextView(this);
        msg.setText(turn.text);
        msg.setTextColor(0xFFff8888);
        msg.setTextSize(13);
        msg.setBackgroundColor(0xFF2a1010);
        msg.setPadding(dp(14), dp(10), dp(14), dp(10));
        msg.setTextIsSelectable(true);

        wrap.addView(label);
        wrap.addView(msg);
        chatContainer.addView(wrap);
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

    private void scrollToBottom() {
        chatScroll.post(() -> chatScroll.fullScroll(View.FOCUS_DOWN));
    }

    private void copyText(String text) {
        android.content.ClipboardManager cm = (android.content.ClipboardManager) getSystemService(CLIPBOARD_SERVICE);
        if (cm != null) cm.setPrimaryClip(android.content.ClipData.newPlainText("kira", text));
        Toast.makeText(this, "Copied", Toast.LENGTH_SHORT).show();
    }

    // ── Suggestions ───────────────────────────────────────────────────────────

    private void buildSuggestions() {
        String[][] s = {
            {"📱","Open YouTube"}, {"🔔","Check notifications"}, {"🔋","Battery status"},
            {"📸","Take screenshot"}, {"🌐","Search web for news"}, {"📋","Read my screen"},
            {"💬","Show recent SMS"}, {"⚡","Running apps"},
        };
        for (String[] item : s) {
            TextView chip = new TextView(this);
            chip.setText(item[0] + " " + item[1]);
            chip.setTextSize(12);
            chip.setTextColor(0xFFcccccc);
            chip.setBackgroundColor(0xFF222222);
            chip.setPadding(dp(12), dp(7), dp(12), dp(7));
            LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(WRAP, WRAP);
            p.setMargins(0, 0, dp(8), 0);
            chip.setLayoutParams(p);
            chip.setOnClickListener(v -> { inputField.setText(item[1]); sendMessage(); });
            suggestionsRow.addView(chip);
        }
    }

    // ── Tools list ────────────────────────────────────────────────────────────

    private void buildToolsList() {
        LinearLayout list = toolsFragment.findViewById(R.id.toolsList);
        Object[][] tools = {
            {"📱","open_app {package}","Open any app — use names like 'youtube', 'whatsapp'"},
            {"👁","read_screen {}","Read all text on current screen"},
            {"👆","tap_screen {x,y}","Tap coordinates"},
            {"🔍","tap_text {text}","Find and tap element by text"},
            {"⌨","type_text {text}","Type into focused field"},
            {"🔔","get_notifications {}","All recent notifications"},
            {"💬","send_sms {number,message}","Send SMS"},
            {"🔍","web_search {query}","Search DuckDuckGo"},
            {"⚡","sh_run {cmd}","Run any shell command"},
            {"📸","sh_screenshot {}","Take screenshot"},
            {"🧠","remember {key,value}","Store a fact permanently"},
            {"📞","call_number {number}","Make phone call"},
            {"🌐","http_get {url}","HTTP GET request"},
            {"📂","list_files {path}","List directory"},
            {"📖","read_file {path}","Read file content"},
            {"🔧","get_setting {namespace,key}","Read system setting"},
            {"⚙","set_setting {namespace,key,value}","Write system setting"},
            {"📶","wifi_on {on}","Toggle WiFi"},
            {"📡","mobile_data {on}","Toggle mobile data"},
            {"🔋","battery_info {}","Battery status"},
        };
        for (Object[] t : tools) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setBackgroundColor(0xFF1a1a1a);
            row.setPadding(dp(14), dp(12), dp(14), dp(12));
            LinearLayout.LayoutParams rp = new LinearLayout.LayoutParams(MATCH, WRAP);
            rp.setMargins(0, 0, 0, dp(2));
            row.setLayoutParams(rp);
            row.setClickable(true); row.setFocusable(true);

            final String example = "<tool:" + t[1] + "></tool>";
            row.setOnClickListener(v -> { showTab(0); });

            TextView icon = new TextView(this);
            icon.setText((String)t[0]); icon.setTextSize(20);
            icon.setWidth(dp(44)); icon.setGravity(Gravity.CENTER);

            LinearLayout info = new LinearLayout(this);
            info.setOrientation(LinearLayout.VERTICAL);
            info.setLayoutParams(new LinearLayout.LayoutParams(0, WRAP, 1));

            TextView name = new TextView(this); name.setText((String)t[1]);
            name.setTextColor(0xFFffffff); name.setTextSize(13);
            name.setTypeface(android.graphics.Typeface.MONOSPACE);

            TextView desc = new TextView(this); desc.setText((String)t[2]);
            desc.setTextColor(0xFF888888); desc.setTextSize(12);

            info.addView(name); info.addView(desc);
            row.addView(icon); row.addView(info);
            list.addView(row);
        }
    }

    // ── History — Claude-style ────────────────────────────────────────────────

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
                card.setBackgroundColor(0xFF1a1a1a);
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

                // Resend — puts user message in input and sends
                TextView resendBtn = makeActionBtn("↑ resend");
                resendBtn.setTextColor(0xFFff8c00);
                resendBtn.setOnClickListener(v -> {
                    showTab(0);
                    inputField.setText(user);
                    sendMessage();
                });

                // Continue — put in input field only (user can edit before sending)
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
                userTv.setText(user.length() > 120 ? user.substring(0, 120) + "…" : user);
                userTv.setTextColor(0xFFdddddd);
                userTv.setTextSize(13);

                // Kira reply preview
                TextView kiraTv = new TextView(this);
                kiraTv.setText(kira.length() > 150 ? kira.substring(0, 150) + "…" : kira);
                kiraTv.setTextColor(0xFF888888);
                kiraTv.setTextSize(12);
                kiraTv.setPadding(0, dp(4), 0, 0);

                // Tap to see full conversation
                card.setOnClickListener(v -> showFullDialog(user, kira, timeStr));

                card.addView(headerRow);
                card.addView(userTv);
                card.addView(kiraTv);
                historyList.addView(card);
            }
        } catch (Exception e) {
            historyCount.setText("error loading history");
        }
    }

    private void showFullDialog(String user, String kira, String time) {
        new AlertDialog.Builder(this)
            .setTitle(time)
            .setMessage("YOU:\n" + user + "\n\n─────────────\n\nKIRA:\n" + kira)
            .setPositiveButton("↑ Resend", (d, w) -> { showTab(0); inputField.setText(user); sendMessage(); })
            .setNeutralButton("Copy Reply", (d, w) -> copyText(kira))
            .setNegativeButton("Close", null)
            .show();
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    private void updateSettingsUI() {
        cfg = KiraConfig.load(this);
        if (apiKeyHint == null) return;
        apiKeyHint.setText(cfg.apiKey.isEmpty() ? "Tap to set" : "••••" + cfg.apiKey.substring(Math.max(0, cfg.apiKey.length()-4)));
        modelHint.setText(cfg.model.isEmpty() ? "Not set" : cfg.model);
        baseUrlHint.setText(cfg.baseUrl.isEmpty() ? "Not set" : cfg.baseUrl);
        tgTokenHint.setText(cfg.tgToken.isEmpty() ? "Not set" : "✅ Configured");
        tgIdHint.setText(cfg.tgAllowed == 0 ? "0 = anyone" : String.valueOf(cfg.tgAllowed));
        updateShizukuStatus();
    }

    private void updateShizukuStatus() {
        if (shizukuStatusTitle == null) return;
        boolean ok = ShizukuShell.isAvailable();
        boolean installed = ShizukuShell.isInstalled();
        String title = ok ? "Shizuku ✅ God Mode Active" : (installed ? "Shizuku ⚠ Permission Needed (tap)" : "Shizuku ❌ Not Running (tap)");
        int color = ok ? 0xFF00cc66 : (installed ? 0xFFffaa00 : 0xFFcc4444);
        shizukuStatusTitle.setText(title);
        shizukuStatusTitle.setTextColor(color);
        shizukuStatusIcon.setText(ok ? "✓" : (installed ? "!" : "✗"));
        shizukuStatusIcon.setTextColor(color);
        shizukuStatus.setBackgroundColor(ok ? 0xFF0a1a0a : (installed ? 0xFF1a1200 : 0xFF1a0a0a));
    }

    private void toggleFloating() {
        if (!Settings.canDrawOverlays(this)) {
            new AlertDialog.Builder(this)
                .setTitle("Overlay Permission Needed")
                .setMessage("For the floating window, Kira needs 'Display over other apps'.\n\nSettings → Apps → Kira → Display over other apps → Enable")
                .setPositiveButton("Open Settings", (d, w) ->
                    startActivity(new Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:" + getPackageName()))))
                .setNegativeButton("Cancel", null).show();
            return;
        }
        floatingActive = !floatingActive;
        if (floatingActive) {
            FloatingWindowService.start(this);
            floatingToggle.setText("ON");
            floatingToggle.setTextColor(0xFFff8c00);
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
        Toast.makeText(this, "Telegram config updated — restarting bot", Toast.LENGTH_SHORT).show();
    }

    interface StringCallback { void onResult(String v); }

    private void editSetting(String title, String current, boolean numeric, StringCallback cb) {
        AlertDialog.Builder b = new AlertDialog.Builder(this);
        b.setTitle(title);
        EditText et = new EditText(this);
        et.setText(current);
        et.setTextColor(0xFFffffff);
        if (numeric) et.setInputType(InputType.TYPE_CLASS_NUMBER);
        LinearLayout w = new LinearLayout(this);
        w.setPadding(dp(48), dp(16), dp(48), 0);
        w.addView(et);
        b.setView(w);
        b.setPositiveButton("Save", (d, x) -> cb.onResult(et.getText().toString().trim()));
        b.setNegativeButton("Cancel", null);
        b.show();
    }

    // ── First setup ───────────────────────────────────────────────────────────

    private void showFirstSetup() {
        setContentView(R.layout.activity_setup);
        EditText nameF  = findViewById(R.id.setupName);
        EditText keyF   = findViewById(R.id.setupApiKey);
        EditText urlF   = findViewById(R.id.setupBaseUrl);
        EditText modelF = findViewById(R.id.setupModel);
        EditText tgTF   = findViewById(R.id.setupTgToken);
        EditText tgIF   = findViewById(R.id.setupTgId);
        android.widget.Button saveBtn = findViewById(R.id.setupSave);
        urlF.setText("https://api.groq.com/openai/v1");
        modelF.setText("llama-3.1-8b-instant");
        saveBtn.setOnClickListener(v -> {
            KiraConfig c = new KiraConfig();
            c.userName  = nameF.getText().toString().trim();
            c.apiKey    = keyF.getText().toString().trim();
            c.baseUrl   = urlF.getText().toString().trim();
            c.model     = modelF.getText().toString().trim();
            c.tgToken   = tgTF.getText().toString().trim();
            String tid  = tgIF.getText().toString().trim();
            c.tgAllowed = tid.isEmpty() ? 0 : Long.parseLong(tid);
            c.setupDone = true;
            if (c.userName.isEmpty()) { nameF.setError("required"); return; }
            if (c.apiKey.isEmpty())   { keyF.setError("required"); return; }
            c.save(this); recreate();
        });
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

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
}
