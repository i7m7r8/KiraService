## v0.1.8 — Session 4: Persistent SessionStore + LLM Compaction (2026-03-22)

### Session 4 — Full Session Model with Disk Persistence

**`gateway/sessions.rs`** — complete rewrite (~450 lines):
- `SessionStore` — authoritative session registry replacing bare `HashMap<String, Session>`
- Per-session LZ4-compressed transcript storage: `VecDeque<Vec<u8>>` (pack_turn/unpack_turn)
- `save_index()` — persists all session metadata to `/data/data/com.kira.service/sessions/index.json`
- `save_transcript(session_id)` — LZ4-compresses transcript, writes to `sessions/<id>.lz4`
- `load_from_disk()` — called at startup; restores all sessions + transcripts from disk
- `build_context(session_id)` — prepends compact summary as synthetic turns before live history
- `compact_collect_dropped()` — trims oldest turns, returns them for LLM summarisation
- `apply_compact_summary()` — stores LLM summary, marks session as compacted
- `needs_compact()` — true when token_estimate > compact_threshold (default 100K tokens)
- `delete_and_purge()` — removes from memory + deletes `.lz4` file + saves index
- `prune_inactive(ttl_ms)` — bulk-delete + purge sessions inactive longer than TTL
- `Session` — new fields: `token_estimate`, `compacted`, `compact_summary`
- Minimal JSON helpers (no serde): `extract_str`, `extract_u64`, `split_json_array`

**`ai/compaction.rs`** — full Session 4 upgrade (~200 lines):
- `compact_session()` — main entry point: collect dropped turns → call LLM → store summary
- `COMPACTION_PROMPT` — OpenClaw-parity summarisation prompt (3–6 sentence concise summary)
- `call_llm_for_summary()` — thin wrapper calling existing `call_llm_sync` with no history
- `extract_content_text()` — parses OpenAI response JSON to get text content
- `CompactionConfig` — `token_threshold()` helper; default 85% of 128K = ~108K tokens
- Legacy `compact_turns()` / `needs_compaction()` kept for `/ai/agent` + `/ai/chain` paths

**`http.rs`** — `/ai/chat` wired to SessionStore + 5 new routes:
- `/ai/chat` now records every user+assistant turn to `session_store`, saves to disk after each reply
- Auto-compaction: spawns background thread if `needs_compact()` returns true after reply
- Context building: `session_store.build_context()` used (includes compact summary if present)
- `GET  /sessions/v2` — list all sessions sorted by recency with full metadata
- `GET  /sessions/v2/:key` — full session metadata + all transcript turns
- `DELETE /sessions/v2/:key` — delete session from memory and disk
- `POST /sessions/v2/:key/compact` — force-compact a session now (calls LLM)
- `POST /sessions/v2/prune` — prune sessions inactive > `ttl_hours` (default 72h)

**`lib.rs`** — `KiraState`:
- New field: `pub session_store: SessionStore` — authoritative store
- New field: `pub last_panic: String` — for JNI panic hook
- Legacy `sessions: HashMap<String, Session>` kept for backwards compat

**`jni_bridge.rs`** — `startServer`:
- Calls `session_store.load_from_disk()` at startup to restore all sessions
- Ensures `default` session always exists in store after load

---

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

## v58 — Sessions 3-6, 9-10: Sub-agents, Persistence, Memory, Skills, Cron, Webhooks (2026-03-22)

### Session 3 — Sub-Agent Spawning
- `ai/subagents.rs` — Full implementation: `SUBAGENT_REGISTRY` global, `spawn_subagent()`,
  depth-limit enforcement (max 5), isolated ReAct loop per agent, kill support
- Routes: `POST /agents/spawn`, `POST /agents/kill`, `GET /agents/list`,
  `GET /agents/running`, `GET /agents/:id/status`

### Session 4 — Persistent Sessions
- `gateway/persistence.rs` — New module: LZ4-compressed session transcripts on disk
- Routes: `GET /sessions/persist/list`, `POST /sessions/persist/save`,
  `POST /sessions/persist/load`, `DELETE /sessions/persist`

### Session 5 — Memory Persistence + Add/Delete
- `gateway/persistence.rs` — Memory index save/load (LZ4), embeddings save/load
- Routes: `POST /memory/add`, `DELETE /memory`,
  `POST /memory/persist/save`, `POST /memory/persist/load`

### Session 6 — Skills from Disk
- `gateway/persistence.rs` — Skill .md file load/save/delete
- Routes: `POST /skills/install`, `DELETE /skills`, `POST /skills/reload`

