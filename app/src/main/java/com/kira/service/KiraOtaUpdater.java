package com.kira.service;

import android.content.Context;
import android.content.Intent;
import android.content.pm.PackageInfo;
import android.content.pm.PackageInstaller;
import android.content.pm.PackageManager;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import org.json.JSONArray;
import org.json.JSONObject;

import com.kira.service.ai.KiraConfig;
import java.io.BufferedInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.security.MessageDigest;
import java.util.concurrent.atomic.AtomicBoolean;

/**
 * KiraService v43 — Intelligent OTA Updater
 *
 * Architecture: Rust owns all state. Java executes what Rust decides.
 *
 * Flow:
 *  1. checkForUpdate() → GitHub Releases API
 *  2. Feed parsed release data to RustBridge.otaOnRelease()
 *  3. Rust decides: "prompt_user" | "up_to_date" | "skipped"
 *  4. On user confirm: streamDownloadWithProgress() streams APK
 *  5. Progress → RustBridge.otaProgress() (Rust tracks %)
 *  6. On complete: SHA256 verify → RustBridge.otaOnDownloaded()
 *  7. Rust returns: method = "shizuku" | "package_installer"
 *  8. installApk() picks the right path automatically
 *     - Shizuku: `pm install -r -t <path>` (silent, no user tap needed)
 *     - PackageInstaller: session-based (single confirm dialog)
 *     - Fallback: ACTION_VIEW intent (legacy)
 *  9. Success/fail → RustBridge.otaOnInstalled/otaOnFailed
 *
 * True OTA = no reinstall. PackageInstaller replaces in place.
 * With Shizuku = completely silent (god mode).
 */
public class KiraOtaUpdater {

    private static final String TAG = "KiraOTA";
    private static final String GITHUB_API =
        "https://api.github.com/repos/%s/releases/latest";
    private static final long CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000L;

    private final Context ctx;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final AtomicBoolean downloading = new AtomicBoolean(false);

    // Callback for UI (Settings screen OTA row)
    public interface OtaCallback {
        void onCheckStart();
        void onUpdateAvailable(String version, String changelog, Runnable onInstall, Runnable onSkip);
        void onProgress(int pct, long bytesDone, long bytesTotal);
        void onInstalling(String method);
        void onSuccess(String version);
        void onError(String msg);
        void onUpToDate();
    }

    private OtaCallback callback;

