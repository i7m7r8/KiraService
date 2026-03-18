package com.kira.service;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.provider.Settings;
import android.widget.Button;
import android.widget.TextView;
import android.view.View;

public class MainActivity extends Activity {

    private TextView statusText;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        statusText = findViewById(R.id.statusText);
        Button btnAccessibility = findViewById(R.id.btnAccessibility);
        Button btnShizuku = findViewById(R.id.btnShizuku);

        btnAccessibility.setOnClickListener(v -> {
            Intent i = new Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS);
            startActivity(i);
        });

        btnShizuku.setOnClickListener(v -> {
            // Open Shizuku app
            Intent i = getPackageManager().getLaunchIntentForPackage("moe.shizuku.privileged.api");
            if (i != null) startActivity(i);
            else {
                statusText.setText("Shizuku not installed. Get it from Play Store or GitHub.");
            }
        });
    }

    @Override
    protected void onResume() {
        super.onResume();
        updateStatus();
    }

    private void updateStatus() {
        boolean accessEnabled = KiraAccessibilityService.instance != null;
        boolean shizukuEnabled = false;
        try {
            shizukuEnabled = rikka.shizuku.Shizuku.checkSelfPermission() ==
                android.content.pm.PackageManager.PERMISSION_GRANTED;
        } catch (Exception ignored) {}

        StringBuilder sb = new StringBuilder();
        sb.append("KiraService Status\n\n");
        sb.append("Accessibility: ").append(accessEnabled ? "✓ ACTIVE" : "✗ disabled").append("\n");
        sb.append("Shizuku:       ").append(shizukuEnabled ? "✓ ACTIVE" : "✗ disabled").append("\n");
        sb.append("HTTP Server:   ").append(accessEnabled ? "✓ localhost:7070" : "✗ waiting").append("\n\n");

        if (accessEnabled && shizukuEnabled) {
            sb.append("🟢 GOD MODE ACTIVE\nKira has full phone control.");
        } else if (accessEnabled) {
            sb.append("🟡 BASIC MODE\nEnable Shizuku for full control.");
        } else {
            sb.append("🔴 NOT ACTIVE\nEnable Accessibility Service first.");
        }

        statusText.setText(sb.toString());
    }
}
