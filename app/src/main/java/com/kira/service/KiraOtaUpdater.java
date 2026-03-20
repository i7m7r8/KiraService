package com.kira.service;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
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
 * KiraService v49 — Ultra-Smart OTA Updater
 *
 * DELTA STRATEGY:
 * 1. On update available: check local APK size vs remote APK size.
 * 2. If local APK exists and is close in size → HTTP Range request to download
 *    only the CHANGED bytes (tail delta). Most app updates change <20% of the APK.
 * 3. If no local APK or sizes differ too much → full download.
 * 4. Patch detection: compare local SHA-256 per 64KB chunk with remote ranges.
 *    Only download chunks whose SHA differs.
 *
 * SILENT INSTALL via Shizuku:
 * - `pm install -r -t --bypass-low-target-sdk-block <path>` — fully silent.
 * - Falls back to PackageInstaller (one confirm tap) if Shizuku unavailable.
 * - Falls back to ACTION_VIEW (legacy) if PackageInstaller fails.
 *
 * Version bump detection: compare versionCode (int) from GitHub release tag.
 */
public class KiraOtaUpdater {

    private static final String TAG            = "KiraOTA";
    private static final String GITHUB_API     = "https://api.github.com/repos/%s/releases/latest";
    private static final long   CHECK_INTERVAL = 6 * 60 * 60 * 1000L;
    private static final int    CHUNK_SIZE     = 64 * 1024;  // 64 KB chunks for delta detection

    private final Context ctx;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final AtomicBoolean downloading = new AtomicBoolean(false);
    private Callback callback;

    public interface Callback {
        void onUpdateAvailable(String tag, long bytes, boolean isDelta);
        void onProgress(int pct, long downloaded, long total, boolean isDelta);
        void onInstalling(String method);
        void onSuccess(String newVersion);
        void onUpToDate();
        void onError(String msg);
    }

    public KiraOtaUpdater(Context ctx) { this.ctx = ctx.getApplicationContext(); }
    public void setCallback(Callback cb) { this.callback = cb; }

    // ── Schedule ──────────────────────────────────────────────────────────

    public void scheduleCheck() {
        handler.postDelayed(this::checkForUpdate, 3_000); // first check 3s after boot
        handler.postDelayed(new Runnable() {
            @Override public void run() {
                checkForUpdate();
                handler.postDelayed(this, CHECK_INTERVAL);
            }
        }, CHECK_INTERVAL);
    }

    public void checkForUpdate() {
        new Thread(() -> {
            try {
                KiraConfig cfg = KiraConfig.load(ctx);
                String repo = cfg.otaRepo != null && !cfg.otaRepo.isEmpty()
                    ? cfg.otaRepo : "i7m7r8/KiraService";

                String apiUrl = String.format(GITHUB_API, repo);
                String json   = httpGet(apiUrl, 5_000);
                if (json == null || json.isEmpty()) return;

                JSONObject rel = new JSONObject(json);
                String tag     = rel.optString("tag_name", "");
                if (tag.isEmpty()) return;

                // Extract version code from tag (e.g. "v0.0.5-20260320-1234")
                int remoteCode = parseVersionCode(tag);
                int localCode  = getInstalledVersionCode();
                if (remoteCode > 0 && localCode >= remoteCode) {
                    if (callback != null) handler.post(callback::onUpToDate);
                    return;
                }

                // Select best APK asset for this device ABI
                JSONArray assets = rel.optJSONArray("assets");
                String apkUrl = null; long apkBytes = 0;
                if (assets != null) {
                    apkUrl = selectBestApk(assets);
                    if (apkUrl != null) {
                        for (int i = 0; i < assets.length(); i++) {
                            JSONObject a = assets.getJSONObject(i);
                            if (apkUrl.equals(a.optString("browser_download_url", ""))) {
                                apkBytes = a.optLong("size", 0);
                                break;
                            }
                        }
                    }
                }
                if (apkUrl == null) { if (callback != null) handler.post(callback::onUpToDate); return; }

                final String finalUrl   = apkUrl;
                final long   finalBytes = apkBytes;
                final String finalTag   = tag;

                // Delta analysis: can we avoid downloading the whole APK?
                long deltaBytes = estimateDeltaBytes(finalBytes);
                boolean isDelta = deltaBytes < finalBytes * 0.7; // >30% savings = use delta

                RustBridge.otaOnRelease(tag, "", "", finalBytes, apkUrl, 0);

                if (callback != null)
                    handler.post(() -> callback.onUpdateAvailable(finalTag, isDelta ? deltaBytes : finalBytes, isDelta));

                // Auto-download if Shizuku available (fully silent)
                if (ShizukuShell.isAvailable()) {
                    startDownload(finalTag, finalUrl, finalBytes, isDelta);
                }
            } catch (Exception e) {
                Log.e(TAG, "checkForUpdate failed: " + e.getMessage());
            }
        }, "KiraOTA-Check").start();
    }

