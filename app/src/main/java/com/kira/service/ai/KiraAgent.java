package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import com.kira.service.ShizukuShell;

/**
 * KiraAgent — Session E: thin wrapper over Rust /ai/agent engine.
 * Original: 212 lines. Rewritten: ~80 lines.
 * All ReAct loop logic (PLAN/THINK/ACT/OBSERVE) now runs in Rust.
 */
public class KiraAgent {
    private static final String TAG       = "KiraAgent";
    private static final int    MAX_STEPS = 25;
    private volatile boolean running = false;

    public interface AgentCallback {
        void onPlan(String plan);
        void onStep(int step, String action, String result);
        void onDone(String summary);
        void onError(String error);
    }

    public KiraAgent(Context ctx) { /* Rust owns all state */ }

    public void stop() {
        running = false;
        try { RustBridge.stopAgent(); } catch (Throwable ignored) {}
    }

    public boolean isRunning() { return running; }

    public void execute(String goal, AgentCallback cb) {
        if (running) { cb.onError("agent already running"); return; }
        running = true;
        new Thread(() -> {
            try {
                cb.onPlan("Starting: " + goal);
                String json = RustBridge.agentSync(goal, MAX_STEPS, "agent_default");
                // Drain any shell jobs queued by agent tool calls
                drainShellQueue(cb);
                String summary = parseStr(json, "final");
                boolean ok     = json.contains("\"success\":true");
                int steps      = (int) parseNum(json, "steps");
                if (ok) cb.onDone(summary.isEmpty() ? "done" : summary);
                else    cb.onError("stopped after " + steps + " steps: " + summary);
            } catch (Exception e) {
                Log.e(TAG, "agent error", e);
                cb.onError(e.getMessage());
            } finally {
                running = false;
            }
        }, "kira-agent").start();
    }

    private void drainShellQueue(AgentCallback cb) {
        for (int i = 0; i < 30; i++) {
            try {
                String job = RustBridge.getNextShellJob();
                if (job == null || job.contains("\"empty\":true")) break;
                String id  = parseStr(job, "id");
                String cmd = parseStr(job, "cmd");
                if (cmd.isEmpty()) break;
                String out = ShizukuShell.isAvailable()
                    ? ShizukuShell.exec(cmd, 15_000) : "shizuku_unavailable";
                RustBridge.postShellResult(id, out != null ? out : "");
                cb.onStep(i, cmd.split(" ")[0], out != null ? out : "");
            } catch (Exception e) { break; }
        }
    }

    private static String parseStr(String json, String key) {
        String k = "\"" + key + "\":\"";
        int s = json.indexOf(k); if (s < 0) return "";
        s += k.length(); int e = s;
        while (e < json.length() && !(json.charAt(e)=='"' && json.charAt(e-1)!='\\')) e++;
        return json.substring(s, e).replace("\\n","\n").replace("\\\"","\"");
    }

    private static double parseNum(String json, String key) {
        String k = "\"" + key + "\":";
        int s = json.indexOf(k); if (s < 0) return 0;
        s += k.length();
        int e = s;
        while (e < json.length() && (Character.isDigit(json.charAt(e)) || json.charAt(e)=='.')) e++;
        try { return Double.parseDouble(json.substring(s, e)); } catch (Exception ex) { return 0; }
    }
}
