package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.List;

/**
 * OpenClaw-style tool chaining / ReAct loop.
 * Implements: Reason -> Act -> Observe -> Repeat
 * This is the core of what makes Kira a true autonomous agent.
 *
 * vs KiraAgent (which plans steps upfront):
 * KiraChain uses the ReAct pattern - the LLM decides each action
 * based on previous observations, allowing dynamic replanning.
 */
public class KiraChain {

    private static final String TAG = "KiraChain";
    private static final int MAX_ITERATIONS = 20;

    private final Context ctx;
    private final KiraAI ai;
    private final KiraTools tools;
    private final KiraMemory memory;

    public interface ChainCallback {
        void onThought(String thought);
        void onAction(String tool, String args);
        void onObservation(String result);
        void onFinal(String answer);
        void onError(String error);
    }

    public KiraChain(Context ctx) {
        this.ctx    = ctx.getApplicationContext();
        this.ai     = new KiraAI(ctx);
        this.tools  = new KiraTools(ctx);
        this.memory = new KiraMemory(ctx);
    }

    /**
     * Run the ReAct loop for a given goal.
     * Format: Thought: ... Action: tool_name(args) Observation: result ... Final Answer: ...
     */
    public void run(String goal, ChainCallback cb) {
        new Thread(() -> {
            try {
                runReActLoop(goal, cb);
            } catch (Exception e) {
                cb.onError(e.getMessage());
            }
        }, "kira-chain").start();
    }

    private void runReActLoop(String goal, ChainCallback cb) throws Exception {
        String memCtx = memory.getContext();
        StringBuilder history = new StringBuilder();

        String systemPrompt = "You are Kira, an autonomous Android AI agent.\n"
            + "Use the ReAct pattern: Thought, Action, Observation, repeat until done.\n\n"
            + "Available tools:\n"
            + "read_screen{} | tap_text{text} | tap_screen{x,y} | type_text{text}\n"
            + "open_app{package} | sh_run{cmd} | web_search{query} | http_get{url}\n"
            + "scrape_web{url,selector} | send_sms{number,message} | remember{key,value}\n"
            + "recall{key} | watch_screen{keyword,action} | analyze_screen{question}\n"
            + "find_element{description} | get_notifications{} | battery_info{}\n"
            + "press_back{} | press_home{} | scroll_screen{direction} | swipe_screen{x1,y1,x2,y2}\n"
            + "sh_screenshot{} | list_files{path} | read_file{path} | write_file{path,content}\n\n"
            + "Format EXACTLY:\n"
            + "Thought: your reasoning\n"
            + "Action: tool_name\n"
            + "Action Input: {\"key\": \"value\"}\n\n"
            + "Or to finish:\n"
            + "Thought: I have completed the task\n"
            + "Final Answer: summary of what was done\n\n"
            + (memCtx.isEmpty() ? "" : "Context:\n" + memCtx + "\n\n");

        for (int iteration = 0; iteration < MAX_ITERATIONS; iteration++) {
            String prompt = systemPrompt + "Goal: " + goal + "\n\n" + history.toString()
                + "What is your next Thought and Action?";

            String response = ai.simpleChat(prompt);
            if (response == null || response.startsWith("error:")) {
                cb.onError("LLM error: " + response);
                return;
            }

            // Parse Final Answer
            if (response.contains("Final Answer:")) {
                int idx = response.indexOf("Final Answer:") + 13;
                String answer = response.substring(idx).trim();
                memory.storeConversation("CHAIN: " + goal, answer);
                cb.onFinal(answer);
                return;
            }

            // Parse Thought
            String thought = extractLine(response, "Thought:");
            if (!thought.isEmpty()) {
                cb.onThought(thought);
                history.append("Thought: ").append(thought).append("\n");
            }

            // Parse Action + Action Input
            String action = extractLine(response, "Action:");
            String actionInput = extractLine(response, "Action Input:");
            if (action.isEmpty()) {
                // Malformed - try to extract any tool call
                action = "read_screen";
                actionInput = "{}";
            }

            cb.onAction(action.trim(), actionInput.trim());
            history.append("Action: ").append(action.trim()).append("\n");
            history.append("Action Input: ").append(actionInput.trim()).append("\n");

            // Execute the tool
            String observation;
            try {
                JSONObject args = new JSONObject(actionInput.trim().isEmpty() ? "{}" : actionInput.trim());
                observation = tools.execute(action.trim(), args);
            } catch (Exception e) {
                observation = "Error executing " + action + ": " + e.getMessage();
            }

            // Truncate long observations
            if (observation.length() > 500) {
                observation = observation.substring(0, 500) + "...(truncated)";
            }

            cb.onObservation(observation);
            history.append("Observation: ").append(observation).append("\n\n");

            // Log step to Rust
            try { com.kira.service.RustBridge.logTaskStep(
                "chain_" + System.currentTimeMillis(), iteration,
                action.trim() + ": " + actionInput.trim(),
                observation, !observation.startsWith("Error")
            ); } catch (Throwable ignored) {}

            // Small pause
            try { Thread.sleep(300); } catch (Exception ignored) {}
        }

        cb.onError("Max iterations reached (" + MAX_ITERATIONS + ")");
    }

    private String extractLine(String text, String prefix) {
        int idx = text.indexOf(prefix);
        if (idx < 0) return "";
        int start = idx + prefix.length();
        int end = text.indexOf("\n", start);
        return end < 0 ? text.substring(start).trim() : text.substring(start, end).trim();
    }
}
