cmake_minimum_required(VERSION 3.22.1)
project(kira_jni)

# Import the pre-built Rust library
add_library(kira_core SHARED IMPORTED)
set_target_properties(kira_core PROPERTIES
    IMPORTED_LOCATION
    ${CMAKE_SOURCE_DIR}/../../../rust-libs/${ANDROID_ABI}/libkira_core.so
)

# JNI wrapper (minimal - Rust handles everything)
add_library(kira_jni SHARED kira_jni.c)

target_link_libraries(kira_jni
    kira_core
    android
    log
)
