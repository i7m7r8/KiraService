package com.kira.service;

public class RustBridge {
    static { System.loadLibrary("kira_core"); }
    public static native void startServer(int port);
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateBattery(String json);
    public static native String nextCommand();
    public static native void pushResult(String id, String result);
    public static native void freeString(String s);
}
