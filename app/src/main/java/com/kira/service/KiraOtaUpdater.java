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
 * Delta strategy: HTTP Range requests per 64KB chunk, only download changed chunks.
 * Silent install: Shizuku → PackageInstaller → ACTION_VIEW fallback.
 */
public class KiraOtaUpdater {

    private static final String TAG            = "KiraOTA";
    private static final String GITHUB_API     = "https://api.github.com/repos/%s/releases/latest";
    private static final long   CHECK_INTERVAL = 6 * 60 * 60 * 1000L;
    private static final int    CHUNK_SIZE     = 64 * 1024;

    private final Context ctx;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final AtomicBoolean downloading = new AtomicBoolean(false);
    private OtaCallback callback;

    // ── Callback interface — matches MainActivity.initOta() exactly ────────
    public interface OtaCallback {
        void onCheckStart();
        void onUpdateAvailable(String ver, String log, Runnable onInstall, Runnable onSkip);
        void onProgress(int pct, long done, long total);
        void onInstalling(String method);
        void onSuccess(String ver);
        void onError(String msg);
        void onUpToDate();
    }

    public KiraOtaUpdater(Context ctx) { this.ctx = ctx.getApplicationContext(); }

    /** Called by MainActivity before scheduleChecks — registers current version with Rust */
    public void init() {
        new Thread(() -> {
            try {
                RustBridge.otaSetCurrentVersion(getInstalledVersion(), getInstalledVersionCode());
                KiraConfig cfg = KiraConfig.load(ctx);
                RustBridge.otaSetRepo(cfg.otaRepo != null && !cfg.otaRepo.isEmpty()
                    ? cfg.otaRepo : "i7m7r8/KiraService");
            } catch (Throwable ignored) {}
        }).start();
    }

    public void setCallback(OtaCallback cb) { this.callback = cb; }

    // ── Schedule ───────────────────────────────────────────────────────────

    public void scheduleChecks() {
        handler.postDelayed(this::checkForUpdate, 3_000);
        handler.postDelayed(new Runnable() {
            @Override public void run() {
                checkForUpdate();
                handler.postDelayed(this, CHECK_INTERVAL);
            }
        }, CHECK_INTERVAL);
    }

    // ── Check ──────────────────────────────────────────────────────────────

    public void checkForUpdate() {
        if (callback != null) handler.post(callback::onCheckStart);
        new Thread(() -> {
            try {
                KiraConfig cfg = KiraConfig.load(ctx);
                String repo = cfg.otaRepo != null && !cfg.otaRepo.isEmpty()
                    ? cfg.otaRepo : "i7m7r8/KiraService";
                String json = httpGet(String.format(GITHUB_API, repo), 5_000);
                if (json == null || json.isEmpty()) return;

                JSONObject rel  = new JSONObject(json);
                String tag      = rel.optString("tag_name", "");
                if (tag.isEmpty()) return;

                int remoteVer = parseTagVersion(tag);
                int localVer  = parseInstalledVersion();
                String installedTag = getInstalledTag();
                Log.i(TAG, "OTA check: local=" + localVer + " remote=" + remoteVer
                    + " tag=" + tag + " installedTag=" + installedTag);
                // Up to date if: same tag OR remote version not newer
                boolean sameTag = !installedTag.isEmpty() && installedTag.equals(tag);
                boolean notNewer = remoteVer > 0 && localVer >= remoteVer;
                if (sameTag || notNewer) {
                    if (callback != null) handler.post(callback::onUpToDate);
                    return;
                }

                JSONArray assets = rel.optJSONArray("assets");
                String apkUrl = null; long apkBytes = 0;
                if (assets != null) {
                    apkUrl = selectBestApk(assets);
                    if (apkUrl != null) {
                        for (int i = 0; i < assets.length(); i++) {
                            JSONObject a = assets.getJSONObject(i);
                            if (apkUrl.equals(a.optString("browser_download_url",""))) {
                                apkBytes = a.optLong("size", 0); break;
                            }
                        }
                    }
                }
                if (apkUrl == null) { if (callback != null) handler.post(callback::onUpToDate); return; }

                final String fUrl   = apkUrl;
                final long   fBytes = apkBytes;
                final String fTag   = tag;

                // Notify Rust
                RustBridge.otaOnRelease(tag, fUrl, "", "", "", fBytes);

                long deltaBytes = estimateDeltaBytes(fBytes);
                boolean isDelta = deltaBytes < fBytes * 0.7;

                if (callback != null) {
                    Runnable doInstall = () -> startDownload(fTag, fUrl, fBytes, isDelta);
                    Runnable doSkip    = () -> RustBridge.otaSkip(fTag);
                    handler.post(() -> callback.onUpdateAvailable(
                        fTag,
                        "v" + fTag + (isDelta ? "  (delta ~" + (deltaBytes/1024/1024) + "MB)" : "  (full ~" + (fBytes/1024/1024) + "MB)"),
                        doInstall, doSkip));
                }

                // Auto-install silently if Shizuku available
                if (ShizukuShell.isAvailable()) startDownload(fTag, fUrl, fBytes, isDelta);

            } catch (Exception e) {
                Log.e(TAG, "checkForUpdate: " + e.getMessage());
                if (callback != null) handler.post(() -> callback.onError(e.getMessage()));
            }
        }, "KiraOTA-Check").start();
    }

