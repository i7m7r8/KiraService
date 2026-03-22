package com.kira.service.ai;

import android.content.Context;
import android.util.Log;
import com.kira.service.RustBridge;

/**
 * KiraChain — Session E: thin wrapper over Rust /ai/chain engine.
 * Original: 158 lines. Rewritten: ~50 lines.
 */
public class KiraChain {
    private static final String TAG = "KiraChain";

    public interface ChainCallback {
        void onStep(String thought);
        void onConclusion(String conclusion);
        void onError(String error);
    }

    public KiraChain(Context ctx) { /* Rust owns all state */ }

    public void run(String goal, int depth, ChainCallback cb) {
        new Thread(() -> {
            try {
                String json = RustBridge.chainSync(goal, depth);
                String conclusion = parseStr(json, "conclusion");
                // Fire onStep for each reasoning step
                int i = 1;
                while (true) {
                    String step = parseArrayItem(json, i++);
                    if (step.isEmpty()) break;
                    cb.onStep(step);
                }
                cb.onConclusion(conclusion.isEmpty() ? json : conclusion);
            } catch (Exception e) {
                Log.e(TAG, "chain error", e);
                cb.onError(e.getMessage());
            }
        }, "kira-chain").start();
    }

    private static String parseStr(String json, String key) {
        String k = "\"" + key + "\":\"";
        int s = json.indexOf(k); if (s < 0) return "";
        s += k.length(); int e = s;
        while (e < json.length() && !(json.charAt(e)=='"' && (e==0||json.charAt(e-1)!='\\'))) e++;
        return json.substring(s, Math.min(e, json.length()));
    }

    private static String parseArrayItem(String json, int idx) {
        // Finds the idx-th quoted string in the reasoning array
        String arr = json;
        int bracket = json.indexOf("\"reasoning\":[");
        if (bracket < 0) return "";
        arr = json.substring(bracket);
        int count = 0;
        int pos = 0;
        while (pos < arr.length()) {
            int q = arr.indexOf('"', pos);
            if (q < 0) break;
            int qe = arr.indexOf('"', q+1);
            if (qe < 0) break;
            String item = arr.substring(q+1, qe);
            if (!item.startsWith("reasoning")) {
                count++;
                if (count == idx) return item;
            }
            pos = qe + 1;
        }
        return "";
    }
}
