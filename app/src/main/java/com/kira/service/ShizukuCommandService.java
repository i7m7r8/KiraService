package com.kira.service;

import android.app.Service;
import android.content.Intent;
import android.os.IBinder;

/**
 * Stub service used by Shizuku UserServiceArgs.
 * The actual execution happens in ShizukuShell.execDirect via Runtime.exec
 * which gets elevated privileges when Shizuku is connected.
 */
public class ShizukuCommandService extends Service {
    @Override
    public IBinder onBind(Intent intent) { return null; }
}
