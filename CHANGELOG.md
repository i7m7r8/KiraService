## v0.0.2 — Crash Fix + Neural Glass Crash Reporter

### Critical Bug Fixes

#### Crash 1 — App never opens (`NullPointerException` at startup)
- **Root cause:** `sendBtn.setOnTouchListener()` called directly with no null check.
  If `R.id.sendBtn` doesn't resolve, NPE before any UI renders.
- **Fix:** Null guard + diagnostic `Log.e` so the missing view ID is visible in logcat.

#### Crash 2 — App crashes on every message send (`NullPointerException`)
- **Root cause:** `showTypingIndicator()` calls `chatContainer.removeView()` and
  `chatContainer.addView()` without checking `chatContainer != null`.
  This is the primary send-crash: every single message triggered it.
- **Fix:** Null guard on entry, safe try/catch on remove.

#### Crash 3 — Crash after reply (`IllegalArgumentException`: view not attached)
- **Root cause:** `hideTypingIndicator()` animation end callback calls
  `chatContainer.removeView(ti)` even after `ti` was already removed
  (race condition between animation end and direct removal).
- **Fix:** `ti.getParent() == chatContainer` check before remove.

#### Crash 4 — `NullPointerException` in `addToolBubble`, `addSystemNotice`, `rebuildChat`
- All called `chatContainer.addView()` or `removeAllViews()` without null check.
- **Fix:** All sites guarded with `if (chatContainer != null)`.

#### Crash 5 — CrashActivity never opens (Android 10+ background start restriction)
- **Root cause:** `startActivity()` from background/crash thread is silently blocked
  on Android 10+. The crash handler had no notification fallback — so when the main
  process died, nothing appeared on screen.
- **Fix:** KiraApp now posts a `💀 Kira crashed` notification via `NotificationManager`
  with a `PendingIntent`. Tapping notification opens `CrashActivity` reliably even
  after process death.

#### Crash 6 — `state.rs` encoding corruption
- 3 embedded null bytes in `state.rs` caused potential Rust build failures.
- **Fix:** Null bytes stripped.

#### Additional guards (16 unguarded NPE sites fixed)
- `inputField`, `headerSubtitle`, `historyList`, `sendBtn` — all now null-checked
  in lambdas, theme callbacks, and history card listeners.

---

### New: Crash Reporter System (Pure Rust + Java)

**`KiraApp` — crash handler:**
- Catches ALL uncaught exceptions (main thread + all background threads)
- Synchronous `SharedPreferences.commit()` before process dies (survives death)
- Calls `RustBridge.logCrash()` JNI directly (no HTTP, no thread needed)
- Posts HTTP to `localhost:7070/crash` as backup
- Posts `💀 Kira crashed` notification (high priority, Catppuccin Pink)
- Launches `CrashActivity` in `:crash` process
- Falls through to system default handler

**`CrashActivity` — standalone crash viewer:**
- Runs in `:crash` process (survives main process death)
- **TRACE tab:** Colored stack trace
  - Kira frames → Lavender
  - Exception types / `Caused by` → Pink
  - `NullPointer`/`ClassCast` → Yellow
  - `at ` frames → muted
- **HISTORY tab:** All stored crashes
  - Loads from SharedPrefs (always available)
  - Polls Rust `GET /crash/log` for full 50-entry history
  - Each card tap-to-expand full trace
- **Buttons:** Copy | Restart | Ask Kira to Fix | Clear
- "Ask Kira to Fix" → copies prompt + opens MainActivity → auto-sends crash trace to AI

**Rust JNI bridge (`jni_bridge.rs`):**
- `logCrash(thread, message, trace, tsMs)` — stores up to 50 entries
- `getLatestCrash()` → `{"has_crash":bool,"ts":...,"thread":...,"message":...}`
- `getCrashLog()` → `{"count":N,"crashes":[...]}`
- `clearCrashLog()` — wipes all entries

**Rust HTTP endpoints (`http.rs`):**
- `POST /crash` — stores crash entry
- `GET /crash/log` — returns all entries with `ts_str` field
- `GET /crash/latest` — fast poll for latest
- `POST /crash/clear` — wipe log

---

## v0.0.1 — Foundation
- Rust automation engine (OpenClaw, NanoBot, ZeroClaw, Roubao, Open-AutoGLM)
- Neural Glass UI — 10 animation layers
- Catppuccin Mocha/Latte theme with auto system detection
- OTA update via GitHub Releases
