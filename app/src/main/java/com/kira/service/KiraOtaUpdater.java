package com.kira.service;

import android.app.DownloadManager;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.net.Uri;
import android.os.Environment;
import android.os.Handler;
import android.os.Looper;
import android.util.Log;

import java.net.URL;
import java.net.HttpURLConnection;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.File;

/**
 * Smart OTA updater - checks GitHub releases API for new APK.
 * Only downloads changed files (APK diff via version compare).
 * Rust engine tracks version state via /ota/* endpoints.
 *
 * Flow:
 *   1. checkForUpdate() -> hits GitHub API
 *   2. Compares tag with current versionName
 *   3. If newer: posts to Rust /ota/check (stores in state)
 *   4. Notifies user via system notification
 *   5. On user approval: DownloadManager fetches APK
 *   6. Prompts PackageInstaller (no root needed)
 */
public class KiraOtaUpdater {

    private static final String TAG         = "KiraOTA";
    private static final String GITHUB_API  = "https://api.github.com/repos/i7m7r8/KiraService/releases/latest";
    private static final String PREF_SKIP   = "ota_skip_version";
    private static final long   CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000L; // 6 hours

    private final Context ctx;
    private final Handler handler = new Handler(Looper.getMainLooper());

    public KiraOtaUpdater(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    /** Schedule periodic checks */
    public void scheduleChecks() {
        handler.postDelayed(this::checkForUpdate, 30_000); // first check 30s after boot
        handler.postDelayed(new Runnable() {
            @Override public void run() {
                checkForUpdate();
                handler.postDelayed(this, CHECK_INTERVAL_MS);
            }
        }, CHECK_INTERVAL_MS);
    }

    /** Check GitHub releases API on background thread */
    public void checkForUpdate() {
        new Thread(() -> {
            try {
                String currentVer = getCurrentVersion();
                String json       = httpGet(GITHUB_API);
                if (json == null || json.isEmpty()) return;

                String latestTag  = extractJson(json, "tag_name");
                String changelog  = extractJson(json, "body");
                String apkUrl     = extractApkUrl(json);

                if (latestTag == null || apkUrl == null) return;

                // Clean tag: "v20260318-1700" -> compare with current
                String latestClean = latestTag.replaceFirst("^v", "");

                // Post to Rust OTA state
                postRustOta(latestClean, apkUrl, changelog != null ? changelog : "");

                // Check if this is actually newer and not skipped
                String skip = ctx.getSharedPreferences("kira_ota", Context.MODE_PRIVATE)
                    .getString(PREF_SKIP, "");
                if (latestClean.equals(skip)) { Log.d(TAG, "Skipped: " + latestClean); return; }

                if (!latestClean.equals(currentVer) && isNewer(latestClean, currentVer)) {
                    Log.i(TAG, "Update available: " + currentVer + " -> " + latestClean);
                    handler.post(() -> notifyUpdate(latestTag, latestClean, apkUrl, changelog));
                }
            } catch (Exception e) {
                Log.w(TAG, "OTA check failed: " + e.getMessage());
            }
        }).start();
    }

    /** Trigger immediate download + install */
    public void downloadAndInstall(String tag, String apkUrl) {
        new Thread(() -> {
            try {
                Log.i(TAG, "Downloading: " + apkUrl);
                DownloadManager dm = (DownloadManager) ctx.getSystemService(Context.DOWNLOAD_SERVICE);
                if (dm == null) return;

                String filename = "kira_" + tag + ".apk";
                DownloadManager.Request req = new DownloadManager.Request(Uri.parse(apkUrl))
                    .setTitle("Kira Update " + tag)
                    .setDescription("Downloading new version...")
                    .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                    .setDestinationInExternalPublicDir(Environment.DIRECTORY_DOWNLOADS, filename)
                    .setMimeType("application/vnd.android.package-archive")
                    .setAllowedOverMetered(true)
                    .setAllowedOverRoaming(true);

                long dlId = dm.enqueue(req);
                Log.i(TAG, "Download enqueued: " + dlId);

                // Register receiver for completion
                BroadcastReceiver onComplete = new BroadcastReceiver() {
                    @Override public void onReceive(Context c, Intent i) {
                        long id = i.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1);
                        if (id == dlId) {
                            ctx.unregisterReceiver(this);
                            File apk = new File(
                                Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS),
                                filename);
                            handler.post(() -> promptInstall(apk));
                        }
                    }
                };
                ctx.registerReceiver(onComplete,
                    new IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE));
            } catch (Exception e) {
                Log.e(TAG, "Download failed: " + e.getMessage());
            }
        }).start();
    }

    private void promptInstall(File apk) {
        try {
            Intent intent = new Intent(Intent.ACTION_VIEW);
            intent.setDataAndType(
                androidx.core.content.FileProvider.getUriForFile(
                    ctx, ctx.getPackageName() + ".provider", apk),
                "application/vnd.android.package-archive");
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(intent);
        } catch (Exception e) {
            Log.e(TAG, "Install prompt failed: " + e.getMessage());
        }
    }

    private void notifyUpdate(String tag, String clean, String url, String notes) {
        try {
            android.app.NotificationManager nm =
                (android.app.NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            String chId = "kira_ota";
            if (android.os.Build.VERSION.SDK_INT >= 26) {
                nm.createNotificationChannel(new android.app.NotificationChannel(
                    chId, "Kira Updates", android.app.NotificationManager.IMPORTANCE_DEFAULT));
            }
            // Tap action -> start download
            Intent dlIntent = new Intent(ctx, OtaDownloadReceiver.class)
                .putExtra("tag", tag)
                .putExtra("url", url);
            android.app.PendingIntent dlPi = android.app.PendingIntent.getBroadcast(
                ctx, 0, dlIntent, android.app.PendingIntent.FLAG_UPDATE_CURRENT | android.app.PendingIntent.FLAG_IMMUTABLE);

            // Skip action
            Intent skipIntent = new Intent(ctx, OtaDownloadReceiver.class)
                .putExtra("skip", clean);
            android.app.PendingIntent skipPi = android.app.PendingIntent.getBroadcast(
                ctx, 1, skipIntent, android.app.PendingIntent.FLAG_UPDATE_CURRENT | android.app.PendingIntent.FLAG_IMMUTABLE);

            android.app.Notification notif = new android.app.Notification.Builder(ctx, chId)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle("Kira update available: " + tag)
                .setContentText(notes != null && !notes.isEmpty() ?
                    notes.substring(0, Math.min(100, notes.length())) : "New version ready")
                .addAction(android.R.drawable.ic_menu_upload, "Install", dlPi)
                .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Skip", skipPi)
                .setAutoCancel(true)
                .build();

            nm.notify(9901, notif);
        } catch (Exception e) {
            Log.w(TAG, "Notify failed: " + e.getMessage());
        }
    }

    private void postRustOta(String latest, String url, String changelog) {
        try {
            String safe = changelog.replace("\"","'").replace("\\","").replace("\n"," ").replace("\r","");
            if (safe.length() > 300) safe = safe.substring(0, 300);
            String body = "{\"latest\":\""+latest+"\",\"download_url\":\""+url+"\",\"changelog\":\""+safe+"\"}";
            new okhttp3.OkHttpClient().newCall(
                new okhttp3.Request.Builder()
                    .url("http://localhost:7070/ota/check")
                    .post(okhttp3.RequestBody.create(body, okhttp3.MediaType.parse("application/json")))
                    .build()
            ).execute();
        } catch (Exception ignored) {}
    }

    private String getCurrentVersion() {
        try {
            return ctx.getPackageManager()
                .getPackageInfo(ctx.getPackageName(), 0).versionName;
        } catch (Exception e) { return "0"; }
    }

    private String httpGet(String url) {
        try {
            HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(8000);
            c.setReadTimeout(10000);
            c.setRequestProperty("Accept", "application/vnd.github+json");
            c.setRequestProperty("X-GitHub-Api-Version", "2022-11-28");
            if (c.getResponseCode() != 200) return null;
            StringBuilder sb = new StringBuilder();
            BufferedReader br = new BufferedReader(new InputStreamReader(c.getInputStream()));
            String line; while ((line = br.readLine()) != null) sb.append(line);
            return sb.toString();
        } catch (Exception e) { return null; }
    }

    private String extractJson(String json, String key) {
        String needle = "\"" + key + "\":\"";
        int i = json.indexOf(needle);
        if (i < 0) return null;
        int start = i + needle.length();
        int end = json.indexOf('"', start);
        if (end < 0) return null;
        return json.substring(start, end);
    }

    private String extractApkUrl(String json) {
        // Find "browser_download_url" in assets array
        String needle = "\"browser_download_url\":\"";
        int i = json.indexOf(needle);
        if (i < 0) return null;
        int start = i + needle.length();
        int end = json.indexOf('"', start);
        if (end < 0) return null;
        String url = json.substring(start, end);
        return url.endsWith(".apk") ? url : null;
    }

    private boolean isNewer(String latest, String current) {
        // Compare timestamp-based tags: "20260318-1700" > "20260317-0900"
        String l = latest.replaceAll("[^0-9]", "");
        String c = current.replaceAll("[^0-9]", "");
        if (l.isEmpty() || c.isEmpty()) return !latest.equals(current);
        try {
            long lv = Long.parseLong(l.substring(0, Math.min(12, l.length())));
            long cv = Long.parseLong(c.substring(0, Math.min(12, c.length())));
            return lv > cv;
        } catch (Exception e) { return !latest.equals(current); }
    }

    /** BroadcastReceiver for OTA notification actions */
    public static class OtaDownloadReceiver extends BroadcastReceiver {
        @Override
        public void onReceive(Context ctx, Intent intent) {
            String skip = intent.getStringExtra("skip");
            if (skip != null) {
                ctx.getSharedPreferences("kira_ota", Context.MODE_PRIVATE)
                    .edit().putString(PREF_SKIP, skip).apply();
                return;
            }
            String tag = intent.getStringExtra("tag");
            String url = intent.getStringExtra("url");
            if (tag != null && url != null) {
                new KiraOtaUpdater(ctx).downloadAndInstall(tag, url);
            }
        }
    }
}