### Session 9 — Full Cron Scheduler
- `lib.rs` `run_cron_scheduler()` — Now actually runs isolated AI agent per job
- Routes: `POST /cron/create`, `POST /cron/run_now`, `GET /cron/log`
- Sub-agent pruning every 60s in cron loop
- Cron run log persisted to disk as JSONL

### Session 10 — Webhooks
- Routes: `POST /webhooks/register` (returns token + URL),
  `GET /webhooks`, `DELETE /webhooks`
- `POST /webhook/:token` — inbound trigger: runs AI agent with payload as goal
- HMAC token generation, fire count tracking

### Internal
- `register_subagent_shims()` called at startServer
- `get_llm_config_snapshot()` helper shared across cron, sub-agents, webhooks
- `push_event_feed()` refactored to delegate to `s_push_event()`

## v59 — Sessions 7+8: Telegram + WhatsApp (2026-03-22)

### Session 7 — Telegram Full Parity
- `channels/telegram.rs` — Full Rust Telegram Bot API client (509 lines)
  - `start_polling_loop()` — background thread, long-polling getUpdates
  - `send_message()` / `edit_message()` — send and streaming-edit
  - `send_with_keyboard()` — inline keyboard buttons (approval flow)
  - `answer_callback()` — dismiss button spinner
  - `send_typing()` — typing indicator
  - `markdown_to_md_v2()` — convert Markdown to MarkdownV2 format
  - `escape_md_v2()` — escape special characters
  - `parse_updates()` — robust JSON parser for getUpdates response
  - DM policy: pairing codes for unknown senders, open mode
  - `TG_STATE` global — config, last_update_id, pending_sends, log
- Routes: `POST /telegram/configure`, `POST /telegram/send`,
  `GET /telegram/status`, `POST /telegram/pairing/approve`
- Auto-starts polling at startServer if tg_token is configured

### Session 8 — WhatsApp
- `channels/whatsapp.rs` — Dual-mode WhatsApp adapter (345 lines)
  - Mode A (Cloud API): direct Meta Graph API calls, no Java needed
  - Mode B (bridge): Java Baileys bridge POSTs to Rust
  - `cloud_send_text()` — Cloud API send
  - `cloud_mark_read()` — mark message read
  - `parse_cloud_webhook()` — parse Meta webhook payloads
  - `process_inbound()` — DM policy + allowlist + AI dispatch
  - Pairing codes for unauthorized senders
- Routes: `POST /whatsapp/configure`, `POST /whatsapp/send`,
  `GET|POST /whatsapp/webhook` (Cloud API verification + inbound),
  `POST /whatsapp/bridge/incoming`, `GET /whatsapp/bridge/next_send`,
  `GET /whatsapp/status`, `POST /whatsapp/pairing/approve`

### Internal
- `register_channel_shims()` called at startServer
- `channel_ai_reply_tg()` / `channel_ai_reply_wa()` — wired to full ReAct loop

## v60 — Sessions 11-19 (2026-03-22)

### Session 11 — Canvas (A2UI)
- Routes: GET /canvas (serves HTML), POST /canvas/push, POST /canvas/reset,
  GET /canvas/state, GET /canvas/stream (SSE for WebView)
- KiraState: canvas_state, canvas_seq
- CANVAS_HTML constant — full A2UI host with SSE long-poll loop

### Session 12 — Browser Tool
- Routes: POST /browser/navigate, POST /browser/snapshot, GET /browser/snapshot,
  POST /browser/act, GET /browser/pending_command, GET /browser/status
- JNI: onBrowserSnapshot(), getBrowserPendingCommand()
- KiraState: browser_snapshot, browser_snapshot_ts, browser_pending_cmd

### Session 13 — Voice / TTS
- Routes: POST /voice/start, POST /voice/audio_chunk, POST /voice/stop,
  POST /voice/transcript, GET /voice/status, GET /voice/tts_text
- JNI: onVoiceChunk(), onVoiceTtsReady(), getVoiceTtsText()
- KiraState: voice_status, voice_audio_chunks, voice_tts_pending, voice_transcript

### Session 14 — Notification Intelligence
- Routes: POST /notifications/trigger/add, DELETE /notifications/trigger,
  GET /notifications/triggers, POST /notifications/clear
- JNI: onNotification() with importance-based proactive AI firing
- check_notif_keyword_triggers(): spawns AI agent for HIGH importance matches
- KiraState: notif_keyword_triggers

### Session 15 — Java Action Queue
- Routes: GET /java/pending_action, POST /java/action_result, GET /java/action_result
- JNI: getPendingJavaAction(), deliverJavaActionResult()
- KiraState: pending_java_actions, java_action_results

