# 🦀 KiraService

**Rust-powered Android automation APK for Kira AI agent.**

Ultra-fast HTTP server on `localhost:7070` giving Kira full phone control.

## Architecture

```
Kira (Node.js in Termux)
    ↓ HTTP calls to localhost:7070
Rust Core (libkira_core.so)
    ↓ HTTP server, state management, command queue
Java Accessibility Service
    ↓ Screen read, tap, type, gestures
Shizuku Bridge
    ↓ ADB-level commands (install, permissions, system settings)
Android System
```

## Why Rust?

- HTTP server responds in <1ms (vs 50-100ms in Java)
- Zero GC pauses — no Android garbage collector jank
- Shared state between threads without locks causing stutters
- Binary is tiny — ~200KB vs 2MB+ Java equivalent

## Features

### Via Accessibility Service
- Read full screen content from any app
- Tap, long press, swipe, scroll
- Type text into any field
- Get/set clipboard
- Open any app
- Read all notifications in real-time
- Control volume, brightness, flashlight
- Wake/lock screen

### Via Shizuku (ADB-level, no root)
- Install/uninstall APKs silently
- Grant/revoke any permission without popup
- Force stop any app
- Toggle WiFi, mobile data, airplane mode
- Read/write system settings
- Run ANY ADB shell command
- Get running processes
- Start activities and services

## Install

1. Download latest APK from [Releases](../../releases)
2. Enable "Install unknown apps"
3. Install APK
4. Open KiraService → tap "Enable Accessibility Service" → find KiraService → enable it
5. Install [Shizuku](https://shizuku.rikka.app/) → start it → grant permission to KiraService

## Build from Source

Push to main branch → GitHub Actions builds automatically → APK appears in Releases.

## Connect to Kira

```bash
# Test server
curl http://localhost:7070/health
# → {"status":"ok","engine":"rust"}

# In Kira config — already works automatically
```
