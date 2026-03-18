package com.kira.service.ai;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.util.Base64;
import android.util.Log;

import com.kira.service.ShizukuShell;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.ByteArrayOutputStream;
import java.io.File;

/**
 * ZeroClaw-style vision-based screen understanding.
 * Takes a screenshot, encodes to base64, sends to vision-capable LLM.
 * Enables: "what do you see?", OCR, element finding by visual description,
 * "click the blue button", reading images, QR code content, etc.
 */
public class KiraVision {
    private static final String TAG = "KiraVision";

    private final Context ctx;

    public KiraVision(Context ctx) {
        this.ctx = ctx.getApplicationContext();
    }

    /**
     * Capture screen and ask LLM to describe / answer question about it.
     * Uses vision-capable models (GPT-4V, Claude 3, Gemini Pro Vision).
     */
    public String analyzeScreen(String question, KiraConfig cfg) {
        try {
            String shotPath = "/sdcard/kira_vision_" + System.currentTimeMillis() + ".png";
            String shotResult = ShizukuShell.screenshot(shotPath);
            if (shotResult.contains("error")) return "screenshot failed: " + shotResult;

            // Wait for file to be written
            Thread.sleep(500);
            File f = new File(shotPath);
            if (!f.exists()) return "screenshot file not found";

            Bitmap bm = BitmapFactory.decodeFile(shotPath);
            if (bm == null) return "could not read screenshot";

            // Scale down to reduce tokens (ZeroClaw uses 512x512 thumbnails)
            int targetW = 512;
            int targetH = (int)(512.0f * bm.getHeight() / bm.getWidth());
            Bitmap scaled = Bitmap.createScaledBitmap(bm, targetW, targetH, true);
            bm.recycle();

            ByteArrayOutputStream baos = new ByteArrayOutputStream();
            scaled.compress(Bitmap.CompressFormat.JPEG, 70, baos);
            scaled.recycle();

            String b64 = Base64.encodeToString(baos.toByteArray(), Base64.NO_WRAP);
            ShizukuShell.exec("rm -f " + shotPath);

            // Build vision API request
            return callVisionAPI(b64, question, cfg);

        } catch (Exception e) {
            Log.e(TAG, "analyzeScreen error", e);
            return "vision error: " + e.getMessage();
        }
    }

    private String callVisionAPI(String imageBase64, String question, KiraConfig cfg) {
        try {
            String baseUrl = cfg.baseUrl.isEmpty() ? "https://api.groq.com/openai/v1" : cfg.baseUrl;
            String model = cfg.visionModel.isEmpty() ? "meta-llama/llama-4-scout-17b-16e-instruct" : cfg.visionModel;

            JSONObject imageUrl = new JSONObject();
            imageUrl.put("type", "base64");
            imageUrl.put("media_type", "image/jpeg");
            imageUrl.put("data", imageBase64);

            JSONObject imageContent = new JSONObject();
            imageContent.put("type", "image");
            imageContent.put("source", imageUrl);

            JSONObject textContent = new JSONObject();
            textContent.put("type", "text");
            textContent.put("text", question.isEmpty() ? "Describe what you see on this Android screen. List all visible text, buttons, and UI elements." : question);

            JSONArray contentArr = new JSONArray();
            contentArr.put(imageContent);
            contentArr.put(textContent);

            JSONObject userMsg = new JSONObject();
            userMsg.put("role", "user");
            userMsg.put("content", contentArr);

            JSONArray messages = new JSONArray();
            messages.put(userMsg);

            JSONObject body = new JSONObject();
            body.put("model", model);
            body.put("max_tokens", 800);
            body.put("messages", messages);

            okhttp3.OkHttpClient client = new okhttp3.OkHttpClient.Builder()
                .connectTimeout(20, java.util.concurrent.TimeUnit.SECONDS)
                .readTimeout(30, java.util.concurrent.TimeUnit.SECONDS).build();

            okhttp3.Request req = new okhttp3.Request.Builder()
                .url(baseUrl + "/chat/completions")
                .addHeader("Authorization", "Bearer " + cfg.apiKey)
                .addHeader("Content-Type", "application/json")
                .post(okhttp3.RequestBody.create(body.toString(),
                    okhttp3.MediaType.parse("application/json")))
                .build();

            okhttp3.Response resp = client.newCall(req).execute();
            if (resp.body() == null) return "(no response)";
            String respStr = resp.body().string();
            JSONObject respJson = new JSONObject(respStr);
            if (respJson.has("error")) return "vision API error: " + respJson.getJSONObject("error").optString("message");
            return respJson.getJSONArray("choices").getJSONObject(0)
                .getJSONObject("message").getString("content");

        } catch (Exception e) {
            return "vision call failed: " + e.getMessage();
        }
    }

    /**
     * Find UI element by visual description using ZeroClaw approach.
     * Returns coordinates to tap.
     */
    public int[] findElementCoords(String description, KiraConfig cfg) {
        String result = analyzeScreen(
            "Find the UI element described as: '" + description + "'. "
            + "If found, respond with ONLY: COORDS:x,y where x,y are center pixel coordinates. "
            + "If not found, respond with: NOT_FOUND", cfg);

        if (result.contains("COORDS:")) {
            try {
                String coords = result.substring(result.indexOf("COORDS:") + 7).trim().split("\\s")[0];
                String[] parts = coords.split(",");
                return new int[]{Integer.parseInt(parts[0].trim()), Integer.parseInt(parts[1].trim())};
            } catch (Exception ignored) {}
        }
        return null;
    }
}