    public KiraOtaUpdater(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    public void setCallback(OtaCallback cb) { this.callback = cb; }

    // ─── Lifecycle ────────────────────────────────────────────────────────────

    /** Call once on app start. Registers current version with Rust. */
    public void init() {
        try {
            PackageInfo pi = ctx.getPackageManager()
                .getPackageInfo(ctx.getPackageName(), 0);
            long code = android.os.Build.VERSION.SDK_INT >= 28
                ? pi.getLongVersionCode()
                : (long) pi.versionCode;
            try { RustBridge.otaSetCurrentVersion(pi.versionName, code); } catch (Throwable ignored) {}
            KiraConfig cfg = KiraConfig.load(ctx);
            String repo = (cfg.otaRepo != null && !cfg.otaRepo.isEmpty()) ? cfg.otaRepo : "i7m7r8/KiraService";
            try { RustBridge.otaSetRepo(repo); } catch (Throwable ignored) {}
            Log.d(TAG, "OTA init: v" + pi.versionName + " (" + code + ")");
        } catch (Exception e) {
            Log.w(TAG, "OTA init failed: " + e.getMessage());
        }
    }

    /** Schedule background checks every 6 hours. */
    public void scheduleChecks() {
        handler.postDelayed(this::checkForUpdate, 45_000);
        handler.postDelayed(new Runnable() {
            @Override public void run() {
                checkForUpdate();
                handler.postDelayed(this, CHECK_INTERVAL_MS);
            }
        }, CHECK_INTERVAL_MS);
    }

    // ─── Check ────────────────────────────────────────────────────────────────

    /** Check GitHub for latest release. Rust decides what to do. */
    public void checkForUpdate() {
        if (callback != null) handler.post(() -> callback.onCheckStart());
        new Thread(() -> {
            try {
                KiraConfig cfg = KiraConfig.load(ctx);
                String repo = cfg.otaRepo.isEmpty() ? "i7m7r8/KiraService" : cfg.otaRepo;
                String apiUrl = String.format(GITHUB_API, repo);

                String json = httpGet(apiUrl);
                if (json == null || json.isEmpty()) {
                    notifyError("No response from GitHub");
                    return;
                }

                JSONObject rel = new JSONObject(json);
                String tag      = rel.optString("tag_name", "");
                String body     = rel.optString("body", "");
                String date     = rel.optString("published_at", "");
                String apkUrl   = null;
                long   apkBytes = 0;

                // Find best APK asset: prefer release, accept debug
                JSONArray assets = rel.optJSONArray("assets");
                if (assets != null) {
                    // First pass: release APK
                    for (int i = 0; i < assets.length(); i++) {
                        JSONObject a = assets.getJSONObject(i);
                        String name = a.optString("name", "");
                        if (name.endsWith(".apk") && !name.contains("debug")) {
                            apkUrl   = a.optString("browser_download_url", "");
                            apkBytes = a.optLong("size", 0);
                            break;
                        }
                    }
                    // Second pass: any APK
                    if (apkUrl == null || apkUrl.isEmpty()) {
                        for (int i = 0; i < assets.length(); i++) {
                            JSONObject a = assets.getJSONObject(i);
                            String name = a.optString("name", "");
                            if (name.endsWith(".apk")) {
                                apkUrl   = a.optString("browser_download_url", "");
                                apkBytes = a.optLong("size", 0);
                                break;
                            }
                        }
                    }
                }

                if (apkUrl == null || apkUrl.isEmpty()) {
                    // No APK asset — check if zipball works
                    Log.w(TAG, "No APK asset in release " + tag);
                    if (callback != null)
                        handler.post(() -> callback.onUpToDate());
                    return;
                }

                // Feed to Rust — Rust decides action
                String action = "{}";
                try {
                    String rustResult = RustBridge.otaOnRelease(
                        tag, apkUrl, body != null ? body : "", date, "", apkBytes
                    );
                    if (rustResult != null && !rustResult.trim().isEmpty()) {
                        action = rustResult;
                    }
                } catch (Throwable rustEx) {
                    Log.w(TAG, "OTA: Rust call failed: " + rustEx);
                }
                JSONObject decision;
                try { decision = new JSONObject(action); }
                catch (Exception ex) {
                    Log.w(TAG, "OTA: bad JSON from Rust: " + action);
                    decision = new JSONObject();
                }
                String act = decision.optString("action", "prompt_user");

                Log.d(TAG, "Rust OTA decision: " + act + " for " + tag);

                final String finalTag = tag;
                final String finalUrl = apkUrl;
                final String safeBody = (body != null) ? body : "";
                final String finalLog = safeBody.length() > 500
                    ? safeBody.substring(0, 500) + "…" : safeBody;

                switch (act) {
                    case "prompt_user":
                        if (callback != null) {
                            handler.post(() -> callback.onUpdateAvailable(
                                finalTag, finalLog,
                                () -> startDownload(finalUrl, finalTag),
                                () -> { RustBridge.otaSkip(finalTag); }
                            ));
                        } else {
                            // No UI callback — send notification
                            sendUpdateNotification(tag, apkUrl, body);
                        }
                        break;
                    case "up_to_date":
                        if (callback != null)
                            handler.post(() -> callback.onUpToDate());
                        break;
                    case "skipped":
                        Log.d(TAG, "Version " + tag + " is skipped by user");
                        break;
                }
            } catch (Exception e) {
                Log.w(TAG, "OTA check error: " + e.getMessage());
                String otaErrMsg = e.getMessage();
                if (otaErrMsg == null) otaErrMsg = e.getClass().getSimpleName();
                notifyError(otaErrMsg);
            }
        }, "KiraOTA-Check").start();
    }

    // ─── Download ─────────────────────────────────────────────────────────────

    /** Stream the APK with live progress updates to Rust. */
    public void startDownload(String url, String tag) {
        if (downloading.getAndSet(true)) {
            Log.d(TAG, "Download already in progress");
            return;
        }
        new Thread(() -> {
            File apkFile = new File(ctx.getCacheDir(), "kira_ota_" + tag + ".apk");
            try {
                Log.i(TAG, "Downloading APK: " + url);
                HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
                conn.setConnectTimeout(15_000);
                conn.setReadTimeout(120_000);
                conn.setRequestProperty("User-Agent", "KiraOTA/43");

                int code = conn.getResponseCode();
                if (code != 200) {
                    throw new Exception("HTTP " + code + " from " + url);
                }

                long total = conn.getContentLengthLong();
                long done = 0;

                try (InputStream in   = new BufferedInputStream(conn.getInputStream(), 65536);
                     FileOutputStream out = new FileOutputStream(apkFile)) {

                    byte[] buf = new byte[65536];
                    int n;
                    while ((n = in.read(buf)) != -1) {
                        out.write(buf, 0, n);
                        done += n;
                        // Report progress to Rust
                        final long fd = done, ft = total;
                        RustBridge.otaProgress(fd, ft);
                        if (callback != null) {
                            int pct = total > 0 ? (int)((done * 100) / total) : 0;
                            final int fpct = pct;
                            handler.post(() -> callback.onProgress(fpct, fd, ft));
                        }
                    }
                }

                // Compute SHA256
                String sha256 = sha256File(apkFile);
                Log.i(TAG, "Download complete: " + apkFile.length() + " bytes, SHA256=" + sha256);

                // Tell Rust: downloaded. Rust verifies and returns install method.
                String instJson = RustBridge.otaOnDownloaded(
                    apkFile.getAbsolutePath(), sha256
                );
                if (instJson == null || instJson.trim().isEmpty()) instJson = "{}";
                JSONObject inst;
                try { inst = new JSONObject(instJson); }
                catch (Exception ex) {
                    Log.w(TAG, "OTA: bad install JSON from Rust: " + instJson);
                    inst = new JSONObject();
                }

                if (!inst.optBoolean("ok", false)) {
                    throw new Exception(inst.optString("error", "SHA256 mismatch"));
                }

                String method = inst.optString("method", "intent");
                Log.i(TAG, "Install method: " + method);
                if (callback != null)
                    handler.post(() -> callback.onInstalling(method));

                installApk(apkFile, method, tag);

            } catch (Exception e) {
                Log.e(TAG, "Download failed: " + e.getMessage());
                apkFile.delete();
                RustBridge.otaOnFailed(e.getMessage());
                notifyError(e.getMessage());
            } finally {
                downloading.set(false);
            }
        }, "KiraOTA-Download").start();
    }

    // ─── Install ─────────────────────────────────────────────────────────────

    private void installApk(File apk, String method, String tag) {
        switch (method) {
            case "shizuku":
                installViaShizuku(apk, tag);
                break;
            case "package_installer":
                installViaPackageInstaller(apk, tag);
                break;
            default:
                installViaIntent(apk, tag);
        }
    }

    /**
     * Shizuku path: `pm install -r -t <path>`
     * Silent — no user interaction needed. Works like adb install.
     * -r = replace existing  -t = allow test APKs
     */
    private void installViaShizuku(File apk, String tag) {
        new Thread(() -> {
            try {
                RustBridge.otaSetCurrentVersion("installing", -1);
                // Shizuku exec
                String cmd = "pm install -r -t \"" + apk.getAbsolutePath() + "\"";
                Log.i(TAG, "Shizuku install: " + cmd);
                String result = ShizukuShell.exec(cmd, 60_000);
                Log.i(TAG, "pm install result: " + result);

                if (result != null && (result.contains("Success") || result.contains("success"))) {
                    String newVer = getInstalledVersion();
                    RustBridge.otaOnInstalled(newVer);
                    if (callback != null)
                        handler.post(() -> callback.onSuccess(newVer));
                    apk.delete();
                    // Soft restart: just inform user
                    handler.post(() -> sendSuccessNotification(tag));
                } else {
                    String err = "pm install failed: " + (result != null ? result : "null");
                    RustBridge.otaOnFailed(err);
                    // Fall back to PackageInstaller
                    Log.w(TAG, "Shizuku failed, falling back: " + err);
                    installViaPackageInstaller(apk, tag);
                }
            } catch (Exception e) {
                Log.e(TAG, "Shizuku install error", e);
                installViaPackageInstaller(apk, tag);
            }
        }, "KiraOTA-ShizukuInstall").start();
    }

    /**
     * PackageInstaller session — replaces APK in place.
     * Shows a single system confirm dialog (one tap). No reinstall needed.
     * This is what app stores use for silent updates.
     */
    private void installViaPackageInstaller(File apk, String tag) {
        try {
            PackageInstaller pi = ctx.getPackageManager().getPackageInstaller();
            PackageInstaller.SessionParams params = new PackageInstaller.SessionParams(
                PackageInstaller.SessionParams.MODE_FULL_INSTALL);
            params.setAppPackageName(ctx.getPackageName());

            int sessionId = pi.createSession(params);
            Log.i(TAG, "PackageInstaller session: " + sessionId);

            try (PackageInstaller.Session session = pi.openSession(sessionId)) {
                // Stream APK bytes into session
                try (InputStream in   = new FileInputStream(apk);
                     OutputStream out = session.openWrite("kira_ota.apk", 0, apk.length())) {
                    byte[] buf = new byte[65536];
                    int n;
                    while ((n = in.read(buf)) != -1) out.write(buf, 0, n);
                    session.fsync(out);
                }

                // Intent fired when install completes/fails
                Intent statusIntent = new Intent(ctx, OtaInstallReceiver.class)
                    .putExtra("tag", tag)
                    .putExtra("apk_path", apk.getAbsolutePath());
                android.app.PendingIntent pi2 = android.app.PendingIntent.getBroadcast(
                    ctx, sessionId, statusIntent,
                    android.app.PendingIntent.FLAG_UPDATE_CURRENT
                    | android.app.PendingIntent.FLAG_IMMUTABLE);

                session.commit(pi2.getIntentSender());
                Log.i(TAG, "PackageInstaller session committed");

                // Notify Rust session opened
                String method = "package_installer";
                RustBridge.otaOnDownloaded(apk.getAbsolutePath(), ""); // re-confirm
            }
        } catch (Exception e) {
            Log.e(TAG, "PackageInstaller failed: " + e.getMessage());
            RustBridge.otaOnFailed(e.getMessage());
            // Last resort: intent fallback
            handler.post(() -> installViaIntent(apk, tag));
        }
    }

    /** Legacy fallback: ACTION_VIEW — opens system installer dialog. */
    private void installViaIntent(File apk, String tag) {
        handler.post(() -> {
            try {
                Uri uri;
                if (android.os.Build.VERSION.SDK_INT >= 24) {
                    uri = androidx.core.content.FileProvider.getUriForFile(
                        ctx, ctx.getPackageName() + ".provider", apk);
                } else {
                    uri = Uri.fromFile(apk);
                }
                Intent intent = new Intent(Intent.ACTION_VIEW)
                    .setDataAndType(uri, "application/vnd.android.package-archive")
                    .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                            | Intent.FLAG_ACTIVITY_NEW_TASK);
                ctx.startActivity(intent);
                Log.i(TAG, "Intent install launched for " + apk.getAbsolutePath());
            } catch (Exception e) {
                Log.e(TAG, "Intent install failed: " + e.getMessage());
                RustBridge.otaOnFailed("intent_failed: " + e.getMessage());
                notifyError(e.getMessage());
            }
        });
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    private String getInstalledVersion() {
        try {
            return ctx.getPackageManager()
                .getPackageInfo(ctx.getPackageName(), 0).versionName;
        } catch (Exception e) { return "unknown"; }
    }

    private static String sha256File(File f) {
        try {
            MessageDigest md = MessageDigest.getInstance("SHA-256");
            try (FileInputStream fis = new FileInputStream(f)) {
                byte[] buf = new byte[65536];
                int n;
                while ((n = fis.read(buf)) != -1) md.update(buf, 0, n);
            }
            StringBuilder sb = new StringBuilder();
            for (byte b : md.digest()) sb.append(String.format("%02x", b));
            return sb.toString();
        } catch (Exception e) { return ""; }
    }

    private String httpGet(String url) {
        try {
            HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(10_000);
            c.setReadTimeout(15_000);
            c.setRequestProperty("Accept", "application/vnd.github+json");
            c.setRequestProperty("X-GitHub-Api-Version", "2022-11-28");
            c.setRequestProperty("User-Agent", "KiraOTA/43");
            if (c.getResponseCode() != 200) return null;
            StringBuilder sb = new StringBuilder();
            java.io.BufferedReader br = new java.io.BufferedReader(
                new java.io.InputStreamReader(c.getInputStream()));
            String line;
            while ((line = br.readLine()) != null) sb.append(line);
            return sb.toString();
        } catch (Exception e) {
            Log.w(TAG, "httpGet failed: " + e.getMessage());
            return null;
        }
    }

    private void notifyError(String msg) {
        RustBridge.otaOnFailed(msg != null ? msg : "unknown error");
        if (callback != null)
            handler.post(() -> callback.onError(msg != null ? msg : "Unknown error"));
    }

    private void sendUpdateNotification(String tag, String url, String notes) {
        try {
            android.app.NotificationManager nm =
                (android.app.NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            String chId = "kira_ota";
            if (android.os.Build.VERSION.SDK_INT >= 26) {
                nm.createNotificationChannel(new android.app.NotificationChannel(
                    chId, "Kira Updates",
                    android.app.NotificationManager.IMPORTANCE_DEFAULT));
            }
            Intent dlIntent = new Intent(ctx, OtaInstallReceiver.class)
                .setAction("com.kira.ota.DOWNLOAD")
                .putExtra("tag", tag).putExtra("url", url);
            android.app.PendingIntent dlPi = android.app.PendingIntent.getBroadcast(
                ctx, 0, dlIntent,
                android.app.PendingIntent.FLAG_UPDATE_CURRENT
                | android.app.PendingIntent.FLAG_IMMUTABLE);

            Intent skipIntent = new Intent(ctx, OtaInstallReceiver.class)
                .setAction("com.kira.ota.SKIP")
                .putExtra("tag", tag);
            android.app.PendingIntent skipPi = android.app.PendingIntent.getBroadcast(
                ctx, 1, skipIntent,
                android.app.PendingIntent.FLAG_UPDATE_CURRENT
                | android.app.PendingIntent.FLAG_IMMUTABLE);

            String snippet = notes != null && !notes.isEmpty()
                ? notes.substring(0, Math.min(120, notes.length())) : "New version ready";

            android.app.Notification notif = new android.app.Notification.Builder(ctx, chId)
                .setSmallIcon(android.R.drawable.stat_sys_download_done)
                .setContentTitle("Kira update: " + tag)
                .setContentText(snippet)
                .setStyle(new android.app.Notification.BigTextStyle().bigText(snippet))
                .addAction(android.R.drawable.ic_menu_upload, "Install", dlPi)
                .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Skip", skipPi)
                .setAutoCancel(true)
                .build();
            nm.notify(9901, notif);
        } catch (Exception e) {
            Log.w(TAG, "Notify failed: " + e.getMessage());
        }
    }

    private void sendSuccessNotification(String tag) {
        try {
            android.app.NotificationManager nm =
                (android.app.NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            android.app.Notification notif = new android.app.Notification.Builder(ctx, "kira_ota")
                .setSmallIcon(android.R.drawable.stat_sys_download_done)
                .setContentTitle("Kira updated to " + tag)
                .setContentText("Update installed successfully. Restart Kira to apply changes.")
                .setAutoCancel(true)
                .build();
            nm.notify(9902, notif);
        } catch (Exception ignored) {}
    }

    // ─── Receivers ────────────────────────────────────────────────────────────

    /**
     * Handles:
     *  - PackageInstaller session status callbacks
     *  - Notification action taps (DOWNLOAD / SKIP)
     */
    public static class OtaInstallReceiver extends android.content.BroadcastReceiver {
        @Override
        public void onReceive(Context ctx, Intent intent) {
            String action = intent.getAction();

            // SECURITY: Notification action: user tapped "Install"
            // Validate URL is a GitHub releases URL before downloading
            if ("com.kira.ota.DOWNLOAD".equals(action)) {
                String tag = intent.getStringExtra("tag");
                String url = intent.getStringExtra("url");
                if (url != null && isValidGithubReleaseUrl(url)) {
                    new KiraOtaUpdater(ctx).startDownload(url, tag);
                } else {
                    Log.w("KiraOTA", "Blocked invalid download URL: " + url);
                }
                return;
            }
            // Notification action: user tapped "Skip"
            if ("com.kira.ota.SKIP".equals(action)) {
                String tag = intent.getStringExtra("tag");
                if (tag != null) RustBridge.otaSkip(tag);
                return;
            }

            // PackageInstaller session result
            int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS,
                PackageInstaller.STATUS_FAILURE);
            String msg = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);
            String tag = intent.getStringExtra("tag");
            String apkPath = intent.getStringExtra("apk_path");

            Log.i("KiraOTA-Receiver", "PackageInstaller status=" + status + " msg=" + msg);

            switch (status) {
                case PackageInstaller.STATUS_SUCCESS:
                    String ver;
                    try {
                        ver = ctx.getPackageManager()
                            .getPackageInfo(ctx.getPackageName(), 0).versionName;
                    } catch (PackageManager.NameNotFoundException e) { ver = tag; }
                    RustBridge.otaOnInstalled(ver);
                    // Clean up APK
                    if (apkPath != null) new File(apkPath).delete();
                    break;

                case PackageInstaller.STATUS_PENDING_USER_ACTION:
                    // Need user confirmation — launch the confirm activity
                    Intent confirm = (Intent) intent.getParcelableExtra(Intent.EXTRA_INTENT);
                    if (confirm != null) {
                        confirm.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                        ctx.startActivity(confirm);
                    }
                    break;

                default:
                    String err = "PackageInstaller status " + status
                        + (msg != null ? ": " + msg : "");
                    RustBridge.otaOnFailed(err);
                    // Try intent fallback
                    if (apkPath != null) {
                        File apk = new File(apkPath);
                        if (apk.exists()) new KiraOtaUpdater(ctx).installViaIntent(apk, tag);
                    }
            }
        }

        /** SECURITY: Only allow GitHub release download URLs */
        private static boolean isValidGithubReleaseUrl(String url) {
            if (url == null || url.isEmpty()) return false;
            return url.startsWith("https://github.com/") ||
                   url.startsWith("https://objects.githubusercontent.com/") ||
                   url.startsWith("https://releases.github.com/");
        }
    }
}