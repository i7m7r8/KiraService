## v0.0.2 — Neural Glass UI + Full Automation Engine

### New Features

**Layer 0 — Living Canvas**
- 3-depth star field with gyroscope parallax (0.3×/0.8×/1.5× per layer)
- Chromatic hue pulse synchronized to Rust uptime (3s sine wave, ±12° HSV)
- Vortex animation when Kira is thinking — stars spiral toward center
- Burst explosion on reply — stars push outward, 60fps spring physics
- Per-star twinkle with unique phase offsets

**Layer 1 — Neural Nav Bar**
- Floating island: Catppuccin Mantle 94% opacity, 24dp corners, elevation 8dp
- Active tab icon grows with OvershootInterpolator(2.8) in 250ms
- Radial Lavender aura blooms beneath active tab (0→32dp, 180ms)
- Nav bar floats up 4dp + 0.97 scale when keyboard opens
- Geometric glyphs: ⬡ Chat, ⠿ Tools, ☰ Log, ⧉ System

**Layer 2 — Chat Interface**
- K badge rotates 360° on every message send
- Subtitle crossfades: ready → thinking → reasoning → processing → composing
- User bubbles spring in from right (+40dp, OvershootInterpolator)
- Kira bubbles spring in from left (−40dp) with Lavender left border
- Sinusoidal typing indicator (3 dots, 120ms stagger, 4dp amplitude)
- Header border pulses Lavender while Kira processes, stops on reply

**Layer 3 — Input Composer**
- Send button: press springs 1.0→0.88→1.05→1.0 (80ms/120ms/80ms)
- Input field: Lavender border appears on focus, fades on blur
- Send button pulses alpha 85%↔100% when field has text
- Keyboard detection via ViewTreeObserver → nav bar float

**Rust Engine v9.1**
- `/theme/anim` endpoint: phase, bpm, activity_level, is_thinking (polled 500ms)
- `/theme/thinking` endpoint: toggles vortex ON/OFF from UI
- ThemeConfig extended with 4 live animation fields
- 17 automation HTTP routes: `/auto/if_then`, `/auto/watch_app`, `/auto/repeat`,
  `/auto/templates`, `/auto/scene`, `/auto/run_now`, `/auto/pause`, `/auto/history`,
  `/auto/stats`, `/auto/clone`, `/auto/batch_enable` + more
- 280+ app package database in `app_name_to_pkg()`
- `parse_nl_condition()`: natural language → trigger kind (battery/screen/wifi/time/app)
- OTA engine: 9 JNI functions, SHA256 verify, 3-tier install (Shizuku/PackageInstaller/Intent)
- Per-ABI APK splits: arm64-v8a, armeabi-v7a, x86_64, universal

### Bug Fixes
- Fixed 57 Java compile errors (unescaped JSON keys in KiraTools string literals)
- Fixed 4 KiraWatcher API mismatches (nextMacroAction args, KiraEventBus.post, tools.execute)
- Fixed u64 type mismatch in `/auto/stats` total_runs sum
- Fixed stray closing brace in jni_bridge module
- Fixed MacroTriggerKind Display error (→ .to_str())
- Fixed MacroAction field error (action_type → kind.to_str())
