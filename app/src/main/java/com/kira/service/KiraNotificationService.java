package com.kira.service;

import android.app.Notification;
import android.os.Bundle;
import android.service.notification.NotificationListenerService;
import android.service.notification.StatusBarNotification;
import android.util.Log;

/**
 * Proper NotificationListenerService - reads ALL notifications reliably.
 * This is the correct way to intercept notifications (not AccessibilityService).
 * AccessibilityService TYPE_NOTIFICATION_STATE_CHANGED misses many notifications
 * because it only fires for Toast-style notifications, not status bar ones.
 */
public class KiraNotificationService extends NotificationListenerService {

    private static final String TAG = "KiraNotif";
    public static KiraNotificationService instance;

    @Override
    public void onListenerConnected() {
        instance = this;
        Log.i(TAG, "notification listener connected");
    }

    @Override
    public void onListenerDisconnected() {
        instance = null;
        Log.i(TAG, "notification listener disconnected");
    }

    @Override
    public void onNotificationPosted(StatusBarNotification sbn) {
        if (sbn == null) return;
        try {
            String pkg = sbn.getPackageName();
            // Skip our own notifications
            if ("com.kira.service".equals(pkg)) return;

            Notification notif = sbn.getNotification();
            if (notif == null) return;

            Bundle extras = notif.extras;
            if (extras == null) return;

            // Extract title
            String title = "";
            CharSequence titleCS = extras.getCharSequence(Notification.EXTRA_TITLE);
            if (titleCS != null) title = titleCS.toString();

            // Extract text (prefer BIG_TEXT for full content)
            String text = "";
            CharSequence bigText = extras.getCharSequence(Notification.EXTRA_BIG_TEXT);
            if (bigText != null && bigText.length() > 0) {
                text = bigText.toString();
            } else {
                CharSequence textCS = extras.getCharSequence(Notification.EXTRA_TEXT);
                if (textCS != null) text = textCS.toString();
            }

            // Extract sub-text
            CharSequence subText = extras.getCharSequence(Notification.EXTRA_SUB_TEXT);
            if (subText != null && !subText.toString().isEmpty() && text.isEmpty()) {
                text = subText.toString();
            }

            if (title.isEmpty() && text.isEmpty()) return;

            Log.d(TAG, "notif: " + pkg + " | " + title + " | " + text.substring(0, Math.min(50, text.length())));
            RustBridge.pushNotification(pkg, title, text);

            // Also fire EventBus for in-app subscribers
            KiraEventBus.post(new KiraEventBus.NotifReceived(pkg, title, text));

        } catch (Exception e) {
            Log.e(TAG, "onNotificationPosted error", e);
        }
    }

    @Override
    public void onNotificationRemoved(StatusBarNotification sbn) {
        // Could track dismissed notifications here
    }

    /**
     * Get all current active notifications (useful for "read all notifications" tool)
     */
    public String getAllNotificationsJson() {
        try {
            StatusBarNotification[] active = getActiveNotifications();
            if (active == null || active.length == 0) return "[]";
            StringBuilder sb = new StringBuilder("[");
            boolean first = true;
            for (StatusBarNotification sbn : active) {
                if (!first) sb.append(",");
                first = false;
                Bundle extras = sbn.getNotification().extras;
                String title = "";
                String text  = "";
                if (extras != null) {
                    CharSequence t = extras.getCharSequence(Notification.EXTRA_TITLE);
                    CharSequence x = extras.getCharSequence(Notification.EXTRA_TEXT);
                    if (t != null) title = t.toString();
                    if (x != null) text  = x.toString();
                }
                sb.append("{\"pkg\":\"").append(sbn.getPackageName())
                  .append("\",\"title\":\"").append(title.replace("\"","\\\"").replace("\n","\\n"))
                  .append("\",\"text\":\"").append(text.replace("\"","\\\"").replace("\n","\\n"))
                  .append("\",\"time\":").append(sbn.getPostTime()).append("}");
            }
            sb.append("]");
            return sb.toString();
        } catch (Exception e) {
            return "[]";
        }
    }
}
