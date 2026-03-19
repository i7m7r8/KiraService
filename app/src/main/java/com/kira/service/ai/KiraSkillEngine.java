package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import java.util.ArrayList;
import java.util.List;

/**
 * OpenClaw Skill Engine - SKILL.md selective injection pattern.
 *
 * OpenClaw injects only relevant skills per turn, not all skills at once.
 * This keeps the prompt small while giving the agent special abilities
 * when needed (e.g. inject "web_search skill" only when user asks about news).
 *
 * Skills are stored in Rust via registerSkill() and fetched here.
 * Each skill has a trigger pattern - if the user message matches,
 * the skill content is prepended to the system prompt for that turn only.
 */
public class KiraSkillEngine {

    private static final String TAG = "KiraSkillEngine";

    private final Context ctx;

    // Built-in skills (OpenClaw SKILL.md pattern)
    private static final String[][] BUILTIN_SKILLS = {
        // {name, trigger_keywords, content}
        {"web_search_skill",
         "search,find online,look up,latest,news,current,today,weather,price",
         "You can search the web using web_search{query}. Always search for current info before answering questions about recent events, news, prices, or anything time-sensitive."},

        {"screen_control_skill",
         "tap,click,open,press,swipe,scroll,type,enter,navigate,go to,launch",
         "You control the Android screen directly. Use: tap_text{text} to tap UI elements, type_text{text} to type, open_app{package} to open apps, read_screen{} to see current screen, swipe_screen{x1,y1,x2,y2} for swipes."},

        {"agent_mode_skill",
         "automatically,autonomously,do it yourself,handle it,take care of,complete the task,step by step",
         "For multi-step tasks: use /agent prefix to invoke the autonomous agent planner. For dynamic reasoning: use /chain for the ReAct loop. Both run tools in sequence without user intervention."},

        {"memory_skill",
         "remember,forget,recall,store,save,note,my name,my preference,always,never",
         "Persistent memory: remember{key,value} stores facts permanently. recall{key} retrieves them. memory_search{query} finds relevant facts. These persist across conversations."},

        {"file_skill",
         "file,folder,directory,read,write,save,download,sdcard,storage",
         "File operations: list_files{path}, read_file{path}, write_file{path,content}, delete_file{path}. Default storage: /sdcard/. You have full file access via Shizuku."},

        {"shell_skill",
         "command,shell,terminal,run,execute,adb,root,system,install,package",
         "Full shell access via Shizuku (no root needed): sh_run{cmd} runs any command. sh_screenshot{} takes screenshot. sh_broadcast{action} sends broadcasts. sh_app_info{package} gets app details."},

        {"telegram_skill",
         "telegram,notify me,send me,alert,message me,let me know,ping me",
         "You can send Telegram messages proactively. Use the Telegram bot to notify the user about events, completed tasks, or important changes. This is already configured if tgToken is set."},

        {"vision_skill",
         "see,look at,what is on screen,describe screen,read image,ocr,analyze",
         "Vision capabilities: analyze_screen{question} sends current screenshot to vision LLM. find_element{description} visually finds and taps UI elements. describe_screen{} gives full UI description."},
    };

    public KiraSkillEngine(Context ctx) {
        this.ctx = ctx.getApplicationContext();
        registerBuiltinSkills();
    }

    private void registerBuiltinSkills() {
        for (String[] skill : BUILTIN_SKILLS) {
            try {
                com.kira.service.RustBridge.registerSkill(skill[0], skill[0].replace("_skill",""), skill[1], skill[2]);
            } catch (Throwable ignored) {}
        }
    }

    /**
     * Get skills relevant to the current user message.
     * Returns injected skill content to prepend to system prompt.
     * OpenClaw pattern: selective injection, not all skills every time.
     */
    public String getRelevantSkillsPrompt(String userMessage) {
        if (userMessage == null || userMessage.isEmpty()) return "";
        String msgLower = userMessage.toLowerCase();
        List<String> injected = new ArrayList<>();

        for (String[] skill : BUILTIN_SKILLS) {
            String[] triggers = skill[1].split(",");
            for (String trigger : triggers) {
                if (msgLower.contains(trigger.trim())) {
                    injected.add("SKILL[" + skill[0].replace("_skill","").toUpperCase() + "]: " + skill[2]);
                    break;
                }
            }
        }

        if (injected.isEmpty()) return "";
        StringBuilder sb = new StringBuilder("\n\n--- Active Skills ---\n");
        for (String s : injected) sb.append(s).append("\n");
        sb.append("--- End Skills ---\n");
        Log.d(TAG, "Injected " + injected.size() + " skills for: " + userMessage.substring(0, Math.min(40, userMessage.length())));
        return sb.toString();
    }

    /**
     * Register a custom skill from user input.
     * Stored in Rust skill registry and persists via memory.
     */
    public void registerCustomSkill(String name, String trigger, String content) {
        try { com.kira.service.RustBridge.registerSkill(name, name, trigger, content); } catch (Throwable ignored) {}
        // Also persist to memory so it survives restarts
        new KiraMemory(ctx).remember("skill_" + name, trigger + "|" + content);
        Log.i(TAG, "Custom skill registered: " + name);
    }

    /**
     * Load custom skills from memory on startup.
     */
    public void loadCustomSkillsFromMemory() {
        KiraMemory mem = new KiraMemory(ctx);
        String all = mem.listAll();
        for (String line : all.split("\n")) {
            if (!line.startsWith("skill_")) continue;
            int colon = line.indexOf(":");
            if (colon < 0) continue;
            String name = line.substring(6, colon).trim(); // strip "skill_"
            String val  = line.substring(colon + 1).trim();
            String[] parts = val.split("\\|", 2);
            if (parts.length == 2) {
                try { com.kira.service.RustBridge.registerSkill(name, name, parts[0], parts[1]); } catch (Throwable ignored) {}
            }
        }
    }
}
