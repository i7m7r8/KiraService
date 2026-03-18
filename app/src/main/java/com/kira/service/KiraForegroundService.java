package com.kira.service;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.IBinder;

/**
 * Persistent foreground service -- keeps Kira alive forever.
 * Without this, Android kills the process after open_app or any background work.
 */
public class KiraForegroundService extends Service {

    private static final String CHANNEL_ID = "kira_persistent";
    private static final int NOTIF_ID = 1337;

    public static void start(Context ctx) {
        Intent intent = new Intent(ctx, KiraForegroundService.class);
        ctx.startForegroundService(intent);
    }

    @Override
    public int onStartCommand(Intent intent, int flags, int startId) {
        createChannel();
        startForeground(NOTIF_ID, buildNotification());
        return START_STICKY; // restart if killed
    }

    @Override
    public IBinder onBind(Intent intent) { return null; }

    private void createChannel() {
        NotificationChannel ch = new NotificationChannel(
            CHANNEL_ID, "Kira Agent", NotificationManager.IMPORTANCE_LOW);
        ch.setDescription("Keeps Kira AI running in background");
        ch.setShowBadge(false);
        getSystemService(NotificationManager.class).createNotificationChannel(ch);
    }

    private Notification buildNotification() {
        Intent openApp = new Intent(this, MainActivity.class);
        PendingIntent pi = PendingIntent.getActivity(this, 0, openApp,
            PendingIntent.FLAG_IMMUTABLE | PendingIntent.FLAG_UPDATE_CURRENT);

        return new Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("Kira is running")
            .setContentText("AI agent active -- tap to open")
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentIntent(pi)
            .setOngoing(true)
            .build();
    }
}
