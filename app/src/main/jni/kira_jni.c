// Minimal JNI shim — Rust handles all actual logic
// This file just exists to satisfy CMake linking
#include <jni.h>
#include <android/log.h>

#define TAG "KiraService"

JNIEXPORT jint JNICALL JNI_OnLoad(JavaVM *vm, void *reserved) {
    __android_log_print(ANDROID_LOG_INFO, TAG, "KiraService Rust core loaded");
    return JNI_VERSION_1_6;
}