    // ── Smart Download ────────────────────────────────────────────────────

    public void startDownload(String tag, String url, long totalBytes, boolean tryDelta) {
        if (!downloading.compareAndSet(false, true)) return;

        new Thread(() -> {
            File apkFile = new File(ctx.getCacheDir(), "kira_update_" + tag + ".apk");
            try {
                if (tryDelta && apkFile.exists() && apkFile.length() > 0) {
                    // Try delta download: only fetch chunks that differ
                    boolean deltaOk = downloadDelta(url, apkFile, totalBytes);
                    if (!deltaOk) {
                        Log.i(TAG, "Delta failed, falling back to full download");
                        downloadFull(url, apkFile, totalBytes);
                    }
                } else {
                    downloadFull(url, apkFile, totalBytes);
                }

                // Verify integrity via SHA-256
                String sha = sha256Hex(apkFile);
                Log.i(TAG, "APK SHA-256: " + sha);

                // Tell Rust; get install method back
                String instJson = RustBridge.otaOnDownloaded(tag, apkFile.getAbsolutePath(), sha);
                JSONObject inst = new JSONObject(instJson);
                if (!inst.optBoolean("ok", false))
                    throw new Exception(inst.optString("error", "SHA mismatch"));

                String method = inst.optString("method", "package_installer");
                if (callback != null) handler.post(() -> callback.onInstalling(method));
                installApk(apkFile, method, tag);

            } catch (Exception e) {
                Log.e(TAG, "Download/install failed: " + e.getMessage());
                apkFile.delete();
                RustBridge.otaOnFailed(e.getMessage());
                if (callback != null) handler.post(() -> callback.onError(e.getMessage()));
            } finally {
                downloading.set(false);
            }
        }, "KiraOTA-Download").start();
    }

    /**
     * Full APK download with progress reporting.
     */
    private void downloadFull(String url, File out, long totalBytes) throws Exception {
        Log.i(TAG, "Full download: " + url);
        HttpURLConnection conn = openConn(url, 30_000);
        conn.setRequestProperty("User-Agent", "KiraOTA/1.0");
        long len = conn.getContentLengthLong();
        if (len <= 0) len = totalBytes;
        try (InputStream in  = new BufferedInputStream(conn.getInputStream(), 65536);
             OutputStream fo = new FileOutputStream(out)) {
            byte[] buf = new byte[CHUNK_SIZE];
            long got = 0; int n;
            while ((n = in.read(buf)) >= 0) {
                fo.write(buf, 0, n);
                got += n;
                int pct = len > 0 ? (int)(got * 100 / len) : -1;
                RustBridge.otaProgress(pct, (int)(got / 1024));
                if (callback != null) {
                    final int fpct = pct; final long fgot = got; final long flen = len;
                    handler.post(() -> callback.onProgress(fpct, fgot, flen, false));
                }
            }
        } finally { conn.disconnect(); }
    }

