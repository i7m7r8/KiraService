package com.kira.service;

/**
 * JNI bridge to Rust core.
 * All methods correspond to exported Rust functions in lib.rs.
 */
public class RustBridge {
    static { System.loadLibrary("kira_core"); }

    // Lifecycle
    public static native void startServer(int port);

    // Push state to Rust
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateScreenPackage(String pkg);
    public static native void updateBattery(int pct, boolean charging);
    public static native void updateAgentContext(String context);
    public static native void logTaskStep(String taskId, int step, String action, String result, boolean success);

    // Command queue (Java reads, Rust stores)
    public static native String nextCommand();
    public static native void   pushResult(String id, String result);

    // Proactive trigger engine (NanoBot-style)
    public static native void   addTrigger(String id, String type, String value, String action, boolean repeat);
    public static native void   removeTrigger(String id);
    public static native String nextFiredTrigger();

    // Memory management
    public static native void   freeString(String s);
}
