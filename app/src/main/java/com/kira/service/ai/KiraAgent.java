package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;
import org.json.JSONArray;
import org.json.JSONObject;

/**
 * OpenClaw/ZeroClaw-style multi-step autonomous agent.
 *
 * Architecture (like Rou Bao):
 *   Manager  -> plans the overall task, breaks into steps
 *   Executor -> executes each step using tools
 *   Reflector-> checks if step succeeded, decides next action
 *   Notetaker-> records everything to persistent memory + Rust task log
 *
 * Flow:
 *   1. Manager receives goal, makes step plan
 *   2. For each step: Executor picks tool + args, runs it
 *   3. Reflector sees result, judges success/failure/retry
 *   4. If success: move to next step
 *   5. If fail: retry up to 3x, then skip or abort
 *   6. Notetaker logs every step to Rust + SharedPrefs
 */
public class KiraAgent {

    private static final String TAG = "KiraAgent";
    private static final int MAX_STEPS = 25;
    private static final int MAX_RETRIES = 3;

    private final Context ctx;
    private final KiraAI ai;
    private final KiraMemory memory;
    private final KiraTools tools;
    private volatile boolean running = false;
    private String currentTaskId = null;

    public interface AgentCallback {
        void onPlan(String plan);
        void onStep(int step, String action, String result);
        void onDone(String summary);
        void onError(String error);
    }

    public KiraAgent(Context ctx) {
        this.ctx    = ctx.getApplicationContext();
        this.ai     = new KiraAI(ctx);
        this.memory = new KiraMemory(ctx);
        this.tools  = new KiraTools(ctx);
    }

    public void stop() { running = false; }
    public boolean isRunning() { return running; }

    /**
     * Execute a multi-step goal autonomously.
     * This is the main OpenClaw/NanoBot entry point.
     */
    public void execute(String goal, AgentCallback cb) {
        if (running) { cb.onError("agent already running"); return; }
        running = true;
        currentTaskId = "task_" + System.currentTimeMillis();

        new Thread(() -> {
            try {
                runAgent(goal, cb);
            } catch (Exception e) {
                Log.e(TAG, "agent error", e);
                cb.onError(e.getMessage());
            } finally {
                running = false;
                currentTaskId = null;
            }
        }, "kira-agent").start();
    }