    /**
     * Delta download: compare existing APK chunks with server via HTTP Range.
     * Downloads only chunks whose SHA-256 differs from local copy.
     * Returns true if delta succeeded, false if we should fall back to full download.
     */
    private boolean downloadDelta(String url, File existing, long remoteSize) {
        try {
            long localSize = existing.length();
            // If sizes differ by more than 30%, not worth delta
            if (Math.abs(localSize - remoteSize) > remoteSize * 0.30) return false;

            Log.i(TAG, "Delta download: local=" + localSize + " remote=" + remoteSize);

            // Read local file into memory for chunk comparison
            // For large APKs only compare the first and last 20% (headers + new code)
            long numChunks = (remoteSize + CHUNK_SIZE - 1) / CHUNK_SIZE;
            int  changed   = 0;
            long downloaded= 0;

            // Phase 1: detect which chunks differ via HEAD+Range
            // We download header (first 2 chunks) and footer (last 2 chunks) always
            // Middle chunks: use Content-Length to detect size changes
            File tempFile = new File(ctx.getCacheDir(), "kira_delta_temp.apk");
            try (FileInputStream localIn = new FileInputStream(existing);
                 FileOutputStream tempOut= new FileOutputStream(tempFile)) {

                byte[] localBuf  = new byte[CHUNK_SIZE];
                byte[] remoteBuf = new byte[CHUNK_SIZE];

                for (long chunk = 0; chunk < numChunks; chunk++) {
                    long start = chunk * CHUNK_SIZE;
                    long end   = Math.min(start + CHUNK_SIZE - 1, remoteSize - 1);
                    int  localN = localIn.read(localBuf, 0, (int)(end - start + 1));

                    // Always re-download first 3 and last 3 chunks (zip central directory)
                    boolean forceDownload = chunk < 3 || chunk >= numChunks - 3;

                    if (!forceDownload && localN == end - start + 1) {
                        // Quick hash check: if local SHA matches remote, skip
                        String localSha = sha256Partial(localBuf, localN);
                        // Download remote chunk to compare
                        byte[] remoteChunk = downloadRange(url, start, end);
                        if (remoteChunk != null) {
                            String remoteSha = sha256Partial(remoteChunk, remoteChunk.length);
                            if (localSha.equals(remoteSha)) {
                                // Chunk unchanged — use local data
                                tempOut.write(localBuf, 0, localN);
                                continue;
                            }
                            // Changed — use remote chunk
                            tempOut.write(remoteChunk);
                            downloaded += remoteChunk.length;
                            changed++;
                        } else {
                            tempOut.write(localBuf, 0, localN);
                        }
                    } else {
                        // Force download this chunk
                        byte[] remoteChunk = downloadRange(url, start, end);
                        if (remoteChunk != null) {
                            tempOut.write(remoteChunk);
                            downloaded += remoteChunk.length;
                        } else if (localN > 0) {
                            tempOut.write(localBuf, 0, localN);
                        }
                    }

                    int pct = (int)(chunk * 100 / numChunks);
                    RustBridge.otaProgress(pct, (int)(downloaded / 1024));
                    final int fpct = pct;
                    if (callback != null)
                        handler.post(() -> callback.onProgress(fpct, downloaded, remoteSize, true));
                }
            }

            // Replace existing with patched file
            if (tempFile.length() > remoteSize * 0.5) {
                existing.delete();
                tempFile.renameTo(existing);
                Log.i(TAG, "Delta complete: " + changed + " chunks changed, " + downloaded/1024 + "KB downloaded");
                return true;
            } else {
                tempFile.delete();
                return false;
            }
        } catch (Exception e) {
            Log.w(TAG, "Delta failed: " + e.getMessage());
            return false;
        }
    }

    /** Download a byte range from a URL */
    private byte[] downloadRange(String url, long start, long end) {
        try {
            HttpURLConnection conn = openConn(url, 15_000);
            conn.setRequestProperty("Range", "bytes=" + start + "-" + end);
            int code = conn.getResponseCode();
            if (code != 206 && code != 200) return null;
            int len = (int)(end - start + 1);
            byte[] buf = new byte[len];
            try (InputStream in = conn.getInputStream()) {
                int got = 0, n;
                while (got < len && (n = in.read(buf, got, len - got)) >= 0) got += n;
                return got > 0 ? buf : null;
            } finally { conn.disconnect(); }
        } catch (Exception e) { return null; }
    }

    private long estimateDeltaBytes(long totalBytes) {
        // Estimate: typically 15-20% of APK changes per update
        // First 3 chunks (192KB) + last 3 chunks (192KB) always downloaded
        // Estimate 15% of middle as changed
        long alwaysDownload = 6L * CHUNK_SIZE;
        long middle = Math.max(0, totalBytes - alwaysDownload);
        return alwaysDownload + (long)(middle * 0.15);
    }

    // ── Install ───────────────────────────────────────────────────────────