    // ── Smart Download ─────────────────────────────────────────────────────

    public void startDownload(String tag, String url, long totalBytes, boolean tryDelta) {
        if (!downloading.compareAndSet(false, true)) return;
        new Thread(() -> {
            File apk = new File(ctx.getCacheDir(), "kira_update.apk");
            try {
                // Delta baseline: use the installed APK (most accurate) or cached update
                File baseline = getInstalledApkBaseline();
                boolean hasSuitableBaseline = baseline != null
                    && baseline.length() > totalBytes * 0.4
                    && baseline.length() < totalBytes * 1.6;
                if (tryDelta && hasSuitableBaseline) {
                    // Copy baseline to apk location for delta patching
                    if (!baseline.getAbsolutePath().equals(apk.getAbsolutePath())) {
                        copyFile(baseline, apk);
                    }
                    Log.i(TAG, "Delta from baseline: " + baseline.length() + " → " + totalBytes);
                    if (!downloadDelta(url, apk, totalBytes)) {
                        Log.i(TAG, "Delta failed, full download");
                        downloadFull(url, apk, totalBytes);
                    }
                } else {
                    Log.i(TAG, "Full download (no suitable baseline)");
                    downloadFull(url, apk, totalBytes);
                }
                String sha = sha256Hex(apk);
                String instJson = RustBridge.otaOnDownloaded(apk.getAbsolutePath(), sha);
                JSONObject inst = new JSONObject(instJson);
                if (!inst.optBoolean("ok", false))
                    throw new Exception(inst.optString("error", "SHA mismatch"));
                String method = inst.optString("method", "package_installer");
                if (callback != null) handler.post(() -> callback.onInstalling(method));
                installApk(apk, method, tag);
            } catch (Exception e) {
                apk.delete();
                RustBridge.otaOnFailed(e.getMessage());
                if (callback != null) handler.post(() -> callback.onError(e.getMessage()));
            } finally {
                downloading.set(false);
            }
        }, "KiraOTA-Download").start();
    }

    private void downloadFull(String url, File out, long totalBytes) throws Exception {
        HttpURLConnection conn = openConn(url, 30_000);
        long len = conn.getContentLengthLong();
        if (len <= 0) len = totalBytes;
        try (InputStream in  = new BufferedInputStream(conn.getInputStream(), CHUNK_SIZE);
             OutputStream fo = new FileOutputStream(out)) {
            byte[] buf = new byte[CHUNK_SIZE]; long got = 0; int n;
            while ((n = in.read(buf)) >= 0) {
                fo.write(buf, 0, n); got += n;
                int pct = len > 0 ? (int)(got * 100 / len) : -1;
                RustBridge.otaProgress(got, len);
                final int fp=pct; final long fg=got, fl=len;
                if (callback != null) handler.post(() -> callback.onProgress(fp, fg, fl));
            }
        } finally { conn.disconnect(); }
    }

