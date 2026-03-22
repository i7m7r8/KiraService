package com.kira.service.ai;

import android.content.Context;
import android.content.SharedPreferences;
import android.util.Log;
import org.json.JSONArray;
import org.json.JSONObject;
import java.util.ArrayList;
import java.util.List;

/**
 * OpenClaw .agent/workflows pattern.
 * Predefined multi-step automations that can be triggered by name.
 * Stored as JSON, run by KiraAgent or KiraChain.
 *
 * Example workflows:
 *   "morning": check battery, read notifications, brief weather
 *   "send_report": screenshot -> analyze -> send via telegram
 *   "cleanup": clear cache, list big files, report
 */
public class KiraWorkflow {

    private static final String TAG      = "KiraWorkflow";
    private static final String PREFS    = "kira_workflows";

    private final SharedPreferences prefs;
    private final Context ctx;

    // Built-in workflows (OpenClaw .agent/workflows pattern)
    private static final String[][] BUILTIN = {
        {"morning_brief",
         "Morning briefing",
         "[battery_info, get_notifications, web_search:today headlines]",
         "Get battery status, read all notifications, search for today's top news"},

        {"screen_report",
         "Screen report",
         "[sh_screenshot, analyze_screen:describe everything, remember:last_screen_report]",
         "Take screenshot, analyze with vision AI, save summary to memory"},

        {"device_health",
         "Device health check",
         "[battery_info, sh_ram_info, sh_cpu_info, sh_storage]",
         "Check battery, RAM, CPU and storage status"},

        {"quick_search",
         "Quick web search",
         "[web_search:{query}, scrape_web:{first_result}]",
         "Search the web and scrape the top result"},

        {"send_status",
         "Send status to Telegram",
         "[battery_info, get_notifications, sh_run:uptime]",
         "Send device status report to Telegram"},
    };

    public KiraWorkflow(Context ctx) {
        this.ctx   = ctx.getApplicationContext();
        this.prefs = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    /** Run a workflow by name - returns the goal string for KiraAgent/KiraChain */
    public String buildGoal(String name) {
        // Check custom workflows first
        String custom = prefs.getString("wf_" + name, null);
        if (custom != null) {
            try {
                JSONObject wf = new JSONObject(custom);
                return wf.optString("description", name);
            } catch (Exception ignored) {}
        }
        // Check built-ins
        for (String[] wf : BUILTIN) {
            if (wf[0].equals(name)) return wf[3];
        }
        return name; // treat name as raw goal
    }

    /** Save a custom workflow */
    public void save(String name, String description, String steps) {
        try {
            JSONObject wf = new JSONObject();
            wf.put("name", name);
            wf.put("description", description);
            wf.put("steps", steps);
            wf.put("created", System.currentTimeMillis());
            prefs.edit().putString("wf_" + name, wf.toString()).apply();
            Log.i(TAG, "workflow saved: " + name);
        } catch (Exception e) { Log.e(TAG, "save failed", e); }
    }

    /** List all available workflows */
    public String listJson() {
        JSONArray arr = new JSONArray();
        // Built-ins
        for (String[] wf : BUILTIN) {
            try {
                JSONObject o = new JSONObject();
                o.put("name", wf[0]); o.put("title", wf[1]);
                o.put("steps", wf[2]); o.put("description", wf[3]);
                o.put("builtin", true);
                arr.put(o);
            } catch (Exception ignored) {}
        }
        // Custom
        for (String key : prefs.getAll().keySet()) {
            if (!key.startsWith("wf_")) continue;
            try { arr.put(new JSONObject(prefs.getString(key, "{}"))); }
            catch (Exception ignored) {}
        }
        return arr.toString();
    }

    /** Get names only */
    public List<String> getNames() {
        List<String> names = new ArrayList<>();
        for (String[] wf : BUILTIN) names.add(wf[0]);
        for (String key : prefs.getAll().keySet()) {
            if (key.startsWith("wf_")) names.add(key.substring(3));
        }
        return names;
    }
}