### Session 16 — Multi-Agent Routing
- Routes: GET /routing/agents, POST /routing/agents, DELETE /routing/agents
- Struct: AgentRouteConfig (id, name, persona, model, channels, skill_ids)
- KiraState: agent_configs

### Session 17 — Model Failover
- Routes: GET /models/failover, POST /models/failover/add,
  POST /models/failover/mark_error, POST /models/failover/pick
- Struct: ModelEntry (id, provider, model, priority, error_count, rate_limit_ms)
- KiraState: model_failover_chain

### Session 18 — Security / DM Policy + Allowlists
- Routes: GET /security/pairing/pending, POST /security/pairing/approve,
  GET /security/allowlists, POST /security/allowlists/add, DELETE /security/allowlists
- KiraState: pairing_codes, channel_allowlists

### Session 19 — Control UI (Web Dashboard)
- Routes: GET /ui, GET /ui/dashboard
- CONTROL_UI_HTML constant — full dashboard (chat, memory, agents, cron)
- Real-time stats via /ui/dashboard JSON: uptime, requests, tools, memory, etc.

## v60 — Sessions 11-19: Canvas, Browser, Voice, Notifications, Device Tools, Routing, Failover, Security, Control UI (2026-03-22)

### Session 11 — Canvas (A2UI)
- Routes: GET /canvas (serves HTML), POST /canvas/push, POST /canvas/reset,
  GET /canvas/state, GET /canvas/stream (SSE for WebView)
- CANVAS_HTML constant — full A2UI WebView host with SSE polling

### Session 12 — Browser Tool
- Routes: POST /browser/navigate, POST /browser/snapshot, GET /browser/snapshot,
  POST /browser/act, GET /browser/pending_command, GET /browser/status
- JNI: onBrowserSnapshot(), getBrowserPendingCommand()
- State: browser_snapshot, browser_snapshot_ts, browser_pending_cmd

### Session 13 — Voice / TTS
- Routes: POST /voice/start, POST /voice/audio_chunk, POST /voice/stop,
  POST /voice/transcript, GET /voice/status, GET /voice/tts_text
- JNI: onVoiceChunk(), onVoiceTtsReady(), getVoiceTtsText()
- State: voice_status, voice_audio_chunks, voice_tts_pending, voice_transcript
- Full ReAct loop triggered from voice transcript

### Session 14 — Notification Intelligence
- Proactive AI on keyword-matched notifications (importance >= HIGH)
- Routes: POST /notifications/trigger/add, DELETE /notifications/trigger,
  GET /notifications/triggers, POST /notifications/clear
- JNI: onNotification() with importance level
- State: notif_keyword_triggers

### Session 15 — Java Action Queue
- Routes: GET /java/pending_action, POST /java/action_result, GET /java/action_result
- JNI: getPendingJavaAction(), deliverJavaActionResult()
- State: pending_java_actions, java_action_results

### Session 16 — Multi-Agent Routing
- Routes: GET /routing/agents, POST /routing/agents, DELETE /routing/agents
- State: agent_configs (Vec<AgentRouteConfig>)
- AgentRouteConfig: id, name, persona, model, channels, skill_ids, memory_scope

### Session 17 — Model Failover
- Routes: GET /models/failover, POST /models/failover/add,
  POST /models/failover/mark_error, POST /models/failover/pick
- State: model_failover_chain (Vec<ModelEntry>)
- Priority-based selection, error counting, rate-limit tracking

### Session 18 — Security
- Routes: GET /security/pairing/pending, POST /security/pairing/approve,
  GET /security/allowlists, POST /security/allowlists/add, DELETE /security/allowlists
- State: pairing_codes, channel_allowlists

### Session 19 — Control UI
- Routes: GET /ui, GET /ui/dashboard
- CONTROL_UI_HTML — full dashboard: chat, memory, agents, cron
- Live refresh every 5s, uses /ai/run + /ai/run/status for streaming

### RustBridge.java
- Added: onBrowserSnapshot, getBrowserPendingCommand
- Added: onVoiceChunk, onVoiceTtsReady, getVoiceTtsText
- Added: onNotification, getPendingJavaAction, deliverJavaActionResult

## v60 — Sessions 11-19: Canvas, Browser, Voice, Notifications, Device Tools, Routing, Failover, Security, Control UI (2026-03-22)

### Session 11 — Canvas (A2UI)
- Routes: GET /canvas (serves HTML), POST /canvas/push, POST /canvas/reset, GET /canvas/state, GET /canvas/stream (SSE)
- CANVAS_HTML constant — A2UI host page, polls /canvas/stream every 500ms, renders text/html payloads
- AI tools: canvas.push, canvas.reset via dispatch

