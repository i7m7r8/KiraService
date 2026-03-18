package com.kira.service;

import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;

public class BootReceiver extends BroadcastReceiver {
    @Override
    public void onReceive(Context context, Intent intent) {
        // Re-enable accessibility service reminder on boot
        // The service itself auto-starts if enabled in accessibility settings
        Intent i = new Intent(context, MainActivity.class);
        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        // Don't auto-open, just ensure service is running
    }
}