    private boolean downloadDelta(String url, File existing, long remoteSize) {
        try {
            long localSize = existing.length();
            if (Math.abs(localSize - remoteSize) > remoteSize * 0.30) return false;
            long numChunks = (remoteSize + CHUNK_SIZE - 1) / CHUNK_SIZE;
            long downloaded = 0;
            File tmp = new File(ctx.getCacheDir(), "kira_delta_tmp.apk");
            try (FileInputStream li = new FileInputStream(existing);
                 FileOutputStream to = new FileOutputStream(tmp)) {
                byte[] lb = new byte[CHUNK_SIZE];
                for (long c = 0; c < numChunks; c++) {
                    long start = c * CHUNK_SIZE;
                    long end   = Math.min(start + CHUNK_SIZE - 1, remoteSize - 1);
                    int  ln    = li.read(lb, 0, (int)(end - start + 1));
                    boolean force = c < 3 || c >= numChunks - 3;
                    if (!force && ln == end - start + 1) {
                        String ls = sha256Partial(lb, ln);
                        byte[] rb = downloadRange(url, start, end);
                        if (rb != null && ls.equals(sha256Partial(rb, rb.length))) {
                            to.write(lb, 0, ln); continue;
                        }
                        if (rb != null) { to.write(rb); downloaded += rb.length; continue; }
                    }
                    byte[] rb = downloadRange(url, start, end);
                    if (rb != null) { to.write(rb); downloaded += rb.length; }
                    else if (ln > 0) to.write(lb, 0, ln);
                    int pct = (int)(c * 100 / numChunks);
                    RustBridge.otaProgress(downloaded, remoteSize);
                    final int fp=pct; final long fd=downloaded, fs=remoteSize;
                    if (callback != null) handler.post(() -> callback.onProgress(fp, fd, fs));
                }
            }
            if (tmp.length() > remoteSize * 0.5) {
                existing.delete(); tmp.renameTo(existing); return true;
            }
            tmp.delete(); return false;
        } catch (Exception e) { Log.w(TAG, "Delta: " + e.getMessage()); return false; }
    }

    private byte[] downloadRange(String url, long start, long end) {
        try {
            HttpURLConnection c = openConn(url, 15_000);
            c.setRequestProperty("Range", "bytes=" + start + "-" + end);
            int code = c.getResponseCode();
            if (code != 206 && code != 200) return null;
            int len = (int)(end - start + 1);
            byte[] buf = new byte[len]; int got=0, n;
            try (InputStream in = c.getInputStream()) {
                while (got < len && (n = in.read(buf, got, len-got)) >= 0) got += n;
            } finally { c.disconnect(); }
            return got > 0 ? buf : null;
        } catch (Exception e) { return null; }
    }

    private long estimateDeltaBytes(long total) { return (long)(6L * CHUNK_SIZE + (total - 6L*CHUNK_SIZE) * 0.15); }

