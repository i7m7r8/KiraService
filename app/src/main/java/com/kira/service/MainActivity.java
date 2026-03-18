package com.kira.service;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.text.method.ScrollingMovementMethod;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;

import com.kira.service.ai.KiraAI;
import com.kira.service.ai.KiraConfig;

public class MainActivity extends Activity {

    private TextView chatLog;
    private EditText inputField;
    private Button sendBtn;
    private ScrollView scrollView;
    private KiraAI ai;
    private Handler uiHandler;
    private StringBuilder log = new StringBuilder();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        uiHandler  = new Handler(Looper.getMainLooper());
        chatLog    = findViewById(R.id.chatLog);
        inputField = findViewById(R.id.inputField);
        sendBtn    = findViewById(R.id.sendBtn);
        scrollView = findViewById(R.id.scrollView);

        chatLog.setMovementMethod(new ScrollingMovementMethod());

        KiraConfig cfg = KiraConfig.load(this);

        if (!cfg.setupDone) {
            showSetup();
            return;
        }

        initChat(cfg);
    }

    private void initChat(KiraConfig cfg) {
        ai = new KiraAI(this);

        // Check accessibility
        if (KiraAccessibilityService.instance == null) {
            appendLog("system", "⚠ Accessibility Service not enabled. tap the button below.");
            Button btn = new Button(this);
            btn.setText("Enable Accessibility Service");
            btn.setOnClickListener(v -> startActivity(new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)));
            ((LinearLayout) findViewById(R.id.mainLayout)).addView(btn);
        } else {
            appendLog("kira", "hey " + cfg.userName.toLowerCase() + ". i'm ready.");
        }

        sendBtn.setOnClickListener(v -> {
            String text = inputField.getText().toString().trim();
            if (text.isEmpty()) return;
            inputField.setText("");
            sendMessage(text);
        });

        inputField.setOnEditorActionListener((v, actionId, event) -> {
            if (actionId == android.view.inputmethod.EditorInfo.IME_ACTION_SEND) {
                sendBtn.performClick();
                return true;
            }
            return false;
        });
    }

    private void sendMessage(String text) {
        appendLog("you", text);
        sendBtn.setEnabled(false);

        ai.chat(text, new KiraAI.Callback() {
            @Override public void onThinking() {
                uiHandler.post(() -> appendLog("kira", "..."));
            }
            @Override public void onTool(String name, String result) {
                uiHandler.post(() -> appendLog("tool", name + ": " + result.substring(0, Math.min(80, result.length()))));
            }
            @Override public void onReply(String reply) {
                uiHandler.post(() -> {
                    // Remove last "..." if present
                    String current = log.toString();
                    if (current.endsWith("kira: ...\n")) {
                        log = new StringBuilder(current.substring(0, current.length() - "kira: ...\n".length()));
                    }
                    appendLog("kira", reply);
                    sendBtn.setEnabled(true);
                });
            }
            @Override public void onError(String error) {
                uiHandler.post(() -> {
                    appendLog("error", error);
                    sendBtn.setEnabled(true);
                });
            }
        });
    }

    private void appendLog(String who, String text) {
        log.append(who).append(": ").append(text).append("\n\n");
        chatLog.setText(log.toString());
        scrollView.post(() -> scrollView.fullScroll(View.FOCUS_DOWN));
    }

    // ── Setup screen ──────────────────────────────────────────────────────────

    private void showSetup() {
        setContentView(R.layout.activity_setup);

        EditText nameField   = findViewById(R.id.setupName);
        EditText apiKeyField = findViewById(R.id.setupApiKey);
        EditText baseUrlField = findViewById(R.id.setupBaseUrl);
        EditText modelField  = findViewById(R.id.setupModel);
        EditText tgField     = findViewById(R.id.setupTgToken);
        EditText tgIdField   = findViewById(R.id.setupTgId);
        Button saveBtn       = findViewById(R.id.setupSave);

        // Defaults
        baseUrlField.setText("https://api.groq.com/openai/v1");
        modelField.setText("llama-3.1-8b-instant");

        saveBtn.setOnClickListener(v -> {
            KiraConfig cfg = new KiraConfig();
            cfg.userName   = nameField.getText().toString().trim();
            cfg.apiKey     = apiKeyField.getText().toString().trim();
            cfg.baseUrl    = baseUrlField.getText().toString().trim();
            cfg.model      = modelField.getText().toString().trim();
            cfg.tgToken    = tgField.getText().toString().trim();
            String tgId    = tgIdField.getText().toString().trim();
            cfg.tgAllowed  = tgId.isEmpty() ? 0 : Long.parseLong(tgId);
            cfg.setupDone  = true;

            if (cfg.userName.isEmpty()) { nameField.setError("required"); return; }
            if (cfg.apiKey.isEmpty())   { apiKeyField.setError("required"); return; }

            cfg.save(this);

            // Restart to chat screen
            recreate();
        });
    }
}
