package com.kira.service;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.Intent;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.view.ViewGroup;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import android.widget.Toast;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;
import com.kira.service.ai.KiraMemory;

import org.json.JSONArray;
import org.json.JSONObject;

public class MainActivity extends Activity {

    private Handler uiHandler;
    private KiraAI ai;
    private KiraConfig cfg;

    // Views
    private LinearLayout contentFrame;
    private View homeFragment, toolsFragment, historyFragment, settingsFragment;
    private int currentTab = 0;

    // Home
    private LinearLayout chatContainer;
    private ScrollView chatScroll;
    private EditText inputField;
    private TextView sendBtn, headerSubtitle;
    private LinearLayout suggestionsRow;
    private View suggestionsScroll;

    // History
    private LinearLayout historyList;
    private TextView historyCount;

    // Settings
    private TextView apiKeyHint, modelHint, baseUrlHint, tgTokenHint, tgIdHint;
    private TextView shizukuTitle, shizukuIcon;

    // Nav items
    private LinearLayout navHome, navAbilities, navHistory, navSettings;
    private TextView[] navIcons, navTexts;

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
    }

    private void initViews() {
        // Content frame
        android.widget.FrameLayout frame = findViewById(R.id.contentFrame);

        // Inflate all fragments
        homeFragment    = getLayoutInflater().inflate(R.layout.fragment_home, frame, false);
        toolsFragment   = getLayoutInflater().inflate(R.layout.fragment_tools, frame, false);
        historyFragment = getLayoutInflater().inflate(R.layout.fragment_history, frame, false);
        settingsFragment= getLayoutInflater().inflate(R.layout.fragment_settings, frame, false);

        frame.addView(homeFragment);
        frame.addView(toolsFragment);
        frame.addView(historyFragment);
        frame.addView(settingsFragment);

        // Nav
        navHome      = findViewById(R.id.navHome);
        navAbilities = findViewById(R.id.navAbilities);
        navHistory   = findViewById(R.id.navHistory);
        navSettings  = findViewById(R.id.navSettings);

        navHome.setOnClickListener(v -> showTab(0));
        navAbilities.setOnClickListener(v -> showTab(1));
        navHistory.setOnClickListener(v -> showTab(2));
        navSettings.setOnClickListener(v -> showTab(3));

        navIcons = new TextView[]{
            (TextView) navHome.getChildAt(0),
            (TextView) navAbilities.getChildAt(0),
            (TextView) navHistory.getChildAt(0),
            (TextView) navSettings.getChildAt(0)
        };
        navTexts = new TextView[]{
            (TextView) navHome.getChildAt(1),
            (TextView) navAbilities.getChildAt(1),
            (TextView) navHistory.getChildAt(1),
            (TextView) navSettings.getChildAt(1)
        };

        // Home
        chatContainer  = homeFragment.findViewById(R.id.chatContainer);
        chatScroll     = homeFragment.findViewById(R.id.chatScroll);
        inputField     = homeFragment.findViewById(R.id.inputField);
        sendBtn        = homeFragment.findViewById(R.id.sendBtn);
        headerSubtitle = homeFragment.findViewById(R.id.headerSubtitle);
        suggestionsRow = homeFragment.findViewById(R.id.suggestionsRow);
        suggestionsScroll = homeFragment.findViewById(R.id.suggestionsScroll);

        sendBtn.setOnClickListener(v -> sendMessage());
        inputField.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == android.view.inputmethod.EditorInfo.IME_ACTION_SEND) {
                sendMessage(); return true;
            }
            return false;
        });

        addSuggestions();

        // History
        historyList  = historyFragment.findViewById(R.id.historyList);
        historyCount = historyFragment.findViewById(R.id.historyCount);

        // Settings
        apiKeyHint  = settingsFragment.findViewById(R.id.apiKeyHint);
        modelHint   = settingsFragment.findViewById(R.id.modelHint);
        baseUrlHint = settingsFragment.findViewById(R.id.baseUrlHint);
        tgTokenHint = settingsFragment.findViewById(R.id.tgTokenHint);
        tgIdHint    = settingsFragment.findViewById(R.id.tgIdHint);
        shizukuTitle= settingsFragment.findViewById(R.id.shizukuTitle);
        shizukuIcon = settingsFragment.findViewById(R.id.shizukuIcon);

        settingsFragment.findViewById(R.id.settingApiKey).setOnClickListener(v -> editSetting("API Key", cfg.apiKey, false, val -> { cfg.apiKey = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingModel).setOnClickListener(v -> editSetting("Model", cfg.model, false, val -> { cfg.model = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingBaseUrl).setOnClickListener(v -> editSetting("Base URL", cfg.baseUrl, false, val -> { cfg.baseUrl = val; cfg.save(this); updateSettingsUI(); }));
        settingsFragment.findViewById(R.id.settingTgToken).setOnClickListener(v -> editSetting("Telegram Bot Token", cfg.tgToken, false, val -> { cfg.tgToken = val; cfg.save(this); updateSettingsUI(); restartTelegram(); }));
        settingsFragment.findViewById(R.id.settingTgId).setOnClickListener(v -> editSetting("Your Telegram ID", String.valueOf(cfg.tgAllowed == 0 ? "" : cfg.tgAllowed), true, val -> { try { cfg.tgAllowed = Long.parseLong(val); cfg.save(this); updateSettingsUI(); } catch (Exception ignored) {} }));

        // Tools list
        buildToolsList();
        updateSettingsUI();

        // Check accessibility
        if (KiraAccessibilityService.instance == null) {
            addSystemMessage("⚠ Accessibility Service not enabled — phone control won't work.\nGo to Settings → Accessibility → Kira → Enable");
        } else {
            headerSubtitle.setText("Ready, " + cfg.userName.toLowerCase() + ". Tell me what to do.");
        }
    }

    // ── Tab switching ─────────────────────────────────────────────────────────

    private void showTab(int tab) {
        currentTab = tab;
        homeFragment.setVisibility(tab == 0 ? View.VISIBLE : View.GONE);
        toolsFragment.setVisibility(tab == 1 ? View.VISIBLE : View.GONE);
        historyFragment.setVisibility(tab == 2 ? View.VISIBLE : View.GONE);
        settingsFragment.setVisibility(tab == 3 ? View.VISIBLE : View.GONE);

        String[] icons = {"⌂", "★", "≡", "⚙"};
        for (int i = 0; i < 4; i++) {
            boolean active = i == tab;
            navIcons[i].setTextColor(active ? 0xFFFF8C00 : 0xFF666666);
            navTexts[i].setTextColor(active ? 0xFFFF8C00 : 0xFF666666);
        }

        if (tab == 2) refreshHistory();
        if (tab == 3) updateSettingsUI();
    }

    // ── Chat ──────────────────────────────────────────────────────────────────

    private void sendMessage() {
        String text = inputField.getText().toString().trim();
        if (text.isEmpty()) return;
        inputField.setText("");
        suggestionsScroll.setVisibility(View.GONE);

        addBubble("you", text, 0xFF2a2a2a, 0xFFcccccc);
        sendBtn.setEnabled(false);
        headerSubtitle.setText("thinking...");

        ai.chat(text, new KiraAI.Callback() {
            @Override public void onThinking() {
                uiHandler.post(() -> addBubble("kira", "...", 0xFF1a1a1a, 0xFF888888));
            }
            @Override public void onTool(String name, String result) {
                uiHandler.post(() -> addToolBubble(name, result));
            }
            @Override public void onReply(String reply) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    addBubble("kira", reply, 0xFF1e2a1e, 0xFFe0e0e0);
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("Ready, " + cfg.userName.toLowerCase() + ".");
                    scrollToBottom();
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    removeThinkingBubble();
                    addBubble("error", error, 0xFF2a1a1a, 0xFFff6666);
                    sendBtn.setEnabled(true);
                    headerSubtitle.setText("error occurred");
                });
            }
        });
    }

    private void addBubble(String who, String text, int bgColor, int textColor) {
        LinearLayout bubble = new LinearLayout(this);
        bubble.setOrientation(LinearLayout.VERTICAL);
        bubble.setTag(who.equals("kira") && text.equals("...") ? "thinking" : null);

        LinearLayout.LayoutParams params = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        params.setMargins(0, 0, 0, 8);
        bubble.setLayoutParams(params);

        TextView whoLabel = new TextView(this);
        whoLabel.setText(who.toUpperCase());
        whoLabel.setTextSize(10);
        whoLabel.setTextColor(who.equals("kira") ? 0xFFff8c00 : 0xFF666666);
        whoLabel.setPadding(0, 0, 0, 4);

        TextView msgView = new TextView(this);
        msgView.setText(text);
        msgView.setTextSize(14);
        msgView.setTextColor(textColor);
        msgView.setBackgroundColor(bgColor);
        msgView.setPadding(14, 10, 14, 10);
        msgView.setLineSpacing(4, 1);

        bubble.addView(whoLabel);
        bubble.addView(msgView);
        chatContainer.addView(bubble);
        scrollToBottom();
    }

    private void addToolBubble(String name, String result) {
        TextView tv = new TextView(this);
        tv.setText("⚡ " + name + ": " + result.substring(0, Math.min(80, result.length())));
        tv.setTextSize(11);
        tv.setTextColor(0xFF666666);
        tv.setPadding(14, 6, 14, 6);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        p.setMargins(0, 0, 0, 4);
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
        scrollToBottom();
    }

    private void removeThinkingBubble() {
        for (int i = chatContainer.getChildCount() - 1; i >= 0; i--) {
            View v = chatContainer.getChildAt(i);
            if ("thinking".equals(v.getTag())) {
                chatContainer.removeViewAt(i);
                break;
            }
        }
    }

    private void addSystemMessage(String msg) {
        TextView tv = new TextView(this);
        tv.setText(msg);
        tv.setTextSize(12);
        tv.setTextColor(0xFFff8c00);
        tv.setBackgroundColor(0xFF1a1500);
        tv.setPadding(14, 10, 14, 10);
        LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
        p.setMargins(0, 0, 0, 8);
        tv.setLayoutParams(p);
        chatContainer.addView(tv);
    }

    private void scrollToBottom() {
        chatScroll.post(() -> chatScroll.fullScroll(View.FOCUS_DOWN));
    }

    // ── Suggestions ───────────────────────────────────────────────────────────

    private void addSuggestions() {
        String[] suggestions = {
            "Open YouTube", "Check notifications", "What's my battery?",
            "Take a screenshot", "Search web for weather", "Read my screen"
        };
        for (String s : suggestions) {
            TextView chip = new TextView(this);
            chip.setText(s);
            chip.setTextSize(13);
            chip.setTextColor(0xFFcccccc);
            chip.setBackgroundColor(0xFF2a2a2a);
            chip.setPadding(16, 10, 16, 10);
            LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            p.setMargins(0, 0, 8, 0);
            chip.setLayoutParams(p);
            chip.setOnClickListener(v -> {
                inputField.setText(s);
                sendMessage();
            });
            suggestionsRow.addView(chip);
        }
    }

    // ── Tools list ────────────────────────────────────────────────────────────

    private void buildToolsList() {
        LinearLayout list = toolsFragment.findViewById(R.id.toolsList);
        String[][] tools = {
            {"📱", "open_app", "Open any app by name or package"},
            {"👁", "read_screen", "Read all text visible on screen"},
            {"👆", "tap_screen", "Tap any coordinate on screen"},
            {"⌨", "type_text", "Type text into focused field"},
            {"🔔", "get_notifications", "Read all notifications"},
            {"📋", "clipboard", "Get or set clipboard content"},
            {"💬", "send_sms", "Send SMS to any number"},
            {"🔍", "web_search", "Search the web"},
            {"🧠", "remember / recall", "Store and retrieve facts"},
            {"⚡", "sh_run", "Run any shell command (Shizuku)"},
            {"📸", "sh_screenshot", "Take a screenshot"},
            {"🌐", "open_url", "Open any URL"},
        };
        for (String[] tool : tools) {
            LinearLayout row = new LinearLayout(this);
            row.setOrientation(LinearLayout.HORIZONTAL);
            row.setBackgroundColor(0xFF1a1a1a);
            row.setPadding(20, 16, 20, 16);
            LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
            p.setMargins(0, 0, 0, 2);
            row.setLayoutParams(p);

            TextView icon = new TextView(this);
            icon.setText(tool[0]);
            icon.setTextSize(20);
            icon.setWidth(48);
            icon.setGravity(Gravity.CENTER);
            icon.setPadding(0, 0, 12, 0);

            LinearLayout info = new LinearLayout(this);
            info.setOrientation(LinearLayout.VERTICAL);
            TextView name = new TextView(this);
            name.setText(tool[1]);
            name.setTextColor(0xFFffffff);
            name.setTextSize(14);
            TextView desc = new TextView(this);
            desc.setText(tool[2]);
            desc.setTextColor(0xFF888888);
            desc.setTextSize(12);
            info.addView(name);
            info.addView(desc);

            row.addView(icon);
            row.addView(info);
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
                LinearLayout.LayoutParams p = new LinearLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT, ViewGroup.LayoutParams.WRAP_CONTENT);
                p.setMargins(0, 0, 0, 8);
                card.setLayoutParams(p);

                TextView userTv = new TextView(this);
                userTv.setText("YOU: " + user.substring(0, Math.min(60, user.length())));
                userTv.setTextColor(0xFFcccccc);
                userTv.setTextSize(13);

                TextView kiraTv = new TextView(this);
                kiraTv.setText("KIRA: " + kira.substring(0, Math.min(80, kira.length())));
                kiraTv.setTextColor(0xFF888888);
                kiraTv.setTextSize(12);
                kiraTv.setPadding(0, 4, 0, 0);

                TextView timeTv = new TextView(this);
                timeTv.setText(at > 0 ? new java.util.Date(at).toString().substring(0, 16) : "");
                timeTv.setTextColor(0xFF555555);
                timeTv.setTextSize(10);
                timeTv.setPadding(0, 4, 0, 0);

                card.addView(userTv);
                card.addView(kiraTv);
                card.addView(timeTv);
                historyList.addView(card);
            }
        } catch (Exception e) {
            historyCount.setText("Error loading history");
        }
    }

    // ── Settings ──────────────────────────────────────────────────────────────

    private void updateSettingsUI() {
        cfg = KiraConfig.load(this);
        if (apiKeyHint  == null) return;
        apiKeyHint.setText(cfg.apiKey.isEmpty() ? "Not set" : "Set (****" + cfg.apiKey.substring(Math.max(0, cfg.apiKey.length()-4)) + ")");
        modelHint.setText(cfg.model.isEmpty() ? "Not set" : cfg.model);
        baseUrlHint.setText(cfg.baseUrl.isEmpty() ? "Not set" : cfg.baseUrl);
        tgTokenHint.setText(cfg.tgToken.isEmpty() ? "Not set" : "Set");
        tgIdHint.setText(cfg.tgAllowed == 0 ? "Not set" : String.valueOf(cfg.tgAllowed));

        boolean shizukuOk = isShizukuRunning();
        shizukuTitle.setText(shizukuOk ? "Shizuku Connected" : "Shizuku Not Connected");
        shizukuTitle.setTextColor(shizukuOk ? 0xFF00cc66 : 0xFFcc4444);
        shizukuIcon.setText(shizukuOk ? "✓" : "✗");
        shizukuIcon.setTextColor(shizukuOk ? 0xFF00cc66 : 0xFFcc4444);
        shizukuIcon.setBackgroundColor(shizukuOk ? 0xFF003311 : 0xFF330011);
        settingsFragment.findViewById(R.id.shizukuStatus).setBackgroundColor(shizukuOk ? 0xFF1a2a1a : 0xFF2a1a1a);
    }

    private boolean isShizukuRunning() {
        try {
            Process p = Runtime.getRuntime().exec("sh -c id");
            byte[] out = p.getInputStream().readAllBytes();
            return new String(out).contains("uid=");
        } catch (Exception e) { return false; }
    }

    interface StringCallback { void onResult(String value); }

    private void editSetting(String title, String current, boolean numeric, StringCallback cb) {
        AlertDialog.Builder builder = new AlertDialog.Builder(this);
        builder.setTitle(title);
        EditText input = new EditText(this);
        input.setText(current);
        input.setTextColor(0xFFffffff);
        input.setBackgroundColor(0xFF1a1a1a);
        input.setPadding(24, 16, 24, 16);
        if (numeric) input.setInputType(InputType.TYPE_CLASS_NUMBER);
        builder.setView(input);
        builder.setPositiveButton("Save", (d, w) -> cb.onResult(input.getText().toString().trim()));
        builder.setNegativeButton("Cancel", null);
        AlertDialog dialog = builder.create();
        if (dialog.getWindow() != null) {
            dialog.getWindow().setBackgroundDrawableResource(android.R.color.black);
        }
        dialog.show();
    }

    private void restartTelegram() {
        // Telegram restarts automatically when KiraAccessibilityService reads new config
        Toast.makeText(this, "Telegram config updated", Toast.LENGTH_SHORT).show();
    }

    // ── First setup ───────────────────────────────────────────────────────────

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
            String tgId = tgIdField.getText().toString().trim();
            c.tgAllowed = tgId.isEmpty() ? 0 : Long.parseLong(tgId);
            c.setupDone = true;
            if (c.userName.isEmpty()) { nameField.setError("required"); return; }
            if (c.apiKey.isEmpty())   { apiKeyField.setError("required"); return; }
            c.save(this);
            recreate();
        });
    }
}