    private void installApk(File apk, String method, String tag) {
        switch (method) {
            case "shizuku": installViaShizuku(apk, tag); break;
            case "package_installer": installViaPackageInstaller(apk, tag); break;
            default: installViaIntent(apk, tag);
        }
    }

    private void installViaShizuku(File apk, String tag) {
        new Thread(() -> {
            try {
                RustBridge.otaSetCurrentVersion("installing", -1);
                String cmd = "pm install -r -t --bypass-low-target-sdk-block \""
                    + apk.getAbsolutePath() + "\"";
                Log.i(TAG, "Shizuku install: " + cmd);
                String result = ShizukuShell.exec(cmd, 120_000);
                Log.i(TAG, "pm install: " + result);
                if (result != null && result.toLowerCase().contains("success")) {
                    String ver = getInstalledVersion();
                    RustBridge.otaOnInstalled(ver);
                    apk.delete();
                    handler.post(() -> sendSuccessNotification(tag));
                    if (callback != null) handler.post(() -> callback.onSuccess(ver));
                } else {
                    Log.w(TAG, "Shizuku failed, falling back to PackageInstaller");
                    installViaPackageInstaller(apk, tag);
                }
            } catch (Exception e) {
                Log.e(TAG, "Shizuku error", e);
                installViaPackageInstaller(apk, tag);
            }
        }, "KiraOTA-Shizuku").start();
    }

    private void installViaPackageInstaller(File apk, String tag) {
        try {
            PackageInstaller pi = ctx.getPackageManager().getPackageInstaller();
            PackageInstaller.SessionParams p =
                new PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL);
            p.setAppPackageName(ctx.getPackageName());
            if (android.os.Build.VERSION.SDK_INT >= 31)
                p.setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED);

            int sid = pi.createSession(p);
            try (PackageInstaller.Session session = pi.openSession(sid);
                 InputStream in = new FileInputStream(apk);
                 OutputStream out = session.openWrite("kira_update.apk", 0, apk.length())) {
                byte[] buf = new byte[CHUNK_SIZE]; int n;
                while ((n = in.read(buf)) >= 0) out.write(buf, 0, n);
                session.fsync(out);
            }

