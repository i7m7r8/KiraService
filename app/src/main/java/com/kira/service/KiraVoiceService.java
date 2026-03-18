package com.kira.service;

import android.content.Context;
import android.content.Intent;
import android.media.AudioManager;
import android.os.Bundle;
import android.speech.RecognitionListener;
import android.speech.RecognizerIntent;
import android.speech.SpeechRecognizer;
import android.speech.tts.TextToSpeech;
import android.util.Log;

import com.kira.service.ai.KiraAI;

import java.util.ArrayList;
import java.util.Locale;

/**
 * Voice assistant -- inspired by openclaw-assistant (yuga-hashimoto):
 * - Wake word detection ("Hey Kira", "Kira", "OK Kira")
 * - Continuous listening + TTS responses
 * - Mute when speaking to avoid feedback
 */
public class KiraVoiceService {
    private static final String TAG = "KiraVoice";

    private final Context ctx;
    private final KiraAI  ai;
    private SpeechRecognizer recognizer;
    private TextToSpeech tts;
    private volatile boolean listening = false;
    private volatile boolean speaking  = false;
    private boolean initialized = false;

    private static final String[] WAKE_WORDS = {"kira", "hey kira", "ok kira", "okay kira"};

    public KiraVoiceService(Context ctx, KiraAI ai) {
        this.ctx = ctx.getApplicationContext();
        this.ai  = ai;
    }

    public void init(Runnable onReady) {
        tts = new TextToSpeech(ctx, status -> {
            if (status == TextToSpeech.SUCCESS) {
                tts.setLanguage(Locale.getDefault());
                tts.setSpeechRate(0.95f);
                initialized = true;
                Log.i(TAG, "TTS ready");
                if (onReady != null) onReady.run();
            }
        });
    }

    public void startListening() {
        if (!SpeechRecognizer.isRecognitionAvailable(ctx)) {
            Log.w(TAG, "speech recognition not available on this device");
            return;
        }
        listening = true;
        listenLoop();
    }

    public void stopListening() {
        listening = false;
        if (recognizer != null) { recognizer.destroy(); recognizer = null; }
    }

    private void listenLoop() {
        if (!listening) return;

        recognizer = SpeechRecognizer.createSpeechRecognizer(ctx);
        Intent intent = new Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH);
        intent.putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM);
        intent.putExtra(RecognizerIntent.EXTRA_PARTIAL_RESULTS, true);
        intent.putExtra(RecognizerIntent.EXTRA_MAX_RESULTS, 3);
        intent.putExtra(RecognizerIntent.EXTRA_LANGUAGE, Locale.getDefault());

        recognizer.setRecognitionListener(new RecognitionListener() {
            @Override public void onReadyForSpeech(Bundle params) {}
            @Override public void onBeginningOfSpeech() {}
            @Override public void onRmsChanged(float rmsdB) {}
            @Override public void onBufferReceived(byte[] buffer) {}
            @Override public void onEndOfSpeech() {}
            @Override public void onError(int error) {
                if (listening) new android.os.Handler(android.os.Looper.getMainLooper())
                    .postDelayed(() -> listenLoop(), 1000);
            }

            @Override
            public void onResults(Bundle results) {
                ArrayList<String> matches = results.getStringArrayList(SpeechRecognizer.RESULTS_RECOGNITION);
                if (matches != null && !matches.isEmpty()) {
                    String heard = matches.get(0).toLowerCase().trim();
                    Log.d(TAG, "heard: " + heard);

                    // Check wake word
                    boolean woken = false;
                    String command = heard;
                    for (String ww : WAKE_WORDS) {
                        if (heard.contains(ww)) {
                            command = heard.replace(ww, "").trim();
                            woken = true;
                            break;
                        }
                    }

                    if (woken && !command.isEmpty()) {
                        processVoiceCommand(command);
                    }
                }
                if (listening) new android.os.Handler(android.os.Looper.getMainLooper())
                    .postDelayed(() -> listenLoop(), 500);
            }

            @Override public void onPartialResults(Bundle partial) {}
            @Override public void onEvent(int eventType, Bundle params) {}
        });

        try { recognizer.startListening(intent); }
        catch (Exception e) { Log.e(TAG, "startListening failed", e); }
    }

    private void processVoiceCommand(String command) {
        if (command.isEmpty()) { speak("Yes?"); return; }
        speak("thinking...");

        ai.chat(command, new KiraAI.Callback() {
            @Override public void onThinking() {}
            @Override public void onTool(String name, String result) {}
            @Override public void onReply(String reply) {
                // Strip markdown for TTS
                String clean = reply
                    .replaceAll("```[\\s\\S]*?```", "code block")
                    .replaceAll("\\*+", "")
                    .replaceAll("#+\\s", "")
                    .replaceAll("\\[([^]]+)]\\([^)]+\\)", "$1");
                speak(clean.length() > 300 ? clean.substring(0, 300) + "." : clean);
            }
            @Override public void onError(String error) { speak("Sorry, " + error); }
        });
    }

    public void speak(String text) {
        if (tts == null || !initialized) return;
        speaking = true;
        tts.speak(text, TextToSpeech.QUEUE_FLUSH, null, "kira_" + System.currentTimeMillis());
        // Mark done after estimated duration
        new android.os.Handler(android.os.Looper.getMainLooper()).postDelayed(
            () -> speaking = false, Math.max(1500, text.length() * 60L));
    }

    public boolean isListening() { return listening; }
    public boolean isSpeaking()  { return speaking;  }

    public void destroy() {
        stopListening();
        if (tts != null) { tts.stop(); tts.shutdown(); tts = null; }
    }
}