    private void runAgent(String goal, AgentCallback cb) throws Exception {
        String taskId = currentTaskId;
        String memContext = memory.getContext();

        // MANAGER: Create step-by-step plan
        String planPrompt = "You are a task planner for an Android AI agent.\n"
            + "Goal: " + goal + "\n"
            + (memContext.isEmpty() ? "" : "Context:\n" + memContext + "\n")
            + "\nAvailable tools: open_app, read_screen, tap_text, tap_screen, type_text, "
            + "swipe_screen, scroll_screen, press_back, press_home, web_search, send_sms, "
            + "sh_run, sh_screenshot, sh_dump_ui, get_notifications, read_file, write_file, "
            + "http_get, scrape_web, set_alarm, remember, recall, battery_info, wifi_on, "
            + "read_sms, read_contacts, call_number, deep_link, share_text, find_app\n"
            + "\nCreate a numbered plan with 1-8 concrete steps. Each step: one tool call.\n"
            + "Format: 1. [tool_name] brief description\n"
            + "2. [tool_name] brief description\n"
            + "Be specific and practical. If the goal is simple, use fewer steps.";

        String plan = ai.simpleChat(planPrompt);
        cb.onPlan(plan);
        try { RustBridge.logTaskStep(taskId, 0, "PLAN: " + goal, plan, true); } catch (Throwable ignored) {}
        memory.storeConversation("AGENT PLAN: " + goal, plan);

        // Parse plan into steps
        String[] lines = plan.split("\n");
        int stepNum = 0;

        for (String line : lines) {
            if (!running) { cb.onError("cancelled"); return; }
            if (stepNum >= MAX_STEPS) break;

            line = line.trim();
            if (line.isEmpty()) continue;

            // Extract tool from [tool_name] format
            String toolName = null;
            String description = line;
            if (line.matches("^\\d+\\..*")) {
                description = line.replaceFirst("^\\d+\\.\\s*", "");
            }
            // Extract [tool_name] if present
            if (description.contains("[") && description.contains("]")) {
                int s = description.indexOf('[');
                int e = description.indexOf(']');
                if (e > s) toolName = description.substring(s+1, e);
                description = description.replace("[" + toolName + "]", "").trim();
            }

            if (toolName == null) continue;
            stepNum++;

            // EXECUTOR: Determine args for this tool
            String execPrompt = "You are executing step " + stepNum + " of a task.\n"
                + "Goal: " + goal + "\n"
                + "Current step: use tool '" + toolName + "' to: " + description + "\n"
                + "Current screen: " + getScreenSummary() + "\n"
                + "\nRespond with ONLY a JSON object of arguments for " + toolName + ".\n"
                + "Examples:\n"
                + "  open_app: {\"package\": \"youtube\"}\n"
                + "  tap_text: {\"text\": \"Search\"}\n"
                + "  type_text: {\"text\": \"hello world\"}\n"
                + "  web_search: {\"query\": \"weather today\"}\n"
                + "  sh_run: {\"cmd\": \"pm list packages | head -10\"}\n"
                + "  read_screen: {}\n"
                + "  press_back: {}\n"
                + "  remember: {\"key\": \"result\", \"value\": \"found it\"}\n"
                + "Respond with ONLY the JSON, no explanation.";

            String argsJson = ai.simpleChat(execPrompt);
            argsJson = argsJson.trim();
            // Strip markdown code blocks if present
            if (argsJson.startsWith("```")) {
                argsJson = argsJson.replaceAll("```[a-z]*", "").trim();
            }

            String result = "(no result)";
            boolean success = false;
            int retries = 0;

            while (retries < MAX_RETRIES && running) {
                try {
                    JSONObject args = new JSONObject(argsJson.isEmpty() ? "{}" : argsJson);
                    result = tools.execute(toolName, args);
                    success = !result.startsWith("error") && !result.startsWith("Error");
                    break;
                } catch (Exception e) {
                    retries++;
                    result = "parse error (retry " + retries + "): " + e.getMessage();
                    if (retries < MAX_RETRIES) {
                        // Ask AI to fix the args
                        argsJson = ai.simpleChat("Fix this JSON for tool '" + toolName + "': " + argsJson + "\nError: " + e.getMessage() + "\nReturn ONLY valid JSON.");
                        argsJson = argsJson.trim().replaceAll("```[a-z]*", "").trim();
                    }
                }
            }

            cb.onStep(stepNum, toolName + ": " + description, result);
            try { RustBridge.logTaskStep(taskId, stepNum, toolName + ": " + description, result, success); } catch (Throwable ignored) {}

            // REFLECTOR: Check if step succeeded and if we should continue
            if (!success && stepNum > 1) {
                String reflectPrompt = "Step failed: " + toolName + " - " + description + "\nResult: " + result
                    + "\nGoal: " + goal + "\nShould we: continue, retry, or abort? Reply with one word.";
                String decision = ai.simpleChat(reflectPrompt).toLowerCase().trim();
                if (decision.startsWith("abort")) {
                    cb.onError("Aborted after step " + stepNum + " failed: " + result);
                    return;
                }
            }

            // Brief pause between steps to let UI settle
            try { Thread.sleep(800); } catch (Exception ignored) {}
        }

        // NOTETAKER: Summarize what was accomplished
        String summaryPrompt = "Summarize what was accomplished for goal: " + goal
            + "\nSteps completed: " + stepNum
            + "\nProvide a brief 1-2 sentence summary.";
        String summary = ai.simpleChat(summaryPrompt);
        memory.storeConversation("AGENT TASK: " + goal, summary);
        cb.onDone(summary);
        try { RustBridge.logTaskStep(taskId, stepNum + 1, "COMPLETE", summary, true); } catch (Throwable ignored) {}
    }

    private String getScreenSummary() {
        try {
            com.kira.service.KiraAccessibilityService svc = com.kira.service.KiraAccessibilityService.instance;
            if (svc != null) {
                String text = svc.getScreenText();
                if (!text.isEmpty()) return text.substring(0, Math.min(300, text.length()));
            }
        } catch (Exception ignored) {}
        return "(screen unavailable)";
    }
}