### Session 12 — Browser Tool
- JNI: onBrowserSnapshot(json), getBrowserPendingCommand()
- Routes: POST /browser/navigate, POST /browser/snapshot, GET /browser/snapshot, POST /browser/act, GET /browser/pending_command, GET /browser/status
- RustBridge.java: +onBrowserSnapshot, +getBrowserPendingCommand

### Session 13 — Voice / TTS
- JNI: onVoiceChunk(base64pcm), onVoiceTtsReady(text), getVoiceTtsText()
- Routes: POST /voice/start, POST /voice/audio_chunk, POST /voice/stop, POST /voice/transcript, GET /voice/status, GET /voice/tts_text
- Full AI loop on transcript, result queued for Java TTS
- RustBridge.java: +onVoiceChunk, +onVoiceTtsReady, +getVoiceTtsText

### Session 14 — Notification Intelligence
- JNI: onNotification(pkg, title, text, importance) — fires proactive AI on keyword match
- Routes: POST /notifications/trigger/add, DELETE /notifications/trigger, GET /notifications/triggers, POST /notifications/clear
- Proactive: importance≥3 notifications matched against keyword triggers → isolated AI agent
- RustBridge.java: +onNotification

### Session 15 — Java Action Queue (Device Tools)
- JNI: getPendingJavaAction(), deliverJavaActionResult(id, json)
- Routes: GET /java/pending_action, POST /java/action_result, GET /java/action_result?id=
- KiraState: +pending_java_actions VecDeque, +java_action_results HashMap
- RustBridge.java: +getPendingJavaAction, +deliverJavaActionResult

### Session 16 — Multi-Agent Routing
- KiraState: +agent_configs Vec<AgentRouteConfig>
- Routes: GET /routing/agents, POST /routing/agents, DELETE /routing/agents

### Session 17 — Model Failover
- KiraState: +model_failover_chain Vec<ModelEntry>
- Routes: GET /models/failover, POST /models/failover/add, POST /models/failover/mark_error, POST /models/failover/pick

### Session 18 — Security: DM Policy + Allowlists
- KiraState: +pairing_codes HashMap, +channel_allowlists HashMap
- Routes: GET /security/pairing/pending, POST /security/pairing/approve, GET /security/allowlists, POST /security/allowlists/add, DELETE /security/allowlists

### Session 19 — Control UI
- GET /ui → full single-page dashboard (HTML/JS, no external deps)
- GET /ui/dashboard → JSON metrics snapshot
- Dashboard: uptime, requests, tool_calls, memory, skills, cron, sub-agents, Telegram, voice status
- Chat, Memory add/search, Agent status, Cron management — all via fetch to Rust endpoints
- CONTROL_UI_HTML constant — ~50 lines vanilla JS

## v62 — Session 20: Real Streaming AI + Integration Fixes (2026-03-22)

### The Real Gap Fixed
Previous sessions added HTTP routes that Java never called. This session
rewrites the actual Java-Rust integration to deliver OpenClaw-parity features.

### KiraAI.java — Full SSE Streaming (447 lines)
- `callLlmStreaming()` — OkHttp SSE stream parser
  - Reads `data: {...}` lines, accumulates `delta.content` chunks
  - Fires `cb.onPartial(text)` throttled to 200ms for real-time UI updates
  - Handles `tool_calls` deltas — accumulates for Rust to parse
  - Reconstructs non-streaming JSON response for Rust `processLlmReply`
- New `Callback.onPartial(String)` method — streaming chunk callback
- `SimpleCallback` adapter for callers that don't need streaming
- Added `write_file:` shell job handler (was missing)
- Fixed tool loop to properly handle all tool result formats

### KiraTelegram.java — Streaming Telegram Replies (291 lines)
- Sends "🤔 Thinking..." placeholder message immediately
- `onPartial()` → `editMessageText()` every 800ms — live streaming in Telegram
- `onTool()` → shows "🔧 toolname..." prefix while tools execute
- `onReply()` → final edit to clean response (removes spinner)
- Edit failure fallback → sends new message
- Proper JSON escaping in all API calls

### KiraNotificationService.java — Proactive AI
- Now calls `RustBridge.onNotification(pkg, title, text, importance)` 
  in addition to `pushNotification`
- Passes channel importance (0-5) — keyword triggers only fire for importance≥3
- This completes the Session 14 notification intelligence pipeline

### What actually works end-to-end now
1. User sends Telegram message → KiraTelegram polls getUpdates
2. "Thinking..." message appears instantly
3. KiraAI starts streaming LLM call
4. Every 800ms: Telegram message edits in-place with current text
5. Tool calls execute → "🔧 tool..." prefix shown
6. Final reply replaces placeholder
7. Notifications with keywords → proactive AI agent fires automatically
