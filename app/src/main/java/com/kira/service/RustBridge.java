package com.kira.service;

/**
 * JNI bridge to Kira Rust Core v7.
 *
 * Implements features from real OpenClaw architecture research:
 *
 * OpenClaw: Gateway pattern, MEMORY.md + SOUL.md + daily log, skill registry
 *           (SKILL.md), session management, webhook surface, context engine
 * ZeroClaw: 17-provider registry, XOR+hash credential encryption, cron jobs
 * AndyClaw: Semantic FTS memory index with relevance scoring, tool policy
 * NanoClaw: Full audit trail (5000 entries), task checkpoints, tool iter limit
 */
public class RustBridge {
    static { System.loadLibrary("kira_core"); }

    // ?? Lifecycle
    public static native void startServer(int port);

    // ?? Device state
    public static native void pushNotification(String pkg, String title, String text);
    public static native void updateScreenNodes(String json);
    public static native void updateScreenPackage(String pkg);
    public static native void updateBattery(int pct, boolean charging);
    public static native void updateAgentContext(String context);

    // ?? OpenClaw: context engine (MEMORY.md + daily log pattern)
    public static native void pushContextTurn(String role, String content);

    // ?? AndyClaw: semantic memory FTS index
    public static native void indexMemory(String key, String value, String tags);

    // ?? ZeroClaw: encrypted credential store
    public static native void storeCredential(String name, String value);

    // ?? OpenClaw: skill registry (SKILL.md pattern)
    public static native void registerSkill(String name, String description, String trigger, String content);

    // ?? OpenClaw-pm: heartbeat checklist
    public static native void addHeartbeatItem(String id, String check, String action, long intervalMs);

    // ?? NanoClaw: tool iteration counter (max 20 per session)
    public static native int  incrementToolIter(String sessionId);
    public static native void resetToolIter(String sessionId);

    // ?? NanoClaw: task log
    public static native void logTaskStep(String taskId, int step, String action, String result, boolean success);

    // ?? Gateway command queue
    public static native String nextCommand();
    public static native void   pushResult(String id, String result);

    // ?? Trigger / webhook surface
    public static native void   addTrigger(String id, String type, String value, String action, boolean repeat);
    public static native void   removeTrigger(String id);
    public static native String nextFiredTrigger();

    // ?? Memory
    public static native void   freeString(String s);
}