    // ── Install ────────────────────────────────────────────────────────────

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
                String result = ShizukuShell.exec(
                    "pm install -r -t --bypass-low-target-sdk-block \"" + apk.getAbsolutePath() + "\"", 120_000);
                if (result != null && result.toLowerCase().contains("success")) {
                    String ver = getInstalledVersion();
                    RustBridge.otaOnInstalled(ver);
                    saveInstalledTag(tag);
                    apk.delete();
                    handler.post(() -> sendSuccessNotification(tag));
                    if (callback != null) handler.post(() -> callback.onSuccess(ver));
                } else installViaPackageInstaller(apk, tag);
            } catch (Exception e) { installViaPackageInstaller(apk, tag); }
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
            try (PackageInstaller.Session s = pi.openSession(sid);
                 InputStream in = new FileInputStream(apk);
                 OutputStream out = s.openWrite("update.apk", 0, apk.length())) {
                byte[] buf = new byte[CHUNK_SIZE]; int n;
                while ((n = in.read(buf)) >= 0) out.write(buf, 0, n);
                s.fsync(out);
            }
            PendingIntent pi2 = PendingIntent.getBroadcast(ctx, sid,
                new Intent(ctx, OtaInstallReceiver.class).putExtra("tag", tag),
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_MUTABLE);
            pi.openSession(sid).commit(pi2.getIntentSender());
            apk.delete();
        } catch (Exception e) { installViaIntent(apk, tag); }
    }

    private void installViaIntent(File apk, String tag) {
        try {
            Uri uri = androidx.core.content.FileProvider.getUriForFile(ctx,
                ctx.getPackageName() + ".provider", apk);
            Intent i = new Intent(Intent.ACTION_VIEW);
            i.setDataAndType(uri, "application/vnd.android.package-archive");
            i.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_ACTIVITY_NEW_TASK);
            ctx.startActivity(i);
        } catch (Exception e) { Log.e(TAG, "Intent install: " + e); }
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    private String selectBestApk(JSONArray assets) throws Exception {
        String[] abis = android.os.Build.SUPPORTED_ABIS;
        boolean arm64 = abis.length > 0 && abis[0].equals("arm64-v8a");
        boolean arm32 = abis.length > 0 && abis[0].equals("armeabi-v7a");
        boolean x86   = abis.length > 0 && abis[0].contains("x86");
        String u64=null, u32=null, ux86=null, uUniv=null, uAny=null;
        for (int i = 0; i < assets.length(); i++) {
            JSONObject a = assets.getJSONObject(i);
            String name = a.optString("name","").toLowerCase();
            String url  = a.optString("browser_download_url","");
            if (!name.endsWith(".apk") || name.contains("debug")) continue;
            if      (name.contains("arm64"))     u64   = url;
            else if (name.contains("armeabi"))   u32   = url;
            else if (name.contains("x86"))       ux86  = url;
            else if (name.contains("universal")) uUniv = url;
            else                                 uAny  = url;
        }
        if (arm64 && u64   != null) return u64;
        if (arm32 && u32   != null) return u32;
        if (x86   && ux86  != null) return ux86;
        if (uUniv != null) return uUniv;
        return uAny;
    }

    /** Parse semver from tag like "v0.0.5-20260320" → int 5 (major*10000+minor*100+patch) */
    private int parseTagVersion(String tag) {
        try {
            // Extract "0.0.5" from "v0.0.5-20260320-1234"
            String s = tag.replaceFirst("^v","").split("-")[0];
            String[] p = s.split("\\.");
            if (p.length >= 3)
                return Integer.parseInt(p[0])*10000 + Integer.parseInt(p[1])*100 + Integer.parseInt(p[2]);
            if (p.length == 2)
                return Integer.parseInt(p[0])*10000 + Integer.parseInt(p[1])*100;
        } catch (Exception ignored) {}
        return -1;
    }

    /** Parse semver from installed versionName like "0.0.5" → int 5 */
    private int parseInstalledVersion() {
        try {
            String ver = ctx.getPackageManager().getPackageInfo(ctx.getPackageName(),0).versionName;
            String[] p = ver.split("\\.");
            if (p.length >= 3)
                return Integer.parseInt(p[0])*10000 + Integer.parseInt(p[1])*100 + Integer.parseInt(p[2]);
        } catch (Exception ignored) {}
        return 0;
    }

    private int getInstalledVersionCode() {
        try { return (int) ctx.getPackageManager().getPackageInfo(ctx.getPackageName(),0).getLongVersionCode(); }
        catch (Exception e) { return 0; }
    }

    private String getInstalledVersion() {
        try { return ctx.getPackageManager().getPackageInfo(ctx.getPackageName(),0).versionName; }
        catch (Exception e) { return "unknown"; }
    }

    /** Get the full build tag stored after last OTA install, or empty string */
    private String getInstalledTag() {
        return ctx.getSharedPreferences("kira_ota", android.content.Context.MODE_PRIVATE)
            .getString("installed_tag", "");
    }

    private void saveInstalledTag(String tag) {
        ctx.getSharedPreferences("kira_ota", android.content.Context.MODE_PRIVATE)
            .edit().putString("installed_tag", tag).apply();
    }

    /** Get the installed APK as delta baseline. Returns null if inaccessible. */
    private File getInstalledApkBaseline() {
        try {
            android.content.pm.ApplicationInfo info =
                ctx.getPackageManager().getApplicationInfo(ctx.getPackageName(), 0);
            File base = new File(info.sourceDir);
            if (base.exists() && base.length() > 0) return base;
        } catch (Exception ignored) {}
        // Fallback: cached update from previous run
        File cached = new File(ctx.getCacheDir(), "kira_baseline.apk");
        return cached.exists() ? cached : null;
    }

    private void copyFile(File src, File dst) throws Exception {
        try (FileInputStream in = new FileInputStream(src);
             FileOutputStream out = new FileOutputStream(dst)) {
            byte[] buf = new byte[CHUNK_SIZE]; int n;
            while ((n = in.read(buf)) >= 0) out.write(buf, 0, n);
        }
    }

    private String sha256Hex(File f) throws Exception {
        MessageDigest md = MessageDigest.getInstance("SHA-256");
        try (FileInputStream in = new FileInputStream(f)) {
            byte[] b = new byte[CHUNK_SIZE]; int n;
            while ((n = in.read(b)) >= 0) md.update(b, 0, n);
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

    private String httpGet(String url, int timeout) throws Exception {
        HttpURLConnection conn = openConn(url, timeout);
        try (java.io.BufferedReader br = new java.io.BufferedReader(
                new java.io.InputStreamReader(conn.getInputStream()))) {
            StringBuilder sb = new StringBuilder(); String line;
            while ((line = br.readLine()) != null) sb.append(line);
            return sb.toString();
        } finally { conn.disconnect(); }
    }

    private HttpURLConnection openConn(String url, int timeout) throws Exception {
        HttpURLConnection c = (HttpURLConnection) new URL(url).openConnection();
        c.setConnectTimeout(timeout); c.setReadTimeout(timeout);
        c.setRequestProperty("User-Agent", "KiraOTA/1.0");
        c.setInstanceFollowRedirects(true);
        return c;
    }

    private void sendSuccessNotification(String tag) {
        try {
            NotificationManager nm = (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
            nm.createNotificationChannel(new NotificationChannel("kira_ota","Kira Updates",NotificationManager.IMPORTANCE_DEFAULT));
            nm.notify(8888, new Notification.Builder(ctx,"kira_ota")
                .setContentTitle("Kira updated to " + tag)
                .setContentText("Installed silently. Restart for full effect.")
                .setSmallIcon(android.R.drawable.stat_sys_download_done)
                .setAutoCancel(true).build());
        } catch (Exception ignored) {}
    }

    // ── BroadcastReceiver ──────────────────────────────────────────────────

    public static class OtaInstallReceiver extends BroadcastReceiver {
        @Override public void onReceive(Context ctx, Intent intent) {
            int status = intent.getIntExtra(PackageInstaller.EXTRA_STATUS, -1);
            if (status == PackageInstaller.STATUS_SUCCESS) {
                try { RustBridge.otaOnInstalled(
                    ctx.getPackageManager().getPackageInfo(ctx.getPackageName(),0).versionName);
                } catch (Exception ignored) {}
            } else if (status == PackageInstaller.STATUS_PENDING_USER_ACTION) {
                Intent ci = (Intent) intent.getParcelableExtra(Intent.EXTRA_INTENT);
                if (ci != null) { ci.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK); ctx.startActivity(ci); }
            }
        }
    }
}
