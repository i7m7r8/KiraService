package com.kira.service;

import org.greenrobot.eventbus.EventBus;

/**
 * OpenClaw-style inter-component event system.
 * Components communicate without tight coupling.
 * Agents, Telegram, UI, and Heartbeat all talk via events.
 */
public class KiraEventBus {

    // Events
    public static class AgentStarted   { public final String goal; public AgentStarted(String g) { goal = g; } }
    public static class AgentStep      { public final int step; public final String action, result; public AgentStep(int s, String a, String r) { step=s; action=a; result=r; } }
    public static class AgentDone      { public final String summary; public AgentDone(String s) { summary = s; } }
    public static class AgentFailed    { public final String error; public AgentFailed(String e) { error = e; } }
    public static class TriggerFired   { public final String id, action; public TriggerFired(String i, String a) { id=i; action=a; } }
    public static class BatteryUpdate  { public final int pct; public final boolean charging; public BatteryUpdate(int p, boolean c) { pct=p; charging=c; } }
    public static class ScreenChanged  { public final String pkg; public ScreenChanged(String p) { pkg = p; } }
    public static class NotifReceived  { public final String pkg, title, text; public NotifReceived(String p, String t, String x) { pkg=p; title=t; text=x; } }
    public static class TelegramCmd    { public final long chatId; public final String text; public TelegramCmd(long c, String t) { chatId=c; text=t; } }
    public static class KiraReply      { public final long chatId; public final String text; public KiraReply(long c, String t) { chatId=c; text=t; } }

    public static void post(Object event) {
        try { EventBus.getDefault().post(event); }
        catch (Exception ignored) {}
    }

    public static void register(Object subscriber) {
        try {
            if (!EventBus.getDefault().isRegistered(subscriber))
                EventBus.getDefault().register(subscriber);
        } catch (Exception ignored) {}
    }

    public static void unregister(Object subscriber) {
        try { EventBus.getDefault().unregister(subscriber); }
        catch (Exception ignored) {}
    }
}