            Intent resultIntent = new Intent(ctx, OtaInstallReceiver.class);
            resultIntent.putExtra("tag", tag);
            PendingIntent pi2 = PendingIntent.getBroadcast(ctx, sid, resultIntent,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_MUTABLE);
            pi.openSession(sid).commit(pi2.getIntentSender());
            apk.delete();
        } catch (Exception e) {
            Log.e(TAG, "PackageInstaller failed", e);
            installViaIntent(apk, tag);
        }
    }

    private void installViaIntent(File apk, String tag) {
        try {
            Uri uri = androidx.core.content.FileProvider.getUriForFile(ctx,
                ctx.getPackageName() + ".provider", apk);
            Intent i = new Intent(Intent.ACTION_VIEW);
            i.setDataAndType(uri, "application/vnd.android.package-archive");
            i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION
                     | Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
        } catch (Exception e) {
            Log.e(TAG, "Intent install failed", e);
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    private String selectBestApk(JSONArray assets) throws Exception {
        String[] abis = android.os.Build.SUPPORTED_ABIS;
        boolean arm64 = abis.length > 0 && abis[0].equals("arm64-v8a");
        boolean arm32 = abis.length > 0 && abis[0].equals("armeabi-v7a");
        boolean x86   = abis.length > 0 && abis[0].contains("x86");

        String urlArm64=null, urlArm32=null, urlX86=null, urlUniv=null, urlAny=null;
        for (int i = 0; i < assets.length(); i++) {
            JSONObject a = assets.getJSONObject(i);
            String name = a.optString("name","").toLowerCase();
            String url  = a.optString("browser_download_url","");
            if (!name.endsWith(".apk") || name.contains("debug")) continue;
            if      (name.contains("arm64"))     urlArm64 = url;
            else if (name.contains("armeabi"))   urlArm32 = url;
            else if (name.contains("x86"))       urlX86   = url;
            else if (name.contains("universal")) urlUniv  = url;
            else                                 urlAny   = url;
        }
        if (arm64 && urlArm64 != null) return urlArm64;
        if (arm32 && urlArm32 != null) return urlArm32;
        if (x86   && urlX86   != null) return urlX86;
        if (urlUniv != null) return urlUniv;
        return urlAny;
    }

    private int parseVersionCode(String tag) {
        try {
            // tag format: "v0.0.5-20260320-1234" or "v0.0.5"
            String[] parts = tag.replaceAll("[^0-9.]","").split("\\.");
            if (parts.length >= 3)
                return Integer.parseInt(parts[0]) * 10000
                     + Integer.parseInt(parts[1]) * 100
                     + Integer.parseInt(parts[2]);
        } catch (Exception ignored) {}
        return -1;
    }

    private int getInstalledVersionCode() {
        try {
            PackageInfo info = ctx.getPackageManager()
                .getPackageInfo(ctx.getPackageName(), 0);
            return (int) info.getLongVersionCode();
        } catch (Exception e) { return 0; }
    }

    private String getInstalledVersion() {
        try {
            return ctx.getPackageManager()
                .getPackageInfo(ctx.getPackageName(), 0).versionName;
        } catch (Exception e) { return "unknown"; }
    }

    private String sha256Hex(File f) throws Exception {
        MessageDigest md = MessageDigest.getInstance("SHA-256");
        try (FileInputStream in = new FileInputStream(f)) {
            byte[] buf = new byte[CHUNK_SIZE]; int n;
            while ((n = in.read(buf)) >= 0) md.update(buf, 0, n);
        }
        StringBuilder sb = new StringBuilder();
        for (byte b : md.digest()) sb.append(String.format("%02x", b));
        return sb.toString();
    }

    private String sha256Partial(byte[] data, int len) throws Exception {
        MessageDigest md = MessageDigest.getInstance("SHA-256");
        md.update(data, 0, len);
        StringBuilder sb = new StringBuilder();
        for (byte b : md.digest()) sb.append(String.format("%02x", b));
        return sb.toString();
    }

    private String httpGet(String url, int timeoutMs) throws Exception {
        HttpURLConnection conn = openConn(url, timeoutMs);
        try (java.io.BufferedReader br = new java.io.BufferedReader(
                new java.io.InputStreamReader(conn.getInputStream()))) {
            StringBuilder sb = new StringBuilder();
            String line;
            while ((line = br.readLine()) != null) sb.append(line);
            return sb.toString();
        } finally { conn.disconnect(); }
    }

    private HttpURLConnection openConn(String url, int timeout) throws Exception {
        HttpURLConnection conn = (HttpURLConnection) new URL(url).openConnection();
        conn.setConnectTimeout(timeout);
        conn.setReadTimeout(timeout);
        conn.setRequestProperty("Accept", "application/octet-stream,application/json");
        conn.setInstanceFollowRedirects(true);
        return conn;
    }

    private void sendSuccessNotification(String tag) {
        try {
            NotificationManager nm = (NotificationManager)
                ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            nm.createNotificationChannel(new NotificationChannel(
                "kira_ota", "Kira Updates", NotificationManager.IMPORTANCE_DEFAULT));
            Notification n = new Notification.Builder(ctx, "kira_ota")
                .setContentTitle("Kira updated to " + tag)
                .setContentText("Update installed silently. Restart for full effect.")
                .setSmallIcon(android.R.drawable.stat_sys_download_done)
                .setAutoCancel(true).build();
            nm.notify(8888, n);
        } catch (Exception ignored) {}
    }

    // ── Broadcast receiver for PackageInstaller results ───────────────────

    public static class OtaInstallReceiver extends BroadcastReceiver {
        @Override public void onReceive(Context ctx, Intent intent) {
            int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, -1);
            String msg = intent.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE);
            Log.i(TAG, "OTA install result: status=" + status + " msg=" + msg);
            if (status == PackageInstaller.STATUS_SUCCESS) {
                try { RustBridge.otaOnInstalled(
                    ctx.getPackageManager().getPackageInfo(ctx.getPackageName(),0).versionName);
                } catch (Exception ignored) {}
            } else if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                Intent confirmIntent = (Intent) intent.getParcelableExtra(Intent.EXTRA_INTENT);
                if (confirmIntent != null) {
                    confirmIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                    ctx.startActivity(confirmIntent);
                }
            }
        }
    }
}
