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

## v0.0.3 — Crash Reporter Fixed + OTA Fixed

### Crash Reporter — Why It Never Appeared

**Root cause:** `startActivity(CrashActivity)` inside `UncaughtExceptionHandler` is
silently blocked by Android 10+ when called from a dying background thread.
The process is dead before Android allows a background activity start.

**Fix:** Replaced `startActivity()` with `AlarmManager.setExact()`.
AlarmManager is a system-level mechanism that fires 600ms after the dying process
schedules it — the process itself is already dead by then, but Android still fires
the alarm and launches CrashActivity. This is the standard reliable pattern.

Added `SCHEDULE_EXACT_ALARM` + `USE_EXACT_ALARM` to manifest (required Android 12+).

### Crash Notification — Why It Didn't Show

**Root cause:** Deprecated `Notification.PRIORITY_MAX` field was set on API 26+ where
channel importance controls priority — not the field. The notification channel was
`IMPORTANCE_HIGH` but the deprecated field conflicted.

**Fix:** Only set `PRIORITY_MAX` on API < 26. On API 26+ let channel importance control it.

### OTA Install — Why Tapping Install Did Nothing

**Bug 1 — Double download:** `startDownload()` was called twice simultaneously —
once by user tapping INSTALL (via `doInstall` runnable) and once automatically
when `ShizukuShell.isAvailable()`. The `AtomicBoolean` guard blocked the second call,
but race conditions meant neither completed properly.
**Fix:** Removed auto-start. User tap is the only trigger.

**Bug 2 — PackageInstaller session reuse:** `installViaPackageInstaller` called
`pi.openSession(sid)` twice — once to write the APK, once to commit.
The second `openSession()` on an already-written session threw `IllegalStateException`,
silently caught and fell to `installViaIntent` which also failed in background context.
**Fix:** Use separate try-with-resources for commit session.

**Bug 3 — Intent install background restriction:** `installViaIntent` with
`ApplicationContext` fails on Android 10+ from background.
**Fix:** Added `FLAG_ACTIVITY_CLEAR_TOP`, plus notification fallback that posts
a "tap to install" notification if all else fails.

### KiraAI — Why Send Crashes Silently

**Root cause:** `catch (Exception e)` does NOT catch JNI errors. Rust panics,
`UnsatisfiedLinkError`, `OutOfMemoryError`, `StackOverflowError` are all `Error`
subclasses — they bypass `Exception` catches and propagate to `UncaughtExceptionHandler`
which then can't show CrashActivity (see above).
**Fix:** Changed to `catch (Throwable e)` — catches both `Exception` and `Error`.
Added `RustBridge.isLoaded()` pre-flight check with clear error message.

## v56 — Session 1: OpenClaw Module Split (2026-03-22)

### Added
- `rust-core/src/ai/` — AI module: `runner.rs` (AiRunStatus, AiRunRequest, Turn, LoopDetector), `models.rs` (ModelConfig, FailoverChain), `tools.rs` (ToolCall, ToolResult, ToolRegistry), `subagents.rs` (SubAgentRegistry), `compaction.rs` (compact_turns)
- `rust-core/src/channels/` — Channel module: `shared.rs` (InboundMessage, OutboundMessage, DmPolicy), `telegram.rs` (TelegramConfig, escape_md_v2, update parser)
- `rust-core/src/memory/` — Memory module: `index.rs` (MemoryStore, cosine_similarity, temporal decay), `search.rs` (MMR re-ranking)
- `rust-core/src/scheduling/` — Scheduling module: `cron.rs` (CronSchedule, interval parser, is_due), `webhooks.rs` (WebhookRegistration)
- `rust-core/src/skills/` — Skills module: full registry + YAML frontmatter parser
- `rust-core/src/gateway/` — Gateway module: `sessions.rs` (SessionStore, TranscriptTurn), `routing.rs` (RouteKey, AgentConfig), `security.rs` (PairingRequest, pairing code generator)
- `rust-core/src/tools/` — Tool implementations: system (read_file, write_file, list_files, run_shell), memory (add_memory, search_memory, list_memories), device (get_notifications, get_location, send_sms, list_contacts, list_calendar, take_photo)
- `rust-core/src/automation/` — Automation module boundary (logic still in lib.rs)
- New routes: GET /ai/run/status, GET /ai/tools/schema, GET /sessions/v2, GET /memory/v2/search, GET /skills/v2, GET /cron/v2, GET /agents/v2, GET /channels/status, GET /modules/health

### Changed
- `lib.rs` — Added `pub mod` declarations for all 8 new modules at top of file
- `lib.rs` — Added `route_openclaw_modules()` function wired into catch-all chain
- `Cargo.toml` — Bumped version to 0.1.2

### No breaking changes — all existing functionality preserved

## v57 — Session 2: ReAct AI Loop (2026-03-22)

### Added
- `ai/runner.rs` — Full ReAct loop implementation (`run_agent()`)
  - Multi-step THINK→ACT→OBSERVE with configurable `max_steps` (default 25)
  - `parse_tool_calls_json()` — OpenAI function-calling JSON format parser
  - `build_messages_json()` — assembles Turn slice into LLM messages array
  - `LoopDetector` — detects repeated (tool, params) within a window of 6
  - In-place context compaction at >100 non-system turns (keeps last 60)
  - `RUN_STATE` global (lazy_static Arc<Mutex>) — live status for polling
  - `register_dispatch()` + `register_llm_call()` — OnceLock function pointers
    avoid circular dependency between runner and lib.rs
- `lib.rs` — New routes:
  - `POST /ai/run` — non-blocking, spawns worker thread, returns immediately
  - `POST /ai/run/abort` — sets abort flag, worker exits cleanly next step
  - `GET /ai/run/status` — live status from RUN_STATE (replaces v56 stub)
- `lib.rs` — `build_kira_tools_schema()` — builds OpenAI tool schema JSON
  for all 20 registered tools from the allowlist
- `lib.rs` — `register_runner_shims()` called at startServer, wires
  `dispatch_for_runner` and `llm_call_for_runner` into runner module

### No breaking changes — POST /ai/chat continues to work unchanged
