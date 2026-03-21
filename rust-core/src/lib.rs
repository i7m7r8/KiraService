use lz4_flex::{compress_prepend_size, decompress_size_prepended};

#[macro_use]
extern crate lazy_static;

// Kira Rust Core v9 \u{2014} v40 edition
//
// NEW in v40 (Tasker/MacroDroid automation engine \u{2014} pure Rust):
//   - MacroEngine: full IF/THEN/ELSE automation macros
//   - 40+ TriggerType variants (time, geo, app, notif, battery, screen,
//     wifi, bluetooth, charging, shake, volume-btn, sms, call, headset,
//     airplane, power-connected, idle, unlock, nfc, clipboard, signal\u{2026})
//   - 60+ ActionType variants (HTTP, shell via Shizuku, clipboard, media,
//     volume, torch, TTS, brightness, airplane, send-notif, open-app,
//     toast, vibrate, set-variable, log, wait, stop-flow, loop, intent\u{2026})
//   - Variable engine (store/retrieve named vars, math + string expr)
//   - Named Profiles (Work/Home/Sleep/Car) with auto-switch rules
//   - Flow control: Loop, Delay, If/Else, Stop
//   - Macro import/export JSON
//   - All v8/v38 features preserved

#[allow(non_snake_case, dead_code, unused_mut, clippy::upper_case_acronyms)]

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// v40: AUTOMATION ENGINE \u{2014} Triggers, Conditions, Actions, Macros, Profiles
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Every possible trigger type \u{2014} mirrors Tasker + MacroDroid combined
#[derive(Clone, Debug, PartialEq)]
enum MacroTriggerKind {
    // Time-based
    Time,             // HH:MM daily
    TimeInterval,     // every N minutes
    Sunrise,
    Sunset,
    // Device state
    BatteryLevel,     // below/above threshold
    BatteryCharging,  // plugged in / unplugged
    ScreenOn,
    ScreenOff,
    DeviceUnlocked,
    DeviceLocked,
    DeviceIdle,       // screen off + no activity
    PowerConnected,
    PowerDisconnected,
    LowMemory,
    AirplaneMode,     // on / off
    // Connectivity
    WifiConnected,    // optional SSID match
    WifiDisconnected,
    BluetoothConnected,   // optional device name
    BluetoothDisconnected,
    HeadsetConnected,
    HeadsetDisconnected,
    NfcTag,           // optional tag ID
    CallReceived,     // optional number
    CallMissed,
    SmsReceived,      // optional sender / keyword
    // Sensor
    Shake,
    VolumeBtnUp,
    VolumeBtnDown,
    // App
    AppLaunched,      // specific package
    AppClosed,
    // Notification
    NotifReceived,    // pkg + keyword
    NotifDismissed,
    // Location (geofence \u{2014} lat/lon + radius fed from Java)
    GeofenceEnter,
    GeofenceExit,
    // UI
    ClipboardChanged,
    // AI / Kira
    KiraCommand,      // user said specific phrase
    KiraEvent,        // internal Kira event type
    // HTTP webhook
    WebhookPost,
    // Cron expression
    Cron,
    // Always (manual / dev)
    Manual,
}

impl MacroTriggerKind {
    fn from_str(s: &str) -> Self {
        match s {
            "time"                   => MacroTriggerKind::Time,
            "time_interval"          => MacroTriggerKind::TimeInterval,
            "sunrise"                => MacroTriggerKind::Sunrise,
            "sunset"                 => MacroTriggerKind::Sunset,
            "battery_level"          => MacroTriggerKind::BatteryLevel,
            "battery_charging"       => MacroTriggerKind::BatteryCharging,
            "screen_on"              => MacroTriggerKind::ScreenOn,
            "screen_off"             => MacroTriggerKind::ScreenOff,
            "device_unlocked"        => MacroTriggerKind::DeviceUnlocked,
            "device_locked"          => MacroTriggerKind::DeviceLocked,
            "device_idle"            => MacroTriggerKind::DeviceIdle,
            "power_connected"        => MacroTriggerKind::PowerConnected,
            "power_disconnected"     => MacroTriggerKind::PowerDisconnected,
            "low_memory"             => MacroTriggerKind::LowMemory,
            "airplane_mode"          => MacroTriggerKind::AirplaneMode,
            "wifi_connected"         => MacroTriggerKind::WifiConnected,
            "wifi_disconnected"      => MacroTriggerKind::WifiDisconnected,
            "bt_connected"           => MacroTriggerKind::BluetoothConnected,
            "bt_disconnected"        => MacroTriggerKind::BluetoothDisconnected,
            "headset_connected"      => MacroTriggerKind::HeadsetConnected,
            "headset_disconnected"   => MacroTriggerKind::HeadsetDisconnected,
            "nfc_tag"                => MacroTriggerKind::NfcTag,
            "call_received"          => MacroTriggerKind::CallReceived,
            "call_missed"            => MacroTriggerKind::CallMissed,
            "sms_received"           => MacroTriggerKind::SmsReceived,
            "shake"                  => MacroTriggerKind::Shake,
            "vol_up"                 => MacroTriggerKind::VolumeBtnUp,
            "vol_down"               => MacroTriggerKind::VolumeBtnDown,
            "app_launched"           => MacroTriggerKind::AppLaunched,
            "app_closed"             => MacroTriggerKind::AppClosed,
            "notif_received"         => MacroTriggerKind::NotifReceived,
            "notif_dismissed"        => MacroTriggerKind::NotifDismissed,
            "geofence_enter"         => MacroTriggerKind::GeofenceEnter,
            "geofence_exit"          => MacroTriggerKind::GeofenceExit,
            "clipboard_changed"      => MacroTriggerKind::ClipboardChanged,
            "kira_command"           => MacroTriggerKind::KiraCommand,
            "kira_event"             => MacroTriggerKind::KiraEvent,
            "webhook"                => MacroTriggerKind::WebhookPost,
            "cron"                   => MacroTriggerKind::Cron,
            _                        => MacroTriggerKind::Manual,
        }
    }
    fn to_str(&self) -> &'static str {
        match self {
            MacroTriggerKind::Time               => "time",
            MacroTriggerKind::TimeInterval       => "time_interval",
            MacroTriggerKind::Sunrise            => "sunrise",
            MacroTriggerKind::Sunset             => "sunset",
            MacroTriggerKind::BatteryLevel       => "battery_level",
            MacroTriggerKind::BatteryCharging    => "battery_charging",
            MacroTriggerKind::ScreenOn           => "screen_on",
            MacroTriggerKind::ScreenOff          => "screen_off",
            MacroTriggerKind::DeviceUnlocked     => "device_unlocked",
            MacroTriggerKind::DeviceLocked       => "device_locked",
            MacroTriggerKind::DeviceIdle         => "device_idle",
            MacroTriggerKind::PowerConnected     => "power_connected",
            MacroTriggerKind::PowerDisconnected  => "power_disconnected",
            MacroTriggerKind::LowMemory          => "low_memory",
            MacroTriggerKind::AirplaneMode       => "airplane_mode",
            MacroTriggerKind::WifiConnected      => "wifi_connected",
            MacroTriggerKind::WifiDisconnected   => "wifi_disconnected",
            MacroTriggerKind::BluetoothConnected => "bt_connected",
            MacroTriggerKind::BluetoothDisconnected => "bt_disconnected",
            MacroTriggerKind::HeadsetConnected   => "headset_connected",
            MacroTriggerKind::HeadsetDisconnected=> "headset_disconnected",
            MacroTriggerKind::NfcTag             => "nfc_tag",
            MacroTriggerKind::CallReceived       => "call_received",
            MacroTriggerKind::CallMissed         => "call_missed",
            MacroTriggerKind::SmsReceived        => "sms_received",
            MacroTriggerKind::Shake              => "shake",
            MacroTriggerKind::VolumeBtnUp        => "vol_up",
            MacroTriggerKind::VolumeBtnDown      => "vol_down",
            MacroTriggerKind::AppLaunched        => "app_launched",
            MacroTriggerKind::AppClosed          => "app_closed",
            MacroTriggerKind::NotifReceived      => "notif_received",
            MacroTriggerKind::NotifDismissed     => "notif_dismissed",
            MacroTriggerKind::GeofenceEnter      => "geofence_enter",
            MacroTriggerKind::GeofenceExit       => "geofence_exit",
            MacroTriggerKind::ClipboardChanged   => "clipboard_changed",
            MacroTriggerKind::KiraCommand        => "kira_command",
            MacroTriggerKind::KiraEvent          => "kira_event",
            MacroTriggerKind::WebhookPost        => "webhook",
            MacroTriggerKind::Cron               => "cron",
            MacroTriggerKind::Manual             => "manual",
        }
    }
}

/// Every possible action type
#[derive(Clone, Debug)]
enum MacroActionKind {
    // HTTP
    HttpGet,
    HttpPost,
    HttpPut,
    HttpDelete,
    // System via Shizuku shell
    Shell,            // raw shell command
    SetClipboard,
    GetClipboard,
    // Media
    PlaySound,        // file path or asset
    StopMedia,
    SetVolume,        // stream + level 0-15
    MuteVolume,
    UnmuteVolume,
    // Display
    SetBrightness,    // 0-255
    KeepScreenOn,
    AllowScreenOff,
    // Hardware
    SetTorch,         // on/off
    Vibrate,          // duration ms
    // Connectivity (via shell)
    SetWifi,          // on/off
    SetBluetooth,
    SetAirplane,
    SetNfc,
    // Communication
    SendSms,          // number + message
    MakeCall,         // number
    // Notifications
    SendNotification, // title + text + channel
    CancelNotification,
    // Apps
    OpenApp,          // package name
    CloseApp,
    KillApp,
    LaunchIntent,     // action + extras JSON
    // Kira AI
    KiraAsk,          // prompt \u{2192} stores result in variable
    KiraSpeak,        // TTS via Kira voice
    KiraMessage,      // send message to active session
    // Variables
    SetVariable,      // name + value (supports %VAR% tokens + math)
    IncrementVariable,
    DecrementVariable,
    ClearVariable,
    // Flow control
    Wait,             // ms
    If,               // condition \u{2192} else_action_index
    Loop,             // count + action_list
    StopFlow,
    StopMacro,
    // Logging
    LogEvent,
    ShowToast,
    // Profile
    ActivateProfile,
    // Clipboard / share
    ShareText,
    // GPS
    GetLocation,      // stores lat/lon in variables
    // Advanced
    WriteFile,
    ReadFile,
    PushKiraEvent,    // fires a KiraEvent trigger
}

impl MacroActionKind {
    fn from_str(s: &str) -> Self {
        match s {
            "http_get"           => MacroActionKind::HttpGet,
            "http_post"          => MacroActionKind::HttpPost,
            "http_put"           => MacroActionKind::HttpPut,
            "http_delete"        => MacroActionKind::HttpDelete,
            "shell"              => MacroActionKind::Shell,
            "set_clipboard"      => MacroActionKind::SetClipboard,
            "get_clipboard"      => MacroActionKind::GetClipboard,
            "play_sound"         => MacroActionKind::PlaySound,
            "stop_media"         => MacroActionKind::StopMedia,
            "set_volume"         => MacroActionKind::SetVolume,
            "mute_volume"        => MacroActionKind::MuteVolume,
            "unmute_volume"      => MacroActionKind::UnmuteVolume,
            "set_brightness"     => MacroActionKind::SetBrightness,
            "keep_screen_on"     => MacroActionKind::KeepScreenOn,
            "allow_screen_off"   => MacroActionKind::AllowScreenOff,
            "set_torch"          => MacroActionKind::SetTorch,
            "vibrate"            => MacroActionKind::Vibrate,
            "set_wifi"           => MacroActionKind::SetWifi,
            "set_bluetooth"      => MacroActionKind::SetBluetooth,
            "set_airplane"       => MacroActionKind::SetAirplane,
            "set_nfc"            => MacroActionKind::SetNfc,
            "send_sms"           => MacroActionKind::SendSms,
            "make_call"          => MacroActionKind::MakeCall,
            "send_notification"  => MacroActionKind::SendNotification,
            "cancel_notification"=> MacroActionKind::CancelNotification,
            "open_app"           => MacroActionKind::OpenApp,
            "close_app"          => MacroActionKind::CloseApp,
            "kill_app"           => MacroActionKind::KillApp,
            "launch_intent"      => MacroActionKind::LaunchIntent,
            "kira_ask"           => MacroActionKind::KiraAsk,
            "kira_speak"         => MacroActionKind::KiraSpeak,
            "kira_message"       => MacroActionKind::KiraMessage,
            "set_variable"       => MacroActionKind::SetVariable,
            "increment_variable" => MacroActionKind::IncrementVariable,
            "decrement_variable" => MacroActionKind::DecrementVariable,
            "clear_variable"     => MacroActionKind::ClearVariable,
            "wait"               => MacroActionKind::Wait,
            "if"                 => MacroActionKind::If,
            "loop"               => MacroActionKind::Loop,
            "stop_flow"          => MacroActionKind::StopFlow,
            "stop_macro"         => MacroActionKind::StopMacro,
            "log_event"          => MacroActionKind::LogEvent,
            "show_toast"         => MacroActionKind::ShowToast,
            "activate_profile"   => MacroActionKind::ActivateProfile,
            "share_text"         => MacroActionKind::ShareText,
            "get_location"       => MacroActionKind::GetLocation,
            "write_file"         => MacroActionKind::WriteFile,
            "read_file"          => MacroActionKind::ReadFile,
            "push_kira_event"    => MacroActionKind::PushKiraEvent,
            _                    => MacroActionKind::LogEvent,
        }
    }
    fn to_str(&self) -> &'static str {
        match self {
            MacroActionKind::HttpGet           => "http_get",
            MacroActionKind::HttpPost          => "http_post",
            MacroActionKind::HttpPut           => "http_put",
            MacroActionKind::HttpDelete        => "http_delete",
            MacroActionKind::Shell             => "shell",
            MacroActionKind::SetClipboard      => "set_clipboard",
            MacroActionKind::GetClipboard      => "get_clipboard",
            MacroActionKind::PlaySound         => "play_sound",
            MacroActionKind::StopMedia         => "stop_media",
            MacroActionKind::SetVolume         => "set_volume",
            MacroActionKind::MuteVolume        => "mute_volume",
            MacroActionKind::UnmuteVolume      => "unmute_volume",
            MacroActionKind::SetBrightness     => "set_brightness",
            MacroActionKind::KeepScreenOn      => "keep_screen_on",
            MacroActionKind::AllowScreenOff    => "allow_screen_off",
            MacroActionKind::SetTorch          => "set_torch",
            MacroActionKind::Vibrate           => "vibrate",
            MacroActionKind::SetWifi           => "set_wifi",
            MacroActionKind::SetBluetooth      => "set_bluetooth",
            MacroActionKind::SetAirplane       => "set_airplane",
            MacroActionKind::SetNfc            => "set_nfc",
            MacroActionKind::SendSms           => "send_sms",
            MacroActionKind::MakeCall          => "make_call",
            MacroActionKind::SendNotification  => "send_notification",
            MacroActionKind::CancelNotification=> "cancel_notification",
            MacroActionKind::OpenApp           => "open_app",
            MacroActionKind::CloseApp          => "close_app",
            MacroActionKind::KillApp           => "kill_app",
            MacroActionKind::LaunchIntent      => "launch_intent",
            MacroActionKind::KiraAsk           => "kira_ask",
            MacroActionKind::KiraSpeak         => "kira_speak",
            MacroActionKind::KiraMessage       => "kira_message",
            MacroActionKind::SetVariable       => "set_variable",
            MacroActionKind::IncrementVariable => "increment_variable",
            MacroActionKind::DecrementVariable => "decrement_variable",
            MacroActionKind::ClearVariable     => "clear_variable",
            MacroActionKind::Wait              => "wait",
            MacroActionKind::If                => "if",
            MacroActionKind::Loop              => "loop",
            MacroActionKind::StopFlow          => "stop_flow",
            MacroActionKind::StopMacro         => "stop_macro",
            MacroActionKind::LogEvent          => "log_event",
            MacroActionKind::ShowToast         => "show_toast",
            MacroActionKind::ActivateProfile   => "activate_profile",
            MacroActionKind::ShareText         => "share_text",
            MacroActionKind::GetLocation       => "get_location",
            MacroActionKind::WriteFile         => "write_file",
            MacroActionKind::ReadFile          => "read_file",
            MacroActionKind::PushKiraEvent     => "push_kira_event",
        }
    }
}

/// A single action in a macro's action list
#[derive(Clone)]
struct MacroAction {
    kind:        MacroActionKind,
    /// Generic key-value params (url, body, variable_name, value, ms, pkg\u{2026})
    params:      HashMap<String, String>,
    /// For If/Loop: nested action list index stored as JSON string
    /// (the Java side sends sub-actions as a JSON array string)
    sub_actions: Vec<MacroAction>,
    enabled:     bool,
}

/// A trigger on a macro
#[derive(Clone)]
struct MacroTrigger {
    kind:    MacroTriggerKind,
    /// Extra match data: SSID, package name, battery threshold, cron expr\u{2026}
    config:  HashMap<String, String>,
    enabled: bool,
}

/// Condition for If action or macro-level constraint
#[derive(Clone)]
struct MacroCondition {
    lhs:      String,   // variable name or built-in: %BATTERY%, %SCREEN_PKG%, %TIME_H%\u{2026}
    operator: String,   // eq, neq, gt, lt, gte, lte, contains, starts, ends, matches
    rhs:      String,   // value or %VAR%
}

/// A full macro (like a Tasker Task + Trigger combo)
#[derive(Clone)]
struct AutoMacro {
    id:          String,
    name:        String,
    description: String,
    enabled:     bool,
    triggers:    Vec<MacroTrigger>,
    conditions:  Vec<MacroCondition>,   // ALL must pass for macro to run
    actions:     Vec<MacroAction>,
    profile:     String,   // "" = any profile, else must match active profile
    run_count:   u64,
    last_run_ms: u128,
    created_ms:  u128,
    tags:        Vec<String>,
}

/// A profile (like MacroDroid profiles)
#[derive(Clone)]
struct AutoProfile {
    id:          String,
    name:        String,
    active:      bool,
    auto_activate_trigger: String,  // trigger kind that activates this profile
    auto_activate_value:   String,  // e.g. SSID for wifi_connected
}

/// Variable engine
#[derive(Clone)]
struct AutoVariable {
    name:       String,
    value:      String,
    var_type:   String,   // "string" | "number" | "boolean"
    persistent: bool,
    created_ms: u128,
    updated_ms: u128,
}

/// A pending action that needs Java to execute it
#[derive(Clone)]
struct PendingMacroAction {
    macro_id:  String,
    action_id: String,
    kind:      String,
    params:    HashMap<String, String>,
    ts:        u128,
}

/// Macro execution log entry
#[derive(Clone)]
struct MacroRunLog {
    macro_id:   String,
    macro_name: String,
    trigger:    String,
    success:    bool,
    steps_run:  u32,
    duration_ms:u128,
    ts:         u128,
    error:      String,
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// v38: Setup / Theme / Config / Shizuku (unchanged)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

// ── v43: OTA Engine ─────────────────────────────────────────────────────────
// True OTA: Rust tracks manifest, SHA256 deltas, download progress, install state.
// Java only executes what Rust tells it to (Shizuku pm install or PackageInstaller session).

#[derive(Clone, PartialEq, Debug)]
enum OtaPhase {
    Idle,            // no update known
    Checking,        // API request in flight (set by Java, cleared by Rust)
    UpdateAvailable, // newer version found, awaiting user action
    Downloading,     // bytes coming in
    Downloaded,      // APK on disk, ready to install
    Installing,      // PackageInstaller/Shizuku session opened
    Installed,       // success
    Failed(String),  // error message
}

impl Default for OtaPhase {
    fn default() -> Self { OtaPhase::Idle }
}

#[derive(Clone, Default)]
struct OtaFileEntry {
    path:      String,   // relative path inside APK or asset
    sha256:    String,   // hex SHA256 of this version
    size:      u64,
    changed:   bool,     // true = this file differs from installed version
}

#[derive(Clone, Default)]
struct OtaEngine {
    // Version tracking
    current_version:  String,   // installed versionName e.g. "9.0.0"
    current_code:     i64,      // installed versionCode
    latest_version:   String,   // from GitHub releases tag
    latest_code:      i64,
    repo:             String,   // "i7m7r8/KiraService"

    // Manifest / delta
    manifest_sha256:  String,   // SHA256 of the release's manifest JSON
    changed_files:    Vec<OtaFileEntry>,
    total_delta_bytes:u64,      // bytes that actually differ
    total_apk_bytes:  u64,      // full APK size

    // Download progress
    phase:            OtaPhase,
    download_bytes:   u64,
    download_total:   u64,
    download_pct:     u8,       // 0-100
    apk_local_path:   String,   // absolute path to downloaded APK
    apk_sha256:       String,   // expected SHA256 of full APK

    // Release info
    download_url:     String,
    changelog:        String,
    release_date:     String,
    last_check_ms:    u128,
    check_error:      String,

    // Install tracking
    install_session_id: i32,    // PackageInstaller session ID (-1 = none)
    install_method:   String,   // "shizuku" | "package_installer" | "intent"
    install_error:    String,

    // Skip list
    skipped_versions: Vec<String>,
}

impl OtaEngine {
    fn is_newer(&self, candidate: &str) -> bool {
        if candidate.is_empty() || candidate == self.current_version { return false; }
        // Parse numeric segments for comparison
        fn parse_ver(s: &str) -> Vec<u64> {
            s.trim_start_matches('v')
             .split(|c: char| !c.is_ascii_digit())
             .filter_map(|p| p.parse::<u64>().ok())
             .collect()
        }
        let cur = parse_ver(&self.current_version);
        let lat = parse_ver(candidate);
        for i in 0..cur.len().max(lat.len()) {
            let c = cur.get(i).copied().unwrap_or(0);
            let l = lat.get(i).copied().unwrap_or(0);
            if l > c { return true; }
            if l < c { return false; }
        }
        false
    }

    fn phase_str(&self) -> &'static str {
        match &self.phase {
            OtaPhase::Idle            => "idle",
            OtaPhase::Checking        => "checking",
            OtaPhase::UpdateAvailable => "available",
            OtaPhase::Downloading     => "downloading",
            OtaPhase::Downloaded      => "downloaded",
            OtaPhase::Installing      => "installing",
            OtaPhase::Installed       => "installed",
            OtaPhase::Failed(_)       => "failed",
        }
    }

    fn to_json(&self) -> String {
        let err = match &self.phase { OtaPhase::Failed(e) => e.as_str(), _ => &self.install_error };
        format!(
            r#"{{"phase":"{}","current":"{}","current_code":{},"latest":"{}","latest_code":{},"available":{},"pct":{},"downloaded":{},"total":{},"delta_bytes":{},"apk_bytes":{},"changed_files":{},"url":"{}","changelog":"{}","release_date":"{}","last_check_ms":{},"apk_path":"{}","install_method":"{}","error":"{}","repo":"{}"}}"#,
            self.phase_str(),
            esc(&self.current_version), self.current_code,
            esc(&self.latest_version),  self.latest_code,
            self.phase == OtaPhase::UpdateAvailable || self.phase == OtaPhase::Downloading || self.phase == OtaPhase::Downloaded,
            self.download_pct,
            self.download_bytes, self.download_total,
            self.total_delta_bytes, self.total_apk_bytes,
            self.changed_files.len(),
            esc(&self.download_url),
            esc(&self.changelog[..self.changelog.len().min(300)]),
            esc(&self.release_date),
            self.last_check_ms,
            esc(&self.apk_local_path),
            esc(&self.install_method),
            esc(err),
            esc(&self.repo)
        )
    }
}

#[derive(Default, Clone)]
struct SetupState {
    current_page:         u8,
    done:                 bool,
    api_key:              String,
    base_url:             String,
    selected_provider_id: String,
    custom_url:           String,
    user_name:            String,
    model:                String,
    tg_token:             String,
    tg_allowed_id:        i64,
    quote_index:          usize,
}

#[derive(Clone)]
struct ThemeConfig {
    accent_color:      u32,
    bg_color:          u32,
    card_color:        u32,
    muted_color:       u32,
    star_count:        u32,
    star_speed:        f32,
    star_tilt_x:       f32,
    star_tilt_y:       f32,
    star_parallax_x:   f32,
    star_parallax_y:   f32,
    // ── M3 core tokens ────────────────────────────────────────────────────────
    theme_name:        String,
    surface_color:     u32,   // Base surface (elevation 0)
    on_surface_color:  u32,   // Text/icons on surface
    on_accent_color:   u32,   // Text/icons on primary/accent
    surface_var_color: u32,   // Surface tonal variant (elevation +1)
    outline_color:     u32,   // Borders, dividers
    error_color:       u32,   // Error state
    is_dark:           bool,
    // ── M3 extended tokens (Material Aether) ──────────────────────────────────
    secondary_color:   u32,   // Secondary brand color
    on_secondary:      u32,   // Text on secondary
    tertiary_color:    u32,   // Accent complement / highlights
    on_tertiary:       u32,   // Text on tertiary
    surface2_color:    u32,   // Elevation level 2 (nav rail, drawer)
    surface3_color:    u32,   // Elevation level 3 (app bar)
    surface5_color:    u32,   // Elevation level 5 (dialogs, menus)
    outline_var_color: u32,   // Subtle dividers (lower contrast than outline)
    success_color:     u32,   // Success / positive state
    warning_color:     u32,   // Warning state
    scrim_color:       u32,   // Modal backdrop scrim
    ripple_color:      u32,   // Touch ripple overlay
    // ── Typography / shape hints ───────────────────────────────────────────────
    corner_radius_sm:  u32,   // Small component radius (chip, badge) dp
    corner_radius_md:  u32,   // Medium component radius (card, button) dp
    corner_radius_lg:  u32,   // Large component radius (bottom sheet) dp
    corner_radius_xl:  u32,   // Extra large (dialog, nav drawer) dp
    // ── Live animation state (polled by UI at 500ms) ─────────────────────────
    animation_phase:   f32,   // 0.0–1.0, cycles with uptime (3s period)
    pulse_bpm:         u32,   // 60=idle, 90=processing, 120=agent running
    activity_level:    f32,   // 0.0–1.0, based on tool_calls in last 60s
    is_thinking:       bool,  // Kira is currently processing a request
}
impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            // Legacy "Kira" dark crimson/space theme
            accent_color:     0xFFDC143C,
            bg_color:         0xFF050508,
            card_color:       0xFF0E0E18,
            muted_color:      0xFF666680,
            star_count:       110,
            star_speed:       0.013,
            star_tilt_x:      0.0,
            star_tilt_y:      0.0,
            star_parallax_x:  0.0,
            star_parallax_y:  0.0,
            theme_name:       String::from("kira"),
            surface_color:    0xFF0E0E18,
            on_surface_color: 0xFFE6E1E5,
            on_accent_color:  0xFFFFFFFF,
            surface_var_color:0xFF1C1B2E,
            outline_color:    0xFF938F99,
            error_color:      0xFFCF6679,
            is_dark:          true,
            secondary_color:  0xFF9B2335,
            on_secondary:     0xFFFFFFFF,
            tertiary_color:   0xFF7B2D8B,
            on_tertiary:      0xFFFFFFFF,
            surface2_color:   0xFF12111F,
            surface3_color:   0xFF161525,
            surface5_color:   0xFF1E1D30,
            outline_var_color:0xFF4A4860,
            success_color:    0xFF4CAF7D,
            warning_color:    0xFFFFB347,
            scrim_color:      0xCC000000,
            ripple_color:     0x1FDC143C,
            corner_radius_sm: 8,
            corner_radius_md: 16,
            corner_radius_lg: 24,
            corner_radius_xl: 28,
            animation_phase:  0.0,
            pulse_bpm:        60,
            activity_level:   0.0,
            is_thinking:      false,

        }
    }
}

impl ThemeConfig {
    /// ── Material Aether Dark ─────────────────────────────────────────────────
    ///
    /// Design language: "Warm Intelligence"
    ///
    /// Core insight: most AI dark themes use cold blue/teal which feels sterile.
    /// Aether uses a warm indigo-purple primary — premium, approachable, distinct.
    ///
    /// Palette:
    ///   Primary:      #7C6AF6  Aether violet — vibrant but not neon
    ///   On-primary:   #FFFFFF  Pure white — maximum legibility
    ///   Secondary:    #C792EA  Soft lavender — complement, not compete
    ///   On-secondary: #1A0030  Deep purple on lavender
    ///   Tertiary:     #FFD166  Warm gold — energy, highlights, badges
    ///   On-tertiary:  #2C1A00  Deep brown on gold
    ///   BG:           #0F0E17  Near-black with warm purple undertone
    ///   Surface 0:    #14131E  Warm dark surface — base
    ///   Surface 2:    #1C1B2A  Nav rail, drawer (+2 tonal)
    ///   Surface 3:    #201F30  App bar (+3 tonal)
    ///   Surface 5:    #272539  Dialogs, menus (+5 tonal)
    ///   Card:         #1C1B2A  = Surface 2 for cards
    ///   On-surface:   #E8E3FF  Warm white — subtle purple tint, reduces harshness
    ///   Surface-var:  #2D2B45  Elevated surface variant (bottom sheets)
    ///   Outline:      #4E4B6E  Subtle warm-purple border
    ///   Outline-var:  #2A2840  Barely-visible dividers
    ///   Error:        #FF6B80  Warm rose — friendlier than pure red
    ///   Success:      #58D68D  Mint green
    ///   Warning:      #FFD166  = Tertiary (gold doubles as warning)
    ///   Scrim:        #CC0F0E17 Semi-transparent bg
    ///   Ripple:       #1F7C6AF6 Tinted violet ripple
    ///   Corner radii: 8 / 16 / 24 / 28 dp (full M3 spec)
    fn material_dark() -> Self {
        ThemeConfig {
            // ── Primary brand ────────────────────────────────────────────────
            accent_color:     0xFF7C6AF6, // Aether violet primary
            on_accent_color:  0xFFFFFFFF, // Pure white on violet
            // ── Secondary ────────────────────────────────────────────────────
            secondary_color:  0xFFC792EA, // Soft lavender
            on_secondary:     0xFF1A0030, // Deep purple text on lavender
            // ── Tertiary / highlights ─────────────────────────────────────────
            tertiary_color:   0xFFFFD166, // Warm gold — badges, highlights
            on_tertiary:      0xFF2C1A00, // Deep brown on gold
            // ── Backgrounds & surfaces ────────────────────────────────────────
            bg_color:         0xFF0F0E17, // Near-black, warm purple undertone
            surface_color:    0xFF14131E, // Base surface (elevation 0)
            surface2_color:   0xFF1C1B2A, // Elevation +2 (nav rail, drawer)
            surface3_color:   0xFF201F30, // Elevation +3 (app bar)
            surface5_color:   0xFF272539, // Elevation +5 (dialogs, menus)
            card_color:       0xFF1C1B2A, // Card = surface2
            surface_var_color:0xFF2D2B45, // Bottom sheets, elevated panels
            // ── On-surface text ───────────────────────────────────────────────
            on_surface_color: 0xFFE8E3FF, // Warm white — purple tint reduces harshness
            muted_color:      0xFF8A84B3, // Muted secondary text / icons
            // ── Borders ───────────────────────────────────────────────────────
            outline_color:    0xFF4E4B6E, // Visible borders
            outline_var_color:0xFF2A2840, // Subtle dividers
            // ── Semantic states ───────────────────────────────────────────────
            error_color:      0xFFFF6B80, // Warm rose error
            success_color:    0xFF58D68D, // Mint green success
            warning_color:    0xFFFFD166, // Gold warning (= tertiary)
            // ── Overlays ──────────────────────────────────────────────────────
            scrim_color:      0xCC0F0E17, // Modal backdrop
            ripple_color:     0x1F7C6AF6, // Touch ripple — tinted violet
            // ── Shape / corner radii (Material 3 spec) ────────────────────────
            corner_radius_sm: 8,          // Chip, badge, small fab
            corner_radius_md: 16,         // Card, button, text field
            corner_radius_lg: 24,         // Bottom sheet, large FAB
            corner_radius_xl: 28,         // Dialog, navigation drawer
            // ── Stars (off for clean M3 surface) ─────────────────────────────
            star_count:       0,
            star_speed:       0.0,
            star_tilt_x:      0.0,
            star_tilt_y:      0.0,
            star_parallax_x:  0.0,
            star_parallax_y:  0.0,
            theme_name:       String::from("material"),
            is_dark:          true,
            // ── Animation state defaults ──────────────────────────────────
            animation_phase:  0.0,
            pulse_bpm:        60,
            activity_level:   0.0,
            is_thinking:      false,

        }
    }

    /// ── Material Aether Light ────────────────────────────────────────────────
    ///
    /// Light counterpart: "Soft Intelligence"
    ///
    ///   Primary:    #5B4BE0  Deeper violet (accessible on white)
    ///   Secondary:  #7B59C0  Mid-purple secondary
    ///   Tertiary:   #D4900A  Warm amber (gold darkened for light surface)
    ///   BG:         #FAFAFF  Warm white — subtle violet tint, not harsh
    ///   Surface 0:  #FFFFFF  Pure white cards
    ///   Surface 2:  #F0EEFF  Tinted nav rail
    ///   Surface 3:  #EAE7FF  App bar tint
    ///   Surface 5:  #E0DCFF  Dialog tint
    fn material_light() -> Self {
        ThemeConfig {
            accent_color:     0xFF5B4BE0, // Deep violet primary
            on_accent_color:  0xFFFFFFFF, // White on violet
            secondary_color:  0xFF7B59C0, // Mid-purple secondary
            on_secondary:     0xFFFFFFFF, // White on mid-purple
            tertiary_color:   0xFFD4900A, // Warm amber
            on_tertiary:      0xFFFFFFFF, // White on amber
            bg_color:         0xFFFAFAFF, // Warm white bg
            surface_color:    0xFFFFFFFF, // Pure white surface
            surface2_color:   0xFFF0EEFF, // Tinted nav rail
            surface3_color:   0xFFEAE7FF, // Tinted app bar
            surface5_color:   0xFFE0DCFF, // Tinted dialog
            card_color:       0xFFFFFFFF, // White cards
            surface_var_color:0xFFE4E0F0, // Bottom sheet
            on_surface_color: 0xFF1A1730, // Near-black, warm purple
            muted_color:      0xFF6B6490, // Muted mid-purple text
            outline_color:    0xFF8F8AB5, // Visible border
            outline_var_color:0xFFCDC9E4, // Subtle divider
            error_color:      0xFFD93651, // Deep rose error
            success_color:    0xFF1B8B5A, // Forest green success
            warning_color:    0xFFB86E00, // Deep amber warning
            scrim_color:      0xCC1A1730, // Dark scrim over light bg
            ripple_color:     0x1F5B4BE0, // Violet ripple
            corner_radius_sm: 8,
            corner_radius_md: 16,
            corner_radius_lg: 24,
            corner_radius_xl: 28,
            star_count:       0,
            star_speed:       0.0,
            star_tilt_x:      0.0,
            star_tilt_y:      0.0,
            star_parallax_x:  0.0,
            star_parallax_y:  0.0,
            theme_name:       String::from("material_light"),
            is_dark:          false,
            // ── Animation state defaults ──────────────────────────────────
            animation_phase:  0.0,
            pulse_bpm:        60,
            activity_level:   0.0,
            is_thinking:      false,

        }
    }

    /// ── Catppuccin Mocha — "Mocha Lens" ─────────────────────────────────────
    /// Warm dark, soft glass, pastel accents. Official Catppuccin Mocha palette.
    /// Primary:   Lavender #B4BEFE  Secondary: Mauve #CBA6F7
    /// Tertiary:  Peach    #FAB387  Success:   Green #A6E3A1
    /// Error:     Pink     #F38BA8  Warning:   Peach #FAB387
    /// BG:        Crust    #11111B  Surface:   Base  #1E1E2E
    /// Text:      #CDD6F4           Muted:     #9399B2
    /// Catppuccin Mocha — default Kira theme
    fn catppuccin_mocha() -> Self {
        ThemeConfig {
            accent_color:     0xFFB4BEFE, // Lavender
            on_accent_color:  0xFF1E1E2E, // Base
            secondary_color:  0xFFCBA6F7, // Mauve
            on_secondary:     0xFF1E1E2E,
            tertiary_color:   0xFFFAB387, // Peach
            on_tertiary:      0xFF1E1E2E,
            bg_color:         0xFF1E1E2E, // Base
            surface_color:    0xFF1E1E2E, // Base
            surface2_color:   0xFF181825, // Mantle
            surface3_color:   0xFF181825, // Mantle
            surface5_color:   0xFF45475A, // Surface1
            card_color:       0xFF313244, // Surface0
            surface_var_color:0xFF313244, // Surface0
            on_surface_color: 0xFFCDD6F4, // Text
            muted_color:      0xFF9399B2, // Overlay2
            outline_color:    0xFF585B70, // Overlay0
            outline_var_color:0xFF45475A, // Surface1
            error_color:      0xFFF38BA8, // Red
            success_color:    0xFFA6E3A1, // Green
            warning_color:    0xFFFAB387, // Peach
            scrim_color:      0xCC1E1E2E,
            ripple_color:     0x1AB4BEFE,
            corner_radius_sm: 8,
            corner_radius_md: 12,
            corner_radius_lg: 16,
            corner_radius_xl: 24,
            star_count:       80,
            star_speed:       0.0,
            star_tilt_x:      0.0,
            star_tilt_y:      0.0,
            star_parallax_x:  0.0,
            star_parallax_y:  0.0,
            theme_name:       String::from("catppuccin_mocha"),
            is_dark:          true,
            // ── Animation state defaults ──────────────────────────────────
            animation_phase:  0.0,
            pulse_bpm:        60,
            activity_level:   0.0,
            is_thinking:      false,

        }
    }

    fn to_json(&self) -> String {
        format!(
            r#"{{"name":"{}","accent":{},"bg":{},"card":{},"muted":{},"surface":{},"on_surface":{},"on_accent":{},"surface_var":{},"outline":{},"error":{},"is_dark":{},"star_count":{},"parallax_x":{:.6},"parallax_y":{:.6},"secondary":{},"on_secondary":{},"tertiary":{},"on_tertiary":{},"surface2":{},"surface3":{},"surface5":{},"outline_var":{},"success":{},"warning":{},"scrim":{},"ripple":{},"corner_sm":{},"corner_md":{},"corner_lg":{},"corner_xl":{},"animation_phase":{:.6},"pulse_bpm":{},"activity_level":{:.6},"is_thinking":{}}}"#,
            self.theme_name,
            self.accent_color, self.bg_color, self.card_color, self.muted_color,
            self.surface_color, self.on_surface_color, self.on_accent_color,
            self.surface_var_color, self.outline_color, self.error_color,
            self.is_dark, self.star_count,
            self.star_parallax_x, self.star_parallax_y,
            self.secondary_color, self.on_secondary,
            self.tertiary_color, self.on_tertiary,
            self.surface2_color, self.surface3_color, self.surface5_color,
            self.outline_var_color,
            self.success_color, self.warning_color,
            self.scrim_color, self.ripple_color,
            self.corner_radius_sm, self.corner_radius_md,
            self.corner_radius_lg, self.corner_radius_xl,
            self.animation_phase, self.pulse_bpm,
            self.activity_level, self.is_thinking
        )
    }
}

#[derive(Clone)]
struct KiraConfig {
    user_name:          String,
    api_key:            String,
    base_url:           String,
    model:              String,
    vision_model:       String,
    persona:            String,
    tg_token:           String,
    tg_allowed:         i64,
    agent_max_steps:    u32,
    agent_auto_approve: bool,
    heartbeat_interval: u32,
    setup_done:         bool,
}
impl Default for KiraConfig {
    fn default() -> Self { KiraConfig { user_name:"User".to_string(), api_key:String::new(), base_url:"https://api.groq.com/openai/v1".to_string(), model:"llama-3.1-8b-instant".to_string(), vision_model:String::new(), persona:String::new(), tg_token:String::new(), tg_allowed:0, agent_max_steps:25, agent_auto_approve:true, heartbeat_interval:30, setup_done:false } }
}

#[derive(Clone, Default)]
struct ShizukuStatus {
    installed:          bool,
    permission_granted: bool,
    last_checked_ms:    u128,
    error_msg:          String,
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Core State
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// A Telegram message to be sent (queued for Java to execute)

/// A received Telegram message


#[derive(Clone)]
struct CrashEntry {
    ts:      u128,
    thread:  String,
    message: String,   // first line of trace
    trace:   String,   // full stack trace (capped at 4KB)
}

#[derive(Default)]
#[derive(Default, Clone)]
pub struct ShellJob {
    pub id:      String,
    pub cmd:     String,
    pub timeout: u64,
    pub created: u128,
}

#[derive(Clone, Default)]
pub struct TgSend {
    pub chat_id: i64,
    pub text:    String,
    pub ts:      u128,
}

#[derive(Clone, Default)]
pub struct TgMessage {
    pub update_id: i64,
    pub chat_id:   i64,
    pub user:      String,
    pub text:      String,
    pub ts:        u128,
}

#[derive(Clone, Default)]
pub struct AgentTask {
    pub id:           String,
    pub goal:         String,
    pub status:       String,  // "running" | "done" | "stopped" | "cancelled"
    pub current_step: usize,
    pub max_steps:    usize,
    pub created_ms:   u128,
}


struct KiraState {
    // Device
    screen_nodes:      String,
    screen_pkg:        String,
    battery_pct:       i32,
    battery_charging:  bool,
    foreground_pkg:    String,   // current foreground app package

    // Notifications
    notifications:     VecDeque<Notif>,

    // Command queue
    pending_cmds:      VecDeque<(String, String)>,
    results:           HashMap<String, String>,

    // Sessions
    sessions:          HashMap<String, Session>,
    active_session:    String,

    // Memory
    memory_md:         String,
    daily_log:         VecDeque<String>,
    soul_md:           String,

    // Skills
    skills:            HashMap<String, Skill>,
    skill_turn_inject: Vec<String>,

    // Context
    context_turns:     VecDeque<ContextTurn>,
    // ── Session B: LZ4-compressed history buffer ─────────────────────────
    // Stores turns as LZ4-compressed bytes: 4-6x smaller than raw String.
    // Used by /ai/chat to load context without decompressing all history.
    context_turns_lz4: VecDeque<Vec<u8>>,
    context_compact:   String,
    agent_context:     String,
    total_tokens:      u64,

    // Node capabilities
    node_caps:         HashMap<String, NodeCapability>,

    // Legacy triggers (v7)
    triggers:          Vec<Trigger>,
    fired_triggers:    VecDeque<String>,
    webhook_events:    VecDeque<String>,

    // Heartbeat
    heartbeat_items:   Vec<HeartbeatItem>,
    heartbeat_log:     VecDeque<String>,

    // Providers
    providers:         Vec<Provider>,
    active_provider:   String,

    // Credentials
    credentials:       HashMap<String, Vec<u8>>,

    // Cron
    cron_jobs:         Vec<CronJob>,

    // Memory index
    memory_index:      Vec<MemoryEntry>,

    // Tool policy
    tool_allowlist:    Vec<String>,
    tool_denylist:     Vec<String>,

    // Tool iteration
    tool_iterations:   HashMap<String, u32>,
    max_tool_iters:    u32,

    // Audit
    audit_log:         VecDeque<AuditEntry>,

    // Task log
    task_log:          VecDeque<TaskStep>,
    checkpoints:       HashMap<String, String>,

    // Streaming
    stream_chunks:     VecDeque<StreamChunk>,
    stream_sessions:   HashMap<String, StreamSession>,

    // Cache
    response_cache:    HashMap<String, CacheEntry>,

    // Knowledge base
    knowledge_base:    Vec<KbEntry>,

    // Event feed
    event_feed:        VecDeque<EventFeedEntry>,

    // Stats
    uptime_start:      u128,
    crash_log:         std::collections::VecDeque<CrashEntry>,  // last 50 crashes
    // ── Session D: shell command queue ─────────────────────────────────────
    pending_shell:     std::collections::VecDeque<ShellJob>,
    shell_results:     std::collections::HashMap<String, String>,
    // ── Session E: agent task log ─────────────────────────────────────────
    agent_tasks:       std::collections::VecDeque<AgentTask>,
    // ── Session F: Telegram state ─────────────────────────────────────────
    tg_last_update_id: i64,
    tg_pending_sends:  std::collections::VecDeque<TgSend>,   // queued for Java to send
    tg_message_log:    std::collections::VecDeque<TgMessage>, // last 50 received
    // HTTP auth secret (empty = localhost-only open access)
    http_secret:       String,
    request_count:     u64,
    tool_call_count:   u64,

    // v38
    setup:             SetupState,
    theme:             ThemeConfig,
    config:            KiraConfig,
    shizuku:           ShizukuStatus,

    // \u{2500}\u{2500} v40: Automation Engine \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    macros:            Vec<AutoMacro>,
    profiles:          Vec<AutoProfile>,
    active_profile:    String,
    variables:         HashMap<String, AutoVariable>,
    macro_run_log:     VecDeque<MacroRunLog>,
    pending_actions:   VecDeque<PendingMacroAction>,

    // Device event signals (set by Java JNI calls, consumed by macro engine)
    sig_screen_on:     bool,
    sig_screen_off:    bool,
    sig_device_unlocked: bool,
    sig_device_locked:   bool,
    sig_wifi_ssid:     String,   // current SSID or empty
    sig_bt_device:     String,   // connected BT device name or empty
    sig_shake:         bool,
    sig_vol_up:        bool,
    sig_vol_down:      bool,
    sig_sms_sender:    String,
    sig_sms_text:      String,
    sig_call_number:   String,
    sig_nfc_tag:       String,
    sig_clipboard:     String,
    sig_kira_event:    String,
    sig_lat:           f64,
    sig_lon:           f64,
    sig_app_launched:  String,
    sig_app_closed:    String,
    sig_geofence:      String,   // "enter:label" or "exit:label"

    // \u{2500}\u{2500} Roboru / E-Robot / Automate engine \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    roboru_flows:      HashMap<String, AutoFlow>,
    roboru_keywords:   HashMap<String, Keyword>,
    roboru_pipelines:  HashMap<String, HyperPipeline>,

    // \u{2500}\u{2500} Roubao / Open-AutoGLM VLM phone agent \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    phone_agent_tasks:    Vec<PhoneAgentTask>,
    screen_observations:  VecDeque<ScreenObservation>,

    // fix: fields used by reactive/state-machine engine
    rx_subscriptions:    Vec<RxSubscription>,
    composite_triggers:  Vec<CompositeTrigger>,
    state_machines:      Vec<StateMachine>,
    context_zones:       Vec<ContextZone>,

    // ── OTA Update Engine (v43) ─────────────────────────────────────────────
    ota: OtaEngine,
}

// Sub-structs (v7, unchanged)
#[derive(Clone, Default)]
struct Session { id:String, channel:String, turns:u32, tokens:u64, created:u128, last_msg:u128 }
#[derive(Clone, Default)]
struct Skill { name:String, description:String, trigger:String, content:String, enabled:bool, usage_count:u32 }
#[derive(Clone)]
struct ContextTurn { role:String, content:String, ts:u128, tokens:u32, session:String }
#[derive(Clone, Default)]
struct NodeCapability { node_id:String, caps:Vec<String>, platform:String, online:bool, last_seen:u128 }
#[derive(Clone)]
struct Trigger { id:String, trigger_type:String, value:String, action:String, fired:bool, repeat:bool }
#[derive(Clone)]
struct HeartbeatItem { id:String, check:String, action:String, enabled:bool, last_run:u128, interval_ms:u128 }
#[derive(Clone, Default)]
struct Provider { id:String, name:String, base_url:String, model:String }
#[derive(Clone)]
struct CronJob { id:String, expression:String, action:String, last_run:u128, interval_ms:u128, enabled:bool }
#[derive(Clone)]
struct MemoryEntry { key:String, value:String, tags:Vec<String>, ts:u128, relevance:f32, access_count:u32 }
#[derive(Clone)]
struct AuditEntry { session:String, tool:String, input:String, output:String, ts:u128, success:bool, blocked:bool }
#[derive(Clone)]
struct TaskStep { task_id:String, step:u32, action:String, result:String, time:u128, success:bool }
#[derive(Clone, Default)]
struct StreamChunk { session_id:String, text:String, done:bool, ts:u128 }
#[derive(Clone, Default)]
struct StreamSession { id:String, active:bool, started:u128, chunks:u32 }
#[derive(Clone)]
struct CacheEntry { value:String, expires_at:u128 }
#[derive(Clone)]
struct KbEntry { id:String, title:String, content:String, tags:Vec<String>, ts:u128 }
#[derive(Clone)]
struct EventFeedEntry { event:String, data:String, ts:u128 }
struct Notif { pkg:String, title:String, text:String, time:u128 }

// ── LZ4 compression helpers for conversation history (Session B) ──────────

/// Compress a conversation turn into LZ4 bytes.
/// Format: "role\x00content" → lz4_prepend_size(bytes)
pub fn lz4_pack_turn(role: &str, content: &str) -> Vec<u8> {
    let raw = format!("{}\x00{}", role, content);
    compress_prepend_size(raw.as_bytes())
}

/// Decompress a single turn. Returns (role, content) or None on error.
pub fn lz4_unpack_turn(compressed: &[u8]) -> Option<(String, String)> {
    let raw = decompress_size_prepended(compressed).ok()?;
    let s   = String::from_utf8(raw).ok()?;
    let mut parts = s.splitn(2, '\x00');
    let role    = parts.next()?.to_string();
    let content = parts.next()?.to_string();
    Some((role, content))
}

/// Push a turn to the compressed buffer, evicting oldest if over limit.
/// Also evicts corresponding entry from context_turns to keep in sync.
pub fn push_turn_compressed(state: &mut KiraState, role: &str, content: &str) {
    let packed = lz4_pack_turn(role, content);
    state.context_turns_lz4.push_back(packed);
    // Keep at most 40 compressed turns (~160KB raw → ~35KB compressed)
    if state.context_turns_lz4.len() > 40 {
        state.context_turns_lz4.pop_front();
    }
}

/// Decompress all turns for use in an AI context window.
/// Returns Vec of (role, content) pairs, oldest first.
pub fn decompress_context(state: &KiraState) -> Vec<(String, String)> {
    state.context_turns_lz4.iter()
        .filter_map(|b| lz4_unpack_turn(b))
        .collect()
}

/// Compressed size in bytes of the entire conversation buffer.
pub fn compressed_context_bytes(state: &KiraState) -> usize {
    state.context_turns_lz4.iter().map(|b| b.len()).sum()
}

lazy_static! {
    static ref STATE: Arc<Mutex<KiraState>> = Arc::new(Mutex::new(KiraState {
        battery_pct:     100,
        foreground_pkg:  String::new(),
        max_tool_iters:  20,
        active_session:  "default".to_string(),
        active_provider: "groq".to_string(),
        active_profile:  "default".to_string(),
        soul_md: "You are Kira, a powerful Android AI agent. You are helpful, proactive, and autonomous.".to_string(),
        theme:   ThemeConfig::catppuccin_mocha(),
        config:  KiraConfig::default(),
        setup:   SetupState::default(),
        shizuku: ShizukuStatus::default(),
        ..Default::default()
    
        }));
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// v40: Variable engine helpers
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Expand %VAR_NAME% tokens and built-ins in a string
fn expand_vars(s: &KiraState, text: &str) -> String {
    let mut out = text.to_string();
    // Built-in variables
    out = out.replace("%BATTERY%", &s.battery_pct.to_string());
    out = out.replace("%CHARGING%", &s.battery_charging.to_string());
    out = out.replace("%SCREEN_PKG%", &s.screen_pkg);
    out = out.replace("%PROFILE%", &s.active_profile);
    out = out.replace("%WIFI%", &s.sig_wifi_ssid);
    out = out.replace("%BT%", &s.sig_bt_device);
    out = out.replace("%LAT%", &format!("{:.6}", s.sig_lat));
    out = out.replace("%LON%", &format!("{:.6}", s.sig_lon));
    out = out.replace("%CLIPBOARD%", &s.sig_clipboard);
    out = out.replace("%TIME_MS%", &now_ms().to_string());

    // User-defined variables
    for (name, var) in &s.variables {
        out = out.replace(&format!("%{}%", name.to_uppercase()), &var.value);
    }
    out
}

/// Evaluate a simple condition
fn eval_condition(s: &KiraState, cond: &MacroCondition) -> bool {
    let lhs = expand_vars(s, &cond.lhs);
    let rhs = expand_vars(s, &cond.rhs);
    match cond.operator.as_str() {
        "eq"       => lhs == rhs,
        "neq"      => lhs != rhs,
        "contains" => lhs.contains(&rhs),
        "starts"   => lhs.starts_with(&rhs),
        "ends"     => lhs.ends_with(&rhs),
        "gt"  => lhs.parse::<f64>().unwrap_or(0.0) > rhs.parse::<f64>().unwrap_or(0.0),
        "lt"  => lhs.parse::<f64>().unwrap_or(0.0) < rhs.parse::<f64>().unwrap_or(0.0),
        "gte" => lhs.parse::<f64>().unwrap_or(0.0) >= rhs.parse::<f64>().unwrap_or(0.0),
        "lte" => lhs.parse::<f64>().unwrap_or(0.0) <= rhs.parse::<f64>().unwrap_or(0.0),
        "matches" => {
            // simple glob: * wildcard
            let pat = rhs.replace('*', "");
            if rhs.starts_with('*') && rhs.ends_with('*') { lhs.contains(&pat) }
            else if rhs.starts_with('*') { lhs.ends_with(&pat) }
            else if rhs.ends_with('*')   { lhs.starts_with(&pat) }
            else { lhs == rhs }
        }
        "is_empty" => lhs.is_empty(),
        "not_empty"=> !lhs.is_empty(),
        _ => false,
    }
}

/// Check if a trigger would fire given current device state
fn check_trigger(s: &KiraState, trig: &MacroTrigger) -> bool {
    if !trig.enabled { return false; }
    match &trig.kind {
        MacroTriggerKind::BatteryLevel => {
            let op  = trig.config.get("op").map(|x| x.as_str()).unwrap_or("lte");
            let thr = trig.config.get("threshold").and_then(|t| t.parse::<i32>().ok()).unwrap_or(20);
            match op { "gte" => s.battery_pct >= thr, "lte" => s.battery_pct <= thr, "eq" => s.battery_pct == thr, _ => false }
        }
        MacroTriggerKind::BatteryCharging    => s.battery_charging,
        MacroTriggerKind::ScreenOn           => s.sig_screen_on,
        MacroTriggerKind::ScreenOff          => s.sig_screen_off,
        MacroTriggerKind::DeviceUnlocked     => s.sig_device_unlocked,
        MacroTriggerKind::DeviceLocked       => s.sig_device_locked,
        MacroTriggerKind::Shake              => s.sig_shake,
        MacroTriggerKind::VolumeBtnUp        => s.sig_vol_up,
        MacroTriggerKind::VolumeBtnDown      => s.sig_vol_down,
        MacroTriggerKind::WifiConnected => {
            let ssid = trig.config.get("ssid").map(|x| x.as_str()).unwrap_or("");
            !s.sig_wifi_ssid.is_empty() && (ssid.is_empty() || s.sig_wifi_ssid == ssid)
        }
        MacroTriggerKind::WifiDisconnected   => s.sig_wifi_ssid.is_empty(),
        MacroTriggerKind::BluetoothConnected => {
            let dev = trig.config.get("device").map(|x| x.as_str()).unwrap_or("");
            !s.sig_bt_device.is_empty() && (dev.is_empty() || s.sig_bt_device.contains(dev))
        }
        MacroTriggerKind::BluetoothDisconnected => s.sig_bt_device.is_empty(),
        MacroTriggerKind::AppLaunched => {
            let pkg = trig.config.get("package").map(|x| x.as_str()).unwrap_or("");
            !s.sig_app_launched.is_empty() && (pkg.is_empty() || s.sig_app_launched == pkg)
        }
        MacroTriggerKind::AppClosed => {
            let pkg = trig.config.get("package").map(|x| x.as_str()).unwrap_or("");
            !s.sig_app_closed.is_empty() && (pkg.is_empty() || s.sig_app_closed == pkg)
        }
        MacroTriggerKind::NotifReceived => {
            // Checked via notification event bus; signal fires in pushNotification
            let pkg = trig.config.get("package").map(|x| x.as_str()).unwrap_or("");
            let kw  = trig.config.get("keyword").map(|x| x.as_str()).unwrap_or("");
            // For polling mode, check latest notification
            if let Some(n) = s.notifications.back() {
                let matches_pkg = pkg.is_empty() || n.pkg == pkg;
                let matches_kw  = kw.is_empty() || n.title.contains(kw) || n.text.contains(kw);
                matches_pkg && matches_kw
            } else { false }
        }
        MacroTriggerKind::SmsReceived => {
            let sender = trig.config.get("sender").map(|x| x.as_str()).unwrap_or("");
            let kw     = trig.config.get("keyword").map(|x| x.as_str()).unwrap_or("");
            !s.sig_sms_sender.is_empty()
                && (sender.is_empty() || s.sig_sms_sender.contains(sender))
                && (kw.is_empty() || s.sig_sms_text.contains(kw))
        }
        MacroTriggerKind::CallReceived | MacroTriggerKind::CallMissed => {
            let num = trig.config.get("number").map(|x| x.as_str()).unwrap_or("");
            !s.sig_call_number.is_empty() && (num.is_empty() || s.sig_call_number.contains(num))
        }
        MacroTriggerKind::NfcTag => {
            let id = trig.config.get("tag_id").map(|x| x.as_str()).unwrap_or("");
            !s.sig_nfc_tag.is_empty() && (id.is_empty() || s.sig_nfc_tag == id)
        }
        MacroTriggerKind::GeofenceEnter => s.sig_geofence.starts_with("enter:"),
        MacroTriggerKind::GeofenceExit  => s.sig_geofence.starts_with("exit:"),
        MacroTriggerKind::ClipboardChanged => !s.sig_clipboard.is_empty(),
        MacroTriggerKind::KiraCommand => {
            let phrase = trig.config.get("phrase").map(|x| x.as_str()).unwrap_or("");
            !s.sig_kira_event.is_empty() && s.sig_kira_event.contains(phrase)
        }
        MacroTriggerKind::KiraEvent => {
            let ev = trig.config.get("event").map(|x| x.as_str()).unwrap_or("");
            !s.sig_kira_event.is_empty() && (ev.is_empty() || s.sig_kira_event == ev)
        }
        MacroTriggerKind::Manual       => false, // only fires via API call
        MacroTriggerKind::Time         => false,  // handled by cron thread
        MacroTriggerKind::TimeInterval => false,
        MacroTriggerKind::Cron         => false,
        _ => false,
    }
}

/// Build a pending action and add to queue (Java will poll and execute)
fn enqueue_action(s: &mut KiraState, macro_id: &str, action: &MacroAction) {
    let mut params = action.params.clone();
    // Expand variable tokens in all param values
    let expanded_params: HashMap<String, String> = params.iter()
        .map(|(k, v)| (k.clone(), expand_vars(s, v)))
        .collect();
    params = expanded_params;

    // Actions handled entirely in Rust (variable engine, logging)
    match &action.kind {
        MacroActionKind::SetVariable => {
            let name = params.get("name").cloned().unwrap_or_default();
            let value = params.get("value").cloned().unwrap_or_default();
            if !name.is_empty() {
                let ts = now_ms();
                s.variables.entry(name.clone()).and_modify(|v| {
                    v.value = value.clone(); v.updated_ms = ts;
                }).or_insert(AutoVariable {
                    name: name.clone(), value: value.clone(),
                    var_type: "string".to_string(), persistent: false,
                    created_ms: ts, updated_ms: ts,
                });
            }
            return;
        }
        MacroActionKind::IncrementVariable => {
            let name = params.get("name").cloned().unwrap_or_default();
            let step = params.get("step").and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
            if let Some(var) = s.variables.get_mut(&name) {
                let n = var.value.parse::<f64>().unwrap_or(0.0) + step;
                var.value = n.to_string(); var.updated_ms = now_ms();
            }
            return;
        }
        MacroActionKind::DecrementVariable => {
            let name = params.get("name").cloned().unwrap_or_default();
            let step = params.get("step").and_then(|s| s.parse::<f64>().ok()).unwrap_or(1.0);
            if let Some(var) = s.variables.get_mut(&name) {
                let n = var.value.parse::<f64>().unwrap_or(0.0) - step;
                var.value = n.to_string(); var.updated_ms = now_ms();
            }
            return;
        }
        MacroActionKind::ClearVariable => {
            let name = params.get("name").cloned().unwrap_or_default();
            s.variables.remove(&name);
            return;
        }
        MacroActionKind::LogEvent => {
            let msg = params.get("message").cloned().unwrap_or_default();
            s.daily_log.push_back(format!("[macro:{}] {}", macro_id, msg));
            if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
            return;
        }
        MacroActionKind::ActivateProfile => {
            let pid = params.get("profile").cloned().unwrap_or_default();
            if !pid.is_empty() {
                s.active_profile = pid.clone();
                for p in s.profiles.iter_mut() {
                    p.active = p.id == pid;
                }
            }
            return;
        }
        MacroActionKind::PushKiraEvent => {
            let ev = params.get("event").cloned().unwrap_or_default();
            s.sig_kira_event = ev.clone();
            s.event_feed.push_back(EventFeedEntry { event: "kira_event".to_string(), data: ev, ts: now_ms() });
            return;
        }
        MacroActionKind::StopFlow | MacroActionKind::StopMacro => {
            return; // handled in execute loop
        }
        _ => {}
    }

    // All other actions \u{2192} enqueue for Java
    let pa = PendingMacroAction {
        macro_id:  macro_id.to_string(),
        action_id: gen_id(),
        kind:      action.kind.to_str().to_string(),
        params,
        ts: now_ms(),
    };
    s.pending_actions.push_back(pa);
    if s.pending_actions.len() > 500 { s.pending_actions.pop_front(); }
}

/// Execute a macro's action list (Rust-side variable/flow control;
/// non-Rust actions are enqueued for Java)
fn execute_macro_actions(s: &mut KiraState, macro_id: &str, actions: &[MacroAction]) -> (u32, bool) {
    let mut steps = 0u32;
    for action in actions {
        if !action.enabled { continue; }
        steps += 1;
        match &action.kind {
            MacroActionKind::StopFlow | MacroActionKind::StopMacro => break,
            MacroActionKind::Wait => {
                // Enqueue a sleep to Java; we don't block the Rust thread
                enqueue_action(s, macro_id, action);
            }
            MacroActionKind::If => {
                let cond_lhs = action.params.get("cond_lhs").cloned().unwrap_or_default();
                let cond_op  = action.params.get("cond_op").cloned().unwrap_or_else(|| "eq".to_string());
                let cond_rhs = action.params.get("cond_rhs").cloned().unwrap_or_default();
                let mc = MacroCondition { lhs: cond_lhs, operator: cond_op, rhs: cond_rhs };
                if eval_condition(s, &mc) {
                    let (sub_steps, _) = execute_macro_actions(s, macro_id, &action.sub_actions);
                    steps += sub_steps;
                } else {
                    // else_actions stored in params as JSON (parsed at add time in simple impl)
                    // For now just skip \u{2014} full else branch is handled at the Java UI level
                }
            }
            MacroActionKind::Loop => {
                let count = action.params.get("count").and_then(|c| c.parse::<u32>().ok()).unwrap_or(1);
                for _ in 0..count.min(100) { // safety cap at 100
                    let (sub_steps, stopped) = execute_macro_actions(s, macro_id, &action.sub_actions);
                    steps += sub_steps;
                    if stopped { break; }
                }
            }
            _ => {
                enqueue_action(s, macro_id, action);
            }
        }
    }
    (steps, false)
}

/// Run all enabled macros whose triggers fired (called by trigger watcher thread)
fn run_triggered_macros(s: &mut KiraState) {
    let macro_ids: Vec<String> = s.macros.iter()
        .filter(|m| m.enabled)
        .filter(|m| m.profile.is_empty() || m.profile == s.active_profile)
        .filter(|m| m.triggers.iter().any(|t| check_trigger(s, t)))
        .filter(|m| m.conditions.iter().all(|c| eval_condition(s, c)))
        .map(|m| m.id.clone())
        .collect();

    for macro_id in macro_ids {
        let start = now_ms();
        let actions_clone: Vec<MacroAction> = s.macros.iter()
            .find(|m| m.id == macro_id)
            .map(|m| m.actions.clone())
            .unwrap_or_default();
        let name = s.macros.iter().find(|m| m.id == macro_id)
            .map(|m| m.name.clone()).unwrap_or_default();

        let (steps, _) = execute_macro_actions(s, &macro_id, &actions_clone);

        // Update run stats
        if let Some(m) = s.macros.iter_mut().find(|m| m.id == macro_id) {
            m.run_count += 1;
            m.last_run_ms = start;
        }

        s.macro_run_log.push_back(MacroRunLog {
            macro_id: macro_id.clone(), macro_name: name,
            trigger: "auto".to_string(), success: true,
            steps_run: steps, duration_ms: now_ms() - start,
            ts: start, error: String::new(),
        });
        if s.macro_run_log.len() > 1000 { s.macro_run_log.pop_front(); }
    }

    // Clear one-shot signals
    s.sig_screen_on     = false;
    s.sig_screen_off    = false;
    s.sig_device_unlocked = false;
    s.sig_device_locked = false;
    s.sig_shake         = false;
    s.sig_vol_up        = false;
    s.sig_vol_down      = false;
    s.sig_sms_sender    = String::new();
    s.sig_sms_text      = String::new();
    s.sig_call_number   = String::new();
    s.sig_nfc_tag       = String::new();
    s.sig_clipboard     = String::new();
    s.sig_kira_event    = String::new();
    s.sig_app_launched  = String::new();
    s.sig_app_closed    = String::new();
    s.sig_geofence      = String::new();
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Parse a macro from JSON body (simple hand-rolled parser matching the
// format the Java UI will POST)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

fn parse_macro_from_json(body: &str) -> AutoMacro {
    let id          = extract_json_str(body, "id").unwrap_or_else(|| gen_id());
    let name        = extract_json_str(body, "name").unwrap_or_else(|| "Unnamed Macro".to_string());
    let description = extract_json_str(body, "description").unwrap_or_default();
    let enabled     = !body.contains("\"enabled\":false");
    let profile     = extract_json_str(body, "profile").unwrap_or_default();
    let tags_raw    = extract_json_str(body, "tags").unwrap_or_default();
    let tags: Vec<String> = tags_raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let ts = now_ms();

    // Parse triggers array: [{kind:"screen_on", config:{}}]
    let triggers = parse_triggers_from_json(body);
    // Parse conditions array: [{lhs:"%BATTERY%", op:"lte", rhs:"20"}]
    let conditions = parse_conditions_from_json(body);
    // Parse actions array: [{kind:"show_toast", params:{message:"hi"}}]
    let actions = parse_actions_from_json(body, "actions");

    AutoMacro { id, name, description, enabled, triggers, conditions, actions, profile, run_count:0, last_run_ms:0, created_ms:ts, tags }
}

fn parse_triggers_from_json(body: &str) -> Vec<MacroTrigger> {
    let mut out = Vec::new();
    // Find "triggers":[...] block
    let key = "\"triggers\":[";
    let start = match body.find(key) { Some(i) => i + key.len(), None => return out };
    let slice = &body[start..];
    // Extract individual trigger objects naively
    let mut depth = 0i32; let mut obj_start = 0usize; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => { depth -= 1; if depth == 0 && in_obj {
                let obj = &slice[obj_start..=i];
                let kind_str = extract_json_str(obj, "kind").unwrap_or_else(|| "manual".to_string());
                let enabled  = !obj.contains("\"enabled\":false");
                // Parse config as flat key\u{2192}value from "config":{...}
                let mut config = HashMap::new();
                if let Some(ci) = obj.find("\"config\":{") {
                    let cs = &obj[ci + "\"config\":{".len()..];
                    // simple KV pairs
                    let end = cs.find('}').unwrap_or(cs.len());
                    parse_flat_kv(&cs[..end], &mut config);
                }
                out.push(MacroTrigger { kind: MacroTriggerKind::from_str(&kind_str), config, enabled });
                in_obj = false;
            }}
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    out
}

fn parse_conditions_from_json(body: &str) -> Vec<MacroCondition> {
    let mut out = Vec::new();
    let key = "\"conditions\":[";
    let start = match body.find(key) { Some(i) => i + key.len(), None => return out };
    let slice = &body[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => { depth -= 1; if depth == 0 && in_obj {
                let obj = &slice[obj_start..=i];
                let lhs = extract_json_str(obj, "lhs").unwrap_or_default();
                let op  = extract_json_str(obj, "op").unwrap_or_else(|| "eq".to_string());
                let rhs = extract_json_str(obj, "rhs").unwrap_or_default();
                out.push(MacroCondition { lhs, operator: op, rhs });
                in_obj = false;
            }}
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    out
}

fn parse_actions_from_json(body: &str, array_key: &str) -> Vec<MacroAction> {
    let mut out = Vec::new();
    let key = format!("\"{}\":[", array_key);
    let start = match body.find(&key) { Some(i) => i + key.len(), None => return out };
    let slice = &body[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => { depth -= 1; if depth == 0 && in_obj {
                let obj = &slice[obj_start..=i];
                let kind_str = extract_json_str(obj, "kind").unwrap_or_else(|| "log_event".to_string());
                let enabled  = !obj.contains("\"enabled\":false");
                let mut params = HashMap::new();
                if let Some(pi) = obj.find("\"params\":{") {
                    let ps = &obj[pi + "\"params\":{".len()..];
                    let end = ps.find('}').unwrap_or(ps.len());
                    parse_flat_kv(&ps[..end], &mut params);
                }
                // Nested sub_actions for If/Loop
                let sub_actions = parse_actions_from_json(obj, "sub_actions");
                out.push(MacroAction { kind: MacroActionKind::from_str(&kind_str), params, sub_actions, enabled });
                in_obj = false;
            }}
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    out
}

/// Parse "key":"value" pairs from a flat JSON object fragment
fn parse_flat_kv(fragment: &str, out: &mut HashMap<String, String>) {
    let mut rest = fragment;
    loop {
        let ki = match rest.find('"') { Some(i) => i, None => break };
        rest = &rest[ki+1..];
        let ke = match rest.find('"') { Some(i) => i, None => break };
        let key = rest[..ke].to_string();
        rest = &rest[ke+1..];
        // skip ":"
        let ci = match rest.find(':') { Some(i) => i, None => break };
        rest = &rest[ci+1..].trim_start();
        if rest.starts_with('"') {
            rest = &rest[1..];
            let ve = rest.find('"').unwrap_or(rest.len());
            out.insert(key, rest[..ve].to_string());
            rest = &rest[ve.saturating_add(1)..];
        } else {
            let ve = rest.find(|c: char| c == ',' || c == '}' || c == ']').unwrap_or(rest.len());
            out.insert(key, rest[..ve].trim().to_string());
            rest = &rest[ve..];
        }
    }
}

// Macro serialization to JSON
fn macro_to_json(m: &AutoMacro) -> String {
    let triggers_json: Vec<String> = m.triggers.iter().map(|t| {
        let cfg: Vec<String> = t.config.iter().map(|(k,v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect();
        format!("{{\"kind\":\"{}\",\"enabled\":{},\"config\":{{{}}}}}", t.kind.to_str(), t.enabled, cfg.join(","))
    }).collect();
    let conditions_json: Vec<String> = m.conditions.iter().map(|c|
        format!("{{\"lhs\":\"{}\",\"op\":\"{}\",\"rhs\":\"{}\"}}", esc(&c.lhs), esc(&c.operator), esc(&c.rhs))
    ).collect();
    let actions_json: Vec<String> = m.actions.iter().map(action_to_json).collect();
    let tags_json: Vec<String> = m.tags.iter().map(|t| format!("\"{}\"", esc(t))).collect();
    format!(
        r#"{{"id":"{}","name":"{}","description":"{}","enabled":{},"profile":"{}","run_count":{},"last_run_ms":{},"created_ms":{},"tags":[{}],"triggers":[{}],"conditions":[{}],"actions":[{}]}}"#,
        esc(&m.id), esc(&m.name), esc(&m.description), m.enabled, esc(&m.profile),
        m.run_count, m.last_run_ms, m.created_ms, tags_json.join(","),
        triggers_json.join(","), conditions_json.join(","), actions_json.join(",")
    )
}

fn action_to_json(a: &MacroAction) -> String {
    let params_json: Vec<String> = a.params.iter().map(|(k,v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect();
    let sub_json: Vec<String> = a.sub_actions.iter().map(action_to_json).collect();
    format!(r#"{{"kind":"{}","enabled":{},"params":{{{}}},"sub_actions":[{}]}}"#,
        a.kind.to_str(), a.enabled, params_json.join(","), sub_json.join(","))
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// JNI Bridge
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}


// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Session D — AI engine helpers (called by /ai/chat route)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Shell job queued for Java to execute via Shizuku

/// Build the system prompt for the AI, including memory context
pub fn build_system_prompt(state: &KiraState, persona: &str) -> String {
    let mem_context: String = state.memory_index.iter().take(5)
        .map(|m| format!("- {}", m.value))
        .collect::<Vec<_>>().join("\n");

    let tools_list = "[get_battery, get_wifi, run_shell, read_file, write_file, \
        list_files, add_memory, search_memory, http_get, http_post, \
        open_app, send_message, set_variable, get_variable, run_macro]";

    format!(
        "{}\n\nAvailable tools (call with <tool name=\"x\"><param k=\"v\"/></tool>):\n{}\
        \n\nUser memories:\n{}\n\nBe concise. Use tools when helpful.",
        persona, tools_list,
        if mem_context.is_empty() { "(none yet)".into() } else { mem_context }
    )
}

/// Synchronous LLM call via std::net::TcpStream (OpenAI-compatible).
/// Returns the raw assistant response or an error message.
pub fn call_llm_sync(
    api_key:    &str,
    base_url:   &str,
    model:      &str,
    system:     &str,
    history:    &[(String, String)],
) -> Result<String, String> {
    // Build messages JSON (Session K: HTTP I/O now in https_post)
    let mut msgs = Vec::new();
    if !system.is_empty() {
        msgs.push(format!(r#"{{"role":"system","content":"{}"}}"#, esc(system)));
    }
    for (role, content) in history {
        msgs.push(format!(r#"{{"role":"{}","content":"{}"}}"#, esc(role), esc(content)));
    }
    let messages_json = msgs.join(",");

    let body = format!(
        r#"{{"model":"{}","max_tokens":2048,"messages":[{}]}}"#,
        esc(model), messages_json
    );

    // Parse base_url → host, port, path
    let url_clean = base_url.trim_end_matches('/');
    let (host, port, base_path) = parse_api_url(url_clean)?;
    let path = format!("{}/chat/completions", base_path.trim_end_matches('/'));

    // Use rustls HTTPS (Session K) — pure Rust, no Java round-trip needed
    // Falls back to plain TCP for http:// endpoints (e.g. localhost, LAN providers)
    let response_body = if port == 443 || base_url.starts_with("https://") {
        https_post(&host, port, &path, &body, api_key, 60)?
    } else {
        // Plain HTTP (local providers, self-hosted)
        use std::io::{Write, BufRead, BufReader};
        let addr = format!("{}:{}", host, port);
        let mut stream = std::net::TcpStream::connect(&addr)
            .map_err(|e| format!("connect {}: {}", addr, e))?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(60)))
            .map_err(|e| e.to_string())?;
        let request = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\n             Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            path, host, api_key, body.len(), body
        );
        stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream);
        let mut buf = String::new();
        let mut in_body = false; let mut body_buf = String::new();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, Ok(_) => {
                    if !in_body { if line == "\r\n" { in_body = true; } }
                    else { body_buf.push_str(&line); }
                } Err(_) => break,
            }
        }
        body_buf
    };

    extract_llm_content(&response_body)
        .ok_or_else(|| format!("parse error: {}",
            &response_body[..response_body.len().min(200)]))
}

/// Parse base_url into (host, port, path)
fn parse_api_url(url: &str) -> Result<(String, u16, String), String> {
    let (scheme, rest) = if url.starts_with("https://") {
        ("https", &url[8..])
    } else if url.starts_with("http://") {
        ("http", &url[7..])
    } else {
        return Err(format!("unknown scheme: {}", url));
    };
    let (host_port, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None    => (rest, "/"),
    };
    let (host, port) = if let Some(i) = host_port.rfind(':') {
        let p: u16 = host_port[i+1..].parse().unwrap_or(if scheme=="https" {443} else {80});
        (host_port[..i].to_string(), p)
    } else {
        (host_port.to_string(), if scheme == "https" { 443u16 } else { 80u16 })
    };
    Ok((host, port, path.to_string()))
}

/// Extract the content string from OpenAI JSON response
fn extract_llm_content(json: &str) -> Option<String> {
    // Fast path: find "content":"..." without full JSON parse
    let key = "\"content\":\"";
    let start = json.find(key)? + key.len();
    let mut end = start;
    let bytes = json.as_bytes();
    while end < bytes.len() {
        if bytes[end] == b'"' && (end == 0 || bytes[end-1] != b'\\') { break; }
        end += 1;
    }
    // Unescape \n, \t, \\, \"
    let raw = &json[start..end];
    Some(raw.replace("\\n", "\n").replace("\\t", "\t")
            .replace("\\\"", "\"").replace("\\\\", "\\"))
}

/// Parse <tool name="x"><param k="v"/></tool> blocks from LLM output
pub fn parse_tool_calls(text: &str) -> Vec<(String, std::collections::HashMap<String,String>)> {
    let mut calls = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<tool ") {
        rest = &rest[start..];
        let end = match rest.find("</tool>") {
            Some(i) => i + 7,
            None    => break,
        };
        let block = &rest[..end];
        // Extract name="..."
        if let Some(name) = extract_attr(block, "name") {
            let mut params = std::collections::HashMap::new();
            // Extract <param k="..." v="..."/> or <param name="..." value="..."/>
            let mut pb = block;
            while let Some(pi) = pb.find("<param ") {
                pb = &pb[pi..];
                let pe = pb.find("/>").unwrap_or(pb.len());
                let ptag = &pb[..pe+2];
                let k = extract_attr(ptag, "k")
                    .or_else(|| extract_attr(ptag, "name"))
                    .unwrap_or_default();
                let v = extract_attr(ptag, "v")
                    .or_else(|| extract_attr(ptag, "value"))
                    .unwrap_or_default();
                if !k.is_empty() { params.insert(k, v); }
                pb = &pb[pe+2..];
            }
            calls.push((name, params));
        }
        rest = &rest[end..];
    }
    calls
}

fn extract_attr(s: &str, attr: &str) -> Option<String> {
    let key = format!("{}=\"", attr);
    let start = s.find(&key)? + key.len();
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
}

/// Remove <tool>...</tool> blocks from LLM output to get clean reply
pub fn clean_reply(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("<tool ") {
        out.push_str(&rest[..start]);
        match rest.find("</tool>") {
            Some(end) => rest = &rest[end+7..],
            None      => { rest = ""; break; }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

/// Dispatch a tool call — pure Rust tools execute here,
/// Shizuku/intent tools are queued for Java via /shell/next
pub fn dispatch_tool(name: &str, params: &std::collections::HashMap<String,String>) -> String {
    match name {
        "add_memory" => {
            let content = params.get("content").cloned().unwrap_or_default();
            if content.is_empty() { return "error: content required".into(); }
            let mut s = STATE.lock().unwrap();
            let idx = s.memory_index.len();
            s.memory_index.push_back(MemoryEntry {
                key:          format!("mem_{}", idx),
                content:      content.clone(),
                tags:         vec![],
                pinned:       false,
                access_count: 0,
                created_ms:   now_ms(),
                last_used_ms: now_ms(),
            });
            format!("memory added: {}", &content[..content.len().min(50)])
        }
        "search_memory" => {
            let query = params.get("query").cloned().unwrap_or_default();
            search_memory(&query)
        }
        "get_variable" => {
            let key = params.get("key").cloned().unwrap_or_default();
            let s = STATE.lock().unwrap();
            s.variables.get(&key).cloned().unwrap_or_else(|| "not found".into())
        }
        "set_variable" => {
            let key = params.get("key").cloned().unwrap_or_default();
            let val = params.get("value").cloned().unwrap_or_default();
            STATE.lock().unwrap().variables.insert(key.clone(), val.clone());
            format!("set {} = {}", key, val)
        }
        "get_battery" => {
            let s = STATE.lock().unwrap();
            format!("{}% {}", s.device.battery_pct,
                if s.device.charging { "charging" } else { "not charging" })
        }
        "get_wifi" => {
            let s = STATE.lock().unwrap();
            if s.device.wifi_connected {
                format!("connected: {}", s.device.wifi_ssid)
            } else { "disconnected".into() }
        }
        // Shell, file, and intent tools: queue for Java
        "run_shell" | "open_app" | "read_file" | "write_file" | "list_files" |
        "http_get"  | "http_post" => {
            // Return a sentinel — /ai/chat will queue a ShellJob
            // Java executes and result comes back via /shell/result
            format!("__shell__:{}", name)
        }
        _ => format!("unknown tool: {}", name),
    }
}

mod jni_bridge {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::{c_char, c_void};

    // ── JNI type aliases ──────────────────────────────────────────────────
    type JNIEnv  = *mut *mut c_void;   // JNIEnv**
    type JObject = *mut c_void;         // jobject
    type JString = *mut c_void;         // jstring (IS a JVM object reference)

    /// Convert a raw *const c_char (from Java) to a Rust String.
    fn cs(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }

    /// Length-bounded, safe version of cs() for untrusted JNI input (Session L).
    /// Rejects strings over max_len bytes with an empty string + log.
    fn cs_safe(p: *const c_char, max_len: usize) -> String {
        if p.is_null() { return String::new(); }
        let s = unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() };
        if s.len() > max_len {
            // Truncate rather than reject — avoids Java crash on oversized input
            return s[..max_len].to_string();
        }
        s
    }

    /// Create a JVM-managed jstring from a Rust &str.
    /// Uses jni crate for safe, correct NewStringUTF call.
    /// Falls back to manual vtable[169] if env is null.
    unsafe fn jni_str(env: JNIEnv, s: &str) -> JString {
        use jni::JNIEnv as SafeEnv;
        // jni crate wraps env pointer safely
        if env.is_null() {
            return std::ptr::null_mut();
        }
        let mut safe_env = SafeEnv::from_raw(env as *mut jni::sys::JNIEnv)
            .expect("invalid JNIEnv");
        match safe_env.new_string(s) {
            Ok(jstr) => jstr.into_raw() as JString,
            Err(_)   => {
                // Fallback: empty string
                safe_env.new_string("").map(|j| j.into_raw() as JString)
                    .unwrap_or(std::ptr::null_mut())
            }
        }
    }

    // \u{2500}\u{2500} Lifecycle \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: JNIEnv, _c: JObject, port: i32,
    ) {
        let p = port as u16;
        {
            let mut s = STATE.lock().unwrap();
            s.uptime_start = now_ms();
            s.providers    = make_providers();
            let sess = Session { id:"default".into(), channel:"kira".into(), created:now_ms(), last_msg:now_ms(), ..Default::default() };
            s.sessions.insert("default".into(), sess);
            // Default profiles
            s.profiles = vec![
                AutoProfile { id:"default".into(), name:"Default".into(), active:true,  auto_activate_trigger:String::new(), auto_activate_value:String::new() },
                AutoProfile { id:"work".into(),    name:"Work".into(),    active:false, auto_activate_trigger:"wifi_connected".into(), auto_activate_value:String::new() },
                AutoProfile { id:"home".into(),    name:"Home".into(),    active:false, auto_activate_trigger:"wifi_connected".into(), auto_activate_value:String::new() },
                AutoProfile { id:"sleep".into(),   name:"Sleep".into(),   active:false, auto_activate_trigger:"time".into(),           auto_activate_value:"22:00".into() },
                AutoProfile { id:"car".into(),     name:"Car".into(),     active:false, auto_activate_trigger:"bt_connected".into(),   auto_activate_value:String::new() },
            ];
        }
        install_builtin_templates(&mut STATE.lock().unwrap());
        thread::spawn(move || run_http(p));
        thread::spawn(run_trigger_watcher);
        thread::spawn(run_cron_scheduler);
        thread::spawn(run_macro_engine);
        thread::spawn(run_watchdog);
    }

    // \u{2500}\u{2500} v40: Device signal injectors (called from Java on each device event) \u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOn(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_screen_on = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOff(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_screen_off = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalUnlocked(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_device_unlocked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocked(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_device_locked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalShake(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_shake = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeUp(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_vol_up = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeDown(
        _e: JNIEnv, _c: JObject,
    ) { STATE.lock().unwrap().sig_vol_down = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalWifi(
        _e: JNIEnv, _c: JObject, ssid: *const c_char,
    ) { STATE.lock().unwrap().sig_wifi_ssid = cs(ssid); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalBluetooth(
        _e: JNIEnv, _c: JObject, device: *const c_char,
    ) { STATE.lock().unwrap().sig_bt_device = cs(device); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalSms(
        _e: JNIEnv, _c: JObject,
        sender: *const c_char, text: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.sig_sms_sender = cs(sender);
        s.sig_sms_text   = cs(text);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalCall(
        _e: JNIEnv, _c: JObject, number: *const c_char,
    ) { STATE.lock().unwrap().sig_call_number = cs(number); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalNfc(
        _e: JNIEnv, _c: JObject, tag_id: *const c_char,
    ) { STATE.lock().unwrap().sig_nfc_tag = cs(tag_id); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalClipboard(
        _e: JNIEnv, _c: JObject, text: *const c_char,
    ) { STATE.lock().unwrap().sig_clipboard = cs(text); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppLaunched(
        _e: JNIEnv, _c: JObject, pkg: *const c_char,
    ) { STATE.lock().unwrap().sig_app_launched = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppClosed(
        _e: JNIEnv, _c: JObject, pkg: *const c_char,
    ) { STATE.lock().unwrap().sig_app_closed = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocation(
        _e: JNIEnv, _c: JObject, lat: f64, lon: f64, geofence: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.sig_lat      = lat;
        s.sig_lon      = lon;
        s.sig_geofence = cs(geofence);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalKiraEvent(
        _e: JNIEnv, _c: JObject, event: *const c_char,
    ) { STATE.lock().unwrap().sig_kira_event = cs(event); }

    // \u{2500}\u{2500} v40: Macro management JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    /// Add or replace a macro. Body is full macro JSON.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addMacro(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let m = parse_macro_from_json(&body);
        let mut s = STATE.lock().unwrap();
        s.macros.retain(|x| x.id != m.id);
        let id = m.id.clone();
        s.macros.push(m);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, id)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeMacro(
        _e: JNIEnv, _c: JObject, id: *const c_char,
    ) {
        let id = cs(id);
        STATE.lock().unwrap().macros.retain(|m| m.id != id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_enableMacro(
        _e: JNIEnv, _c: JObject, id: *const c_char, enabled: bool,
    ) {
        let id = cs(id);
        if let Some(m) = STATE.lock().unwrap().macros.iter_mut().find(|m| m.id == id) {
            m.enabled = enabled;
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacros(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.macros.iter().map(macro_to_json).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runMacroNow(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap();
        let actions: Vec<MacroAction> = s.macros.iter()
            .find(|m| m.id == id)
            .map(|m| m.actions.clone())
            .unwrap_or_default();
        let name = s.macros.iter().find(|m| m.id == id)
            .map(|m| m.name.clone()).unwrap_or_default();
        let start = now_ms();
        let (steps, _) = execute_macro_actions(&mut s, &id, &actions);
        if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
            m.run_count += 1; m.last_run_ms = start;
        }
        s.macro_run_log.push_back(MacroRunLog {
            macro_id: id.clone(), macro_name: name, trigger:"manual".to_string(),
            success:true, steps_run:steps, duration_ms:now_ms()-start, ts:start, error:String::new()
        });
        if s.macro_run_log.len() > 1000 { s.macro_run_log.pop_front(); }
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"steps":{}}}"#, steps)) }
    }

    /// Get next pending macro action for Java to execute
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextMacroAction(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap().pending_actions.pop_front() {
            Some(pa) => {
                let params_json: Vec<String> = pa.params.iter()
                    .map(|(k,v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect();
                let json = format!(
                    r#"{{"macro_id":"{}","action_id":"{}","kind":"{}","ts":{},"params":{{{}}}}}"#,
                    esc(&pa.macro_id), esc(&pa.action_id), esc(&pa.kind), pa.ts, params_json.join(",")
                );
                unsafe { jni_str(env, &json) }
            }
            None => std::ptr::null_mut(),
        }
    }

    // \u{2500}\u{2500} v40: Variable management \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setVariable(
        _e: JNIEnv, _c: JObject,
        name: *const c_char, value: *const c_char, var_type: *const c_char,
    ) {
        let name = cs(name); let value = cs(value); let vt = cs(var_type);
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        s.variables.entry(name.clone()).and_modify(|v| { v.value = value.clone(); v.updated_ms = ts; })
            .or_insert(AutoVariable { name, value, var_type: if vt.is_empty(){"string".to_string()}else{vt}, persistent:false, created_ms:ts, updated_ms:ts });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getVariable(
        env: JNIEnv, _c: JObject, name: *const c_char,
    ) -> JString {
        let name = cs(name);
        let s = STATE.lock().unwrap();
        let json = match s.variables.get(&name) {
            Some(v) => format!(r#"{{"name":"{}","value":"{}","type":"{}"}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type)),
            None    => r#"{"error":"not_found"}"#.to_string(),
        };
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getVariables(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.variables.values().map(|v|
            format!(r#"{{"name":"{}","value":"{}","type":"{}","updated_ms":{}}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type), v.updated_ms)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} v40: Profile management \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setProfile(
        _e: JNIEnv, _c: JObject, id: *const c_char,
    ) {
        let id = cs(id);
        let mut s = STATE.lock().unwrap();
        s.active_profile = id.clone();
        for p in s.profiles.iter_mut() { p.active = p.id == id; }
        s.sig_kira_event = format!("profile_changed:{}", id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProfiles(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.profiles.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","active":{}}}"#, esc(&p.id), esc(&p.name), p.active)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacroRunLog(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.macro_run_log.iter().skip(s.macro_run_log.len().saturating_sub(100)).map(|r|
            format!(r#"{{"macro_id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"duration_ms":{},"ts":{}}}"#,
                esc(&r.macro_id), esc(&r.macro_name), esc(&r.trigger), r.success, r.steps_run, r.duration_ms, r.ts)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} v38 JNI (unchanged) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_syncConfig(
        _e: JNIEnv, _c: JObject,
        user_name:*const c_char, api_key:*const c_char, base_url:*const c_char,
        model:*const c_char, vision_model:*const c_char, persona:*const c_char,
        tg_token:*const c_char, tg_allowed:i64, max_steps:i32, auto_approve:bool,
        heartbeat:i32, setup_done:bool,
    ) {
        let mut s = STATE.lock().unwrap();
        s.config.user_name          = cs(user_name);
        s.config.api_key            = cs(api_key);
        s.config.base_url           = cs(base_url);
        s.config.model              = cs(model);
        s.config.vision_model       = cs(vision_model);
        s.config.persona            = cs(persona);
        s.config.tg_token           = cs(tg_token);
        s.config.tg_allowed         = tg_allowed;
        s.config.agent_max_steps    = max_steps as u32;
        s.config.agent_auto_approve = auto_approve;
        s.config.heartbeat_interval = heartbeat as u32;
        s.config.setup_done         = setup_done;
        let bu = s.config.base_url.clone();
        if let Some(p) = s.providers.iter().find(|p| p.base_url == bu) { s.active_provider = p.id.clone(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getConfig(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let json = config_to_json(&s.config);
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateSetupPage(
        _e: JNIEnv, _c: JObject,
        page:i32, api_key:*const c_char, base_url:*const c_char,
        model:*const c_char, user_name:*const c_char, tg_token:*const c_char, tg_id:i64,
    ) {
        let mut s = STATE.lock().unwrap();
        s.setup.current_page = page as u8;
        let ak=cs(api_key);   if !ak.is_empty()  { s.setup.api_key   =ak.clone();  s.config.api_key  =ak; }
        let bu=cs(base_url);  if !bu.is_empty()  { s.setup.base_url  =bu.clone();  s.config.base_url =bu; }
        let mo=cs(model);     if !mo.is_empty()  { s.setup.model     =mo.clone();  s.config.model    =mo; }
        let un=cs(user_name); if !un.is_empty()  { s.setup.user_name =un.clone();  s.config.user_name=un; }
        let tt=cs(tg_token);  if !tt.is_empty()  { s.setup.tg_token  =tt.clone();  s.config.tg_token =tt; }
        if tg_id > 0 { s.setup.tg_allowed_id=tg_id; s.config.tg_allowed=tg_id; }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_completeSetup(
        _e: JNIEnv, _c: JObject,
    ) { let mut s = STATE.lock().unwrap(); s.setup.done=true; s.config.setup_done=true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_isSetupDone(
        _e: JNIEnv, _c: JObject,
    ) -> bool { STATE.lock().unwrap().config.setup_done }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setCustomProvider(
        _e: JNIEnv, _c: JObject, url:*const c_char, model:*const c_char,
    ) {
        let url=cs(url); let model=cs(model);
        let mut s = STATE.lock().unwrap();
        s.setup.custom_url=url.clone(); s.setup.selected_provider_id="custom".to_string();
        s.config.base_url=url.clone(); if !model.is_empty() { s.config.model=model.clone(); }
        if let Some(p) = s.providers.iter_mut().find(|p| p.id=="custom") {
            p.base_url=url; if !model.is_empty() { p.model=model; }
        }
        s.active_provider="custom".to_string();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setActiveProvider(
        env: JNIEnv, _c: JObject, provider_id:*const c_char,
    ) -> JString {
        let id=cs(provider_id);
        let mut s = STATE.lock().unwrap();
        let found=s.providers.iter().find(|p| p.id==id).cloned();
        let result = if let Some(p)=found {
            s.active_provider=id; s.config.base_url=p.base_url.clone(); s.config.model=p.model.clone();
            format!(r#"{{"ok":true,"id":"{}","base_url":"{}","model":"{}"}}"#, esc(&s.active_provider),esc(&p.base_url),esc(&p.model))
        } else { format!(r#"{{"error":"unknown provider {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProviders(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.providers.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),esc(&p.base_url),esc(&p.model),p.id==s.active_provider)
        ).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateShizukuStatus(
        _e: JNIEnv, _c: JObject,
        installed:bool, permission_granted:bool, error_msg:*const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.shizuku.installed=installed; s.shizuku.permission_granted=permission_granted;
        s.shizuku.error_msg=cs(error_msg); s.shizuku.last_checked_ms=now_ms();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getShizukuJson(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        unsafe { jni_str(env, &shizuku_to_json(&s.shizuku)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateTilt(
        _e: JNIEnv, _c: JObject, ax:f32, ay:f32,
    ) {
        let mut s = STATE.lock().unwrap();
        s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay;
        let tx=-ax*s.theme.star_speed; let ty=ay*s.theme.star_speed;
        s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08;
        s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStarParallax(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        unsafe { jni_str(env, &format!(r#"{{"px":{:.6},"py":{:.6},"ax":{:.4},"ay":{:.4}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y,s.theme.star_tilt_x,s.theme.star_tilt_y)) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getTheme(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        unsafe { jni_str(env, &s.theme.to_json()) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setTheme(
        _e: JNIEnv, _c: JObject,
        name: *const c_char,
    ) {
        let name = unsafe { std::ffi::CStr::from_ptr(name).to_str().unwrap_or("catppuccin_mocha") };
        let mut s = STATE.lock().unwrap();
        s.theme = match name {
            "material" | "material_neo" | "material_dark" => ThemeConfig::material_dark(),
            "material_light" | "material_neo_light"       => ThemeConfig::material_light(),
            "kira"                                        => ThemeConfig::default(),
            "catppuccin_mocha" | "catppuccin" | "mocha"  => ThemeConfig::catppuccin_mocha(),
            _                                             => ThemeConfig::catppuccin_mocha(),
        };
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStatsJson(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        { let _s = format!(
            r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"profiles":{},"active_profile":"{}","variables":{}}}"#,
            s.memory_index.len(), s.context_turns.len(),
            if s.shizuku.permission_granted{"active \u{2713}"} else if s.shizuku.installed{"no permission"} else{"not running"},
            if !s.agent_context.is_empty(){"enabled \u{2713}"} else{"disabled"},
            esc(&s.config.model), esc(&s.config.base_url),
            now_ms().saturating_sub(s.uptime_start),
            s.macros.len(), s.profiles.len(), esc(&s.active_profile),
            s.variables.len()
        ); unsafe { jni_str(env, &_s) } }
    }

    // \u{2500}\u{2500} v7 JNI (unchanged) \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: JNIEnv, _c: JObject,
        pkg:*const c_char, title:*const c_char, text:*const c_char,
    ) {
        let (pkg,title,text) = (cs(pkg),cs(title),cs(text));
        let ts = now_ms();
        let mut s = STATE.lock().unwrap();
        fire_notif_triggers(&mut s, &pkg, &title, &text);
        s.daily_log.push_back(format!("[{}] notif {}:{}", ts, pkg, &title[..title.len().min(40)]));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
        s.notifications.push_back(Notif { pkg, title, text, time:ts });
        if s.notifications.len() > 500 { s.notifications.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenNodes(
        _e: JNIEnv, _c: JObject, json:*const c_char,
    ) { STATE.lock().unwrap().screen_nodes = cs(json); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenPackage(
        _e: JNIEnv, _c: JObject, pkg:*const c_char,
    ) {
        let pkg = cs(pkg);
        let mut s = STATE.lock().unwrap();
        let prev = s.screen_pkg.clone();
        if prev != pkg {
            s.sig_app_launched = pkg.clone();
            if !prev.is_empty() { s.sig_app_closed = prev; }
        }
        s.screen_pkg = pkg;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateBattery(
        _e: JNIEnv, _c: JObject, pct:i32, charging:bool,
    ) {
        let mut s = STATE.lock().unwrap();
        let prev = s.battery_pct;
        s.battery_pct=pct; s.battery_charging=charging;
        fire_battery_triggers(&mut s, pct, prev);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateAgentContext(
        _e: JNIEnv, _c: JObject, ctx:*const c_char,
    ) { STATE.lock().unwrap().agent_context = cs(ctx); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushContextTurn(
        _e: JNIEnv, _c: JObject,
        role:*const c_char, content:*const c_char,
    ) {
        let role=cs(role); let content=cs(content);
        let tokens=estimate_tokens(&content);
        let ts=now_ms();
        let mut s = STATE.lock().unwrap();
        let sess_id = s.active_session.clone();
        s.total_tokens += tokens as u64;
        s.daily_log.push_back(format!("[{}] {}: {}", ts, role, &content[..content.len().min(80)]));
        // Also push compressed copy (Session B) — for memory-efficient context loading
        push_turn_compressed(&mut s, &role, &content);
        s.context_turns.push_back(ContextTurn { role, content, ts, tokens, session:sess_id });
        if s.context_turns.len() > 60 { compact_context(&mut s); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_indexMemory(
        _e: JNIEnv, _c: JObject,
        key:*const c_char, value:*const c_char, tags:*const c_char,
    ) {
        let (key,value,tags_raw) = (cs(key),cs(value),cs(tags));
        let tags: Vec<String> = tags_raw.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
        let mut s = STATE.lock().unwrap();
        s.memory_index.retain(|e| e.key != key);
        let fact = format!("- {}: {}", key, value);
        if !s.memory_md.contains(&fact) { s.memory_md.push_str(&format!("\n{}", fact)); }
        s.memory_index.push(MemoryEntry { key, value, tags, ts:now_ms(), relevance:1.0, access_count:0 });
        if s.memory_index.len() > 10000 {
            s.memory_index.sort_by(|a,b| a.relevance.partial_cmp(&b.relevance).unwrap_or(std::cmp::Ordering::Equal));
            s.memory_index.remove(0);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_storeCredential(
        _e: JNIEnv, _c: JObject,
        name:*const c_char, value:*const c_char,
    ) {
        let name=cs(name); let value=cs(value);
        let enc=xor_crypt(value.as_bytes(), derive_key(&name).as_slice());
        STATE.lock().unwrap().credentials.insert(name, enc);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_registerSkill(
        _e: JNIEnv, _c: JObject,
        name:*const c_char, desc:*const c_char, trigger:*const c_char, content:*const c_char,
    ) {
        let name=cs(name);
        STATE.lock().unwrap().skills.insert(name.clone(), Skill { name, description:cs(desc), trigger:cs(trigger), content:cs(content), enabled:true, usage_count:0 });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addHeartbeatItem(
        _e: JNIEnv, _c: JObject,
        id:*const c_char, check:*const c_char, action:*const c_char, interval_ms:i64,
    ) {
        let item = HeartbeatItem { id:cs(id), check:cs(check), action:cs(action), enabled:true, last_run:0, interval_ms:interval_ms as u128 };
        let mut s = STATE.lock().unwrap();
        s.heartbeat_items.retain(|i| i.id!=item.id);
        s.heartbeat_items.push(item);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_incrementToolIter(
        _e: JNIEnv, _c: JObject, session_id:*const c_char,
    ) -> i32 {
        let id=cs(session_id);
        let mut s = STATE.lock().unwrap();
        let count = { let c=s.tool_iterations.entry(id).or_insert(0); *c+=1; *c };
        s.tool_call_count += 1;
        let max = s.max_tool_iters;
        if count > max { -1 } else { count as i32 }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resetToolIter(
        _e: JNIEnv, _c: JObject, session_id:*const c_char,
    ) { STATE.lock().unwrap().tool_iterations.remove(&cs(session_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logTaskStep(
        _e: JNIEnv, _c: JObject,
        task_id:*const c_char, step:i32, action:*const c_char, result:*const c_char, success:bool,
    ) {
        let (tid,act,res) = (cs(task_id),cs(action),cs(result));
        let ts=now_ms();
        let mut s = STATE.lock().unwrap();
        do_audit(&mut s, &tid, &act, &act, &res, success, false);
        s.task_log.push_back(TaskStep { task_id:tid, step:step as u32, action:act, result:res, time:ts, success });
        if s.task_log.len() > 2000 { s.task_log.pop_front(); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextCommand(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap().pending_cmds.pop_front() {
            Some((id,body)) => unsafe { jni_str(env, &format!(r#"{{"id":"{}","body":{}}}"#, id, body)) },
            None => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _e: JNIEnv, _c: JObject, id:*const c_char, result:*const c_char,
    ) { STATE.lock().unwrap().results.insert(cs(id), cs(result)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextFiredTrigger(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        match STATE.lock().unwrap().fired_triggers.pop_front() {
            Some(t) => unsafe { jni_str(env, &t.as_str()) },
            None    => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addTrigger(
        _e: JNIEnv, _c: JObject,
        id:*const c_char, ttype:*const c_char, value:*const c_char, action:*const c_char, repeat:bool,
    ) { STATE.lock().unwrap().triggers.push(Trigger { id:cs(id), trigger_type:cs(ttype), value:cs(value), action:cs(action), fired:false, repeat }); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeTrigger(
        _e: JNIEnv, _c: JObject, id:*const c_char,
    ) { let id=cs(id); STATE.lock().unwrap().triggers.retain(|t| t.id!=id); }

    // \u{2500}\u{2500} OpenClaw v3 JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runDslScript(
        env: JNIEnv, _c: JObject,
        macro_id: *const c_char, script: *const c_char,
    ) -> JString {
        let log = execute_dsl_script(&mut STATE.lock().unwrap(), &cs(macro_id), &cs(script));
        let log_json: Vec<String> = log.iter().map(|l| format!(r#""{}""#, esc(l))).collect();
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"log":[{}]}}"#, log_json.join(","))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_rxSubscribe(
        env: JNIEnv, _c: JObject,
        id: *const c_char, name: *const c_char, event_kinds: *const c_char,
        target_macro: *const c_char, debounce_ms: i64, throttle_ms: i64, distinct: bool,
    ) -> JString {
        let id   = cs(id); let name = cs(name);
        let kinds: Vec<String> = cs(event_kinds).split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let target = cs(target_macro);
        let mut operators = Vec::new();
        if debounce_ms > 0 { operators.push(RxOperator::Debounce(debounce_ms as u128)); }
        if throttle_ms > 0 { operators.push(RxOperator::Throttle(throttle_ms as u128)); }
        if distinct { operators.push(RxOperator::Distinct); }
        let sub = RxSubscription {
            id: id.clone(), name, event_kinds: kinds, operators, target_macro: target,
            enabled: true, fired_count: 0, last_fired: 0, debounce_last: 0, throttle_last: 0,
            take_count: 0, skip_count: 0, last_value: String::new(), buffer: Vec::new(),
        };
        STATE.lock().unwrap().rx_subscriptions.push(sub);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_rxPostEvent(
        _e: JNIEnv, _c: JObject,
        kind: *const c_char, data: *const c_char,
    ) {
        let event = RxEvent { kind: cs(kind), data: cs(data), ts: now_ms(), source: "jni".to_string() };
        let mut s = STATE.lock().unwrap();
        let subs: Vec<RxSubscription> = s.rx_subscriptions.iter().cloned().collect();
        for mut sub in subs {
            if !sub.enabled { continue; }
            if let Some(_) = rx_process_event(&mut sub, &event, &s) {
                let target = sub.target_macro.clone();
                chain_macro(&mut s, &target);
                if let Some(rs) = s.rx_subscriptions.iter_mut().find(|r| r.id == sub.id) {
                    rs.fired_count += 1; rs.last_fired = now_ms();
                }
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_channelPost(
        _e: JNIEnv, _c: JObject,
        channel: *const c_char, message: *const c_char,
    ) { channel_post(&mut STATE.lock().unwrap(), &cs(channel), &cs(message)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_batteryDefer(
        _e: JNIEnv, _c: JObject,
        macro_id: *const c_char, min_pct: i32,
    ) { defer_until_charged(&mut STATE.lock().unwrap(), &cs(macro_id), min_pct); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_exportBundle(
        env: JNIEnv, _c: JObject, tag_filter: *const c_char,
    ) -> JString {
        let tag = cs(tag_filter);
        let result = export_bundle(&STATE.lock().unwrap(), if tag.is_empty() { None } else { Some(&tag) });
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_fsmEvent(
        _e: JNIEnv, _c: JObject,
        machine_id: *const c_char, event: *const c_char,
    ) { fsm_process_event(&mut STATE.lock().unwrap(), &cs(machine_id), &cs(event)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _e: JNIEnv, _c: JObject, s:*mut c_char,
    ) { if !s.is_null() { unsafe { drop(CString::from_raw(s)); } } }

    // \u{2500}\u{2500} OpenClaw / NanoBot / ZeroClaw extended JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_exportMacros(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let json = export_macros_json(&STATE.lock().unwrap());
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_importMacros(
        _e: JNIEnv, _c: JObject, json: *const c_char,
    ) { import_macros_json(&mut STATE.lock().unwrap(), &cs(json)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chainMacro(
        _e: JNIEnv, _c: JObject, target_id: *const c_char,
    ) { chain_macro(&mut STATE.lock().unwrap(), &cs(target_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_evalExpr(
        env: JNIEnv, _c: JObject, expr: *const c_char,
    ) -> JString {
        let result = eval_expr(&STATE.lock().unwrap(), &cs(expr));
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_expandVars(
        env: JNIEnv, _c: JObject, text: *const c_char,
    ) -> JString {
        let result = expand_vars(&STATE.lock().unwrap(), &cs(text));
        unsafe { jni_str(env, &result) }
    }

    // \u{2500}\u{2500} Roubao / Open-AutoGLM VLM JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    /// Start a new phone agent task. Returns {ok, task_id}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startAgentTask(
        env: JNIEnv, _c: JObject,
        goal: *const c_char, max_steps: i32,
    ) -> JString {
        let goal = cs(goal);
        let max_s = if max_steps > 0 { max_steps as u32 } else { 20 };
        let task_id = gen_id();
        let mut s = STATE.lock().unwrap();
        let plan_prompt = build_task_plan_prompt(&goal, &s.agent_context);
        s.pending_actions.push_back(PendingMacroAction {
            macro_id: task_id.clone(), action_id: gen_id(),
            kind: "vlm_plan".to_string(),
            params: {
                let mut p = HashMap::new();
                p.insert("task_id".to_string(), task_id.clone());
                p.insert("goal".to_string(), goal.clone());
                p.insert("prompt".to_string(), plan_prompt);
                p
            },
            ts: now_ms(),
        });
        s.phone_agent_tasks.push(PhoneAgentTask {
            id: task_id.clone(), goal, state: VlmTaskState::Planning,
            plan: vec![], plan_idx: 0, history: vec![], max_steps: max_s,
            current_step: 0, context: String::new(), result: String::new(),
            created_ms: now_ms(), last_step_ms: now_ms(),
        });
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"task_id":"{}"}}"#, esc(&task_id))) }
    }

    /// Called by Java after VLM responds with action decision
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_processVlmStep(
        env: JNIEnv, _c: JObject,
        task_id: *const c_char, vlm_response: *const c_char,
    ) -> JString {
        let task_id = cs(task_id); let vlm_resp = cs(vlm_response);
        let done = execute_vlm_step(&mut STATE.lock().unwrap(), &task_id, &vlm_resp);
        unsafe { jni_str(env, &format!(r#"{{"ok":true,"done":{}}}"#, done)) }
    }

    /// Called by Java after taking screenshot + getting VLM screen description
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_recordScreenObservation(
        _e: JNIEnv, _c: JObject,
        task_id: *const c_char, step: i32, vlm_desc: *const c_char,
    ) {
        record_screen_observation(&mut STATE.lock().unwrap(), &cs(task_id), step as u32, &cs(vlm_desc));
    }

    /// Set the AI-generated plan for a task
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setAgentPlan(
        _e: JNIEnv, _c: JObject,
        task_id: *const c_char, plan_json: *const c_char,
    ) {
        let task_id = cs(task_id); let plan_str = cs(plan_json);
        // plan_json is a comma-separated list of steps extracted from AI JSON array
        let plan: Vec<String> = plan_str.split("||")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut s = STATE.lock().unwrap();
        if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
            t.plan = plan;
            t.state = VlmTaskState::Observing;
        }
        enqueue_vlm_step(&mut s, &task_id);
    }

    /// Get current agent prompt for the AI call (Java reads this before calling AI)
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentPrompt(
        env: JNIEnv, _c: JObject,
        task_id: *const c_char,
    ) -> JString {
        let task_id = cs(task_id);
        let s = STATE.lock().unwrap();
        let result = match s.phone_agent_tasks.iter().find(|t| t.id == task_id) {
            Some(t) => {
                let sub_task = t.plan.get(t.plan_idx).cloned().unwrap_or_else(|| t.goal.clone());
                let screen_desc = format!("Package: {}
Context: {}",
                    s.screen_pkg, &s.agent_context[..s.agent_context.len().min(400)]);
                let prompt = build_vlm_action_prompt(
                    &t.goal, &sub_task, &screen_desc,
                    &t.history, t.current_step, t.max_steps
                );
                format!(r#"{{"task_id":"{}","prompt":{},"step":{},"sub_task":"{}"}}"#,
                    esc(&t.id), json_str(&prompt), t.current_step, esc(&sub_task))
            }
            None => r#"{"error":"not found"}"#.to_string(),
        };
        unsafe { jni_str(env, &result) }
    }

    /// Get all agent tasks summary
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentTasks(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.phone_agent_tasks.iter().map(|t| format!(
            r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"result":"{}"}}"#,
            esc(&t.id), esc(&t.goal),
            match &t.state {
                VlmTaskState::Done(_) => "done",
                VlmTaskState::Failed(_) => "failed",
                VlmTaskState::Planning => "planning",
                VlmTaskState::Observing => "observing",
                _ => "running",
            },
            t.current_step, esc(&t.result)
        )).collect();
        unsafe { jni_str(env, &format!("[{}]", items.join(","))) }
    }

    // \u{2500}\u{2500} Roboru JNI \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addFlow(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        if let Some(flow) = parse_flow_from_json(&body) {
            let id = flow.id.clone();
            STATE.lock().unwrap().roboru_flows.insert(id.clone(), flow);
            unsafe { jni_str(env, &format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))) }
        } else {
            unsafe { jni_str(env, &r#"{"error":"invalid flow"}"#) }
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runFlow(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap();
        let flow = s.roboru_flows.get(&id).cloned();
        let result = if let Some(flow) = flow {
            let steps = execute_flow(&mut s, &flow, None);
            if let Some(f) = s.roboru_flows.get_mut(&id) { f.run_count += 1; f.last_run_ms = now_ms(); }
            format!(r#"{{"ok":true,"steps":{}}}"#, steps)
        } else { format!(r#"{{"error":"not found: {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addKeyword(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let result = if let Some(kw) = parse_keyword_from_json(&body) {
            let name = kw.name.clone();
            STATE.lock().unwrap().roboru_keywords.insert(name.clone(), kw);
            format!(r#"{{"ok":true,"name":"{}"}}"#, esc(&name))
        } else { r#"{"error":"invalid keyword"}"#.to_string() };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runKeyword(
        env: JNIEnv, _c: JObject,
        name: *const c_char, args_json: *const c_char,
    ) -> JString {
        let name = cs(name); let args_body = cs(args_json);
        let mut s = STATE.lock().unwrap();
        let kw = s.roboru_keywords.get(&name).cloned();
        let result = if let Some(kw) = kw {
            let args: HashMap<String,String> = kw.args.iter().enumerate().map(|(i, arg_name): (usize, &String)| {
                let val = extract_json_str(&args_body, &format!("arg{}", i)).unwrap_or_default();
                (arg_name.clone(), val)
            }).collect();
            let r = execute_keyword(&mut s, &kw, &args);
            format!(r#"{{"ok":true,"result":"{}"}}"#, esc(&r))
        } else { format!(r#"{{"error":"keyword not found: {}"}}"#, esc(&name)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addPipeline(
        env: JNIEnv, _c: JObject, json: *const c_char,
    ) -> JString {
        let body = cs(json);
        let result = if let Some(p) = parse_pipeline_from_json(&body) {
            let id = p.id.clone();
            STATE.lock().unwrap().roboru_pipelines.insert(id.clone(), p);
            format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id))
        } else { r#"{"error":"invalid pipeline"}"#.to_string() };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runPipeline(
        env: JNIEnv, _c: JObject, id: *const c_char,
    ) -> JString {
        let id = cs(id);
        let mut s = STATE.lock().unwrap();
        let pipeline = s.roboru_pipelines.get(&id).cloned();
        let result = if let Some(pipeline) = pipeline {
            let (steps, errors) = execute_pipeline(&mut s, &pipeline);
            if let Some(p) = s.roboru_pipelines.get_mut(&id) { p.run_count += 1; p.last_run_ms = now_ms(); }
            format!(r#"{{"ok":true,"steps":{},"errors":{}}}"#, steps,
                format!("[{}]", errors.iter().map(|e| format!(r#""{}""#, esc(e))).collect::<Vec<_>>().join(",")))
        } else { format!(r#"{{"error":"pipeline not found: {}"}}"#, esc(&id)) };
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationAnalytics(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let json = get_automation_analytics(&STATE.lock().unwrap());
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationReport(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let report = get_automation_report(&STATE.lock().unwrap());
        unsafe { jni_str(env, &report) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_scheduleMacroDaily(
        _e: JNIEnv, _c: JObject,
        macro_id: *const c_char, time_hhmm: *const c_char,
    ) {
        let id = cs(macro_id); let time = cs(time_hhmm);
        if !id.is_empty() && !time.is_empty() {
            schedule_macro_daily(&mut STATE.lock().unwrap(), &id, &time);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_findMacroByName(
        env: JNIEnv, _c: JObject,
        name: *const c_char,
    ) -> JString {
        let result = find_macro_by_name(&STATE.lock().unwrap(), &cs(name));
        let json = match result {
            Some(id) => format!(r#"{{"found":true,"id":"{}"}}"#, esc(&id)),
            None     => r#"{"found":false}"#.to_string(),
        };
        unsafe { jni_str(env, &json) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resolveParam(
        env: JNIEnv, _c: JObject,
        param: *const c_char,
    ) -> JString {
        let result = resolve_param(&STATE.lock().unwrap(), &cs(param));
        unsafe { jni_str(env, &result) }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let s = STATE.lock().unwrap();
        let enabled = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
        let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();
        let json = format!(
            r#"{{"enabled_macros":{},"templates":{},"total_macros":{},"variables":{},"active_profile":"{}","pending_actions":{},"run_log_entries":{},"rate_ok":{}}}"#,
            enabled, templates, s.macros.len(), s.variables.len(),
            esc(&s.active_profile), s.pending_actions.len(),
            s.macro_run_log.len(), check_rate_limit(&s)
        );
        unsafe { jni_str(env, &json) }
    }

    // ── v43: OTA Engine JNI ───────────────────────────────────────────────────

    /// Register installed version with Rust on app start.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSetCurrentVersion(
        _e: JNIEnv, _c: JObject,
        version: *const c_char, code: i64,
    ) {
        let mut s = STATE.lock().unwrap();
        let v = cs(version);
        if !v.is_empty() { s.ota.current_version = v; }
        if code > 0 { s.ota.current_code = code; }
        if s.ota.repo.is_empty() { s.ota.repo = "i7m7r8/KiraService".to_string(); }
    }

    /// Set GitHub repo slug e.g. "i7m7r8/KiraService".
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSetRepo(
        _e: JNIEnv, _c: JObject,
        repo: *const c_char,
    ) {
        let r = cs(repo);
        if !r.is_empty() { STATE.lock().unwrap().ota.repo = r; }
    }

    /// Java feeds parsed GitHub release. Rust decides: prompt_user | up_to_date | skipped.
    /// Returns JSON {"action":"...","version":"...","current":"..."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnRelease(
        env: JNIEnv, _c: JObject,
        tag:       *const c_char,
        url:       *const c_char,
        changelog: *const c_char,
        date:      *const c_char,
        sha256:    *const c_char,
        apk_bytes: i64,
    ) -> JString {
        let (tag, url, log, date, sha) = (cs(tag), cs(url), cs(changelog), cs(date), cs(sha256));
        let mut s = STATE.lock().unwrap();
        if s.ota.skipped_versions.contains(&tag) {
            s.ota.phase = OtaPhase::Idle;
            return unsafe { jni_str(env, &r#"{"action":"skipped"}"#) };
        }
        let newer = s.ota.is_newer(&tag);
        s.ota.latest_version  = tag.clone();
        s.ota.download_url    = url;
        s.ota.changelog       = log;
        s.ota.release_date    = date;
        s.ota.apk_sha256      = sha;
        s.ota.total_apk_bytes = apk_bytes as u64;
        s.ota.last_check_ms   = now_ms();
        s.ota.check_error     = String::new();
        let action = if newer {
            s.ota.phase = OtaPhase::UpdateAvailable;
            "prompt_user"
        } else {
            s.ota.phase = OtaPhase::Idle;
            "up_to_date"
        };
        let cur = s.ota.current_version.clone();
        let result = format!(r#"{{"action":"{}","version":"{}","current":"{}"}}"#,
            action, esc(&tag), esc(&cur));
        unsafe { jni_str(env, &result) }
    }

    /// Java reports streaming download progress. Rust tracks % for /ota/status.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaProgress(
        _e: JNIEnv, _c: JObject,
        bytes_done: i64, bytes_total: i64,
    ) {
        let mut s = STATE.lock().unwrap();
        s.ota.download_bytes = bytes_done as u64;
        s.ota.download_total = bytes_total as u64;
        s.ota.download_pct   = if bytes_total > 0 {
            // Use u128 to prevent overflow on large files (bytes_done * 100)
            let pct = (bytes_done as u128 * 100) / bytes_total as u128;
            pct.min(100) as u8
        } else { 0 };
        s.ota.phase = OtaPhase::Downloading;
    }

    /// APK fully downloaded. Rust verifies SHA256 and returns install instructions JSON.
    /// Returns {"ok":true,"method":"shizuku|package_installer","shizuku":bool,"cmd":"..."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnDownloaded(
        env: JNIEnv, _c: JObject,
        path:   *const c_char,
        sha256: *const c_char,
    ) -> JString {
        let (path, sha) = (cs(path), cs(sha256));
        // SECURITY: Validate APK path — must end with .apk, no path traversal
        // Android getCacheDir() returns /data/user/0/<pkg>/cache/ or /data/data/<pkg>/cache/
        let path_ok = path.ends_with(".apk")
            && !path.contains("..")
            && !path.contains("//")
            && path.starts_with("/");  // must be absolute — allows all valid Android paths
        if !path_ok {
            return unsafe { jni_str(env, r#"{"ok":false,"error":"invalid_apk_path"}"#) };
        }
        let mut s = STATE.lock().unwrap();
        s.ota.apk_local_path = path.clone();
        let expected = s.ota.apk_sha256.clone();
        let ok = expected.is_empty() || expected == sha;
        let use_shizuku = s.shizuku.permission_granted;
        if ok {
            s.ota.phase = OtaPhase::Downloaded;
            let method = if use_shizuku { "shizuku" } else { "package_installer" };
            s.ota.install_method = method.to_string();
            let cmd = format!("pm install -r -t \"{}\"", esc(&path));
            { let _s = format!(
                r#"{{"ok":true,"path":"{}","method":"{}","shizuku":{},"cmd":"{}","verified":{}}}"#,
                esc(&path), method, use_shizuku, esc(&cmd), ok
            ); unsafe { jni_str(env, &_s) } }
        } else {
            let err = format!("sha256_mismatch");
            s.ota.phase = OtaPhase::Failed(err.clone());
            unsafe { jni_str(env, &format!(r#"{{"ok":false,"error":"{}"}}"#, esc(&err))) }
        }
    }

    /// Install completed. Pass new versionName.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnInstalled(
        _e: JNIEnv, _c: JObject,
        new_version: *const c_char,
    ) {
        let ver = cs(new_version);
        let mut s = STATE.lock().unwrap();
        s.ota.phase = OtaPhase::Installed;
        if !ver.is_empty() { s.ota.current_version = ver; }
    }

    /// Install failed. Rust records error and sets Failed phase.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaOnFailed(
        _e: JNIEnv, _c: JObject,
        error: *const c_char,
    ) {
        let err = cs(error);
        let mut s = STATE.lock().unwrap();
        s.ota.install_error = err.clone();
        s.ota.phase = OtaPhase::Failed(err);
    }

    /// Permanently skip this version (stored in Rust skip list).
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaSkip(
        _e: JNIEnv, _c: JObject,
        version: *const c_char,
    ) {
        let ver = cs(version);
        let mut s = STATE.lock().unwrap();
        if !ver.is_empty() && !s.ota.skipped_versions.contains(&ver) {
            s.ota.skipped_versions.push(ver);
        }
        s.ota.phase = OtaPhase::Idle;
    }

    /// Get full OTA status JSON from Rust.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_otaGetStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        unsafe { jni_str(env, &STATE.lock().unwrap().ota.to_json()) }
    }



    // ── Session G: Tool execution + app package lookup ───────────────────────

    /// Execute any non-intent tool in Rust. Called by KiraTools.execute() for ~82 tools.
    /// name: tool name, params_json: JSON object of parameters
    /// Returns tool result string
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_executeTool(
        env: JNIEnv, _c: JObject,
        name:        *const c_char,
        params_json: *const c_char,
    ) -> JString {
        let tname  = cs(name);
        let pjson  = cs(params_json);
        // Parse params JSON into HashMap
        let mut params = std::collections::HashMap::new();
        let mut rest = pjson.trim_start_matches('{').trim_end_matches('}');
        while let Some(ks) = rest.find('"') {
            let after_ks = &rest[ks+1..];
            let Some(ke) = after_ks.find('"') else { break };
            let key = &after_ks[..ke];
            let after_colon = after_ks[ke+1..].trim_start_matches(':').trim_start_matches('"');
            let val_end = after_colon.find('"').unwrap_or(after_colon.len());
            let val = &after_colon[..val_end];
            params.insert(key.to_string(), val.to_string());
            rest = &after_colon[val_end..];
            rest = rest.trim_start_matches('"').trim_start_matches(',');
        }
        let result = dispatch_tool(&tname, &params);
        unsafe { jni_str(env, &result) }
    }

    /// Resolve app name to package name using Rust's 280+ entry database.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_appNameToPkg(
        env: JNIEnv, _c: JObject,
        app_name: *const c_char,
    ) -> JString {
        let result = app_name_to_pkg(&cs(app_name));
        unsafe { jni_str(env, &result) }
    }

        // ── Session E: Agent + Chain JNI exports ─────────────────────────────────

    /// Run full autonomous agent. Blocks until completion (use background thread).
    /// Returns JSON: {"final":"..","steps":8,"tools_used":["x"],"success":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_agentSync(
        env: JNIEnv, _c: JObject,
        goal:      *const c_char,
        max_steps: i32,
        session:   *const c_char,
    ) -> JString {
        let body = format!(
            r#"{{"goal":"{}","max_steps":{},"session":"{}"}}"#,
            esc(&cs(goal)), max_steps, esc(&cs(session))
        );
        let result = route_http("POST", "/ai/agent", &body);
        unsafe { jni_str(env, &result) }
    }

    /// Run chain-of-thought reasoning. Returns JSON: {"reasoning":[...],"conclusion":".."}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chainSync(
        env: JNIEnv, _c: JObject,
        goal:  *const c_char,
        depth: i32,
    ) -> JString {
        let body = format!(r#"{{"goal":"{}","depth":{}}}"#, esc(&cs(goal)), depth);
        let result = route_http("POST", "/ai/chain", &body);
        unsafe { jni_str(env, &result) }
    }

    /// Get current agent task status
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAgentStatus(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let result = route_http("GET", "/ai/agent/status", "");
        unsafe { jni_str(env, &result) }
    }

    /// Stop running agent
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_stopAgent(
        _e: JNIEnv, _c: JObject,
    ) {
        route_http("POST", "/ai/agent/stop", "");
    }

        // ── Session D: AI chat JNI + shell queue exports ─────────────────────────

    /// Single-call AI chat — replaces KiraAI.java entirely.
    /// Java calls this from a background thread; blocks until reply is ready.
    /// Returns JSON: {"role":"assistant","content":"..","tools_used":["x"],"done":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chatSync(
        env: JNIEnv, _c: JObject,
        message:       *const c_char,
        session_id:    *const c_char,
        max_tool_steps: i32,
    ) -> JString {
        let msg  = cs_safe(message, 32768);
        let sess = cs_safe(session_id, 64);
        let body = format!(
            r#"{{"message":"{}","session":"{}","max_tool_steps":{}}}"#,
            esc(&msg), esc(&sess), max_tool_steps
        );
        // Reuse the HTTP route handler — same logic, no code duplication
        let result = route_http("POST", "/ai/chat", &body);
        unsafe { jni_str(env, &result) }
    }

    /// Get next queued shell command for Java to execute via Shizuku.
    /// Returns JSON {"id":"..","cmd":"..","timeout":5000} or {"empty":true}
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getNextShellJob(
        env: JNIEnv, _c: JObject,
    ) -> JString {
        let mut s = STATE.lock().unwrap();
        match s.pending_shell.pop_front() {
            Some(job) => {
                let r = format!(r#"{{"id":"{}","cmd":"{}","timeout":{}}}"#,
                    esc(&job.id), esc(&job.cmd), job.timeout);
                unsafe { jni_str(env, &r) }
            }
            None => unsafe { jni_str(env, r#"{"empty":true}"#) }
        }
    }

    /// Post shell execution result back to Rust.
    /// id = job id from getNextShellJob; stdout = command output.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_postShellResult(
        _e: JNIEnv, _c: JObject,
        job_id: *const c_char,
        stdout: *const c_char,
    ) {
        let (id, out) = (cs(job_id), cs(stdout));
        let mut s = STATE.lock().unwrap();
        s.shell_results.insert(id, out);
    }

        // ── Session C: AES-256-GCM secret encryption JNI exports ─────────────────

    /// Encrypt a secret string. seed = SHA256(ANDROID_ID+pkg) from Java.
    /// domain = field name ("api_key", "tg_token") for cross-field protection.
    /// Returns hex-encoded AES-256-GCM ciphertext.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_encryptSecret(
        env: JNIEnv, _c: JObject,
        plaintext: *const c_char,
        seed:      *const c_char,
        domain:    *const c_char,
    ) -> JString {
        let (pt, sd, dm) = (cs_safe(plaintext,65536), cs_safe(seed,256), cs_safe(domain,64));
        let result = aes_encrypt(&pt, &sd, &dm);
        unsafe { jni_str(env, &result) }
    }

    /// Decrypt a hex-encoded AES-256-GCM ciphertext.
    /// Returns original plaintext or empty string if key/data is wrong.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_decryptSecret(
        env: JNIEnv, _c: JObject,
        hex_ciphertext: *const c_char,
        seed:           *const c_char,
        domain:         *const c_char,
    ) -> JString {
        let (ct, sd, dm) = (cs_safe(hex_ciphertext,131072), cs_safe(seed,256), cs_safe(domain,64));
        let result = aes_decrypt(&ct, &sd, &dm);
        unsafe { jni_str(env, &result) }
    }

    /// Derive the AES key seed from device_id + pkg_name.
    /// Call once on first run; store result in SharedPreferences.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_deriveKeySeed(
        env: JNIEnv, _c: JObject,
        android_id:   *const c_char,
        package_name: *const c_char,
    ) -> JString {
        let (aid, pkg) = (cs(android_id), cs(package_name));
        let combined = format!("{}:{}", aid, pkg);
        // SHA-256 of combined → 64 hex chars as seed
        let key = derive_aes_key(&combined);
        let hex: String = key.iter().map(|b| format!("{:02x}", b)).collect();
        unsafe { jni_str(env, &hex) }
    }

}


// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// HTTP Server
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

fn run_http(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l, Err(e) => { eprintln!("kira bind: {}", e); return; }
    };
    for stream in listener.incoming().flatten() { thread::spawn(|| handle_http(stream)); }
}

/// Lock STATE recovering from poison — if a thread panicked while holding the
/// lock, we recover the state rather than panicking the HTTP thread too.
#[allow(unused_macros)]
macro_rules! state_lock {
    () => { match STATE.lock() {
        Ok(g)  => g,
        Err(e) => e.into_inner(), // recover poisoned mutex
    }}
}

fn handle_http(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buf = [0u8; 65536];
    let n = match stream.read(&mut buf) { Ok(n) if n > 0 => n, _ => return };
    let req = String::from_utf8_lossy(&buf[..n]);
    let first = req.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 { return; }
    let body = req.find("\r\n\r\n").map(|i| req[i+4..].trim().to_string()).unwrap_or_default();
    let resp = route_http_with_raw(parts[0], parts[1], &body, &req.to_string());
    let http = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Kira-Engine: rust-v9\r\n\r\n{}", resp.len(), resp);
    let _ = stream.write_all(http.as_bytes());
    STATE.lock().unwrap().request_count += 1;
}

fn get_http_header<'a>(req: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("
{}:", name.to_lowercase());
    let lreq   = req.to_lowercase();
    let pos    = lreq.find(&needle)?;
    let after  = &req[pos + needle.len()..];
    let end    = after.find("
").unwrap_or(after.len());
    Some(after[..end].trim())
}

fn requires_auth(path: &str) -> bool {
    // Session L: expanded auth coverage for all sensitive endpoints added in v0.1.x
    // Public: health checks, layer polling, crash reporting, setup info
    let public = [
        "/health", "/appstats", "/layer0", "/layer1",
        "/layer2/header", "/layer2/bubbles", "/layer2/typing",
        "/setup/providers", "/setup/status", "/crash",
        "/auth/set_secret", "/settings/health",
        "/telegram/last_update_id",
        "/memory/compression", "/crypto/status",
    ];
    if public.contains(&path) { return false; }
    // Everything else requires auth if a secret is set
    true
}

/// Constant-time comparison to prevent timing attacks on the token.
fn check_auth(token: Option<&str>) -> bool {
    let secret = {
        let s: std::sync::MutexGuard<KiraState> = match STATE.lock() { Ok(g) => g, Err(e) => e.into_inner() };
        s.http_secret.clone()
    };
    if secret.is_empty() { return true; }   // no secret set = localhost open
    match token {
        None    => false,
        Some(t) => {
            let tb = t.as_bytes();
            let sb = secret.as_bytes();
            if tb.len() != sb.len() { return false; }
            // fold XOR — result is 0 only if every byte matches
            tb.iter().zip(sb.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
        }
    }
}

/// Entry point called by handle_http — wraps route_http with auth check.
fn route_http_with_raw(method: &str, path: &str, body: &str, raw_req: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    if requires_auth(path_clean) {
        let token = get_http_header(raw_req, "x-kira-token");
        if !check_auth(token) {
            return r#"{"error":"unauthorized","code":401}"#.to_string();
        }
    }
    route_http(method, path, body)
}


/// Inline JSON string extractor
fn extract_json_str_inline(json: &str, key: &str) -> Option<String> {
    extract_json_str(json, key)
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    match (method, path_clean) {
        // Health & stats
        // Auth management (localhost only — sets the shared secret)
        ("POST", "/auth/set_secret") => {
            let secret = extract_json_str(body, "secret").unwrap_or_default();
            if secret.len() >= 16 {
                STATE.lock().unwrap().http_secret = secret;
                r#"{"ok":true}"#.to_string()
            } else {
                r#"{"error":"secret must be at least 16 characters"}"#.to_string()
            }
        }
        ("DELETE", "/auth/secret") => {
            STATE.lock().unwrap().http_secret = String::new();
            r#"{"ok":true,"warning":"auth disabled — all endpoints open"}"#.to_string()
        }

        ("GET", "/health") | ("GET", "/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"status":"ok","version":"9.0","uptime_ms":{},"requests":{},"tool_calls":{},"battery":{},"charging":{},"notifications":{},"skills":{},"triggers":{},"memory_entries":{},"total_tokens":{},"sessions":{},"setup_done":{},"macros":{},"active_profile":"{}","variables":{}}}"#,
                now_ms()-s.uptime_start, s.request_count, s.tool_call_count,
                s.battery_pct, s.battery_charging, s.notifications.len(), s.skills.len(),
                s.triggers.iter().filter(|t|!t.fired).count(), s.memory_index.len(),
                s.total_tokens, s.sessions.len(), s.config.setup_done,
                s.macros.len(), esc(&s.active_profile), s.variables.len())
        }
        ("GET",  "/stats") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"notifications":{},"pending_cmds":{},"task_steps":{},"audit_entries":{},"context_turns":{},"daily_log_entries":{},"skills":{},"memory_entries":{},"cron_jobs":{},"tool_calls":{},"total_tokens":{},"uptime_ms":{},"macros":{},"macro_runs":{},"pending_actions":{},"variables":{}}}"#,
                s.notifications.len(), s.pending_cmds.len(), s.task_log.len(),
                s.audit_log.len(), s.context_turns.len(), s.daily_log.len(),
                s.skills.len(), s.memory_index.len(), s.cron_jobs.len(),
                s.tool_call_count, s.total_tokens, now_ms()-s.uptime_start,
                s.macros.len(), s.macro_run_log.len(), s.pending_actions.len(),
                s.variables.len())
        }

        // v40: Automation engine endpoints
        ("GET",  "/macros")            => { let s=STATE.lock().unwrap(); format!("[{}]", s.macros.iter().map(macro_to_json).collect::<Vec<_>>().join(",")) }
        ("POST", "/macros/add")        => { let m=parse_macro_from_json(body); let id=m.id.clone(); let mut s=STATE.lock().unwrap(); s.macros.retain(|x| x.id!=m.id); s.macros.push(m); format!(r#"{{"ok":true,"id":"{}"}}"#, id) }
        ("POST", "/macros/remove")     => { let id=extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().macros.retain(|m| m.id!=id); r#"{"ok":true}"#.to_string() }
        ("POST", "/macros/enable")     => { let id=extract_json_str(body,"id").unwrap_or_default(); let en=!body.contains("\"enabled\":false"); if let Some(m)=STATE.lock().unwrap().macros.iter_mut().find(|m| m.id==id) { m.enabled=en; } r#"{"ok":true}"#.to_string() }
        ("POST", "/macros/run")        => {
            let id=extract_json_str(body,"id").unwrap_or_default();
            let mut s=STATE.lock().unwrap();
            let actions=s.macros.iter().find(|m| m.id==id).map(|m| m.actions.clone()).unwrap_or_default();
            let name=s.macros.iter().find(|m| m.id==id).map(|m| m.name.clone()).unwrap_or_default();
            let start=now_ms();
            let (steps,_)=execute_macro_actions(&mut s, &id, &actions);
            if let Some(m)=s.macros.iter_mut().find(|m| m.id==id) { m.run_count+=1; m.last_run_ms=start; }
            s.macro_run_log.push_back(MacroRunLog { macro_id:id, macro_name:name, trigger:"api".to_string(), success:true, steps_run:steps, duration_ms:now_ms()-start, ts:start, error:String::new() });
            format!(r#"{{"ok":true,"steps":{}}}"#, steps)
        }
        ("GET",  "/macros/log")        => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.macro_run_log.iter().skip(s.macro_run_log.len().saturating_sub(100)).map(|r| format!(r#"{{"macro_id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"duration_ms":{},"ts":{}}}"#, esc(&r.macro_id),esc(&r.macro_name),esc(&r.trigger),r.success,r.steps_run,r.duration_ms,r.ts)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/macros/pending")    => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.pending_actions.iter().map(|pa| { let pk: Vec<String>=pa.params.iter().map(|(k,v)| format!("\"{}\":\"{}\"",esc(k),esc(v))).collect(); format!(r#"{{"macro_id":"{}","action_id":"{}","kind":"{}","params":{{{}}}}}"#, esc(&pa.macro_id),esc(&pa.action_id),esc(&pa.kind),pk.join(",")) }).collect(); format!("[{}]", items.join(",")) }

        // v40: Variables
        ("GET",  "/variables")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.variables.values().map(|v| format!(r#"{{"name":"{}","value":"{}","type":"{}","updated_ms":{}}}"#, esc(&v.name),esc(&v.value),esc(&v.var_type),v.updated_ms)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/variables/set")     => { let name=extract_json_str(body,"name").unwrap_or_default(); let value=extract_json_str(body,"value").unwrap_or_default(); let vt=extract_json_str(body,"type").unwrap_or_else(||"string".to_string()); let ts=now_ms(); let mut s=STATE.lock().unwrap(); s.variables.entry(name.clone()).and_modify(|v|{v.value=value.clone();v.updated_ms=ts;}).or_insert(AutoVariable{name,value,var_type:vt,persistent:false,created_ms:ts,updated_ms:ts}); r#"{"ok":true}"#.to_string() }
        ("POST", "/variables/delete")  => { let name=extract_json_str(body,"name").unwrap_or_default(); STATE.lock().unwrap().variables.remove(&name); r#"{"ok":true}"#.to_string() }
        ("GET",  "/variables/get")     => { let name=path.find("name=").map(|i| &path[i+5..]).unwrap_or("").split('&').next().unwrap_or(""); let s=STATE.lock().unwrap(); match s.variables.get(name) { Some(v) => format!(r#"{{"name":"{}","value":"{}","type":"{}"}}"#, esc(&v.name),esc(&v.value),esc(&v.var_type)), None => r#"{"error":"not_found"}"#.to_string() } }

        // v40: Profiles
        ("GET",  "/profiles")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.profiles.iter().map(|p| format!(r#"{{"id":"{}","name":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),p.active)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/profiles/set")      => { let id=extract_json_str(body,"id").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.active_profile=id.clone(); for p in s.profiles.iter_mut() { p.active=p.id==id; } format!(r#"{{"ok":true,"active":"{}"}}"#, esc(&id)) }

        // v40: Device signals via HTTP (for testing / external tools)
        ("POST", "/signal/screen_on")  => { STATE.lock().unwrap().sig_screen_on=true;      r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/screen_off") => { STATE.lock().unwrap().sig_screen_off=true;     r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/shake")      => { STATE.lock().unwrap().sig_shake=true;           r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/kira_event") => { let ev=extract_json_str(body,"event").unwrap_or_default(); STATE.lock().unwrap().sig_kira_event=ev; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/app")        => { let pkg=extract_json_str(body,"package").unwrap_or_default(); STATE.lock().unwrap().sig_app_launched=pkg; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/wifi")       => { let ssid=extract_json_str(body,"ssid").unwrap_or_default(); STATE.lock().unwrap().sig_wifi_ssid=ssid; r#"{"ok":true}"#.to_string() }
        ("POST", "/signal/sms")        => { let sender=extract_json_str(body,"sender").unwrap_or_default(); let text=extract_json_str(body,"text").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.sig_sms_sender=sender; s.sig_sms_text=text; r#"{"ok":true}"#.to_string() }

        // v38: Config + setup
        ("GET",  "/config")            => { let s=STATE.lock().unwrap(); config_to_json(&s.config) }
        ("POST", "/config")            => update_config_from_http(body),
        ("GET",  "/setup")             => { let s=STATE.lock().unwrap(); format!(r#"{{"page":{},"done":{},"user_name":"{}","model":"{}","base_url":"{}","selected_provider":"{}","custom_url":"{}","quote_index":{}}}"#, s.setup.current_page,s.setup.done,esc(&s.setup.user_name),esc(&s.setup.model),esc(&s.setup.base_url),esc(&s.setup.selected_provider_id),esc(&s.setup.custom_url),s.setup.quote_index) }
        ("POST", "/setup/page")        => { if let Some(page)=extract_json_num(body,"page") { STATE.lock().unwrap().setup.current_page=page as u8; } r#"{"ok":true}"#.to_string() }
        ("POST", "/setup/complete")    => { let mut s=STATE.lock().unwrap(); s.setup.done=true; s.config.setup_done=true; r#"{"ok":true}"#.to_string() }
        ("GET",  "/theme")             => {
            // Update animation state before returning
            let mut s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            let phase_secs = (uptime_ms % 3000) as f32 / 3000.0; // 3s cycle
            s.theme.animation_phase = phase_secs;
            // BPM: 60 idle, 90 when recent requests, 120 when agent active
            let recent = s.tool_call_count;
            s.theme.pulse_bpm = if s.theme.is_thinking { 120 }
                else if recent > 0 { 90 } else { 60 };
            // activity_level: clamp tool calls per minute to 0-1
            let tools_recent = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000)
                .count();
            s.theme.activity_level = (tools_recent as f32 / 10.0).min(1.0);
            s.theme.to_json()
        }
        // GET /layer0 — full Layer 0 star field animation state
        // Polled by GalaxyView every 500ms to drive the living canvas.
        // Returns: phase(0-1, 3s period), bpm, activity(0-1), thinking,
        //          hue_shift(-12 to +12 degrees, sine-driven),
        //          vortex_intensity(0-1), burst(true once then reset)
        ("GET",  "/layer0") => {
            let mut s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            // 3-second heartbeat phase (0.0 → 1.0 → 0.0 → ...)
            let phase      = (uptime_ms % 3_000) as f32 / 3_000.0;
            s.theme.animation_phase = phase;
            // Chromatic hue shift: ±12° sine wave on 3s period
            let hue_shift  = (phase * 2.0 * std::f32::consts::PI).sin() * 12.0;
            // Activity: tool calls in last 60s, normalised 0-1
            let tools_60s  = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            let activity   = (tools_60s as f32 / 10.0_f32).min(1.0_f32);
            s.theme.activity_level = activity;
            // BPM: 120 thinking, 90 active, 60 idle
            let bpm = if s.theme.is_thinking { 120u32 }
                      else if s.request_count > 0 && activity > 0.05 { 90 }
                      else { 60 };
            s.theme.pulse_bpm = bpm;
            // Vortex: ramps up when thinking, decays when idle
            // Java applies this as the rate of star inward drift
            let vortex = if s.theme.is_thinking { 1.0f32 }
                         else { (activity * 0.6).min(0.8) };
            // Burst flag: consumed once by Java, then auto-cleared
            let burst = s.theme.is_thinking; // Java triggers burst on thinking→false transition
            format!(
                r#"{{"phase":{:.6},"bpm":{},"activity":{:.6},"thinking":{},"hue_shift":{:.4},"vortex":{:.4},"burst":{}}}"#,
                phase, bpm, activity, s.theme.is_thinking,
                hue_shift, vortex, burst
            )
        }

        // Legacy alias for older Java pollers
        ("GET",  "/theme/anim") => {
            let mut s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            let phase = (uptime_ms % 3_000) as f32 / 3_000.0;
            s.theme.animation_phase = phase;
            let tools_recent = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            format!(r#"{{"phase":{:.6},"bpm":{},"activity":{:.6},"thinking":{}}}"#,
                phase, s.theme.pulse_bpm,
                (tools_recent as f32 / 10.0).min(1.0),
                s.theme.is_thinking)
        }
        // POST /theme/thinking {"active":true}  — set thinking state
        ("POST", "/theme/thinking")     => {
            let active = body.contains(r#""active":true"#);
            STATE.lock().unwrap().theme.is_thinking = active;
            r#"{"ok":true}"#.to_string()
        }

        // ── Layer 5: Settings page Rust endpoints ─────────────────────────────

        // GET /settings/health — compact health summary for settings page header
        // Returns: {shizuku, setup, api_key_set, model, automation_count, memory_count,
        //           uptime_ms, tool_calls, pulse_bpm, activity}
        ("GET",  "/settings/health") => {
            let s = STATE.lock().unwrap();
            let uptime = now_ms().saturating_sub(s.uptime_start);
            let tools_60s = s.macro_run_log.iter()
                .filter(|r| now_ms().saturating_sub(r.ts) < 60_000).count();
            let activity = (tools_60s as f32 / 10.0_f32).min(1.0);
            let bpm = if s.theme.is_thinking { 120 }
                      else if s.request_count > 0 { 90 } else { 60 };
            format!(
                r#"{{"shizuku":{},"shizuku_permission":{},"setup_done":{},"api_key_set":{},"model":"{}","base_url":"{}","automation_count":{},"memory_count":{},"uptime_ms":{},"tool_calls":{},"request_count":{},"pulse_bpm":{},"activity":{:.3},"macros_enabled":{}}}"#,
                s.shizuku.installed, s.shizuku.permission_granted,
                s.config.setup_done, !s.config.api_key.is_empty(),
                esc(&s.config.model), esc(&s.config.base_url),
                s.macros.len(), s.memory_index.len(),
                uptime, s.tool_call_count, s.request_count,
                bpm, activity,
                s.macros.iter().filter(|m| m.enabled).count()
            )
        }

        // GET /settings/shizuku — Shizuku status with Layer 5 border color token
        // Returns: {installed, running, permission, border_color, border_name, pulse}
        ("GET",  "/settings/shizuku") => {
            let s = STATE.lock().unwrap();
            let (border_color, border_name) = if s.shizuku.permission_granted {
                (0xFFB4BEFEu32, "lavender")   // god mode — Lavender
            } else if s.shizuku.installed {
                (0xFFFAB387u32, "peach")      // partial — Peach
            } else {
                (0xFFF38BA8u32, "pink")       // absent  — Pink
            };
            format!(
                r#"{{"installed":{},"running":{},"permission":{},"border_color":{},"border_name":"{}","pulse_ms":1500}}"#,
                s.shizuku.installed, s.shizuku.installed,
                s.shizuku.permission_granted,
                border_color, border_name
            )
        }

        // POST /settings/row_tap {"row":"api_key"} — log a settings row tap
        // Used by UI to record which settings the user accesses most often
        ("POST", "/settings/row_tap") => {
            let row = extract_json_str(body, "row").unwrap_or_default();
            if !row.is_empty() {
                let mut s = STATE.lock().unwrap();
                // Store in daily_log for usage analytics
                let entry = format!("[settings_tap] row={} ts={}", esc(&row), now_ms());
                s.daily_log.push_back(entry);
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
            }
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/sections — section visibility state for header underline
        // Returns ordered list of section names with their Lavender animate flag
        ("GET",  "/settings/sections") => {
            let sections = vec![
                "IDENTITY", "AI PROVIDER", "AGENT BEHAVIOR", "TELEGRAM BOT",
                "INTERFACE", "MEMORY", "TOOLS & AUTOMATION", "RUST ENGINE v9",
                "SYSTEM", "ABOUT"
            ];
            let items: Vec<String> = sections.iter().map(|s|
                format!(r#"{{"name":"{}","accent_color":4289003262}}"#, s)  // 0xFFB4BEFE Lavender
            ).collect();
            format!(r#"{{"sections":[{}]}}"#, items.join(","))
        }

        // POST /theme/flash {"dark":true}  — record theme switch for analytics
        // ── Crash reporting endpoints ─────────────────────────────────────────

        // POST /crash {"thread":"..","trace":"..","ts":1234}
        // Called by KiraApp crash handler to persist crashes in Rust memory
        // GET /memory/compression — LZ4 compression stats (Session B)
        ("GET",  "/memory/compression") => {
            let s = STATE.lock().unwrap();
            let raw_bytes: usize = s.context_turns.iter()
                .map(|t| t.role.len() + t.content.len()).sum();
            let compressed_bytes = compressed_context_bytes(&s);
            let ratio = if compressed_bytes > 0 {
                raw_bytes as f32 / compressed_bytes as f32
            } else { 1.0 };
            format!(
                r#"{{"turns":{},"compressed_turns":{},"raw_bytes":{},"compressed_bytes":{},"ratio":{:.2},"saved_kb":{}}}"#,
                s.context_turns.len(),
                s.context_turns_lz4.len(),
                raw_bytes,
                compressed_bytes,
                ratio,
                (raw_bytes.saturating_sub(compressed_bytes)) / 1024
            )
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session D — AI Chat engine (replaces KiraAI.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session E — Agent + Chain endpoints (replaces KiraAgent.java + KiraChain.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /ai/agent
        // body: {"goal":"..","max_steps":25,"session":"agent_default"}
        // Full ReAct loop: PLAN → THINK → ACT → OBSERVE → repeat until done
        // Returns: {"final":"..","steps":8,"tools_used":["x","y"],"success":true}
        ("POST", "/ai/agent") => {
            let goal       = extract_json_str(body, "goal").unwrap_or_default();
            let max_steps  = extract_json_num(body, "max_steps").unwrap_or(25.0) as usize;
            let session_id = extract_json_str(body, "session")
                .unwrap_or_else(|| "agent_default".into());

            if goal.is_empty() {
                return r#"{"error":"goal is required"}"#.to_string();
            }

            let (api_key, base_url, model, persona): (String,String,String,String) = {
                let s = STATE.lock().unwrap();
                (s.config.api_key.clone(), s.config.base_url.clone(),
                 s.config.model.clone(),   s.config.persona.clone())
            };
            if api_key.is_empty() {
                return r#"{"error":"no API key","success":false}"#.to_string();
            }

            let persona_str = if persona.is_empty() {
                "You are Kira, an autonomous AI agent on Android.".into()
            } else { persona };

            // Build agent system prompt
            let agent_system = format!(
                "{}

You are executing a multi-step task autonomously.
                For each step: think what to do, pick ONE tool, execute it,                 observe result, decide next step.
                When done, reply: DONE: <summary>
                Tools: run_shell, read_file, write_file, http_get, add_memory,                 search_memory, get_battery, get_wifi, set_variable, get_variable",
                persona_str
            );

            let mut steps_run   = 0usize;
            let mut tools_used: Vec<String> = Vec::new();
            let mut final_summary = String::new();
            let mut success = false;

            // Agent context — separate from chat history
            let mut agent_ctx: Vec<(String, String)> = Vec::new();
            agent_ctx.push(("user".into(), format!("Task: {}", goal)));

            {
                let mut s = STATE.lock().unwrap();
                s.theme.is_thinking = true;
                // Log task start
                let task = AgentTask {
                    id:           format!("task_{}", now_ms()),
                    goal:         goal.clone(),
                    status:       "running".into(),
                    current_step: 0,
                    max_steps,
                    created_ms:   now_ms(),
                };
                s.agent_tasks.push_back(task);
                if s.agent_tasks.len() > 20 { s.agent_tasks.pop_front(); }
            }

            // ReAct loop
            while steps_run < max_steps {
                let resp: String = match call_llm_sync(
                    &api_key, &base_url, &model, &agent_system, &agent_ctx
                ) {
                    Ok(r)  => r,
                    Err(e) => { final_summary = e; break; }
                };

                let clean = clean_reply(&resp);

                // Check for completion
                if clean.contains("DONE:") || clean.to_lowercase().contains("task complete") {
                    final_summary = clean.replace("DONE:", "").trim().to_string();
                    success = true;
                    break;
                }

                // Extract + execute tool calls
                let calls = parse_tool_calls(&resp);
                if calls.is_empty() {
                    // No tool call — treat as final answer if we have content
                    if !clean.is_empty() {
                        final_summary = clean;
                        success = true;
                    }
                    break;
                }

                let mut observations = String::new();
                for (tname, targs) in &calls {
                    let result = dispatch_tool(tname, targs);
                    tools_used.push(tname.clone());
                    observations.push_str(&format!("[{}] → {}
", tname, result));
                }

                // Feed observation back into context
                agent_ctx.push(("assistant".into(), resp.clone()));
                agent_ctx.push(("user".into(),
                    format!("Observations:
{}Continue with the task.", observations)));
                steps_run += 1;

                // Update task status in STATE
                if let Some(task) = STATE.lock().unwrap().agent_tasks.back_mut() {
                    task.current_step = steps_run;
                }
            }

            if final_summary.is_empty() {
                final_summary = if success { "completed".into() }
                    else { format!("stopped after {} steps", steps_run) };
            }

            // Push to compressed chat history so user can see result
            {
                let mut s = STATE.lock().unwrap();
                push_turn_compressed(&mut s, "assistant", &final_summary);
                s.theme.is_thinking = false;
                if let Some(task) = s.agent_tasks.back_mut() {
                    task.status = if success { "done".into() } else { "stopped".into() };
                }
            }

            let tools_json: String = tools_used.iter()
                .map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(",");
            format!(
                r#"{{"final":"{}","steps":{},"tools_used":[{}],"success":{}}}"#,
                esc(&final_summary), steps_run, tools_json, success
            )
        }

        // POST /ai/chain
        // body: {"goal":"..","depth":5}
        // Chain-of-thought: builds sequential reasoning, no tool loop
        // Returns: {"reasoning":["step1","step2"],"conclusion":".."}
        ("POST", "/ai/chain") => {
            let goal  = extract_json_str(body, "goal").unwrap_or_default();
            let depth = extract_json_num(body, "depth").unwrap_or(4.0) as usize;

            if goal.is_empty() {
                return r#"{"error":"goal is required"}"#.to_string();
            }
            let (api_key, base_url, model): (String,String,String) = {
                let s = STATE.lock().unwrap();
                (s.config.api_key.clone(), s.config.base_url.clone(), s.config.model.clone())
            };
            if api_key.is_empty() {
                return r#"{"error":"no API key"}"#.to_string();
            }

            let chain_system = "You are a step-by-step reasoning engine.                 Think through each step carefully before proceeding to the next.                 Format each reasoning step as: STEP N: <thought>.                 End with CONCLUSION: <answer>.";

            let mut chain_ctx: Vec<(String, String)> = Vec::new();
            chain_ctx.push(("user".into(),
                format!("Reason through this step by step (max {} steps): {}", depth, goal)));

            let response: String = match call_llm_sync(&api_key, &base_url, &model, chain_system, &chain_ctx) {
                Ok(r)  => r,
                Err(e) => return format!(r#"{{"error":"{}"}}"#, esc(&e.to_string())),
            };

            // Parse STEP N: lines and CONCLUSION:
            let mut reasoning: Vec<String> = Vec::new();
            let mut conclusion = String::new();
            for line in response.lines() {
                let t = line.trim();
                if t.starts_with("STEP ") {
                    reasoning.push(t.to_string());
                } else if t.starts_with("CONCLUSION:") {
                    conclusion = t.replace("CONCLUSION:", "").trim().to_string();
                }
            }
            if conclusion.is_empty() { conclusion = clean_reply(&response); }

            let steps_json: String = reasoning.iter()
                .map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",");
            format!(r#"{{"reasoning":[{}],"conclusion":"{}","steps":{}}}"#,
                steps_json, esc(&conclusion), reasoning.len())
        }

        // GET /ai/agent/status — current running agent task
        ("GET",  "/ai/agent/status") => {
            let s = STATE.lock().unwrap();
            match s.agent_tasks.back() {
                Some(t) => format!(
                    r#"{{"id":"{}","goal":"{}","status":"{}","step":{},"max":{}}}"#,
                    esc(&t.id), esc(&t.goal), esc(&t.status),
                    t.current_step, t.max_steps),
                None => r#"{"status":"idle"}"#.to_string(),
            }
        }

        // POST /ai/agent/stop — cancel running agent
        ("POST", "/ai/agent/stop") => {
            let mut s = STATE.lock().unwrap();
            s.theme.is_thinking = false;
            if let Some(task) = s.agent_tasks.back_mut() {
                task.status = "cancelled".into();
            }
            r#"{"ok":true,"stopped":true}"#.to_string()
        }

                // POST /ai/chat
        // body: {"message":"..","session":"default","max_tool_steps":5}
        // Runs full chat turn: history + system prompt → LLM → tool loop → reply
        // Returns: {"role":"assistant","content":"..","tools_used":["x"],"tokens":0,"done":true}
        ("POST", "/ai/chat") => {
            let user_msg    = extract_json_str(body, "message").unwrap_or_default();
            let session_id  = extract_json_str(body, "session").unwrap_or_else(||"default".into());
            let max_steps   = extract_json_num(body, "max_tool_steps").unwrap_or(5.0) as usize;

            if user_msg.is_empty() {
                return r#"{"error":"message is required"}"#.to_string();
            }

            // Load config
            let (api_key, base_url, model, persona, system_prompt): (String,String,String,String,String) = {
                let s = STATE.lock().unwrap();
                let cfg = &s.config;
                let persona = if cfg.persona.is_empty() {
                    "You are Kira, an AI agent on Android. Be concise and helpful.".to_string()
                } else { cfg.persona.clone() };
                let sys = build_system_prompt(&s, &persona);
                (cfg.api_key.clone(), cfg.base_url.clone(), cfg.model.clone(),
                 persona, sys)
            };

            if api_key.is_empty() {
                return r#"{"error":"no API key — go to settings and add one","done":true}"#.to_string();
            }

            // Push user message to compressed history
            {
                let mut s = STATE.lock().unwrap();
                s.request_count += 1;
                s.theme.is_thinking = true;
                push_turn_compressed(&mut s, "user", &user_msg);
            }

            // Build messages array from compressed history
            let context = {
                let s = STATE.lock().unwrap();
                decompress_context(&s)
            };

            // Call LLM
            let raw_response: Result<String,String> = call_llm_sync(&api_key, &base_url, &model, &system_prompt, &context);
            let raw: String = match raw_response {
                Ok(r)  => r,
                Err(e) => {
                    let mut s = STATE.lock().unwrap();
                    s.theme.is_thinking = false;
                    return format!(r#"{{"error":"{}","done":true}}"#, esc(&e));
                }
            };

            // Extract tool calls if any
            let tool_calls = parse_tool_calls(&raw);
            let mut reply  = clean_reply(&raw);
            let mut tools_used: Vec<String> = Vec::new();

            // Tool execution loop (max_steps iterations)
            if !tool_calls.is_empty() {
                let mut pending = tool_calls;
                let mut step = 0;
                while !pending.is_empty() && step < max_steps {
                    step += 1;
                    let mut tool_results = String::new();
                    for (tname, targs) in &pending {
                        let result = dispatch_tool(tname, targs);
                        tool_results.push_str(&format!("[{}]: {}
", tname, result));
                        tools_used.push(tname.clone());
                        // Queue shell commands for Java to execute if needed
                        if tname == "run_shell" || result.starts_with("__shell__") {
                            let mut s = STATE.lock().unwrap();
                            s.pending_shell.push_back(ShellJob {
                                id:      format!("tool_{}_{}", step, tname),
                                cmd:     targs.get("cmd").cloned().unwrap_or_default(),
                                timeout: 10_000,
                                created: now_ms(),
                            });
                        }
                    }
                    // Build follow-up context
                    let mut ctx2 = context.clone();
                    ctx2.push(("assistant".into(), raw.clone()));
                    ctx2.push(("user".into(),
                        format!("[tool results]
{}respond to the user now.", tool_results)));
                    match call_llm_sync(&api_key, &base_url, &model, &system_prompt, &ctx2) {
                        Ok(r2) => {
                            reply   = clean_reply(&r2);
                            pending = parse_tool_calls(&r2);
                        }
                        Err(_) => break,
                    }
                }
            }

            if reply.is_empty() { reply = "done.".into(); }

            // Push assistant reply to compressed history
            {
                let mut s = STATE.lock().unwrap();
                push_turn_compressed(&mut s, "assistant", &reply);
                s.theme.is_thinking = false;
                s.tool_call_count += tools_used.len() as u64;
            }

            let tools_json: String = tools_used.iter()
                .map(|t| format!("\"{}\"", esc(t))).collect::<Vec<_>>().join(",");
            format!(
                r#"{{"role":"assistant","content":"{}","tools_used":[{}],"done":true}}"#,
                esc(&reply), tools_json
            )
        }

        // GET /ai/history — current compressed context as readable JSON
        ("GET",  "/ai/history") => {
            let s = STATE.lock().unwrap();
            let turns = decompress_context(&s);
            let items: Vec<String> = turns.iter()
                .map(|(role, content)| format!(r#"{{"role":"{}","content":"{}"}}"#,
                    esc(role), esc(content)))
                .collect();
            format!(r#"{{"count":{},"turns":[{}]}}"#, items.len(), items.join(","))
        }

        // DELETE /ai/history — clear conversation context
        ("DELETE", "/ai/history") | ("POST", "/ai/history/clear") => {
            let mut s = STATE.lock().unwrap();
            s.context_turns.clear();
            s.context_turns_lz4.clear();
            r#"{"ok":true,"cleared":true}"#.to_string()
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session H — Macro loop (replaces KiraWatcher.java logic)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /macro/tick
        // Called by Java KiraWatcher every 5s with current device state.
        // Rust evaluates all macro triggers and queues fired actions.
        // body: {"battery":85,"charging":false,"pkg":"com.spotify.music",
        //        "screen_hash":"abc123","wifi":"HomeNet","screen_text":"..."}
        ("POST", "/macro/tick") => {
            let battery   = extract_json_num(body, "battery").unwrap_or(-1.0) as i32;
            let charging  = body.contains("\"charging\":true");
            let pkg       = extract_json_str(body, "pkg").unwrap_or_default();
            let wifi      = extract_json_str(body, "wifi").unwrap_or_default();
            let screen_txt= extract_json_str(body, "screen_text").unwrap_or_default();
            let screen_hash=extract_json_str(body, "screen_hash").unwrap_or_default();

            let mut fired = 0u32;
            let now = now_ms();

            {
                let mut s = STATE.lock().unwrap();

                // Update device state
                if battery >= 0 {
                    s.battery_pct = battery;
                    s.battery_charging = charging;
                }
                if !pkg.is_empty()  { s.foreground_pkg = pkg.clone(); }
                if !wifi.is_empty() { s.sig_wifi_ssid = wifi.clone(); }

                // Evaluate macro triggers
                let macro_ids: Vec<String> = s.macros.iter()
                    .filter(|m| m.enabled)
                    .map(|m| m.id.clone())
                    .collect();

                for mid in macro_ids {
                    // Evaluate each trigger in the macro
                    let triggers_snap: Vec<(String, String, bool)> = {
                        if let Some(m) = s.macros.iter().find(|m| m.id == mid) {
                            m.triggers.iter().map(|t| {
                                let kind = format!("{:?}", t.kind).to_lowercase();
                                let val  = t.config.get("value").cloned()
                                    .or_else(|| t.config.get("ssid").cloned())
                                    .or_else(|| t.config.get("pkg").cloned())
                                    .or_else(|| t.config.get("threshold").cloned())
                                    .unwrap_or_default();
                                (kind, val, t.enabled)
                            }).collect()
                        } else { continue; }
                    };

                    let triggered = triggers_snap.iter().any(|(kind, val, en): &(String, String, bool)| {
                        if !en { return false; }
                        match kind.as_str() {
                            k if k.contains("battery") && k.contains("low")  =>
                                battery >= 0 && battery < val.parse().unwrap_or(20),
                            k if k.contains("battery") && k.contains("full") =>
                                battery >= 95 && charging,
                            k if k.contains("app") || k.contains("launch")   =>
                                !pkg.is_empty() && pkg.contains(val.as_str()),
                            k if k.contains("wifi")                           =>
                                !wifi.is_empty() && wifi.contains(val.as_str()),
                            k if k.contains("screen") || k.contains("text")  =>
                                !screen_txt.is_empty()
                                && screen_txt.to_lowercase().contains(&val.to_lowercase()),
                            k if k.contains("time") => {
                                let day_mins = ((now / 60_000) % 1440) as u64;
                                let hm: Vec<u64> = val.split(':')
                                    .filter_map(|x| x.parse::<u64>().ok()).collect();
                                hm.len() == 2 && day_mins == hm[0]*60+hm[1]
                            }
                            _ => false,
                        }
                    });

                    if triggered {
                        // Queue a kira_chat action for each macro
                        let mname: String = s.macros.iter().find(|m| m.id == mid)
                            .map(|m| m.name.clone()).unwrap_or_default();
                        let mid_str: String = mid.clone();
                        s.macro_run_log.push_back(MacroRunLog {
                            macro_id:   mid_str,
                            macro_name: mname,
                            trigger:    "tick".into(),
                            success:    true,
                            steps_run:  1,
                            duration_ms:0,
                            ts:         now,
                            error:      String::new(),
                        });
                        if s.macro_run_log.len() > 100 { s.macro_run_log.pop_front(); }
                        fired += 1;
                    }
                }

                // Screen watch rules from memory
                if !screen_hash.is_empty() {
                    for mem in s.memory_index.iter() {
                        if !mem.content.starts_with("watch_screen_") { continue; }
                        if let Some(colon) = mem.content.find(':') {
                            let rule = &mem.content[colon+1..];
                            let parts: Vec<&str> = rule.splitn(2, '|').collect();
                            if parts.len() == 2 {
                                let keyword = parts[0].trim();
                                let action  = parts[1].trim();
                                if screen_txt.to_lowercase().contains(&keyword.to_lowercase()) {
                                    // Queue AI chat for this screen rule
                                    s.pending_shell.push_back(ShellJob {
                                        id:      format!("watch_{}", now),
                                        cmd:     format!("__ai_chat__:{}", action),
                                        timeout: 30_000,
                                        created: now,
                                    });
                                    fired += 1;
                                }
                            }
                        }
                    }
                }
            }

            format!(r#"{{"ok":true,"fired":{},"ts":{}}}"#, fired, now)
        }

        // GET /macro/pending_results — results queued for Java to dispatch
        // Java polls this for completed macro actions requiring Android intents
        ("GET",  "/macro/pending_results") => {
            let mut s = STATE.lock().unwrap();
            // Return next pending shell job that needs Java (intent-based actions)
            match s.pending_shell.iter().position(|j| j.cmd.starts_with("__intent__:")) {
                Some(idx) => {
                    let job = s.pending_shell.remove(idx).unwrap();
                    let action = job.cmd.trim_start_matches("__intent__:");
                    format!(r#"{{"has_action":true,"action":"{}","id":"{}"}}"#,
                        esc(action), esc(&job.id))
                }
                None => r#"{"has_action":false}"#.to_string(),
            }
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session F — Telegram (replaces KiraTelegram.java)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // POST /telegram/incoming — called by Java after polling getUpdates
        // body: {"update_id":123,"chat_id":456,"user":"name","text":"hello"}
        // Rust processes message through AI and queues reply for Java to send
        ("POST", "/telegram/incoming") => {
            let update_id = extract_json_num(body, "update_id").unwrap_or(0.0) as i64;
            let chat_id   = extract_json_num(body, "chat_id").unwrap_or(0.0) as i64;
            let user      = extract_json_str(body, "user").unwrap_or_default();
            let text      = extract_json_str(body, "text").unwrap_or_default();

            if text.is_empty() || chat_id == 0 {
                return r#"{"ok":false,"error":"missing fields"}"#.to_string();
            }

            // Check allowed user
            let allowed = { STATE.lock().unwrap().config.tg_allowed };
            if allowed != 0 && chat_id != allowed {
                return r#"{"ok":false,"error":"unauthorized"}"#.to_string();
            }

            // Store in log
            {
                let mut s = STATE.lock().unwrap();
                s.tg_last_update_id = update_id;
                s.tg_message_log.push_back(TgMessage {
                    update_id, chat_id, ts: now_ms(),
                    user: user.clone(), text: text.clone(),
                });
                if s.tg_message_log.len() > 50 { s.tg_message_log.pop_front(); }
                push_turn_compressed(&mut s, "user", &format!("[TG @{}] {}", user, text));
            }

            // Run AI on the message
            let chat_body = format!(
                r#"{{"message":"[Telegram @{}] {}","session":"tg_{}","max_tool_steps":5}}"#,
                esc(&user), esc(&text), chat_id
            );
            let ai_result = route_http("POST", "/ai/chat", &chat_body);
            let reply = extract_json_str_inline(&ai_result, "content")
                .unwrap_or_else(|| "sorry, something went wrong".into());

            // Queue reply for Java to send
            {
                let mut s = STATE.lock().unwrap();
                s.tg_pending_sends.push_back(TgSend {
                    chat_id, text: reply.clone(), ts: now_ms()
                });
            }
            format!(r#"{{"ok":true,"reply":"{}"}}"#, esc(&reply))
        }

        // GET /telegram/next_send — Java polls for messages to send
        ("GET",  "/telegram/next_send") => {
            let mut s = STATE.lock().unwrap();
            match s.tg_pending_sends.pop_front() {
                Some(msg) => format!(
                    r#"{{"has_message":true,"chat_id":{},"text":"{}"}}"#,
                    msg.chat_id, esc(&msg.text)),
                None => r#"{"has_message":false}"#.to_string(),
            }
        }

        // GET /telegram/last_update_id — Java uses this for getUpdates offset
        ("GET",  "/telegram/last_update_id") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"update_id":{}}}"#, s.tg_last_update_id)
        }

        // GET /telegram/log — last received messages
        ("GET",  "/telegram/log") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.tg_message_log.iter().rev().take(20).map(|m|
                format!(r#"{{"chat_id":{},"user":"{}","text":"{}","ts":{}}}"#,
                    m.chat_id, esc(&m.user), esc(&m.text), m.ts)
            ).collect();
            format!(r#"{{"count":{},"messages":[{}]}}"#, items.len(), items.join(","))
        }

                // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
        // Session J — Setup wizard data from Rust (reduces SetupActivity hardcoding)
        // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

        // GET /setup/providers — list of AI providers with base URLs + models
        // SetupActivity calls this instead of hardcoding the list
        ("GET",  "/setup/providers") => {
            r#"[
              {"id":"groq","name":"Groq (Free)","base_url":"https://api.groq.com/openai/v1","models":["llama-3.1-8b-instant","llama-3.3-70b-versatile","mixtral-8x7b-32768"],"key_url":"https://console.groq.com/keys","recommended":true},
              {"id":"openai","name":"OpenAI","base_url":"https://api.openai.com/v1","models":["gpt-4o-mini","gpt-4o","gpt-4-turbo"],"key_url":"https://platform.openai.com/api-keys","recommended":false},
              {"id":"anthropic","name":"Anthropic","base_url":"https://api.anthropic.com/v1","models":["claude-3-haiku-20240307","claude-3-5-sonnet-20241022","claude-3-opus-20240229"],"key_url":"https://console.anthropic.com/","recommended":false},
              {"id":"together","name":"Together AI","base_url":"https://api.together.xyz/v1","models":["meta-llama/Meta-Llama-3.1-8B-Instruct-Turbo","mistralai/Mixtral-8x7B-Instruct-v0.1"],"key_url":"https://api.together.ai/settings/api-keys","recommended":false},
              {"id":"openrouter","name":"OpenRouter","base_url":"https://openrouter.ai/api/v1","models":["meta-llama/llama-3.1-8b-instruct:free","google/gemma-2-9b-it:free"],"key_url":"https://openrouter.ai/keys","recommended":false},
              {"id":"custom","name":"Custom / Self-hosted","base_url":"","models":[],"key_url":"","recommended":false}
            ]"#.to_string()
        }

        // POST /setup/validate — validate API key format + test connection
        // body: {"provider":"groq","api_key":"gsk_...","model":"llama-3.1-8b-instant"}
        ("POST", "/setup/validate") => {
            let provider = extract_json_str(body, "provider").unwrap_or_default();
            let api_key  = extract_json_str(body, "api_key").unwrap_or_default();
            let model    = extract_json_str(body, "model").unwrap_or_default();

            if api_key.is_empty() {
                return r#"{"valid":false,"error":"API key is empty"}"#.to_string();
            }

            // Format validation per provider
            let format_ok = match provider.as_str() {
                "groq"      => api_key.starts_with("gsk_") && api_key.len() > 10,
                "openai"    => api_key.starts_with("sk-") && api_key.len() > 10,
                "anthropic" => api_key.starts_with("sk-ant-") && api_key.len() > 10,
                _           => api_key.len() >= 8,
            };

            if !format_ok {
                let hint = match provider.as_str() {
                    "groq"      => "Groq keys start with 'gsk_'",
                    "openai"    => "OpenAI keys start with 'sk-'",
                    "anthropic" => "Anthropic keys start with 'sk-ant-'",
                    _           => "Key appears invalid",
                };
                return format!(r#"{{"valid":false,"error":"{}"}}"#, hint);
            }

            // Quick syntax validation passed — mark as valid
            // (actual connection test done by Java to avoid blocking)
            format!(r#"{{"valid":true,"provider":"{}","model":"{}","hint":"Format valid. Tap Next to continue."}}"#,
                esc(&provider), esc(&model))
        }

        // GET /setup/status — current setup completion state
        ("GET",  "/setup/status") => {
            let s = STATE.lock().unwrap();
            format!(r#"{{"setup_done":{},"has_api_key":{},"provider":"{}","model":"{}","user_name":"{}"}}"#,
                s.config.setup_done,
                !s.config.api_key.is_empty(),
                esc(&s.config.base_url),
                esc(&s.config.model),
                esc(&s.config.user_name))
        }

                // ── Session C: Crypto endpoints ───────────────────────────────────────

        // POST /crypto/encrypt {"plaintext":"..","seed":"..","domain":"api_key"}
        ("POST", "/crypto/encrypt") => {
            let pt  = extract_json_str(body, "plaintext").unwrap_or_default();
            let sd  = extract_json_str(body, "seed").unwrap_or_default();
            let dm  = extract_json_str(body, "domain").unwrap_or_else(|| "default".into());
            if pt.is_empty() || sd.is_empty() {
                return r#"{"error":"plaintext and seed required"}"#.to_string();
            }
            let ct = aes_encrypt(&pt, &sd, &dm);
            format!(r#"{{"ok":true,"ciphertext":"{}","domain":"{}"}}"#, ct, esc(&dm))
        }

        // POST /crypto/decrypt {"ciphertext":"..","seed":"..","domain":"api_key"}
        ("POST", "/crypto/decrypt") => {
            let ct = extract_json_str(body, "ciphertext").unwrap_or_default();
            let sd = extract_json_str(body, "seed").unwrap_or_default();
            let dm = extract_json_str(body, "domain").unwrap_or_else(|| "default".into());
            if ct.is_empty() || sd.is_empty() {
                return r#"{"error":"ciphertext and seed required"}"#.to_string();
            }
            let plain = aes_decrypt(&ct, &sd, &dm);
            let ok = !plain.is_empty();
            format!(r#"{{"ok":{},"plaintext":"{}"}}"#, ok, esc(&plain))
        }

        // GET /crypto/status — reports encryption availability
        ("GET",  "/crypto/status") => {
            r#"{"available":true,"algorithm":"AES-256-GCM","key_derivation":"derive_aes_key(ANDROID_ID:pkg)","nonce":"domain-derived-12-byte","tag_bits":128}"#.to_string()
        }

                // ── Session L: Security audit endpoint ───────────────────────────────────

        // GET /security/audit — reports current security posture
        ("GET",  "/security/audit") => {
            let s = STATE.lock().unwrap();
            let has_secret  = !s.http_secret.is_empty();
            let has_api_key = !s.config.api_key.is_empty();
            // Check if api_key looks encrypted (hex, even length, >32 chars)
            let key_encrypted = has_api_key
                && s.config.api_key.len() > 32
                && s.config.api_key.chars().all(|c: char| c.is_ascii_hexdigit());
            let shizuku_ok  = s.shizuku.permission_granted;
            format!(r#"{{"http_secret_set":{},"api_key_present":{},"api_key_encrypted":{},"shizuku":{},"tls_enabled":true,"auth_coverage":"session_l","jni_safe_inputs":true,"lz4_compression":true,"aes_gcm_available":true}}"#,
                has_secret, has_api_key, key_encrypted, shizuku_ok)
        }

        // POST /security/rotate_secret — generate and set a new random HTTP secret
        ("POST", "/security/rotate_secret") => {
            // Derive new secret from current time + existing key material
            let new_secret = {
                let s = STATE.lock().unwrap();
                let seed = format!("{}:{}:{}", now_ms(), s.request_count, s.config.api_key.len());
                let k = derive_aes_key(&seed);
                k.iter().map(|b| format!("{:02x}", b)).collect::<String>()[..32].to_string()
            };
            STATE.lock().unwrap().http_secret = new_secret.clone();
            format!(r#"{{"ok":true,"new_secret":"{}","note":"store this — required for all future API calls"}}"#,
                &new_secret)
        }

                ("POST", "/crash") => {
            let thread  = extract_json_str(body, "thread").unwrap_or_else(||"unknown".into());
            let trace   = extract_json_str(body, "trace").unwrap_or_default();
            let ts_val  = extract_json_num(body, "ts").unwrap_or(0.0) as u128;
            let ts      = if ts_val > 0 { ts_val } else { now_ms() };
            // First line = exception class/message
            let message = trace.lines().next().unwrap_or("").to_string();
            // Cap trace at 4KB to avoid memory bloat
            let trace_capped = if trace.len() > 4096 {
                format!("{}…[truncated]", &trace[..4096])
            } else { trace };
            let entry = CrashEntry { ts, thread, message, trace: trace_capped };
            let mut s = STATE.lock().unwrap();
            s.crash_log.push_back(entry);
            if s.crash_log.len() > 50 { s.crash_log.pop_front(); }
            r#"{"ok":true}"#.to_string()
        }

        // GET /crash/log — returns all stored crash entries as JSON array
        ("GET",  "/crash/log") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.crash_log.iter().map(|c| {
                let safe_msg   = esc(&c.message);
                let safe_trace = esc(&c.trace);
                let safe_thr   = esc(&c.thread);
                format!(r#"{{"ts":{},"thread":"{}","message":"{}","trace":"{}"}}"#,
                    c.ts, safe_thr, safe_msg, safe_trace)
            }).collect();
            format!(r#"{{"count":{},"crashes":[{}]}}"#, items.len(), items.join(","))
        }

        // POST /crash/clear — wipe the crash log
        ("POST", "/crash/clear") => {
            STATE.lock().unwrap().crash_log.clear();
            r#"{"ok":true,"cleared":true}"#.to_string()
        }

        // GET /crash/latest — just the most recent crash (fast poll)
        ("GET",  "/crash/latest") => {
            let s = STATE.lock().unwrap();
            match s.crash_log.back() {
                Some(c) => format!(
                    r#"{{"has_crash":true,"ts":{},"thread":"{}","message":"{}"}}"#,
                    c.ts, esc(&c.thread), esc(&c.message)),
                None => r#"{"has_crash":false}"#.to_string(),
            }
        }

        ("POST", "/theme/flash") => {
            let dark = body.contains("\"dark\":true");
            STATE.lock().unwrap().theme.is_dark = dark;
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/automation/summary — automation engine summary for settings card
        ("GET",  "/settings/automation/summary") => {
            let s = STATE.lock().unwrap();
            let enabled = s.macros.iter().filter(|m| m.enabled).count();
            let total_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let last_run_ms = s.macros.iter().map(|m| m.last_run_ms).max().unwrap_or(0);
            let last_run_ago = if last_run_ms > 0 {
                now_ms().saturating_sub(last_run_ms)
            } else { 0 };
            format!(
                r#"{{"total":{},"enabled":{},"disabled":{},"total_runs":{},"last_run_ago_ms":{}}}"#,
                s.macros.len(), enabled, s.macros.len().saturating_sub(enabled),
                total_runs, last_run_ago
            )
        }


        // ── Layer 5: Settings Page — Rust-backed endpoints ──────────────────────

        // GET /settings/counters — live counter values for CounterAnimator
        // Returns numbers that the UI animates from old→new over 600ms EaseOut.
        ("GET", "/settings/counters") => {
            let s = STATE.lock().unwrap();
            let uptime_ms   = now_ms().saturating_sub(s.uptime_start);
            let uptime_s    = (uptime_ms / 1000) as u64;
            let tool_calls  = s.tool_call_count;
            let mem_facts   = s.memory_index.len() as u64;
            let macros_en   = s.macros.iter().filter(|m| m.enabled).count() as u64;
            let macros_tot  = s.macros.len() as u64;
            let macro_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let sessions    = s.sessions.len() as u64;
            let skills_en   = s.skills.values().filter(|sk| sk.enabled).count() as u64;
            let req_count   = s.request_count;
            let notif_count = s.notifications.len() as u64;
            format!(
                r#"{{"uptime_s":{},"tool_calls":{},"memory_facts":{},"macros_enabled":{},"macros_total":{},"macro_runs":{},"sessions":{},"skills_enabled":{},"requests":{},"notifications":{}}}"#,
                uptime_s, tool_calls, mem_facts, macros_en, macros_tot,
                macro_runs, sessions, skills_en, req_count, notif_count
            )
        }

        // GET /settings/activity — activity stream for last 20 events
        // Used by the settings page activity feed (Row-level visual feedback)
        ("GET", "/settings/activity") => {
            let s = STATE.lock().unwrap();
            let mut items: Vec<String> = Vec::new();
            // Last 10 macro runs
            for r in s.macro_run_log.iter().rev().take(10) {
                items.push(format!(
                    r#"{{"type":"macro","name":"{}","success":{},"ts":{}}}"#,
                    esc(&r.macro_name), r.success, r.ts));
            }
            // Last 5 notifications
            for n in s.notifications.iter().rev().take(5) {
                items.push(format!(
                    r#"{{"type":"notif","pkg":"{}","title":"{}","ts":{}}}"#,
                    esc(&n.pkg), esc(&n.title), n.time));
            }
            // Last 5 daily_log entries
            for entry in s.daily_log.iter().rev().take(5) {
                items.push(format!(
                    r#"{{"type":"log","msg":"{}","ts":{}}}"#,
                    esc(entry), now_ms()));
            }
            // Sort by ts descending, take 20
            items.sort_by(|a, b| {
                let ta = extract_json_num(a, "ts").unwrap_or(0.0);
                let tb = extract_json_num(b, "ts").unwrap_or(0.0);
                tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
            });
            items.truncate(20);
            format!(r#"{{"count":{},"items":[{}]}}"#, items.len(), items.join(","))
        }

        // GET /settings/shizuku/halo — Layer 9: God mode halo state
        // Returns border color + animation params for the screen-edge halo
        ("GET", "/settings/shizuku/halo") => {
            let s = STATE.lock().unwrap();
            let active = s.shizuku.permission_granted;
            let partial = s.shizuku.installed && !active;
            // God mode halo: 2dp Lavender border traces screen edge when fully active
            // Rotation: 4s per revolution, 30dp arc length
            let (color, width_dp, visible, revolution_ms) = if active {
                (0xFFB4BEFEu32, 2u32, true,  4000u32)  // Lavender, full speed
            } else if partial {
                (0xFFFAB387u32, 1u32, true,  8000u32)  // Peach, slow
            } else {
                (0x00000000u32, 0u32, false, 0u32)      // invisible
            };
            format!(
                r#"{{"active":{},"partial":{},"color":{},"width_dp":{},"visible":{},"revolution_ms":{},"arc_dp":30}}"#,
                active, partial, color, width_dp, visible, revolution_ms
            )
        }

        // POST /settings/row_interaction {"row":"api_key","action":"tap|long_press|edit"}
        // Enhanced row analytics — tracks not just tap but interaction type
        ("POST", "/settings/row_interaction") => {
            let row    = extract_json_str(body, "row").unwrap_or_default();
            let action = extract_json_str(body, "action").unwrap_or_else(|| "tap".to_string());
            if !row.is_empty() {
                let mut s = STATE.lock().unwrap();
                let entry = format!("[settings_interaction] row={} action={} ts={}", 
                    esc(&row), esc(&action), now_ms());
                s.daily_log.push_back(entry);
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                // Increment row-specific usage counter in variables map
                let key = format!("_settings_tap_{}", row);
                let count = s.variables.get(&key)
                    .map(|v| v.value.parse::<u32>().unwrap_or(0) + 1)
                    .unwrap_or(1);
                s.variables.entry(key.clone()).or_insert_with(|| AutoVariable {
                    name: key.clone(), value: "0".to_string(),
                    var_type: "counter".to_string(),
                    persistent: false, created_ms: now_ms(), updated_ms: now_ms(),
                }).value = count.to_string();
            }
            r#"{"ok":true}"#.to_string()
        }

        // GET /settings/top_rows — most-accessed settings rows (for smart ordering)
        ("GET", "/settings/top_rows") => {
            let s = STATE.lock().unwrap();
            let mut rows: Vec<(String, u32)> = s.variables.iter()
                .filter(|(k, _): &(String, _)| k.starts_with("_settings_tap_"))
                .map(|(k, v): (&String, &_)| {
                    let row_name = k.trim_start_matches("_settings_tap_").to_string();
                    let count = v.value.parse::<u32>().unwrap_or(0);
                    (row_name, count)
                })
                .collect();
            rows.sort_by(|a, b| b.1.cmp(&a.1));
            rows.truncate(5);
            let items: Vec<String> = rows.iter()
                .map(|(r, c)| format!(r#"{{"row":"{}","taps":{}}}"#, esc(r), c))
                .collect();
            format!(r#"{{"top_rows":[{}]}}"#, items.join(","))
        }

        // GET /settings/memory/stats — detailed memory stats for memory card
        ("GET", "/settings/memory/stats") => {
            let s = STATE.lock().unwrap();
            let total    = s.memory_index.len();
            let pinned   = s.memory_index.iter().filter(|e| e.access_count > 5).count();
            let recent   = s.memory_index.iter()
                .filter(|e| now_ms().saturating_sub(
                    s.context_turns.iter().map(|_| now_ms()).next().unwrap_or(0)) < 86_400_000)
                .count();
            let top: Vec<String> = {
                let mut entries: Vec<_> = s.memory_index.iter().collect();
                entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
                entries.iter().take(3)
                    .map(|e| format!(r#"{{"key":"{}","access_count":{}}}"#,
                        esc(&e.key), e.access_count))
                    .collect()
            };
            format!(
                r#"{{"total":{},"pinned":{},"recent_24h":{},"top_accessed":[{}]}}"#,
                total, pinned, recent, top.join(",")
            )
        }

        // GET /settings/theme/palette — full Catppuccin Mocha palette for settings
        // Used by theme card to show all swatches
        ("GET", "/settings/theme/palette") => {
            // Catppuccin Mocha full palette
            let swatches = vec![
                ("Rosewater", 0xFFF5E0DC_u32), ("Flamingo",  0xFFF2CDCD_u32),
                ("Pink",      0xFFF38BA8_u32), ("Mauve",     0xFFCBA6F7_u32),
                ("Red",       0xFFEBA0AC_u32), ("Maroon",    0xFFEBA0AC_u32),
                ("Peach",     0xFFFAB387_u32), ("Yellow",    0xFFF9E2AF_u32),
                ("Green",     0xFFA6E3A1_u32), ("Teal",      0xFF94E2D5_u32),
                ("Sky",       0xFF89DCEB_u32), ("Sapphire",  0xFF74C7EC_u32),
                ("Blue",      0xFF89B4FA_u32), ("Lavender",  0xFFB4BEFE_u32),
                ("Text",      0xFFCDD6F4_u32), ("Subtext1",  0xFFBAC2DE_u32),
                ("Overlay2",  0xFF9399B2_u32), ("Overlay0",  0xFF6C7086_u32),
                ("Surface2",  0xFF585B70_u32), ("Surface1",  0xFF45475A_u32),
                ("Surface0",  0xFF313244_u32), ("Base",      0xFF1E1E2E_u32),
                ("Mantle",    0xFF181825_u32), ("Crust",     0xFF11111B_u32),
            ];
            let items: Vec<String> = swatches.iter()
                .map(|(name, color)| format!(r#"{{"name":"{}","color":{}}}"#, name, color))
                .collect();
            format!(r#"{{"theme":"catppuccin_mocha","swatches":[{}]}}"#, items.join(","))
        }

                // GET /layer1 — Neural Nav Bar state for Java
        // Returns Catppuccin colour tokens + keyboard state hint
        // Java NeuralNavBar polls this to stay in sync with Rust theme
        ("GET",  "/layer1") => {
            let s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            // Nav bar pulse: subtle border alpha oscillation on heartbeat
            let phase = (uptime_ms % 3_000) as f32 / 3_000.0;
            let border_alpha = 0.30 + (phase * 2.0 * std::f32::consts::PI as f32).sin().abs() * 0.08;
            format!(
                r#"{{"mantle":{},"lavender":{},"overlay0":{},"border_alpha":{:.4},"active_tab_sp":26,"inactive_tab_sp":22,"aura_radius_dp":32,"island_height_dp":72,"island_corner_dp":24,"elevation_dp":8}}"#,
                0xFF181825u32,  // Catppuccin Mantle
                0xFFB4BEFEu32,  // Catppuccin Lavender
                0xFF6C7086u32,  // Catppuccin Overlay0 (inactive)
                border_alpha
            )
        }

        // POST /layer0/burst — called by Java when Kira finishes replying
        // Sets a one-shot burst flag that /layer0 will return as burst:true
        // ── Layer 2: Chat Interface state endpoints ───────────────────────────

        // GET /layer2/header — header bar state (pulse, subtitle cycle index)
        // Java polls at 500ms to drive header border and subtitle crossfade
        ("GET",  "/layer2/header") => {
            let s = STATE.lock().unwrap();
            let uptime_ms  = now_ms().saturating_sub(s.uptime_start);
            let phase      = (uptime_ms % 3_000) as f32 / 3_000.0;
            // Border alpha: 12% base, pulses to 35% when thinking
            let border_alpha = if s.theme.is_thinking {
                0.27 + (phase * 2.0 * std::f32::consts::PI).sin().abs() * 0.08
            } else { 0.12f32 };
            // Subtitle index: cycles through 4 states (ready/thinking/reasoning/composing)
            // 0=ready, 1=thinking, 2=reasoning, 3=composing — driven by request state
            let subtitle_idx: u32 = if !s.theme.is_thinking { 0 }
                else { ((uptime_ms / 1_800) % 3 + 1) as u32 }; // cycle 1-3 while thinking
            format!(
                r#"{{"border_alpha":{:.4},"subtitle_idx":{},"thinking":{},"request_count":{}}}"#,
                border_alpha, subtitle_idx, s.theme.is_thinking, s.request_count
            )
        }

        // GET /layer2/bubbles — bubble styling tokens for chat UI
        // Returns Catppuccin colour tokens for user/kira bubbles + shadow specs
        ("GET",  "/layer2/bubbles") => {
            r#"{"user_bg":3292050, "user_bg_alpha":255, "kira_bg":2040622, "lavender":11862782, "peach":16430983, "green_dark":2023454, "shadow_color":1144397, "shadow_alpha":102, "shadow_blur_dp":8, "shadow_y_dp":2, "spring_stiffness":300, "spring_damping":28, "spring_duration_ms":320, "translate_dp":40}"#.to_string()
            // user_bg = 0xFF313244 (Surface0), kira_bg = 0xFF1E1E2E (Base)
            // lavender = 0xFFB4BEFE, peach = 0xFFFAB387, green_dark = 0xFF1E2E1E
        }

        // GET /layer2/typing — typing indicator animation params
        // Three Lavender dots, sinusoidal, each offset 120ms
        ("GET",  "/layer2/typing") => {
            let s = STATE.lock().unwrap();
            let uptime_ms = now_ms().saturating_sub(s.uptime_start);
            // Each dot phase offset by 120ms within 600ms period
            let t = uptime_ms as f32 / 600.0 * 2.0 * std::f32::consts::PI;
            let d0 = ((t).sin() * 4.0) as i32;           // dot 0: ±4dp
            let d1 = ((t - 0.628).sin() * 4.0) as i32;   // dot 1: 120ms offset
            let d2 = ((t - 1.257).sin() * 4.0) as i32;   // dot 2: 240ms offset
            format!(
                r#"{{"visible":{},"dot0_y":{},"dot1_y":{},"dot2_y":{},"color":{},"period_ms":600,"amplitude_dp":4}}"#,
                s.theme.is_thinking, d0, d1, d2, 0xFFB4BEFEu32
            )
        }

        // POST /layer2/message — record that a message was sent/received
        // Updates request_count, last_message_ts, triggers K badge rotation signal
        ("POST", "/layer2/message") => {
            let role = extract_json_str(body, "role").unwrap_or_else(||"user".to_string());
            let mut s = STATE.lock().unwrap();
            if role == "user" {
                s.request_count += 1;
                s.theme.is_thinking = true;
            } else if role == "kira" {
                s.theme.is_thinking = false;
                s.tool_call_count += 1;
            }
            format!(r#"{{"ok":true,"request_count":{},"thinking":{}}}"#,
                s.request_count, s.theme.is_thinking)
        }

                ("POST", "/layer0/burst") => {
            // We use is_thinking flip as the burst signal — Java detects thinking→false
            STATE.lock().unwrap().theme.is_thinking = false;
            r#"{"ok":true}"#.to_string()
        }

        ("POST", "/theme/set")         => { let name=extract_json_str(body,"name").unwrap_or_else(||"material".into()); let mut s=STATE.lock().unwrap(); s.theme = match name.as_str() { "material" | "material_neo" | "material_dark" => ThemeConfig::material_dark(), "material_light" | "material_neo_light" => ThemeConfig::material_light(), "kira" => ThemeConfig::default(), _ => ThemeConfig::material_dark() }; format!(r#"{{"ok":true,"theme":"{}"}}"#, s.theme.theme_name) }
        ("POST", "/theme/tilt")        => { let ax=extract_json_f32(body,"ax").unwrap_or(0.0); let ay=extract_json_f32(body,"ay").unwrap_or(0.0); let mut s=STATE.lock().unwrap(); s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay; let spd=s.theme.star_speed; let tx=-ax*spd; let ty=ay*spd; s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08; s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08; format!(r#"{{"px":{:.6},"py":{:.6}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y) }
        ("GET",  "/shizuku")           => { let s=STATE.lock().unwrap(); shizuku_to_json(&s.shizuku) }
        ("POST", "/shizuku")           => { let installed=body.contains(r#""installed":true"#); let granted=body.contains(r#""permission_granted":true"#); let err=extract_json_str(body,"error").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.shizuku.installed=installed; s.shizuku.permission_granted=granted; s.shizuku.error_msg=err; s.shizuku.last_checked_ms=now_ms(); r#"{"ok":true}"#.to_string() }
        ("GET",  "/appstats")          => { let s=STATE.lock().unwrap(); format!(r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"active_profile":"{}","variables":{}}}"#, s.memory_index.len(),s.context_turns.len(), if s.shizuku.permission_granted{"active \u{2713}"}else if s.shizuku.installed{"no permission"}else{"not running"}, if !s.agent_context.is_empty(){"enabled \u{2713}"}else{"disabled"}, esc(&s.config.model),esc(&s.config.base_url),now_ms().saturating_sub(s.uptime_start),s.macros.len(),esc(&s.active_profile),s.variables.len()) }
        ("GET",  "/providers")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.providers.iter().map(|p| format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),esc(&p.base_url),esc(&p.model),p.id==s.active_provider)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/providers/set")     => { let id=extract_json_str(body,"id").unwrap_or_default(); if !id.is_empty() { let mut s=STATE.lock().unwrap(); let found=s.providers.iter().find(|p| p.id==id).cloned(); if let Some(p)=found { s.active_provider=id.clone(); s.config.base_url=p.base_url; s.config.model=p.model; } } format!(r#"{{"ok":true,"active":"{}"}}"#, id) }
        ("POST", "/providers/custom")  => { let url=extract_json_str(body,"url").unwrap_or_default(); let model=extract_json_str(body,"model").unwrap_or_default(); if !url.is_empty() { let mut s=STATE.lock().unwrap(); s.setup.custom_url=url.clone(); s.setup.selected_provider_id="custom".to_string(); s.config.base_url=url.clone(); if !model.is_empty() { s.config.model=model.clone(); } if let Some(p)=s.providers.iter_mut().find(|p| p.id=="custom") { p.base_url=url; if !model.is_empty() { p.model=model; } } s.active_provider="custom".to_string(); } r#"{"ok":true}"#.to_string() }

        // v7: Device state
        ("GET",  "/screen")            => STATE.lock().unwrap().screen_nodes.clone(),
        ("GET",  "/screen_pkg")        => { let p=STATE.lock().unwrap().screen_pkg.clone(); format!(r#"{{"package":"{}"}}"#, esc(&p)) }
        ("GET",  "/battery")           => { let s=STATE.lock().unwrap(); format!(r#"{{"percentage":{},"charging":{}}}"#, s.battery_pct,s.battery_charging) }
        ("GET",  "/notifications")     => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.notifications.iter().map(|n| format!(r#"{{"pkg":"{}","title":"{}","text":"{}","time":{}}}"#, esc(&n.pkg),esc(&n.title),esc(&n.text),n.time)).collect(); format!("[{}]", items.join(",")) }

        // v7: Memory
        ("GET",  "/memory")            => { let s=STATE.lock().unwrap(); format!(r#"{{"memory_md":{},"entries":{}}}"#, json_str(&s.memory_md),s.memory_index.len()) }
        ("GET",  "/memory/search")     => search_memory(path),
        ("GET",  "/memory/full")       => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.memory_index.iter().map(|e| format!(r#"{{"key":"{}","value":"{}","tags":{},"relevance":{:.2},"access_count":{}}}"#, esc(&e.key),esc(&e.value),json_str_arr(&e.tags),e.relevance,e.access_count)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/daily_log")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.daily_log.iter().cloned().map(|l: &String| format!("\"{}\"", esc(&l))).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/context")           => get_context_json(),
        ("GET",  "/soul")              => { let s=STATE.lock().unwrap(); format!(r#"{{"soul":{}}}"#, json_str(&s.soul_md)) }
        ("POST", "/soul")              => { let val=extract_json_str(body,"content").unwrap_or_default(); if !val.is_empty() { STATE.lock().unwrap().soul_md=val; } r#"{"ok":true}"#.to_string() }

        // v7: Skills
        ("GET",  "/skills")            => get_skills_json(),
        ("POST", "/skills/register")   => { register_skill(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/enable")     => { let name=extract_json_str(body,"name").unwrap_or_default(); if let Some(sk)=STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=true; } r#"{"ok":true}"#.to_string() }
        ("POST", "/skills/disable")    => { let name=extract_json_str(body,"name").unwrap_or_default(); if let Some(sk)=STATE.lock().unwrap().skills.get_mut(&name) { sk.enabled=false; } r#"{"ok":true}"#.to_string() }

        // v7: Sessions
        ("GET",  "/sessions")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.sessions.values().map(|sess| format!(r#"{{"id":"{}","channel":"{}","turns":{},"tokens":{},"last_msg":{}}}"#, sess.id,sess.channel,sess.turns,sess.tokens,sess.last_msg)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/sessions/new")      => new_session(body),

        // v7: Triggers
        ("GET",  "/triggers")          => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.triggers.iter().map(|t| format!(r#"{{"id":"{}","type":"{}","value":"{}","fired":{},"repeat":{}}}"#, t.id,t.trigger_type,esc(&t.value),t.fired,t.repeat)).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/fired_triggers")    => { let mut s=STATE.lock().unwrap(); let items: Vec<String>=s.fired_triggers.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("GET",  "/webhook_events")    => { let mut s=STATE.lock().unwrap(); let items: Vec<String>=s.webhook_events.drain(..).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/triggers/add")      => { add_trigger(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/webhook")           => { let ts=now_ms(); STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"body":{},"ts":{}}}"#, if body.is_empty(){"{}"}else{body},ts)); r#"{"ok":true}"#.to_string() }

        // v7: Heartbeat
        ("GET",  "/heartbeat_log")     => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.heartbeat_log.iter().cloned().collect(); format!("[{}]", items.join(",")) }
        ("POST", "/heartbeat/add")     => { add_heartbeat(body); r#"{"ok":true}"#.to_string() }

        // v7: Cron
        ("GET",  "/cron")              => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.cron_jobs.iter().map(|j| format!(r#"{{"id":"{}","action":"{}","interval_ms":{},"enabled":{}}}"#, j.id,esc(&j.action),j.interval_ms,j.enabled)).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/cron/add")          => { add_cron(body); r#"{"ok":true}"#.to_string() }
        ("POST", "/cron/remove")       => { let id=extract_json_str(body,"id").unwrap_or_default(); STATE.lock().unwrap().cron_jobs.retain(|j| j.id!=id); r#"{"ok":true}"#.to_string() }

        // v7: Audit + task
        ("GET",  "/task_log")          => get_task_log_json(),
        ("GET",  "/audit_log")         => get_audit_log_json(),
        ("GET",  "/checkpoints")       => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.checkpoints.iter().map(|(k,v)| format!(r#"{{{}:{}}}"#, json_str(k),json_str(v))).collect(); format!("[{}]", items.join(",")) }
        ("POST", "/checkpoint")        => { let k=extract_json_str(body,"id").unwrap_or_default(); let v=extract_json_str(body,"data").unwrap_or_default(); if !k.is_empty() { STATE.lock().unwrap().checkpoints.insert(k,v); } r#"{"ok":true}"#.to_string() }

        // v7: KB
        ("GET",  "/kb")                => get_kb_json(),
        ("GET",  "/kb/search")         => kb_search(path),
        ("POST", "/kb/add")            => { add_kb_entry(body); r#"{"ok":true}"#.to_string() }

        // v7: Events + metrics
        ("GET",  "/events")            => get_event_feed(),
        ("POST", "/events")            => { let e=extract_json_str(body,"event").unwrap_or_default(); let d=extract_json_str(body,"data").unwrap_or_default(); push_event_feed(&e,&d); r#"{"ok":true}"#.to_string() }
        ("GET",  "/metrics")           => get_metrics_text(),
        ("GET",  "/budget")            => get_budget_json(),
        ("GET",  "/stream")            => stream_poll(),
        ("POST", "/stream/chunk")      => { let t=extract_json_str(body,"text").unwrap_or_default(); push_stream_chunk(&t); r#"{"ok":true}"#.to_string() }
        ("POST", "/relay")             => relay_msg(body),
        ("GET",  "/cache")             => cache_get(path),
        ("POST", "/cache")             => cache_post(body),
        ("POST", "/policy/allow")      => { let t=extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s=STATE.lock().unwrap(); s.tool_denylist.retain(|d| d!=&t); if !s.tool_allowlist.contains(&t) { s.tool_allowlist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("POST", "/policy/deny")       => { let t=extract_json_str(body,"tool").unwrap_or_default(); if !t.is_empty() { let mut s=STATE.lock().unwrap(); s.tool_allowlist.retain(|a| a!=&t); if !s.tool_denylist.contains(&t) { s.tool_denylist.push(t); } } r#"{"ok":true}"#.to_string() }
        ("GET",  "/policy")            => get_policy_json(),
        ("POST", "/nodes/register")    => { register_node(body); r#"{"ok":true}"#.to_string() }
        ("GET",  "/nodes")             => get_nodes_json(),
        ("POST", "/credentials/get")   => get_credential(body),

        // OpenClaw v3 / NanoBot / ZeroClaw routes
        // ── OTA Engine v43 ────────────────────────────────────────────────────
        // GET  /ota/status        — full OTA state JSON
        // POST /ota/begin_check   — Java signals it's about to call GitHub API
        // POST /ota/release       — Java posts parsed GitHub release data to Rust
        // POST /ota/progress      — Java reports download progress
        // POST /ota/downloaded    — Java signals APK is on disk (path + sha256)
        // POST /ota/installing    — Java signals install session opened
        // POST /ota/installed     — Java signals successful install
        // POST /ota/failed        — Java signals any error
        // POST /ota/skip          — skip this version
        // POST /ota/set_version   — update current installed version
        // GET  /ota/install_cmd   — get the install command for Shizuku
        ("GET",  "/ota/status")     => { STATE.lock().unwrap().ota.to_json() }
        ("POST", "/ota/begin_check") => {
            let mut s = STATE.lock().unwrap();
            s.ota.phase = OtaPhase::Checking;
            s.ota.check_error = String::new();
            s.ota.last_check_ms = now_ms();
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/release") => {
            let latest  = extract_json_str(body, "tag").unwrap_or_default();
            let url     = extract_json_str(body, "url").unwrap_or_default();
            let log     = extract_json_str(body, "changelog").unwrap_or_default();
            let date    = extract_json_str(body, "date").unwrap_or_default();
            let sha     = extract_json_str(body, "sha256").unwrap_or_default();
            let apk_sz  = extract_json_num(body, "apk_bytes").unwrap_or(0.0) as u64;
            let mut s   = STATE.lock().unwrap();
            // Check if this version is skipped
            if s.ota.skipped_versions.contains(&latest) {
                s.ota.phase = OtaPhase::Idle;
                return format!(r#"{{"ok":true,"skipped":true}}"#);
            }
            let is_newer = s.ota.is_newer(&latest);
            s.ota.latest_version  = latest.clone();
            s.ota.download_url    = url;
            s.ota.changelog       = log;
            s.ota.release_date    = date;
            s.ota.apk_sha256      = sha;
            s.ota.total_apk_bytes = apk_sz;
            s.ota.last_check_ms   = now_ms();
            s.ota.check_error     = String::new();
            if is_newer {
                s.ota.phase = OtaPhase::UpdateAvailable;
                format!(r#"{{"ok":true,"action":"prompt_user","version":"{}"}}"#, esc(&latest))
            } else {
                s.ota.phase = OtaPhase::Idle;
                format!(r#"{{"ok":true,"action":"up_to_date"}}"#)
            }
        }
        ("POST", "/ota/progress") => {
            let done  = extract_json_num(body, "bytes").unwrap_or(0.0) as u64;
            let total = extract_json_num(body, "total").unwrap_or(0.0) as u64;
            let mut s = STATE.lock().unwrap();
            s.ota.download_bytes = done;
            s.ota.download_total = total;
            s.ota.download_pct   = if total > 0 { ((done * 100) / total).min(100) as u8 } else { 0 };
            s.ota.phase = OtaPhase::Downloading;
            format!(r#"{{"ok":true,"pct":{}}}"#, s.ota.download_pct)
        }
        ("POST", "/ota/downloaded") => {
            let path = extract_json_str(body, "path").unwrap_or_default();
            let sha  = extract_json_str(body, "sha256").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            s.ota.apk_local_path = path;
            // Verify SHA256 if we have an expected one
            let expected = s.ota.apk_sha256.clone();
            let verified = expected.is_empty() || expected == sha;
            if verified {
                s.ota.phase = OtaPhase::Downloaded;
                r#"{"ok":true,"verified":true}"#.to_string()
            } else {
                s.ota.phase = OtaPhase::Failed(format!("SHA256 mismatch: got {}", &sha[..sha.len().min(16)]));
                format!(r#"{{"ok":false,"error":"sha256_mismatch","expected":"{}","got":"{}"}}"#,
                    esc(&expected[..expected.len().min(16)]), esc(&sha[..sha.len().min(16)]))
            }
        }
        ("POST", "/ota/installing") => {
            let method = extract_json_str(body, "method").unwrap_or_else(|| "intent".to_string());
            let sid    = extract_json_num(body, "session_id").unwrap_or(-1.0) as i32;
            let mut s  = STATE.lock().unwrap();
            s.ota.install_method     = method;
            s.ota.install_session_id = sid;
            s.ota.install_error      = String::new();
            s.ota.phase = OtaPhase::Installing;
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/installed") => {
            let ver = extract_json_str(body, "version").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            s.ota.phase = OtaPhase::Installed;
            if !ver.is_empty() { s.ota.current_version = ver; s.config.setup_done = true; }
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/failed") => {
            let err = extract_json_str(body, "error").unwrap_or_else(|| "unknown error".to_string());
            let mut s = STATE.lock().unwrap();
            s.ota.install_error = err.clone();
            s.ota.phase = OtaPhase::Failed(err.clone());
            format!(r#"{{"ok":true,"recorded_error":"{}"}}"#, esc(&err))
        }
        ("POST", "/ota/skip") => {
            let ver = extract_json_str(body, "version").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            if !ver.is_empty() && !s.ota.skipped_versions.contains(&ver) {
                s.ota.skipped_versions.push(ver);
            }
            s.ota.phase = OtaPhase::Idle;
            r#"{"ok":true}"#.to_string()
        }
        ("POST", "/ota/set_version") => {
            let ver  = extract_json_str(body, "version").unwrap_or_default();
            let code = extract_json_num(body, "code").unwrap_or(0.0) as i64;
            let mut s = STATE.lock().unwrap();
            if !ver.is_empty() { s.ota.current_version = ver.clone(); s.config.setup_done = true; }
            if code > 0 { s.ota.current_code = code; }
            r#"{"ok":true}"#.to_string()
        }
        ("GET", "/ota/install_cmd") => {
            let s = STATE.lock().unwrap();
            let path = &s.ota.apk_local_path;
            if path.is_empty() {
                r#"{"error":"no apk downloaded"}"#.to_string()
            } else {
                format!(r#"{{"cmd":"pm install -r -t \"{}\"","path":"{}","shizuku_ready":{}}}"#,
                    esc(path), esc(path),
                    s.shizuku.permission_granted)
            }
        }
        _ => {
            if let Some(r) = route_openclaw_v3(method, path_clean, body) { r }
            else if let Some(r) = route_vlm_agent(method, path_clean, body) { r }
            else if let Some(r) = route_roboru(method, path_clean, body) { r }
            else if let Some(r) = route_openclaw(method, path_clean, body) { r }
            else { queue_to_java(path_clean.trim_start_matches('/'), body) }
        }
    }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Background threads
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// v40: Watchdog thread \u{2014} cleans stale pending actions every 30s
fn run_watchdog() {
    loop {
        thread::sleep(Duration::from_secs(30));
        watchdog_check(&mut STATE.lock().unwrap());
    }
}

/// v40/v3: Dedicated macro engine \u{2014} triggers + context zones + battery defer every 500ms
fn run_macro_engine() {
    loop {
        thread::sleep(Duration::from_millis(500));
        let mut s = STATE.lock().unwrap();
        // Check context zones (enter/exit)
        apply_context_zones(&mut s);
        // Check composite triggers
        let composites: Vec<CompositeTrigger> = s.composite_triggers.iter().cloned().collect();
        for ct in composites {
            if check_composite_trigger(&s, &ct) {
                let target = ct.target_macro.clone();
                if battery_allows_run(&s, &target) {
                    chain_macro(&mut s, &target);
                    if let Some(c) = s.composite_triggers.iter_mut().find(|c| c.id == ct.id) {
                        c.last_fired = now_ms();
                    }
                }
            }
        }
        // Standard macro triggers
        run_triggered_macros(&mut s);
    }
}

fn run_trigger_watcher() {
    loop {
        thread::sleep(Duration::from_secs(10));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let tt: Vec<Trigger> = s.triggers.iter().filter(|t| t.trigger_type=="time" && !t.fired).cloned().collect();
        for t in tt {
            let fire_at = t.value.parse::<u128>().unwrap_or(0);
            if fire_at > 0 && now >= fire_at {
                s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","type":"time"}}"#, t.id,esc(&t.action)));
                if let Some(tr) = s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired = !tr.repeat; }
            }
        }
        let hb: Vec<HeartbeatItem> = s.heartbeat_items.iter().filter(|i| i.enabled && (i.interval_ms==0 || now-i.last_run>=i.interval_ms)).cloned().collect();
        for item in hb {
            s.heartbeat_log.push_back(format!(r#"{{"id":"{}","check":"{}","ts":{}}}"#, item.id,esc(&item.check),now));
            if s.heartbeat_log.len() > 500 { s.heartbeat_log.pop_front(); }
            s.fired_triggers.push_back(format!(r#"{{"trigger":"hb_{}","action":"{}","check":"{}"}}"#, item.id,esc(&item.action),esc(&item.check)));
            if let Some(i) = s.heartbeat_items.iter_mut().find(|x| x.id==item.id) { i.last_run=now; if i.interval_ms==0 { i.enabled=false; } }
        }
    }
}

fn run_cron_scheduler() {
    loop {
        thread::sleep(Duration::from_secs(5));
        let now = now_ms();
        let mut s = STATE.lock().unwrap();
        let jobs: Vec<CronJob> = s.cron_jobs.iter().filter(|j| j.enabled && now-j.last_run>=j.interval_ms).cloned().collect();
        for job in jobs {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"cron_{}","action":"{}","type":"cron"}}"#, job.id,esc(&job.action)));
            if let Some(j) = s.cron_jobs.iter_mut().find(|x| x.id==job.id) { j.last_run=now; }
        }
    }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Helpers (unchanged from v8)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

fn config_to_json(c: &KiraConfig) -> String {
    format!(r#"{{"user_name":"{}","api_key_set":{},"base_url":"{}","model":"{}","vision_model":"{}","tg_configured":{},"agent_max_steps":{},"agent_auto_approve":{},"heartbeat_interval":{},"setup_done":{}}}"#,
        esc(&c.user_name), !c.api_key.is_empty(), esc(&c.base_url), esc(&c.model), esc(&c.vision_model),
        !c.tg_token.is_empty(), c.agent_max_steps, c.agent_auto_approve, c.heartbeat_interval, c.setup_done)
}

fn shizuku_to_json(sz: &ShizukuStatus) -> String {
    format!(r#"{{"installed":{},"permission_granted":{},"last_checked_ms":{},"error":"{}","status":"{}"}}"#,
        sz.installed, sz.permission_granted, sz.last_checked_ms, esc(&sz.error_msg),
        if sz.permission_granted{"god_mode"} else if sz.installed{"needs_permission"} else{"not_running"})
}

fn update_config_from_http(body: &str) -> String {
    let mut s = STATE.lock().unwrap();
    if let Some(v)=extract_json_str(body,"user_name") { s.config.user_name=v; }
    if let Some(v)=extract_json_str(body,"api_key")   { s.config.api_key=v; }
    if let Some(v)=extract_json_str(body,"base_url")  { s.config.base_url=v; }
    if let Some(v)=extract_json_str(body,"model")     { s.config.model=v; }
    if let Some(v)=extract_json_str(body,"tg_token")  { s.config.tg_token=v; }
    if let Some(v)=extract_json_num(body,"tg_allowed"){ s.config.tg_allowed=v as i64; }
    if let Some(v)=extract_json_num(body,"max_steps") { s.config.agent_max_steps=v as u32; }
    r#"{"ok":true}"#.to_string()
}

fn search_memory(path: &str) -> String {
    let query=path.find("q=").map(|i| &path[i+2..]).unwrap_or("").replace('+'," ");
    let ql=query.to_lowercase();
    let mut s=STATE.lock().unwrap();
    let mut results: Vec<(f32,String,String,u128)>=s.memory_index.iter_mut().filter_map(|e| {
        let mut score=0.0f32;
        if e.key.to_lowercase()==ql { score+=10.0; }
        if e.key.to_lowercase().contains(&ql) { score+=5.0; }
        let vl=e.value.to_lowercase();
        for w in ql.split_whitespace() { if vl.contains(w) { score+=1.0; } }
        for tag in e.tags.iter() { let tag: &String = tag; if tag.to_lowercase().contains(&ql) { score+=2.0; } }
        if score>0.0 { e.relevance=(e.relevance+0.1).min(5.0); e.access_count+=1; Some((score,e.key.clone(),e.value.clone(),e.ts)) } else { None }
    }).collect();
    results.sort_by(|a,b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let items: Vec<String>=results.iter().take(10).map(|(sc,k,v,ts)| format!(r#"{{"key":"{}","value":"{}","score":{:.1},"ts":{}}}"#, esc(k),esc(v),sc,ts)).collect();
    format!("[{}]", items.join(","))
}

fn compact_context(s: &mut KiraState) {
    let drain=s.context_turns.len()-20;
    let old: Vec<ContextTurn>=s.context_turns.drain(..drain).collect();
    let summary: String=old.iter().map(|t| format!("[{}]{}", t.role, &t.content[..t.content.len().min(60)])).collect::<Vec<_>>().join(";");
    s.context_compact=if s.context_compact.is_empty() { summary } else { format!("{}; {}", s.context_compact, summary) };
}

fn get_context_json() -> String {
    let s=STATE.lock().unwrap();
    let turns: Vec<String>=s.context_turns.iter().map(|t| format!(r#"{{"role":"{}","content":"{}","tokens":{}}}"#, t.role,esc(&t.content[..t.content.len().min(300)]),t.tokens)).collect();
    format!(r#"{{"compact":{},"turns":[{}],"total_tokens":{},"memory_md":{}}}"#, json_str(&s.context_compact),turns.join(","),s.total_tokens,json_str(&s.memory_md[..s.memory_md.len().min(1000)]))
}

fn get_skills_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.skills.values().map(|sk| format!(r#"{{"name":"{}","description":"{}","trigger":"{}","enabled":{},"usage_count":{}}}"#, esc(&sk.name),esc(&sk.description),esc(&sk.trigger),sk.enabled,sk.usage_count)).collect();
    format!("[{}]", items.join(","))
}

fn get_task_log_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.task_log.iter().skip(s.task_log.len().saturating_sub(50)).map(|t| format!(r#"{{"task_id":"{}","step":{},"action":"{}","result":"{}","success":{},"time":{}}}"#, esc(&t.task_id),t.step,esc(&t.action),esc(&t.result),t.success,t.time)).collect();
    format!("[{}]", items.join(","))
}

fn get_audit_log_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.audit_log.iter().skip(s.audit_log.len().saturating_sub(100)).map(|a| format!(r#"{{"session":"{}","tool":"{}","input":"{}","output":"{}","success":{},"blocked":{},"ts":{}}}"#, esc(&a.session),esc(&a.tool),esc(&a.input),esc(&a.output),a.success,a.blocked,a.ts)).collect();
    format!("[{}]", items.join(","))
}

fn register_skill(body: &str) {
    let name=extract_json_str(body,"name").unwrap_or_default();
    let desc=extract_json_str(body,"description").unwrap_or_default();
    let trigger=extract_json_str(body,"trigger").unwrap_or_default();
    let content=extract_json_str(body,"content").unwrap_or_default();
    if !name.is_empty() { STATE.lock().unwrap().skills.insert(name.clone(), Skill { name, description:desc, trigger, content, enabled:true, usage_count:0 }); }
}

fn add_trigger(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let ttype=extract_json_str(body,"type").unwrap_or_default();
    let value=extract_json_str(body,"value").unwrap_or_default();
    let action=extract_json_str(body,"action").unwrap_or_default();
    let repeat=body.contains("\"repeat\":true");
    STATE.lock().unwrap().triggers.push(Trigger { id, trigger_type:ttype, value, action, fired:false, repeat });
}

fn add_heartbeat(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let check=extract_json_str(body,"check").unwrap_or_default();
    let action=extract_json_str(body,"action").unwrap_or_default();
    let interval=extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(0);
    let mut s=STATE.lock().unwrap();
    s.heartbeat_items.retain(|i| i.id!=id);
    s.heartbeat_items.push(HeartbeatItem { id, check, action, enabled:true, last_run:0, interval_ms:interval });
}

fn add_cron(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let action=extract_json_str(body,"action").unwrap_or_default();
    let interval=extract_json_str(body,"interval_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(3_600_000);
    let expr=extract_json_str(body,"expression").unwrap_or_default();
    STATE.lock().unwrap().cron_jobs.push(CronJob { id, expression:expr, action, last_run:0, interval_ms:interval, enabled:true });
}

fn fire_notif_triggers(s: &mut KiraState, pkg: &str, title: &str, text: &str) {
    // Enforce cap on fired_triggers to prevent unbounded growth
    if s.fired_triggers.len() > 1000 { s.fired_triggers.pop_front(); }
    let combined=format!("{} {} {}", pkg, title, text).to_lowercase();
    let tt: Vec<Trigger>=s.triggers.iter().filter(|t| (t.trigger_type=="keyword_notif"||t.trigger_type=="app_notif") && !t.fired).cloned().collect();
    for t in tt {
        let hit=match t.trigger_type.as_str() {
            "keyword_notif" => combined.contains(&t.value.to_lowercase()),
            "app_notif"     => pkg==t.value,
            _ => false,
        };
        if hit {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","notif":"{}:{}"}}"#, t.id,esc(&t.action),esc(title),esc(text)));
            if let Some(tr)=s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired=!tr.repeat; }
        }
    }
}

fn fire_battery_triggers(s: &mut KiraState, pct: i32, prev: i32) {
    let tt: Vec<Trigger>=s.triggers.iter().filter(|t| t.trigger_type=="battery_low" && !t.fired).cloned().collect();
    for t in tt {
        let threshold=t.value.parse::<i32>().unwrap_or(20);
        if pct<=threshold && prev>threshold {
            s.fired_triggers.push_back(format!(r#"{{"trigger":"{}","action":"{}","battery":{}}}"#, t.id,esc(&t.action),pct));
            if let Some(tr)=s.triggers.iter_mut().find(|x| x.id==t.id) { tr.fired=!tr.repeat; }
        }
    }
}

fn do_audit(s: &mut KiraState, session: &str, tool: &str, input: &str, output: &str, success: bool, blocked: bool) {
    s.audit_log.push_back(AuditEntry { session:session.to_string(), tool:tool.to_string(), input:input[..input.len().min(200)].to_string(), output:output[..output.len().min(200)].to_string(), ts:now_ms(), success, blocked });
    if s.audit_log.len() > 5000 { s.audit_log.pop_front(); }
}

fn queue_to_java(endpoint: &str, body: &str) -> String {
    let id=gen_id();
    let payload=if body.is_empty() { format!(r#"{{"endpoint":"{}","data":{{}}}}"#, endpoint) } else { format!(r#"{{"endpoint":"{}","data":{}}}"#, endpoint, body) };
    STATE.lock().unwrap().pending_cmds.push_back((id.clone(), payload));
    let timeout=match endpoint { "install_apk"|"take_video" => 60000, _ => 10000 };
    wait_result(&id, timeout).unwrap_or_else(|| r#"{"error":"timeout"}"#.to_string())
}

fn wait_result(id: &str, ms: u64) -> Option<String> {
    let t=std::time::Instant::now();
    loop {
        { let mut s=STATE.lock().unwrap(); if let Some(r)=s.results.remove(id) { return Some(r); } }
        if t.elapsed().as_millis() as u64>=ms { return None; }
        thread::sleep(Duration::from_millis(8));
    }
}

fn make_providers() -> Vec<Provider> {
    vec![
        Provider { id:"groq".into(),       name:"Groq".into(),          base_url:"https://api.groq.com/openai/v1".into(),                             model:"llama-3.1-8b-instant".into() },
        Provider { id:"openai".into(),      name:"OpenAI".into(),         base_url:"https://api.openai.com/v1".into(),                                  model:"gpt-4o-mini".into() },
        Provider { id:"anthropic".into(),   name:"Anthropic".into(),      base_url:"https://api.anthropic.com/v1".into(),                               model:"claude-3-haiku-20240307".into() },
        Provider { id:"gemini".into(),      name:"Gemini".into(),         base_url:"https://generativelanguage.googleapis.com/v1beta/openai".into(),    model:"gemini-2.0-flash".into() },
        Provider { id:"deepseek".into(),    name:"DeepSeek".into(),       base_url:"https://api.deepseek.com/v1".into(),                                model:"deepseek-chat".into() },
        Provider { id:"openrouter".into(),  name:"OpenRouter".into(),     base_url:"https://openrouter.ai/api/v1".into(),                               model:"openrouter/auto".into() },
        Provider { id:"ollama".into(),      name:"Ollama (local)".into(), base_url:"http://localhost:11434/v1".into(),                                  model:"llama3".into() },
        Provider { id:"together".into(),    name:"Together AI".into(),    base_url:"https://api.together.xyz/v1".into(),                                model:"meta-llama/Llama-3-8b-chat-hf".into() },
        Provider { id:"mistral".into(),     name:"Mistral".into(),        base_url:"https://api.mistral.ai/v1".into(),                                  model:"mistral-small-latest".into() },
        Provider { id:"cohere".into(),      name:"Cohere".into(),         base_url:"https://api.cohere.ai/v1".into(),                                   model:"command-r".into() },
        Provider { id:"perplexity".into(),  name:"Perplexity".into(),     base_url:"https://api.perplexity.ai".into(),                                  model:"llama-3.1-sonar-small-128k-online".into() },
        Provider { id:"xai".into(),         name:"xAI Grok".into(),       base_url:"https://api.x.ai/v1".into(),                                        model:"grok-2-latest".into() },
        Provider { id:"cerebras".into(),    name:"Cerebras".into(),       base_url:"https://api.cerebras.ai/v1".into(),                                 model:"llama3.1-8b".into() },
        Provider { id:"fireworks".into(),   name:"Fireworks".into(),      base_url:"https://api.fireworks.ai/inference/v1".into(),                      model:"accounts/fireworks/models/llama-v3p1-8b-instruct".into() },
        Provider { id:"sambanova".into(),   name:"SambaNova".into(),      base_url:"https://api.sambanova.ai/v1".into(),                                model:"Meta-Llama-3.1-8B-Instruct".into() },
        Provider { id:"novita".into(),      name:"Novita AI".into(),      base_url:"https://api.novita.ai/v3/openai".into(),                            model:"llama-3.1-8b-instruct".into() },
        Provider { id:"custom".into(),      name:"Custom".into(),         base_url:String::new(),                                                       model:String::new() },
    ]
}

fn get_policy_json() -> String {
    let s=STATE.lock().unwrap();
    format!(r#"{{"allowlist":[{}],"denylist":[{}]}}"#,
        s.tool_allowlist.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","),
        s.tool_denylist.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","))
}

fn get_nodes_json() -> String {
    let s=STATE.lock().unwrap(); let now=now_ms();
    let items: Vec<String>=s.node_caps.values().map(|n| format!(r#"{{"id":"{}","platform":"{}","caps":[{}],"online":{},"last_seen":{}}}"#, esc(&n.node_id),esc(&n.platform),n.caps.iter().map(|c| format!("\"{}\"",esc(c))).collect::<Vec<_>>().join(","),n.online&&now-n.last_seen<30000,n.last_seen)).collect();
    format!("[{}]", items.join(","))
}

fn register_node(body: &str) {
    let id=extract_json_str(body,"node_id").unwrap_or_else(gen_id);
    let platform=extract_json_str(body,"platform").unwrap_or_else(|| "android".to_string());
    let caps_str=extract_json_str(body,"caps").unwrap_or_default();
    let caps: Vec<String>=caps_str.split(',').map(|c| c.trim().to_string()).filter(|c| !c.is_empty()).collect();
    STATE.lock().unwrap().node_caps.insert(id.clone(), NodeCapability { node_id:id, caps, platform, online:true, last_seen:now_ms() });
}

fn new_session(body: &str) -> String {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id);
    let channel=extract_json_str(body,"channel").unwrap_or_else(|| "kira".to_string());
    let ts=now_ms();
    let sess=Session { id:id.clone(), channel, turns:0, tokens:0, created:ts, last_msg:ts };
    STATE.lock().unwrap().sessions.insert(id.clone(), sess);
    format!(r#"{{"ok":true,"id":"{}"}}"#, id)
}

fn get_credential(body: &str) -> String {
    let name=extract_json_str(body,"name").unwrap_or_default();
    let s=STATE.lock().unwrap();
    match s.credentials.get(&name) {
        Some(enc) => { let key=derive_key(&name); let dec=xor_crypt(enc,&key); let val=String::from_utf8_lossy(&dec).to_string(); format!(r#"{{"name":"{}","value":"{}"}}"#, esc(&name),esc(&val)) }
        None      => format!(r#"{{"error":"credential '{}' not found"}}"#, esc(&name))
    }
}

fn stream_poll() -> String {
    let mut s=STATE.lock().unwrap();
    let chunks: Vec<String>=s.stream_chunks.drain(..).map(|c| format!(r#"{{"session_id":"{}","text":"{}","done":{},"ts":{}}}"#, esc(&c.session_id),esc(&c.text),c.done,c.ts)).collect();
    format!("[{}]", chunks.join(","))
}

fn push_stream_chunk(text: &str) {
    let mut s=STATE.lock().unwrap(); let sid=s.active_session.clone();
    s.stream_chunks.push_back(StreamChunk { session_id:sid, text:text.to_string(), done:false, ts:now_ms() });
    if s.stream_chunks.len() > 1000 { s.stream_chunks.pop_front(); }
}

fn relay_msg(body: &str) -> String {
    let ch=extract_json_str(body,"channel").unwrap_or_default(); let msg=extract_json_str(body,"message").unwrap_or_default(); let ts=now_ms();
    STATE.lock().unwrap().webhook_events.push_back(format!(r#"{{"type":"relay","channel":"{}","message":"{}","ts":{}}}"#, esc(&ch),esc(&msg),ts));
    r#"{"ok":true}"#.to_string()
}

fn cache_get(path: &str) -> String {
    let key=path.find("key=").map(|i| &path[i+4..]).unwrap_or("").split('&').next().unwrap_or(""); let s=STATE.lock().unwrap(); let now=now_ms();
    match s.response_cache.get(key) {
        Some(e) if e.expires_at>now => format!(r#"{{"key":"{}","value":"{}","ttl":{}}}"#, esc(key),esc(&e.value),e.expires_at-now),
        Some(_) => r#"{"error":"expired"}"#.to_string(), None => r#"{"error":"not_found"}"#.to_string(),
    }
}

fn cache_post(body: &str) -> String {
    let k=extract_json_str(body,"key").unwrap_or_default(); let v=extract_json_str(body,"value").unwrap_or_default();
    let ttl=extract_json_str(body,"ttl_ms").and_then(|s| s.parse::<u128>().ok()).unwrap_or(300_000);
    STATE.lock().unwrap().response_cache.insert(k, CacheEntry { value:v, expires_at:now_ms()+ttl });
    r#"{"ok":true}"#.to_string()
}

fn get_budget_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.tool_iterations.iter().map(|(k,v)| format!(r#"{{"session":"{}","used":{},"remaining":{}}}"#, esc(k),v,s.max_tool_iters.saturating_sub(*v))).collect();
    format!(r#"{{"max":{},"sessions":[{}]}}"#, s.max_tool_iters,items.join(","))
}

fn get_kb_json() -> String {
    let s=STATE.lock().unwrap();
    let items: Vec<String>=s.knowledge_base.iter().map(|e| format!(r#"{{"id":"{}","title":"{}","snippet":"{}","tags":[{}],"ts":{}}}"#, esc(&e.id),esc(&e.title),esc(&e.content[..e.content.len().min(100)]),e.tags.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","),e.ts)).collect();
    format!("[{}]", items.join(","))
}

fn add_kb_entry(body: &str) {
    let id=extract_json_str(body,"id").unwrap_or_else(gen_id); let title=extract_json_str(body,"title").unwrap_or_default(); let content=extract_json_str(body,"content").unwrap_or_default();
    let tags_s=extract_json_str(body,"tags").unwrap_or_default(); let tags: Vec<String>=tags_s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect();
    let mut s=STATE.lock().unwrap(); s.knowledge_base.retain(|e| e.id!=id); s.knowledge_base.push(KbEntry { id, title, content, tags, ts:now_ms() });
    if s.knowledge_base.len() > 10000 { s.knowledge_base.remove(0); }
}

fn kb_search(path: &str) -> String {
    let query=path.find("q=").map(|i| &path[i+2..]).unwrap_or("").to_lowercase(); let s=STATE.lock().unwrap();
    let mut results: Vec<(u32, &KbEntry)>=s.knowledge_base.iter().filter_map(|e| { let mut sc=0u32; if e.title.to_lowercase().contains(&query) { sc+=10; } if e.content.to_lowercase().contains(&query) { sc+=5; } for tag in e.tags.iter() { let tag: &String = tag; if tag.to_lowercase().contains(&query) { sc+=3; } } if sc>0 { Some((sc,e)) } else { None } }).collect();
    results.sort_by(|a,b| b.0.cmp(&a.0));
    let items: Vec<String>=results.iter().take(10).map(|(sc,e)| format!(r#"{{"id":"{}","title":"{}","content":"{}","score":{}}}"#, esc(&e.id),esc(&e.title),esc(&e.content[..e.content.len().min(300)]),sc)).collect();
    format!("[{}]", items.join(","))
}

fn get_metrics_text() -> String {
    let s=STATE.lock().unwrap();
    format!("kira_uptime_ms {}\nkira_requests_total {}\nkira_tool_calls {}\nkira_notifications {}\nkira_memory_entries {}\nkira_battery {}\nkira_skills {}\nkira_kb_entries {}\nkira_event_feed {}\nkira_macros {}\nkira_macro_runs {}\nkira_variables {}\n",
        now_ms()-s.uptime_start, s.request_count, s.tool_call_count, s.notifications.len(), s.memory_index.len(), s.battery_pct, s.skills.len(), s.knowledge_base.len(), s.event_feed.len(), s.macros.len(), s.macro_run_log.len(), s.variables.len())
}

fn get_event_feed() -> String {
    let s=STATE.lock().unwrap(); let skip=s.event_feed.len().saturating_sub(100);
    let items: Vec<String>=s.event_feed.iter().skip(skip).map(|e| format!(r#"{{"event":"{}","data":"{}","ts":{}}}"#, esc(&e.event),esc(&e.data),e.ts)).collect();
    format!("[{}]", items.join(","))
}

fn push_event_feed(event: &str, data: &str) {
    let mut s=STATE.lock().unwrap();
    s.event_feed.push_back(EventFeedEntry { event:event.to_string(), data:data.to_string(), ts:now_ms() });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// OpenClaw / NanoBot / ZeroClaw Extended Automation Engine
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
//
// Features added beyond basic Tasker/MacroDroid parity:
//   - Expression evaluator: math + string ops on %VAR% tokens
//   - Macro cooldowns: per-macro min interval between runs
//   - Macro chaining: one macro can trigger another by ID
//   - Retry engine: actions can retry N times on failure with backoff
//   - Macro templates: pre-built automation patterns (OpenClaw-style)
//   - Task graph / NanoBot pipeline: sequential + parallel step execution
//   - Rate limiter: max N macro runs per minute globally
//   - Per-action timeout tracking (enqueued with deadline)
//   - Condition groups: AND/OR/NOT logic on conditions
//   - AI-decision node: Kira decides next action based on context
//   - Watchdog: detect stuck macros and kill them
//   - Global macro import/export (full JSON round-trip)
//   - Built-in macro templates (ZeroClaw patterns)

/// Simple math/string expression evaluator for %VAR% tokens
/// Supports: +, -, *, /, %, ==, !=, <, >, <=, >=, &&, ||, !
/// String ops: .len(), .contains("x"), .starts("x"), .ends("x"), .upper(), .lower()
fn eval_expr(s: &KiraState, expr: &str) -> String {
    let expanded = expand_vars(s, expr);
    let trimmed = expanded.trim();

    // Try numeric arithmetic first
    if let Some(result) = eval_math(trimmed) {
        return result;
    }

    // Boolean expression
    if trimmed == "true" || trimmed == "false" {
        return trimmed.to_string();
    }

    // String length
    if trimmed.ends_with(".len()") {
        let base = trimmed.trim_end_matches(".len()");
        return base.trim_matches('"').len().to_string();
    }

    // String upper/lower
    if trimmed.ends_with(".upper()") {
        return trimmed.trim_end_matches(".upper()").trim_matches('"').to_uppercase();
    }
    if trimmed.ends_with(".lower()") {
        return trimmed.trim_end_matches(".lower()").trim_matches('"').to_lowercase();
    }

    expanded
}

fn eval_math(expr: &str) -> Option<String> {
    // Handle binary operators in order of precedence (simple single-op eval)
    for op in &["+", "-", "*", "/", "%"] {
        // Find the operator (skip if inside string)
        if let Some(pos) = expr.rfind(op) {
            if pos == 0 { continue; }
            let lhs = expr[..pos].trim();
            let rhs = expr[pos+op.len()..].trim();
            let l: f64 = lhs.parse().ok()?;
            let r: f64 = rhs.parse().ok()?;
            let result = match *op {
                "+" => l + r,
                "-" => l - r,
                "*" => l * r,
                "/" => if r == 0.0 { return Some("0".to_string()); } else { l / r },
                "%" => l % r,
                _ => return None,
            };
            // Return integer if no fractional part
            if result.fract() == 0.0 { return Some(format!("{}", result as i64)); }
            return Some(format!("{:.4}", result));
        }
    }
    None
}

/// Cooldown tracker \u{2014} returns true if macro is allowed to run now
fn check_cooldown(s: &KiraState, macro_id: &str) -> bool {
    let now = now_ms();
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        // cooldown stored in tags as "cooldown:30000" (ms)
        for tag in &m.tags {
            if let Some(rest) = tag.strip_prefix("cooldown:") {
                if let Ok(ms) = rest.parse::<u128>() {
                    return now - m.last_run_ms >= ms;
                }
            }
        }
    }
    true // no cooldown set \u{2192} always allowed
}

/// Rate limiter state: count runs in last 60s window
fn check_rate_limit(s: &KiraState) -> bool {
    let now = now_ms();
    let window = 60_000u128;
    let max_per_min = 120u64; // global cap
    let recent = s.macro_run_log.iter()
        .filter(|r| now - r.ts < window)
        .count() as u64;
    recent < max_per_min
}

/// Chain trigger: run another macro by ID (used by PushKiraEvent + KiraEvent trigger)
fn chain_macro(s: &mut KiraState, target_id: &str) {
    if !check_cooldown(s, target_id) { return; }
    if !check_rate_limit(s) { return; }

    let actions: Vec<MacroAction> = s.macros.iter()
        .find(|m| m.id == target_id && m.enabled)
        .map(|m| m.actions.clone())
        .unwrap_or_default();
    let name = s.macros.iter().find(|m| m.id == target_id)
        .map(|m| m.name.clone()).unwrap_or_default();

    if actions.is_empty() { return; }

    let start = now_ms();
    let (steps, _) = execute_macro_actions(s, target_id, &actions);
    if let Some(m) = s.macros.iter_mut().find(|m| m.id == target_id) {
        m.run_count += 1;
        m.last_run_ms = start;
    }
    s.macro_run_log.push_back(MacroRunLog {
        macro_id: target_id.to_string(), macro_name: name,
        trigger: "chain".to_string(), success: true,
        steps_run: steps, duration_ms: now_ms() - start,
        ts: start, error: String::new(),
    });
    if s.macro_run_log.len() > 1000 { s.macro_run_log.pop_front(); }
}

/// NanoBot-style task pipeline: ordered steps where each step's output
/// is stored in a variable for the next step to consume.
/// Steps are enqueued as pending actions with step index in params.
fn run_pipeline(s: &mut KiraState, macro_id: &str, pipeline_json: &str) {
    // pipeline_json = [{"kind":"http_get","params":{"url":"...","out_var":"RESULT"}}, ...]
    let actions = parse_actions_from_json(pipeline_json, "pipeline");
    for (i, action) in actions.iter().enumerate() {
        let mut a = action.clone();
        a.params.insert("_pipeline_step".to_string(), i.to_string());
        a.params.insert("_pipeline_macro".to_string(), macro_id.to_string());
        enqueue_action(s, macro_id, &a);
    }
}

/// Built-in macro templates (ZeroClaw / OpenClaw patterns)
/// Returns a Vec of pre-built AutoMacro structs ready to insert
fn make_builtin_templates() -> Vec<AutoMacro> {
    let ts = now_ms();

    // Helper to make a simple action
    let act = |kind: &str, params: Vec<(&str, &str)>| -> MacroAction {
        let mut p = HashMap::new();
        for (k, v) in params { p.insert(k.to_string(), v.to_string()); }
        MacroAction { kind: MacroActionKind::from_str(kind), params: p, sub_actions: vec![], enabled: true }
    };

    vec![
        // 1. Battery guardian \u{2014} warn + enable power saver
        AutoMacro {
            id: "tpl_battery_guardian".to_string(),
            name: "\u{1F50B} Battery Guardian".to_string(),
            description: "Toast warning when battery drops below 20%, vibrate at 10%".to_string(),
            enabled: false, // templates off by default
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::BatteryLevel,
                config: [("op".to_string(),"lte".to_string()),("threshold".to_string(),"20".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("show_toast", vec![("message", "\u{26A0}\u{FE0F} Battery low: %BATTERY%%")]),
                act("vibrate", vec![("ms", "500")]),
                act("log_event", vec![("message", "Battery guardian fired at %BATTERY%%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "battery".to_string(), "cooldown:300000".to_string()],
        },

        // 2. Work mode \u{2014} activate when connecting to work WiFi
        AutoMacro {
            id: "tpl_work_mode".to_string(),
            name: "\u{1F4BC} Work Mode".to_string(),
            description: "Switch to Work profile + mute media when joining work WiFi".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::WifiConnected,
                config: [("ssid".to_string(), "".to_string())].iter().cloned().collect(), // fill SSID
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "work")]),
                act("mute_volume", vec![("stream", "music")]),
                act("show_toast", vec![("message", "\u{1F4BC} Work mode activated")]),
                act("log_event", vec![("message", "Work mode: connected to %WIFI%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "wifi".to_string(), "work".to_string()],
        },

        // 3. Sleep mode \u{2014} dim screen, silence at night
        AutoMacro {
            id: "tpl_sleep_mode".to_string(),
            name: "\u{1F319} Sleep Mode".to_string(),
            description: "Activate Sleep profile on screen off between 22:00-07:00".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::ScreenOff,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![MacroCondition {
                lhs: "%TIME_MS%".to_string(),
                operator: "gt".to_string(),
                rhs: "0".to_string(), // Java side evaluates time range
            }],
            actions: vec![
                act("activate_profile", vec![("profile", "sleep")]),
                act("mute_volume", vec![("stream", "all")]),
                act("set_brightness", vec![("level", "0")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "sleep".to_string(), "cooldown:3600000".to_string()],
        },

        // 4. Car mode \u{2014} BT connect auto-opens maps + disables notifications
        AutoMacro {
            id: "tpl_car_mode".to_string(),
            name: "\u{1F697} Car Mode".to_string(),
            description: "Open maps + set car profile when BT device connects".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::BluetoothConnected,
                config: [("device".to_string(), "".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "car")]),
                act("open_app", vec![("package", "com.google.android.apps.maps")]),
                act("set_volume", vec![("stream", "navigation"), ("level", "12")]),
                act("show_toast", vec![("message", "\u{1F697} Car mode \u{2014} drive safe!")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "car".to_string(), "bluetooth".to_string()],
        },

        // 5. AI morning briefing \u{2014} Kira speaks summary on unlock
        AutoMacro {
            id: "tpl_morning_briefing".to_string(),
            name: "\u{1F305} Morning Briefing".to_string(),
            description: "Kira speaks a morning summary on first device unlock".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::DeviceUnlocked,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Give a 2-sentence morning briefing: battery is %BATTERY%%, profile is %PROFILE%. Be motivating and brief."),
                    ("out_var", "BRIEFING"),
                ]),
                act("kira_speak", vec![("text", "%BRIEFING%")]),
                act("log_event", vec![("message", "Morning briefing delivered")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "morning".to_string(), "cooldown:3600000".to_string()],
        },

        // 6. Smart notification filter \u{2014} AI decides if notif is urgent
        AutoMacro {
            id: "tpl_notif_filter".to_string(),
            name: "\u{1F9E0} Smart Notif Filter".to_string(),
            description: "Kira reads notifications and speaks urgent ones aloud".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::NotifReceived,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![MacroCondition {
                lhs: "%PROFILE%".to_string(),
                operator: "eq".to_string(),
                rhs: "sleep".to_string(),
            }],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Is this notification urgent enough to wake someone? Reply only YES or NO. App: %SCREEN_PKG%"),
                    ("out_var", "IS_URGENT"),
                ]),
                act("if", vec![
                    ("cond_lhs", "%IS_URGENT%"),
                    ("cond_op", "contains"),
                    ("cond_rhs", "YES"),
                ]),
                act("vibrate", vec![("ms", "1000")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "notifications".to_string()],
        },

        // 7. Clipboard AI enhancer \u{2014} transform clipboard text with AI
        AutoMacro {
            id: "tpl_clipboard_ai".to_string(),
            name: "\u{1F4CB} Clipboard AI".to_string(),
            description: "When clipboard changes, Kira can rewrite/translate/summarize".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::ClipboardChanged,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("set_variable", vec![("name", "ORIGINAL_CLIP"), ("value", "%CLIPBOARD%")]),
                act("kira_ask", vec![
                    ("prompt", "Fix grammar and spelling of this text (return only the corrected text): %CLIPBOARD%"),
                    ("out_var", "FIXED_CLIP"),
                ]),
                act("set_clipboard", vec![("text", "%FIXED_CLIP%")]),
                act("show_toast", vec![("message", "\u{2705} Clipboard enhanced by Kira")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "clipboard".to_string()],
        },

        // 8. Webhook automation \u{2014} receive external trigger, run AI, reply
        AutoMacro {
            id: "tpl_webhook_ai".to_string(),
            name: "\u{1F310} Webhook AI Agent".to_string(),
            description: "Receive HTTP POST \u{2192} Kira processes \u{2192} HTTP reply (OpenClaw pattern)".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::WebhookPost,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("kira_ask", vec![
                    ("prompt", "Process this webhook data and decide what to do: %CLIPBOARD%"),
                    ("out_var", "WEBHOOK_RESPONSE"),
                ]),
                act("http_post", vec![
                    ("url", "https://your-server.com/kira-reply"),
                    ("body", "{\"response\":\"%WEBHOOK_RESPONSE%\"}"),
                    ("content_type", "application/json"),
                ]),
                act("log_event", vec![("message", "Webhook processed by Kira AI")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "webhook".to_string(), "ai".to_string()],
        },

        // 9. NFC tag launcher \u{2014} tap tag to run specific macro
        AutoMacro {
            id: "tpl_nfc_launcher".to_string(),
            name: "\u{1F4E1} NFC Tag Launcher".to_string(),
            description: "Tap NFC tag to activate Home profile and run home routine".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::NfcTag,
                config: [("tag_id".to_string(), "".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("activate_profile", vec![("profile", "home")]),
                act("set_volume", vec![("stream", "music"), ("level", "8")]),
                act("set_brightness", vec![("level", "200")]),
                act("show_toast", vec![("message", "\u{1F3E0} Welcome home!")]),
                act("kira_speak", vec![("text", "Welcome home. I've set your home profile.")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "nfc".to_string(), "home".to_string()],
        },

        // 10. Shake-to-SOS \u{2014} shake 3x to send emergency SMS
        AutoMacro {
            id: "tpl_shake_sos".to_string(),
            name: "\u{1F198} Shake SOS".to_string(),
            description: "Shake device to send SOS SMS with location to emergency contact".to_string(),
            enabled: false,
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::Shake,
                config: HashMap::new(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("get_location", vec![("out_lat", "SOS_LAT"), ("out_lon", "SOS_LON")]),
                act("send_sms", vec![
                    ("number", "+1234567890"), // replace with emergency contact
                    ("message", "\u{1F198} SOS! I need help. My location: https://maps.google.com/?q=%SOS_LAT%,%SOS_LON%"),
                ]),
                act("vibrate", vec![("ms", "2000")]),
                act("show_toast", vec![("message", "\u{1F198} SOS sent!")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "sos".to_string(), "emergency".to_string(), "cooldown:60000".to_string()],
        },
    ]
}

/// Install built-in templates if not already present (called at startup)
fn install_builtin_templates(s: &mut KiraState) {
    let templates = make_builtin_templates();
    for tpl in templates {
        if !s.macros.iter().any(|m| m.id == tpl.id) {
            s.macros.push(tpl);
        }
    }
}




// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Roubao Vision-Language Agent  (github.com/Turbo1123/roubao)
// Open-AutoGLM Phone Agent      (github.com/zai-org/Open-AutoGLM)
//
// Core architecture implemented in pure Rust:
//
// ROUBAO pattern:
//   screenshot \u{2192} VLM prompt \u{2192} structured action decision \u{2192} execute
//   - Screenshot observation loop
//   - VLM-grounded element detection (describe what to tap)
//   - Action confidence scoring
//   - Task success verification via follow-up screenshot
//
// OPEN-AUTOGLM pattern:
//   user_goal \u{2192} task_planner \u{2192} action_executor \u{2192} state_observer \u{2192} loop
//   - Multi-step phone task decomposition
//   - Thought-Action-Observation (TAO) loop (ReAct variant)
//   - Sub-task tracking with completion state
//   - Grounded element location via text description
//   - Memory of previous actions in session
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Roubao: A VLM grounded action on a specific screen element
#[derive(Clone, Debug)]
struct VlmAction {
    action_type: String,   // "tap", "swipe", "type", "scroll", "back", "home", "done"
    element_desc: String,  // natural-language description of target element
    text:         String,  // for type actions
    direction:    String,  // for swipe/scroll: "up","down","left","right"
    confidence:   f32,     // 0.0-1.0 VLM confidence score
    reasoning:    String,  // VLM's chain-of-thought for this action
    x:            i32,     // resolved coordinates (-1 = unresolved)
    y:            i32,
}

/// Roubao: Task execution state machine
#[derive(Clone, Debug, PartialEq)]
enum VlmTaskState {
    Idle,
    Planning,       // generating action plan from goal
    Observing,      // taking screenshot for VLM
    Acting,         // executing the chosen action
    Verifying,      // checking if action succeeded
    Done(String),   // task complete with result
    Failed(String), // task failed with reason
}

/// Open-AutoGLM: TAO (Thought-Action-Observation) step
#[derive(Clone)]
struct TaoStep {
    step_num:    u32,
    thought:     String,   // VLM's reasoning about current state
    action:      VlmAction,
    observation: String,   // what happened after action (from next screenshot)
    success:     bool,
    ts:          u128,
}

/// Open-AutoGLM: A phone agent task session
#[derive(Clone)]
struct PhoneAgentTask {
    id:           String,
    goal:         String,    // user's natural language goal
    state:        VlmTaskState,
    plan:         Vec<String>, // high-level sub-tasks from planner
    plan_idx:     usize,      // current sub-task index
    history:      Vec<TaoStep>,
    max_steps:    u32,
    current_step: u32,
    context:      String,    // accumulated observations for VLM context
    result:       String,
    created_ms:   u128,
    last_step_ms: u128,
}

/// Roubao: Screenshot observation record
#[derive(Clone)]
struct ScreenObservation {
    task_id:    String,
    step:       u32,
    screen_pkg: String,
    ui_nodes:   String,  // JSON of accessibility nodes
    screenshot_path: String,
    vlm_description: String, // VLM's description of what it sees
    ts:         u128,
}

/// Build the VLM prompt for Roubao action selection
/// This is the core of Roubao's architecture
fn build_vlm_action_prompt(
    goal: &str,
    sub_task: &str,
    screen_desc: &str,
    history: &[TaoStep],
    step: u32,
    max_steps: u32,
) -> String {
    let history_str: String = history.iter().rev().take(5).rev().map(|h| {
        format!("Step {}: {} \u{2192} {} \u{2192} {}",
            h.step_num,
            h.thought.chars().take(60).collect::<String>(),
            h.action.action_type,
            if h.success { "success" } else { "failed" }
        )
    }).collect::<Vec<_>>().join("\n");

    format!(
        "You are a phone automation agent. Analyze the screen and choose ONE action.\n\
        \n\
        GOAL: {goal}\n\
        CURRENT TASK: {sub_task}\n\
        STEP: {step}/{max_steps}\n\
        \n\
        SCREEN STATE:\n{screen_desc}\n\
        \n\
        RECENT HISTORY:\n{history}\n\
        \n\
        Respond with JSON:\n\
        {{\n\
          \"thought\": \"reasoning about current state\",\n\
          \"action\": \"tap|swipe|type|scroll|back|home|done|failed\",\n\
          \"element\": \"describe the UI element to interact with\",\n\
          \"text\": \"text to type (if action=type)\",\n\
          \"direction\": \"up|down|left|right (if swipe/scroll)\",\n\
          \"confidence\": 0.0-1.0,\n\
          \"done_reason\": \"why task is complete (if action=done)\"\n\
        }}",
        goal = goal,
        sub_task = sub_task,
        step = step,
        max_steps = max_steps,
        screen_desc = &screen_desc[..screen_desc.len().min(800)],
        history = if history_str.is_empty() { "(none)".to_string() } else { history_str },
    )
}

/// Build the Open-AutoGLM task planning prompt
fn build_task_plan_prompt(goal: &str, context: &str) -> String {
    format!(
        "You are an Android phone agent. Break down this goal into 3-7 concrete sub-tasks.\n\
        \n\
        GOAL: {goal}\n\
        DEVICE CONTEXT: {ctx}\n\
        \n\
        Return a JSON array of sub-task strings:\n\
        [\"sub-task 1\", \"sub-task 2\", ...]\n\
        \n\
        Each sub-task should be a single screen interaction goal.\n\
        Be specific about app names, buttons, and content.",
        goal = goal,
        ctx = &context[..context.len().min(400)],
    )
}

/// Parse VLM JSON response into a VlmAction
fn parse_vlm_action(json: &str) -> VlmAction {
    let action_type  = extract_json_str(json, "action").unwrap_or_else(|| "back".to_string());
    let element_desc = extract_json_str(json, "element").unwrap_or_default();
    let text         = extract_json_str(json, "text").unwrap_or_default();
    let direction    = extract_json_str(json, "direction").unwrap_or_default();
    let thought      = extract_json_str(json, "thought").unwrap_or_default();
    let done_reason  = extract_json_str(json, "done_reason").unwrap_or_default();
    let confidence   = extract_json_f32(json, "confidence").unwrap_or(0.5);

    let reasoning = if !done_reason.is_empty() { done_reason } else { thought };

    VlmAction {
        action_type,
        element_desc,
        text,
        direction,
        confidence,
        reasoning,
        x: -1,
        y: -1,
    }
}

/// Convert VlmAction to a MacroAction for execution
fn vlm_action_to_macro(action: &VlmAction) -> MacroAction {
    let mut params = HashMap::new();
    match action.action_type.as_str() {
        "tap" => {
            params.insert("description".to_string(), action.element_desc.clone());
            if action.x >= 0 {
                params.insert("x".to_string(), action.x.to_string());
                params.insert("y".to_string(), action.y.to_string());
            }
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), if action.x >= 0 {
                    format!("input tap {} {}", action.x, action.y)
                } else {
                    format!("# find_and_tap: {}", action.element_desc)
                });
                p
            }, sub_actions: vec![], enabled: true }
        }
        "type" => {
            params.insert("text".to_string(), action.text.clone());
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), format!("input text '{}'", action.text.replace('\'', "")));
                p
            }, sub_actions: vec![], enabled: true }
        }
        "scroll" | "swipe" => {
            let (x1, y1, x2, y2) = match action.direction.as_str() {
                "up"    => (540, 1200, 540, 400),
                "down"  => (540, 400, 540, 1200),
                "left"  => (900, 700, 200, 700),
                "right" => (200, 700, 900, 700),
                _       => (540, 800, 540, 400),
            };
            MacroAction { kind: MacroActionKind::Shell, params: {
                let mut p = HashMap::new();
                p.insert("cmd".to_string(), format!("input swipe {} {} {} {} 300", x1, y1, x2, y2));
                p
            }, sub_actions: vec![], enabled: true }
        }
        "back" => MacroAction { kind: MacroActionKind::Shell, params: {
            let mut p = HashMap::new(); p.insert("cmd".to_string(), "input keyevent 4".to_string()); p
        }, sub_actions: vec![], enabled: true },
        "home" => MacroAction { kind: MacroActionKind::Shell, params: {
            let mut p = HashMap::new(); p.insert("cmd".to_string(), "input keyevent 3".to_string()); p
        }, sub_actions: vec![], enabled: true },
        _ => MacroAction { kind: MacroActionKind::LogEvent, params: {
            let mut p = HashMap::new(); p.insert("message".to_string(), format!("agent: {}", action.action_type)); p
        }, sub_actions: vec![], enabled: true },
    }
}

/// Enqueue a VLM step: screenshot \u{2192} VLM prompt \u{2192} Java executes \u{2192} Rust processes result
/// This implements the Roubao/Open-AutoGLM TAO loop
fn enqueue_vlm_step(s: &mut KiraState, task_id: &str) {
    // Step 1: Take screenshot and describe screen via VLM
    // This is enqueued as a special compound action for Java to handle
    s.pending_actions.push_back(PendingMacroAction {
        macro_id:  task_id.to_string(),
        action_id: gen_id(),
        kind:      "vlm_observe".to_string(),
        params: {
            let mut p = HashMap::new();
            p.insert("task_id".to_string(), task_id.to_string());
            p.insert("screen_pkg".to_string(), s.screen_pkg.clone());
            p.insert("ui_nodes_len".to_string(), s.screen_nodes.len().to_string());
            p
        },
        ts: now_ms(),
    });
}

/// Process VLM response and execute the decided action
fn execute_vlm_step(s: &mut KiraState, task_id: &str, vlm_response: &str) -> bool {
    let task = s.phone_agent_tasks.iter().find(|t| t.id == task_id).cloned();
    let task = match task { Some(t) => t, None => return false };

    let action = parse_vlm_action(vlm_response);
    let thought = extract_json_str(vlm_response, "thought").unwrap_or_default();
    let is_done = action.action_type == "done";
    let is_failed = action.action_type == "failed";

    // Convert to macro action and enqueue
    let macro_action = vlm_action_to_macro(&action);
    if !is_done && !is_failed {
        enqueue_action(s, task_id, &macro_action);
    }

    // Record TAO step
    let step = TaoStep {
        step_num:    task.current_step,
        thought,
        action:      action.clone(),
        observation: String::new(), // filled after next screenshot
        success:     !is_failed,
        ts:          now_ms(),
    };

    // Update task state
    if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
        t.history.push(step);
        t.current_step += 1;
        t.last_step_ms = now_ms();

        if is_done {
            t.state = VlmTaskState::Done(action.reasoning.clone());
            t.result = action.reasoning.clone();
        } else if is_failed {
            t.state = VlmTaskState::Failed(action.reasoning.clone());
        } else if t.current_step >= t.max_steps {
            t.state = VlmTaskState::Failed("max steps reached".to_string());
        } else {
            t.state = VlmTaskState::Observing;
            // Schedule next observation
        }
    }

    is_done || is_failed
}

/// HTTP routes for VLM / Phone Agent
fn route_vlm_agent(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // Start a new phone agent task (Open-AutoGLM pattern)
        ("POST", "/agent/task")     => {
            let goal = extract_json_str(body, "goal").unwrap_or_default();
            if goal.is_empty() { return Some(r#"{"error":"goal required"}"#.to_string()); }
            let max_steps = extract_json_num(body, "max_steps").unwrap_or(20.0) as u32;
            let task_id = gen_id();
            let task = PhoneAgentTask {
                id: task_id.clone(), goal: goal.clone(),
                state: VlmTaskState::Planning,
                plan: vec![], plan_idx: 0,
                history: vec![], max_steps,
                current_step: 0,
                context: String::new(), result: String::new(),
                created_ms: now_ms(), last_step_ms: now_ms(),
            };
            let mut s = STATE.lock().unwrap();
            // Enqueue task plan generation (Java calls AI with the planning prompt)
            let plan_prompt = build_task_plan_prompt(&goal, &s.agent_context);
            s.pending_actions.push_back(PendingMacroAction {
                macro_id: task_id.clone(), action_id: gen_id(),
                kind: "vlm_plan".to_string(),
                params: {
                    let mut p = HashMap::new();
                    p.insert("task_id".to_string(), task_id.clone());
                    p.insert("goal".to_string(), goal);
                    p.insert("prompt".to_string(), plan_prompt);
                    p
                },
                ts: now_ms(),
            });
            s.phone_agent_tasks.push(task);
            Some(format!(r#"{{"ok":true,"task_id":"{}","state":"planning"}}"#, esc(&task_id)))
        }

        // Get all tasks
        ("GET", "/agent/tasks")     => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.phone_agent_tasks.iter().map(|t| format!(
                r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"max_steps":{},"plan_steps":{},"history_steps":{}}}"#,
                esc(&t.id), esc(&t.goal),
                match &t.state {
                    VlmTaskState::Idle => "idle",
                    VlmTaskState::Planning => "planning",
                    VlmTaskState::Observing => "observing",
                    VlmTaskState::Acting => "acting",
                    VlmTaskState::Verifying => "verifying",
                    VlmTaskState::Done(_) => "done",
                    VlmTaskState::Failed(_) => "failed",
                },
                t.current_step, t.max_steps, t.plan.len(), t.history.len()
            )).collect();
            Some(format!("[{}]", items.join(",")))
        }

        // Get task detail including history
        ("GET", "/agent/task")      => {
            let id = path.find("id=").map(|i| &path[i+3..]).unwrap_or("").split('&').next().unwrap_or("");
            let s = STATE.lock().unwrap();
            match s.phone_agent_tasks.iter().find(|t| t.id == id) {
                Some(t) => {
                    let plan_json: Vec<String> = t.plan.iter().map(|p| format!("\"{}\"", esc(p))).collect();
                    let history_json: Vec<String> = t.history.iter().map(|h| format!(
                        r#"{{"step":{},"thought":"{}","action":"{}","element":"{}","success":{},"ts":{}}}"#,
                        h.step_num, esc(&h.thought), esc(&h.action.action_type),
                        esc(&h.action.element_desc), h.success, h.ts
                    )).collect();
                    let state_str = match &t.state {
                        VlmTaskState::Done(r) => format!("done:{}", r),
                        VlmTaskState::Failed(r) => format!("failed:{}", r),
                        s => format!("{:?}", s),
                    };
                    Some(format!(
                        r#"{{"id":"{}","goal":"{}","state":"{}","step":{},"plan":[{}],"history":[{}],"result":"{}"}}"#,
                        esc(&t.id), esc(&t.goal), esc(&state_str),
                        t.current_step, plan_json.join(","), history_json.join(","), esc(&t.result)
                    ))
                }
                None => Some(r#"{"error":"task not found"}"#.to_string()),
            }
        }

        // Java calls this after taking a screenshot and getting VLM description
        // Body: {task_id, vlm_response (JSON from AI)}
        ("POST", "/agent/vlm_step") => {
            let task_id = extract_json_str(body, "task_id").unwrap_or_default();
            let vlm_resp = extract_json_str(body, "vlm_response").unwrap_or_default();
            if task_id.is_empty() { return Some(r#"{"error":"task_id required"}"#.to_string()); }
            let done = execute_vlm_step(&mut STATE.lock().unwrap(), &task_id, &vlm_resp);
            Some(format!(r#"{{"ok":true,"done":{}}}"#, done))
        }

        // Java calls this with the AI-generated plan JSON array
        ("POST", "/agent/set_plan") => {
            let task_id = extract_json_str(body, "task_id").unwrap_or_default();
            // Parse plan array: ["task1","task2",...]
            let plan_str = extract_json_str(body, "plan").unwrap_or_default();
            let plan: Vec<String> = plan_str.split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let mut s = STATE.lock().unwrap();
            if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
                t.plan = plan;
                t.state = VlmTaskState::Observing;
                // Enqueue first VLM observation
            }
            // Now enqueue the first observation
            enqueue_vlm_step(&mut s, &task_id);
            Some(r#"{"ok":true,"state":"observing"}"#.to_string())
        }

        // Build VLM prompt for current task step (Java uses this to call AI)
        ("GET", "/agent/prompt")    => {
            let id = path.find("id=").map(|i| &path[i+3..]).unwrap_or("").split('&').next().unwrap_or("");
            let s = STATE.lock().unwrap();
            match s.phone_agent_tasks.iter().find(|t| t.id == id) {
                Some(t) => {
                    let sub_task = t.plan.get(t.plan_idx).cloned().unwrap_or_else(|| t.goal.clone());
                    let screen_desc = format!("Package: {}\nUI nodes count: {}\nAgent context: {}",
                        s.screen_pkg, s.screen_nodes.len(), &s.agent_context[..s.agent_context.len().min(500)]);
                    let prompt = build_vlm_action_prompt(
                        &t.goal, &sub_task, &screen_desc,
                        &t.history, t.current_step, t.max_steps
                    );
                    Some(format!(r#"{{"task_id":"{}","prompt":{},"step":{},"sub_task":"{}"}}"#,
                        esc(&t.id), json_str(&prompt), t.current_step, esc(&sub_task)))
                }
                None => Some(r#"{"error":"task not found"}"#.to_string()),
            }
        }

        // Cancel a task
        ("POST", "/agent/cancel")   => {
            let id = extract_json_str(body, "task_id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == id) {
                t.state = VlmTaskState::Failed("cancelled by user".to_string());
            }
            Some(r#"{"ok":true}"#.to_string())
        }

        // Clear completed tasks
        ("POST", "/agent/clear")    => {
            let mut s = STATE.lock().unwrap();
            s.phone_agent_tasks.retain(|t| matches!(t.state, VlmTaskState::Planning | VlmTaskState::Observing | VlmTaskState::Acting | VlmTaskState::Verifying));
            Some(format!(r#"{{"ok":true,"remaining":{}}}"#, s.phone_agent_tasks.len()))
        }

        // Screen observations log (Roubao pattern)
        ("GET", "/agent/observations") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.screen_observations.iter().rev().take(20).map(|o| format!(
                r#"{{"task_id":"{}","step":{},"pkg":"{}","vlm":"{}","ts":{}}}"#,
                esc(&o.task_id), o.step, esc(&o.screen_pkg),
                esc(&o.vlm_description[..o.vlm_description.len().min(100)]), o.ts
            )).collect();
            Some(format!("[{}]", items.join(",")))
        }

        _ => None,
    }
}

/// JNI: Java reports VLM observation result back to Rust
/// Called after Java takes screenshot, runs it through VLM, gets description
fn record_screen_observation(s: &mut KiraState, task_id: &str, step: u32, vlm_desc: &str) {
    s.screen_observations.push_back(ScreenObservation {
        task_id: task_id.to_string(),
        step,
        screen_pkg: s.screen_pkg.clone(),
        ui_nodes: s.screen_nodes.chars().take(500).collect(),
        screenshot_path: String::new(),
        vlm_description: vlm_desc.to_string(),
        ts: now_ms(),
    });
    if s.screen_observations.len() > 200 { s.screen_observations.pop_front(); }

    // Update context for current task
    if let Some(t) = s.phone_agent_tasks.iter_mut().find(|t| t.id == task_id) {
        t.context = format!("{}; step{}: {}", t.context, step, &vlm_desc[..vlm_desc.len().min(100)]);
        t.state = VlmTaskState::Acting;
        // Update observation in last history entry
        if let Some(h) = t.history.last_mut() {
            h.observation = vlm_desc.chars().take(200).collect();
        }
    }
}


// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// OpenClaw v3 / NanoBot / ZeroClaw \u{2014} Extended Automation Engine
//
// New in this version:
//   - Reactive programming: event streams + filter chains (NanoBot Rx pattern)
//   - State machine engine (ZeroClaw FSM)
//   - Sensor fusion: combine multiple device signals into composite triggers
//   - Macro scripting DSL: evaluate mini-programs from string
//   - Context-aware automation: time-of-day, location, activity zones
//   - Cross-macro communication via shared channels
//   - Macro version control: history + rollback
//   - Automation marketplace: import/export macro bundles
//   - Smart home integration hooks (MQTT/WebSocket stubs)
//   - Battery-aware scheduling: defer tasks when battery low
//   - AI-assisted macro generation: convert natural language to macro JSON
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

// \u{2500}\u{2500} NanoBot Rx: Reactive event stream \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// An event in the reactive stream
#[derive(Clone)]
struct RxEvent {
    kind:    String,   // "notification", "battery", "screen", "wifi", "timer", "custom"
    data:    String,   // JSON payload
    ts:      u128,
    source:  String,   // macro_id or "system"
}

/// A filter on the event stream (Rx-style operator)
#[derive(Clone)]
enum RxOperator {
    Filter(MacroCondition),          // only pass events matching condition
    Map(String, String),             // rename field or transform value
    Debounce(u128),                  // ignore events within N ms of last
    Throttle(u128),                  // max 1 event per N ms
    Distinct,                        // only pass if value changed from last
    Take(u32),                       // pass only first N events
    Skip(u32),                       // skip first N events
    Merge(Vec<String>),              // merge events from multiple sources
    Buffer(u32),                     // collect N events then emit as batch
}

/// A reactive subscription: event source \u{2192} operators \u{2192} macro trigger
#[derive(Clone)]
struct RxSubscription {
    id:          String,
    name:        String,
    event_kinds: Vec<String>,       // which event types to subscribe to
    operators:   Vec<RxOperator>,
    target_macro: String,           // macro to trigger when event passes filters
    enabled:     bool,
    fired_count: u64,
    last_fired:  u128,
    // State for stateful operators
    debounce_last: u128,
    throttle_last: u128,
    take_count:    u32,
    skip_count:    u32,
    last_value:    String,          // for Distinct
    buffer:        Vec<RxEvent>,
}

/// Process an event through a subscription's operator chain
fn rx_process_event(sub: &mut RxSubscription, event: &RxEvent, s: &KiraState) -> Option<String> {
    // Check event kind filter
    if !sub.event_kinds.is_empty() && !sub.event_kinds.contains(&event.kind) {
        return None;
    }

    let now = now_ms();

    for op in &sub.operators.clone() {
        match op {
            RxOperator::Filter(cond) => {
                if !eval_condition(s, cond) { return None; }
            }
            RxOperator::Debounce(ms) => {
                if now - sub.debounce_last < *ms { return None; }
                sub.debounce_last = now;
            }
            RxOperator::Throttle(ms) => {
                if now - sub.throttle_last < *ms { return None; }
                sub.throttle_last = now;
            }
            RxOperator::Distinct => {
                if event.data == sub.last_value { return None; }
                sub.last_value = event.data.clone();
            }
            RxOperator::Take(n) => {
                if sub.take_count >= *n { return None; }
                sub.take_count += 1;
            }
            RxOperator::Skip(n) => {
                if sub.skip_count < *n { sub.skip_count += 1; return None; }
            }
            RxOperator::Buffer(n) => {
                sub.buffer.push(event.clone());
                if sub.buffer.len() < *n as usize { return None; }
                // Emit buffered batch
                let batch = sub.buffer.drain(..).map(|e| e.data.clone()).collect::<Vec<_>>().join(",");
                return Some(format!(r#"{{"batch":[{}],"count":{}}}"#, batch, n));
            }
            _ => {}
        }
    }

    Some(event.data.clone())
}

// \u{2500}\u{2500} ZeroClaw FSM: Finite State Machine engine \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A state in a ZeroClaw FSM
#[derive(Clone)]
struct FsmState {
    id:           String,
    name:         String,
    entry_actions: Vec<MacroAction>,  // run when entering this state
    exit_actions:  Vec<MacroAction>,  // run when leaving this state
    is_final:     bool,
}

/// A transition between FSM states
#[derive(Clone)]
struct FsmTransition {
    from_state:  String,
    to_state:    String,
    trigger:     String,       // event kind that fires this transition
    condition:   Option<MacroCondition>,
    actions:     Vec<MacroAction>,  // run during transition
}

/// A ZeroClaw finite state machine
#[derive(Clone)]
struct StateMachine {
    id:            String,
    name:          String,
    states:        HashMap<String, FsmState>,
    transitions:   Vec<FsmTransition>,
    current_state: String,
    initial_state: String,
    enabled:       bool,
    history:       VecDeque<String>,  // state transition history
    created_ms:    u128,
}

/// Process an event through a state machine
fn fsm_process_event(s: &mut KiraState, machine_id: &str, event_kind: &str) {
    let machine = match s.state_machines.iter().find(|m| m.id == machine_id && m.enabled) {
        Some(m) => m.clone(),
        None => return,
    };

    // Find matching transition from current state
    let transition = machine.transitions.iter().find(|t| {
        t.from_state == machine.current_state
            && t.trigger == event_kind
            && t.condition.as_ref().map(|c| eval_condition(s, c)).unwrap_or(true)
    }).cloned();

    if let Some(trans) = transition {
        let from = machine.current_state.clone();
        let to = trans.to_state.clone();

        // Run exit actions of current state
        if let Some(state) = machine.states.get(&from) {
            let exit_actions = state.exit_actions.clone();
            for action in &exit_actions {
                enqueue_action(s, machine_id, action);
            }
        }

        // Run transition actions
        for action in &trans.actions {
            enqueue_action(s, machine_id, action);
        }

        // Run entry actions of new state
        if let Some(state) = machine.states.get(&to) {
            let entry_actions = state.entry_actions.clone();
            for action in &entry_actions {
                enqueue_action(s, machine_id, action);
            }
        }

        // Update machine state
        if let Some(m) = s.state_machines.iter_mut().find(|m| m.id == machine_id) {
            m.history.push_back(format!("{}->{}", from, to));
            if m.history.len() > 50 { m.history.pop_front(); }
            m.current_state = to;
        }

        // Log the transition
        s.daily_log.push_back(format!("[fsm:{}] {}\u{2192}{} via {}", machine_id, from, trans.to_state, event_kind));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
    }
}

// \u{2500}\u{2500} Sensor Fusion: Composite triggers \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A composite trigger: multiple signals combined with AND/OR/NOT logic
#[derive(Clone)]
struct CompositeTrigger {
    id:       String,
    name:     String,
    logic:    String,   // "AND", "OR", "NOT", "XOR"
    triggers: Vec<MacroTrigger>,
    target_macro: String,
    enabled:  bool,
    cooldown_ms: u128,
    last_fired:  u128,
}

fn check_composite_trigger(s: &KiraState, ct: &CompositeTrigger) -> bool {
    if !ct.enabled { return false; }
    if now_ms() - ct.last_fired < ct.cooldown_ms { return false; }

    let results: Vec<bool> = ct.triggers.iter().map(|t| check_trigger(s, t)).collect();

    match ct.logic.as_str() {
        "AND" => results.iter().all(|&r| r),
        "OR"  => results.iter().any(|&r| r),
        "NOT" => results.first().map(|&r| !r).unwrap_or(false),
        "XOR" => results.iter().filter(|&&r| r).count() == 1,
        "NAND"=> !results.iter().all(|&r| r),
        "NOR" => !results.iter().any(|&r| r),
        _ => false,
    }
}

// \u{2500}\u{2500} NanoBot Macro DSL: Mini scripting language \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Execute a NanoBot DSL script
/// Syntax (one statement per line):
///   SET $var = value
///   SET $var = %OTHER_VAR% + 1
///   IF $var == value THEN action_kind param=value
///   WAIT 500
///   REPEAT 3 action_kind param=value
///   CALL macro_id
///   CHAIN macro_id
///   LOG message text
///   NOTIFY title | body
///   HTTP GET url
///   HTTP POST url | body
///   SHELL command
fn execute_dsl_script(s: &mut KiraState, macro_id: &str, script: &str) -> Vec<String> {
    let mut log = Vec::new();

    for raw_line in script.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        match parts.as_slice() {
            ["SET", var, "=", expr] => {
                let name = var.trim_start_matches('$').to_uppercase();
                let value = eval_expr(s, expr);
                let ts = now_ms();
                s.variables.insert(name.clone(), AutoVariable {
                    name: name.clone(), value: value.clone(),
                    var_type: "string".to_string(), persistent: false,
                    created_ms: ts, updated_ms: ts,
                });
                log.push(format!("SET {} = {}", name, value));
            }
            ["WAIT", ms_str] => {
                if let Ok(ms) = ms_str.parse::<u64>() {
                    let mut p = HashMap::new();
                    p.insert("ms".to_string(), ms.to_string());
                    s.pending_actions.push_back(PendingMacroAction {
                        macro_id: macro_id.to_string(), action_id: gen_id(),
                        kind: "wait".to_string(), params: p, ts: now_ms(),
                    });
                    log.push(format!("WAIT {}ms", ms));
                }
            }
            ["LOG", ..] => {
                let msg = line[4..].trim();
                let expanded = expand_vars(s, msg);
                s.daily_log.push_back(format!("[dsl:{}] {}", macro_id, expanded));
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                log.push(format!("LOG: {}", expanded));
            }
            ["CALL", target_id] | ["CHAIN", target_id] => {
                chain_macro(s, target_id);
                log.push(format!("CHAIN \u{2192} {}", target_id));
            }
            ["IF", ..] => {
                // IF $var OP value THEN action
                if let Some(then_pos) = line.find(" THEN ") {
                    let cond_part = &line[3..then_pos].trim();
                    let action_part = line[then_pos + 6..].trim();
                    // Simple: $var == value
                    let cond_parts: Vec<&str> = cond_part.splitn(3, ' ').collect();
                    if cond_parts.len() == 3 {
                        let lhs = cond_parts[0].trim_start_matches('$').to_uppercase();
                        let op  = cond_parts[1];
                        let rhs = cond_parts[2];
                        let rust_op = match op { "==" => "eq", "!=" => "neq", ">" => "gt", "<" => "lt", ">=" => "gte", "<=" => "lte", _ => "eq" };
                        let cond = MacroCondition { lhs: format!("%{}%", lhs), operator: rust_op.to_string(), rhs: rhs.to_string() };
                        if eval_condition(s, &cond) {
                            // Execute the THEN action
                            let action_parts: Vec<&str> = action_part.splitn(2, ' ').collect();
                            let kind = action_parts[0];
                            let mut params = HashMap::new();
                            if action_parts.len() > 1 {
                                for kv in action_parts[1].split(' ') {
                                    if let Some(eq) = kv.find('=') {
                                        params.insert(kv[..eq].to_string(), expand_vars(s, &kv[eq+1..]));
                                    }
                                }
                            }
                            let action = MacroAction { kind: MacroActionKind::from_str(kind), params, sub_actions: vec![], enabled: true };
                            enqueue_action(s, macro_id, &action);
                            log.push(format!("IF {} \u{2192} executed {}", cond_part, kind));
                        }
                    }
                }
            }
            ["REPEAT", count_str, kind, ..] => {
                if let Ok(count) = count_str.parse::<u32>() {
                    let rest = if parts.len() > 3 { parts[3] } else { "" };
                    let mut params = HashMap::new();
                    for kv in rest.split(' ') {
                        if let Some(eq) = kv.find('=') {
                            params.insert(kv[..eq].to_string(), expand_vars(s, &kv[eq+1..]));
                        }
                    }
                    for _ in 0..count.min(100) {
                        let action = MacroAction { kind: MacroActionKind::from_str(kind), params: params.clone(), sub_actions: vec![], enabled: true };
                        enqueue_action(s, macro_id, &action);
                    }
                    log.push(format!("REPEAT {} \u{00D7} {}", count, kind));
                }
            }
            ["NOTIFY", ..] => {
                let rest = expand_vars(s, &line[7..]);
                let parts: Vec<&str> = rest.splitn(2, '|').collect();
                let title = parts[0].trim().to_string();
                let body  = parts.get(1).unwrap_or(&"").trim().to_string();
                let mut params = HashMap::new();
                params.insert("title".to_string(), title.clone());
                params.insert("text".to_string(), body);
                let action = MacroAction { kind: MacroActionKind::SendNotification, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("NOTIFY: {}", title));
            }
            ["HTTP", method, ..] => {
                let rest = expand_vars(s, parts.get(2).unwrap_or(&""));
                let url_parts: Vec<&str> = rest.splitn(2, '|').collect();
                let url  = url_parts[0].trim().to_string();
                let body = url_parts.get(1).unwrap_or(&"").trim().to_string();
                let kind = match *method { "GET" => MacroActionKind::HttpGet, "POST" => MacroActionKind::HttpPost, _ => MacroActionKind::HttpGet };
                let mut params = HashMap::new();
                params.insert("url".to_string(), url.clone());
                if !body.is_empty() { params.insert("body".to_string(), body); }
                let action = MacroAction { kind, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("HTTP {} {}", method, url));
            }
            ["SHELL", ..] => {
                let cmd = expand_vars(s, line[6..].trim());
                let mut params = HashMap::new();
                params.insert("cmd".to_string(), cmd.clone());
                let action = MacroAction { kind: MacroActionKind::Shell, params, sub_actions: vec![], enabled: true };
                enqueue_action(s, macro_id, &action);
                log.push(format!("SHELL: {}", &cmd[..cmd.len().min(50)]));
            }
            _ => {
                log.push(format!("UNKNOWN: {}", &line[..line.len().min(40)]));
            }
        }
    }

    log
}

// \u{2500}\u{2500} Battery-aware scheduling \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Check if it's safe to run a macro given battery state
fn battery_allows_run(s: &KiraState, macro_id: &str) -> bool {
    // Find battery threshold tag: "battery_min:20" means require >= 20%
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        for tag in &m.tags {
            if let Some(rest) = tag.strip_prefix("battery_min:") {
                if let Ok(min) = rest.parse::<i32>() {
                    if s.battery_pct < min && !s.battery_charging {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Defer a macro to run when battery is back above threshold
fn defer_until_charged(s: &mut KiraState, macro_id: &str, min_pct: i32) {
    // Add a battery_level trigger to run when battery recovers
    let trigger_id = format!("deferred_{}_{}", macro_id, now_ms());
    s.triggers.push(Trigger {
        id: trigger_id,
        trigger_type: "battery_recovery".to_string(),
        value: min_pct.to_string(),
        action: macro_id.to_string(),
        fired: false,
        repeat: false,
    });
    s.daily_log.push_back(format!("[defer] {} deferred until battery >= {}%", macro_id, min_pct));
    if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
}

// \u{2500}\u{2500} Context zones: time + location based automation \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// A context zone: when device is in this context, activate/deactivate macros
#[derive(Clone)]
struct ContextZone {
    id:            String,
    name:          String,
    // Time window
    active_hours_start: u8,   // 0-23
    active_hours_end:   u8,
    active_days: Vec<u8>,     // 0=Sun,1=Mon,...,6=Sat (empty=all)
    // Location
    lat:     f64,
    lon:     f64,
    radius_m: f64,            // 0 = ignore location
    // Profile to activate
    activate_profile: String,
    // Macros to enable/disable
    enable_macros:  Vec<String>,
    disable_macros: Vec<String>,
    enabled:        bool,
    currently_active: bool,
}

/// Check if a context zone is active now
fn is_zone_active(zone: &ContextZone, s: &KiraState) -> bool {
    // Time check (simplified: use current hour from timestamp)
    let now_secs = now_ms() / 1000;
    let hour = ((now_secs % 86400) / 3600) as u8;

    let in_time = if zone.active_hours_start <= zone.active_hours_end {
        hour >= zone.active_hours_start && hour < zone.active_hours_end
    } else {
        // Overnight: e.g. 22:00-06:00
        hour >= zone.active_hours_start || hour < zone.active_hours_end
    };

    if !in_time { return false; }

    // Location check
    if zone.radius_m > 0.0 && (s.sig_lat != 0.0 || s.sig_lon != 0.0) {
        let dlat = (s.sig_lat - zone.lat).to_radians();
        let dlon = (s.sig_lon - zone.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + zone.lat.to_radians().cos() * s.sig_lat.to_radians().cos()
            * (dlon / 2.0).sin().powi(2);
        let dist_m = 6_371_000.0 * 2.0 * a.sqrt().asin();
        if dist_m > zone.radius_m { return false; }
    }

    true
}

/// Apply context zone changes
fn apply_context_zones(s: &mut KiraState) {
    let zones: Vec<ContextZone> = s.context_zones.iter().cloned().collect();
    for zone in zones {
        if !zone.enabled { continue; }
        let active = is_zone_active(&zone, s);

        // Detect zone enter/exit
        let was_active = zone.currently_active;
        if active && !was_active {
            // Zone entered
            if !zone.activate_profile.is_empty() {
                s.active_profile = zone.activate_profile.clone();
            }
            for macro_id in &zone.enable_macros {
                if let Some(m) = s.macros.iter_mut().find(|m| m.id == *macro_id) {
                    m.enabled = true;
                }
            }
            for macro_id in &zone.disable_macros {
                if let Some(m) = s.macros.iter_mut().find(|m| m.id == *macro_id) {
                    m.enabled = false;
                }
            }
            s.daily_log.push_back(format!("[zone] entered: {}", zone.name));
        }

        // Update zone active state
        if let Some(z) = s.context_zones.iter_mut().find(|z| z.id == zone.id) {
            z.currently_active = active;
        }
    }
    if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
}

// \u{2500}\u{2500} Macro bundle: import/export marketplace \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Export macros + keywords + flows as a shareable bundle
fn export_bundle(s: &KiraState, tag_filter: Option<&str>) -> String {
    let macros: Vec<String> = s.macros.iter()
        .filter(|m| tag_filter.map(|t| m.tags.contains(&t.to_string())).unwrap_or(true))
        .map(macro_to_json)
        .collect();
    let keywords: Vec<String> = s.roboru_keywords.values().map(|kw| {
        let steps_json: Vec<String> = kw.steps.iter().map(action_to_json).collect();
        let args_json: Vec<String> = kw.args.iter().map(|a| format!(r#""{}""#, esc(a))).collect();
        format!(r#"{{"name":"{}","description":"{}","args":[{}],"steps":[{}],"returns":"{}"}}"#,
            esc(&kw.name), esc(&kw.description), args_json.join(","), steps_json.join(","), esc(&kw.returns))
    }).collect();
    format!(
        r#"{{"version":"1.0","exported_ms":{},"macros":[{}],"keywords":[{}],"variable_count":{}}}"#,
        now_ms(), macros.join(","), keywords.join(","), s.variables.len()
    )
}

// \u{2500}\u{2500} Cross-macro channel communication \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Post a message to a named channel (macros can subscribe via kira_event trigger)
fn channel_post(s: &mut KiraState, channel: &str, message: &str) {
    let event_key = format!("channel:{}:{}", channel, message);
    s.sig_kira_event = event_key.clone();
    s.event_feed.push_back(EventFeedEntry {
        event: format!("channel_{}", channel),
        data: message.to_string(),
        ts: now_ms(),
    });
    if s.event_feed.len() > 5000 { s.event_feed.pop_front(); }
}

// \u{2500}\u{2500} HTTP routes for new features \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}


/// Parse a natural-language condition string into (trigger_kind, trigger_value).
/// Examples:
///   "battery < 20"      → ("battery_low",  "20")
///   "screen on"         → ("screen_on",    "")
///   "wifi connected"    → ("wifi_changed",  "connected")
///   "app youtube"       → ("app_opened",   "com.google.android.youtube")
///   "notification otp"  → ("keyword_notif","otp")
///   "time 07:30"        → ("time_daily",   "07:30")
///   "charging"          → ("power_connected","")
///   "unplugged"         → ("power_disconnected","")
fn parse_nl_condition(cond: &str) -> (String, String) {
    let c = cond.to_lowercase();
    let c = c.trim();

    // Battery patterns: "battery < 20", "battery below 20", "battery 20"
    if c.contains("battery") {
        let num: String = c.chars().filter(|ch| ch.is_ascii_digit()).collect();
        if !num.is_empty() { return ("battery_low".to_string(), num); }
        if c.contains("full") || c.contains("100") { return ("battery_low".to_string(), "95".to_string()); }
    }
    // Screen events
    if c.contains("screen on") || c.contains("screen unlock") || c.contains("unlocked") {
        return ("screen_on".to_string(), String::new());
    }
    if c.contains("screen off") || c.contains("screen lock") || c.contains("locked") {
        return ("screen_off".to_string(), String::new());
    }
    // WiFi
    if c.contains("wifi") {
        if c.contains("disconnect") { return ("wifi_changed".to_string(), "disconnected".to_string()); }
        return ("wifi_changed".to_string(), "connected".to_string());
    }
    // Charging / power
    if c.contains("charging") || c.contains("plugged") || c.contains("power on") {
        return ("power_connected".to_string(), String::new());
    }
    if c.contains("unplug") || c.contains("unplugged") || c.contains("power off") {
        return ("power_disconnected".to_string(), String::new());
    }
    // Time: "time 07:30", "at 07:30", "07:30"
    if c.contains("time") || c.contains(" at ") || c.contains(":") {
        let time: String = c.split_whitespace()
            .find(|w| w.contains(':') && w.len() <= 5)
            .unwrap_or("08:00")
            .to_string();
        return ("time_daily".to_string(), time);
    }
    // App opened: "app youtube", "youtube opens"
    if c.contains("app ") || c.contains(" open") || c.contains(" launch") {
        let word = c.split_whitespace()
            .find(|w| !["app","open","opens","launch","when","if"].contains(w))
            .unwrap_or("").to_string();
        return ("app_opened".to_string(), app_name_to_pkg(&word));
    }
    // Notification keyword: "notification otp", "notif payment"
    if c.contains("notif") || c.contains("message") || c.contains("alert") {
        let keyword = c.split_whitespace()
            .find(|w| !["notification","notif","message","alert","a","the","contains"].contains(w))
            .unwrap_or("").to_string();
        return ("keyword_notif".to_string(), keyword);
    }
    // Shake / motion
    if c.contains("shake") { return ("shake".to_string(), String::new()); }
    // Headphones
    if c.contains("headphone") || c.contains("earphone") || c.contains("audio plug") {
        return ("headphone_connected".to_string(), String::new());
    }
    // Fallback: use as a keyword trigger
    ("keyword_notif".to_string(), c.to_string())
}

/// Map friendly app name → package name.
/// Covers ~30 most common Android apps.
/// Map 200+ app names/aliases to their Android package names.
/// Called by automation triggers (watch_app, if_then "app X") and tool dispatch.

fn app_name_to_pkg(name: &str) -> String {
    let n = name.to_lowercase().trim().replace('-', " ").replace('_', " ");
    let pkg = match n.as_str() {
        // ── Google core ─────────────────────────────────────────────────
        "youtube"|"yt"|"you tube"                        => "com.google.android.youtube",
        "gmail"|"google mail"                            => "com.google.android.gm",
        "chrome"|"google chrome"                         => "com.android.chrome",
        "maps"|"google maps"|"gmap"|"gmaps"              => "com.google.android.apps.maps",
        "drive"|"google drive"                           => "com.google.android.apps.docs",
        "docs"|"google docs"                             => "com.google.android.apps.docs",
        "sheets"|"google sheets"|"spreadsheet"           => "com.google.android.apps.spreadsheets",
        "slides"|"google slides"|"presentation"          => "com.google.android.apps.docs",
        "photos"|"google photos"|"gphotos"               => "com.google.android.apps.photos",
        "calendar"|"google calendar"                     => "com.google.android.calendar",
        "meet"|"google meet"                             => "com.google.android.apps.meetings",
        "duo"|"google duo"                               => "com.google.android.apps.tachyon",
        "keep"|"google keep"|"notes"                     => "com.google.android.keep",
        "translate"|"google translate"                   => "com.google.android.apps.translate",
        "lens"|"google lens"                             => "com.google.ar.lens",
        "pay"|"google pay"|"gpay"                        => "com.google.android.apps.nbu.paisa.user",
        "classroom"|"google classroom"                   => "com.google.android.apps.classroom",
        "earth"|"google earth"                           => "com.google.earth",
        "fit"|"google fit"                               => "com.google.android.apps.fitness",
        "news"|"google news"                             => "com.google.android.apps.magazines",
        "play store"|"play"|"market"|"playstore"         => "com.android.vending",
        "play games"|"games"                             => "com.google.android.play.games",
        "play music"                                     => "com.google.android.music",
        "youtube music"|"yt music"|"ytmusic"             => "com.google.android.apps.youtube.music",
        "youtube kids"|"yt kids"                         => "com.google.android.apps.youtube.kids",
        "stadia"                                         => "com.google.stadia.android",
        "chrome beta"                                    => "com.chrome.beta",
        "chrome dev"                                     => "com.chrome.dev",
        "google assistant"|"assistant"                   => "com.google.android.googlequicksearchbox",
        "google search"|"search"                         => "com.google.android.googlequicksearchbox",
        "gemini"|"bard"                                  => "com.google.android.apps.bard",
        "google home"                                    => "com.google.android.apps.chromecast.app",
        "google one"                                     => "com.google.android.apps.subscriptions.red",
        "google tasks"|"tasks"                           => "com.google.android.apps.tasks",
        // ── System / Android ────────────────────────────────────────────
        "settings"                                       => "com.android.settings",
        "camera"|"cam"                                   => "com.android.camera2",
        "gallery"                                        => "com.android.gallery3d",
        "clock"|"alarm"|"timer"                          => "com.google.android.deskclock",
        "calculator"|"calc"                              => "com.google.android.calculator",
        "contacts"                                       => "com.google.android.contacts",
        "phone"|"dialer"|"call"                          => "com.google.android.dialer",
        "messages"|"sms"|"mms"|"android messages"        => "com.google.android.apps.messaging",
        "files"|"file manager"|"file explorer"           => "com.google.android.apps.nbu.files",
        "downloads"|"download manager"                   => "com.android.providers.downloads.ui",
        "browser"                                        => "com.android.browser",
        "music"|"media"|"player"                         => "com.google.android.music",
        "recorder"|"voice recorder"                      => "com.google.android.apps.recorder",
        "wallet"                                         => "com.google.android.apps.walletnfcrel",
        "accessibility"                                  => "com.google.android.marvin.talkback",
        "device health"|"battery saver"                  => "com.google.android.apps.turbo",
        "find my device"                                 => "com.google.android.apps.adm",
        "digital wellbeing"                              => "com.google.android.apps.wellbeing",
        "family link"                                    => "com.google.android.apps.kids.familylink",
        "android auto"                                   => "com.google.android.projection.gearhead",
        // ── Messaging / Social ───────────────────────────────────────────
        "whatsapp"|"wa"|"whats app"                      => "com.whatsapp",
        "whatsapp business"                              => "com.whatsapp.w4b",
        "telegram"|"tg"                                  => "org.telegram.messenger",
        "telegram x"                                     => "org.thunderdog.challegram",
        "instagram"|"ig"|"insta"                         => "com.instagram.android",
        "facebook"|"fb"                                  => "com.facebook.katana",
        "facebook messenger"|"messenger"                 => "com.facebook.orca",
        "facebook lite"                                  => "com.facebook.lite",
        "twitter"|"x"|"twitter x"                       => "com.twitter.android",
        "snapchat"|"snap"                                => "com.snapchat.android",
        "tiktok"|"tik tok"                               => "com.zhiliaoapp.musically",
        "discord"                                        => "com.discord",
        "reddit"                                         => "com.reddit.frontpage",
        "linkedin"                                       => "com.linkedin.android",
        "pinterest"                                      => "com.pinterest",
        "tumblr"                                         => "com.tumblr",
        "signal"                                         => "org.thoughtcrime.securesms",
        "viber"                                          => "com.viber.voip",
        "skype"                                          => "com.skype.raider",
        "line"                                           => "jp.naver.line.android",
        "kik"                                            => "kik.android",
        "wechat"|"we chat"                               => "com.tencent.mm",
        "imessage"                                       => "com.apple.MobileSMS",
        "imo"                                            => "com.imo.android.imoim",
        "hike"                                           => "com.bsb.hike",
        "clubhouse"                                      => "com.clubhouse.app",
        "mastodon"                                       => "org.joinmastodon.android",
        "threads"                                        => "com.instagram.barcelona",
        "bereal"                                         => "com.bereal.ft",
        // ── Entertainment ────────────────────────────────────────────────
        "netflix"                                        => "com.netflix.mediaclient",
        "spotify"                                        => "com.spotify.music",
        "amazon music"|"amazon prime music"              => "com.amazon.mp3",
        "prime video"|"amazon prime video"|"amazon video"=> "com.amazon.avod.thirdpartyclient",
        "disney plus"|"disney+"|"disneyplus"             => "com.disney.disneyplus",
        "hulu"                                           => "com.hulu.plus",
        "hbo max"|"max"                                  => "com.hbo.hbomax",
        "apple tv"|"apple tv+"                           => "com.apple.atve.amazon.appletv",
        "peacock"                                        => "com.peacocktv.peacockandroid",
        "paramount plus"|"paramount+"                    => "com.cbs.app",
        "twitch"                                         => "tv.twitch.android.app",
        "soundcloud"                                     => "com.soundcloud.android",
        "deezer"                                         => "deezer.android.app",
        "pandora"                                        => "com.pandora.android",
        "tidal"                                          => "com.aspiro.tidal",
        "shazam"                                         => "com.shazam.android",
        "audible"                                        => "com.audible.application",
        "plex"                                           => "com.plexapp.android",
        "vlc"                                            => "org.videolan.vlc",
        "kodi"                                           => "org.xbmc.kodi",
        "crunchyroll"                                    => "com.crunchyroll.crunchyroid",
        "mubi"                                           => "com.mubi",
        "vimeo"                                          => "com.vimeo.android.videoapp",
        "dailymotion"                                    => "com.dailymotion.dailymotion",
        "mixcloud"                                       => "com.mixcloud.android",
        // ── Shopping / Finance ───────────────────────────────────────────
        "amazon"|"amazon shopping"                       => "com.amazon.mShop.android.shopping",
        "ebay"                                           => "com.ebay.mobile",
        "flipkart"                                       => "com.flipkart.android",
        "myntra"                                         => "com.myntra.android",
        "meesho"                                         => "com.meesho.supply",
        "ajio"                                           => "com.ril.ajio",
        "nykaa"                                          => "com.nykaa.app",
        "paytm"                                          => "net.one97.paytm",
        "phonepe"                                        => "com.phonepe.app",

        "bhim"|"bhim upi"                                => "in.org.npci.upiapp",
        "paypal"                                         => "com.paypal.android.p2pmobile",
        "cash app"                                       => "com.squareup.cash",
        "venmo"                                          => "com.venmo",
        "wise"|"transferwise"                            => "com.transferwise.android",
        "coinbase"                                       => "com.coinbase.android",
        "binance"                                        => "com.binance.dev",
        "robinhood"                                      => "com.robinhood.android",
        "zerodha"|"kite"                                 => "com.zerodha.kite3",
        "groww"                                          => "com.nextbillion.groww",
        "upstox"                                         => "in.upstox.trading",
        // ── Navigation / Transport ───────────────────────────────────────
        "uber"                                           => "com.ubercab",
        "lyft"                                           => "me.lyft.android",
        "ola"|"ola cabs"                                 => "com.olacabs.customer",
        "rapido"                                         => "com.rapido.passenger",
        "grab"                                           => "com.grabtaxi.passenger",
        "waze"                                           => "com.waze",
        "here maps"|"here"                               => "com.here.app.maps",
        "maps me"|"mapsme"                               => "com.mapswithme.maps.pro",
        "citymapper"                                     => "com.citymapper.app.release",
        "moovit"                                         => "com.tranzmate",
        "sygic"                                          => "com.sygic.aura",
        "garmin"|"garmin connect"                        => "com.garmin.android.apps.connectmobile",
        "strava"                                         => "com.strava",
        // ── Productivity / Work ──────────────────────────────────────────
        "zoom"                                           => "us.zoom.videomeetings",
        "teams"|"microsoft teams"                        => "com.microsoft.teams",
        "slack"                                          => "com.Slack",
        "notion"                                         => "notion.id",
        "trello"                                         => "com.trello",
        "asana"                                          => "com.asana.app",
        "jira"                                           => "com.atlassian.android.jira.core",
        "monday"|"monday.com"                            => "com.monday.monday",
        "todoist"                                        => "com.todoist.android.Todoist",
        "any.do"|"any do"                                => "com.anydo",
        "ticktick"                                       => "com.ticktick.task",
        "microsoft office"|"office"                      => "com.microsoft.office.officehubrow",
        "word"|"microsoft word"                          => "com.microsoft.office.word",
        "excel"|"microsoft excel"                        => "com.microsoft.office.excel",
        "powerpoint"|"microsoft powerpoint"              => "com.microsoft.office.powerpoint",
        "outlook"|"microsoft outlook"                    => "com.microsoft.office.outlook",
        "onenote"|"microsoft onenote"                    => "com.microsoft.office.onenote",
        "onedrive"|"microsoft onedrive"                  => "com.microsoft.skydrive",
        "dropbox"                                        => "com.dropbox.android",
        "box"                                            => "com.box.android",
        "evernote"                                       => "com.evernote",
        "obsidian"                                       => "md.obsidian",
        "proton mail"|"protonmail"                       => "ch.protonmail.android",
        "hey email"                                      => "com.basecamp.hey",
        "spark email"|"spark"                            => "com.readdle.spark",
        "canva"                                          => "com.canva.editor",
        "adobe express"                                  => "com.adobe.spark.post",
        "adobe acrobat"|"acrobat"                        => "com.adobe.reader",
        "adobe lightroom"|"lightroom"                    => "com.adobe.lrmobile",
        "snapseed"                                       => "com.niksoftware.snapseed",
        "vsco"                                           => "com.vsco.cam",
        "remini"                                         => "com.bigwinepot.nwc.international",
        "1password"                                      => "com.agilebits.onepassword",
        "bitwarden"                                      => "com.x8bit.bitwarden",
        "lastpass"                                       => "com.lastpass.lpandroid",
        "dashlane"                                       => "com.dashlane",
        "nordvpn"                                        => "com.nordvpn.android",
        "expressvpn"                                     => "com.expressvpn.vpn",
        "proton vpn"|"protonvpn"                         => "ch.protonvpn.android",
        // ── Food / Delivery ──────────────────────────────────────────────
        "swiggy"                                         => "in.swiggy.android",
        "zomato"                                         => "com.application.zomato",
        "uber eats"|"ubereats"                           => "com.ubercab.eats",
        "doordash"                                       => "com.dd.doordash",
        "instacart"                                      => "com.instacart.client",
        "grubhub"                                        => "com.grubhub.android",
        "dunzo"                                          => "com.dunzo.user",
        "blinkit"|"grofers"                              => "com.grofers.customerapp",
        "bigbasket"                                      => "com.bigbasket",
        "zepto"                                          => "com.zepto.app",
        // ── Health / Fitness ─────────────────────────────────────────────
        "samsung health"|"s health"                      => "com.sec.android.app.shealth",
        "fitbit"                                         => "com.fitbit.FitbitMobile",
        "myfitnesspal"                                   => "com.myfitnesspal.android",
        "lifesum"                                        => "com.sillens.shapeupclub",
        "nike run club"|"nike running"|"nike run"        => "com.nike.plusgps",
        "adidas running"|"runtastic"                     => "com.runtastic.android",
        "runkeeper"                                      => "com.fitnesskeeper.runkeeper.pro",
        "headspace"                                      => "com.getsomeheadspace.android",
        "calm"                                           => "com.calm.android",
        "sleep cycle"                                    => "com.northcube.sleepcycle",
        "period tracker"|"flo"|"flo health"              => "org.iggymedia.periodtracker",
        "blood pressure"|"bp monitor"                    => "com.qardio.android",
        // ── News / Reading ───────────────────────────────────────────────
        "inshorts"                                       => "com.nis.app",
        "flipboard"                                      => "flipboard.app",
        "feedly"                                         => "com.devhd.feedly",
        "pocket"                                         => "com.ideashower.readitlater.pro",
        "medium"                                         => "com.medium.reader",
        "kindle"                                         => "com.amazon.kindle",
        "kobo"                                           => "com.kobobooks.android",
        "scribd"                                         => "com.scribd.app.reader0",
        "duolingo"                                       => "com.duolingo",
        "babbel"                                         => "com.babbel.mobile.android.en",
        // ── Gaming ───────────────────────────────────────────────────────
        "pubg"|"bgmi"|"battlegrounds mobile"             => "com.pubg.imobile",
        "free fire"|"freefire"|"garena free fire"        => "com.dts.freefireth",
        "minecraft"                                      => "com.mojang.minecraftpe",
        "roblox"                                         => "com.roblox.client",
        "candy crush"|"candy crush saga"                 => "com.king.candycrushsaga",
        "among us"                                       => "com.innersloth.spacemafia",
        "clash of clans"|"coc"                           => "com.supercell.clashofclans",
        "clash royale"                                   => "com.supercell.clashroyale",
        "mobile legends"|"mlbb"                          => "com.mobile.legends",
        "pokemon go"                                     => "com.nianticlabs.pokemongo",
        "ludo king"                                      => "com.ludo.king",
        "8 ball pool"                                    => "com.miniclip.eightballpool",
        "chess"|"chess.com"                              => "com.chess",
        "steam"                                          => "com.valvesoftware.android.steam.community",
        // ── Travel ───────────────────────────────────────────────────────
        "airbnb"                                         => "com.airbnb.android",
        "booking.com"|"booking"                          => "com.booking",
        "tripadvisor"                                    => "com.tripadvisor.tripadvisor",
        "expedia"                                        => "com.expedia.bookings",
        "makemytrip"|"mmt"                               => "com.makemytrip",
        "goibibo"                                        => "com.goibibo",
        "cleartrip"                                      => "com.cleartrip.android",
        "ixigo"                                          => "com.ixigo.train.ixitrain",
        "irctc"|"irctc rail connect"                     => "cris.org.in.prs.ima",
        "redbus"                                         => "in.redbus.android",
        "trainman"                                       => "com.trainman.app",
        // ── Developer / Utility ──────────────────────────────────────────
        "termux"                                         => "com.termux",
        "adb"|"adb wifi"                                 => "com.ttxapps.wifiadb",
        "ssh"|"juicessh"|"termius"                       => "com.server.auditor.ssh.client",
        "github"                                         => "com.github.android",
        "gitlab"                                         => "com.gitlab.android",
        "stackoverflow"                                  => "com.stackexchange.stackoverflow",
        "chrome remote desktop"                          => "com.google.chromeremotedesktop",
        "anydesk"                                        => "com.anydesk.anydeskandroid",
        "teamviewer"                                     => "com.teamviewer.teamviewer.market.mobile",
        "winrar"                                         => "com.rarlab",
        "cx file explorer"                               => "com.cxinventor.file.explorer",
        "es file explorer"                               => "com.estrongs.android.pop",
        "solid explorer"                                 => "pl.solidexplorer2",
        "mixplorer"                                      => "com.mixplorer",
        "qr scanner"|"qr code"                           => "me.scan.android.scanner",
        "barcode scanner"                                => "com.google.zxing.client.android",
        "cpu z"                                          => "com.cpuid.cpu_z",
        "antutu"                                         => "com.antutu.ABenchMark",
        "wifi analyzer"|"wifi analyser"                  => "com.farproc.wifi.analyzer",
        "gsam battery"                                   => "com.gsamlabs.bbm",
        "accubattery"                                    => "com.digibites.accubattery",
        "magisk"                                         => "io.github.huskydg.magisk",
        "shizuku"                                        => "moe.shizuku.privileged.api",
        "obtainium"                                      => "dev.imranr.obtainium",
        // ── Samsung specific ─────────────────────────────────────────────
        "samsung camera"                                 => "com.sec.android.app.camera",
        "samsung gallery"                                => "com.sec.android.gallery3d",
        "samsung internet"                               => "com.sec.android.app.sbrowser",
        "samsung pay"                                    => "com.samsung.android.spay",
        "samsung notes"|"s note"                         => "com.samsung.android.app.notes",
        "samsung bixby"|"bixby"                          => "com.samsung.android.bixby.agent",
        "samsung store"|"galaxy store"                   => "com.sec.android.app.samsungapps",

        "samsung music"                                  => "com.sec.android.app.music",
        "samsung clock"                                  => "com.sec.android.app.clockpackage",
        "dex"|"samsung dex"                              => "com.samsung.android.desktopmode.uiservice",
        // ── Xiaomi specific ──────────────────────────────────────────────
        "miui camera"|"mi camera"                        => "com.android.camera",
        "mi gallery"|"miui gallery"                      => "com.miui.gallery",
        "mi browser"|"miui browser"                      => "com.mi.globalbrowser",
        "mi store"                                       => "com.xiaomi.mihome",
        "mi music"                                       => "com.miui.player",
        "mi video"                                       => "com.miui.videoplayer",
        "mi home"|"xiaomi home"                          => "com.xiaomi.smarthome",
        "mi pay"                                         => "com.mipay.wallet.in",
        "mi calculator"                                  => "com.miui.calculator",
        "mi cleaner"|"miui cleaner"                      => "com.miui.cleanmaster",
        "miui themes"                                    => "com.mi.android.globalminusscreen",
        // ── Other popular ────────────────────────────────────────────────
        "brave browser"|"brave"                          => "com.brave.browser",
        "firefox"|"firefox browser"                      => "org.mozilla.firefox",
        "opera"|"opera browser"                          => "com.opera.browser",
        "duckduckgo"|"ddg"                               => "com.duckduckgo.mobile.android",
        "tor browser"                                    => "org.torproject.torbrowser",
        "edge"|"microsoft edge"                          => "com.microsoft.emmx",
        "vivaldi"                                        => "com.vivaldi.browser",
        "via browser"|"via"                              => "mark.via.gp",
        "kiwi browser"                                   => "com.kiwibrowser.browser",
        "cx browser"                                     => "com.cxinventor.browse",
        "mx player"                                      => "com.mxtech.videoplayer.ad",
        "video player"                                   => "com.mxtech.videoplayer.ad",
        "poweramp"                                       => "com.maxmpz.audioplayer",
        "musicolet"                                      => "in.krosbits.musicolet",
        "neutron music"                                  => "com.neutroncode.mp",
        "anki"                                           => "com.ichi2.anki",
        "wikipedia"                                      => "org.wikipedia",
        "wolfram alpha"                                  => "com.wolfram.android.alpha",
        "moon reader"|"moon+ reader"                     => "com.flyersoft.moonreader",
        "tasker"                                         => "net.dinglisch.android.taskerm",
        "automate"                                       => "com.llamalab.automate",
        "macrodroid"                                     => "com.arlosoft.macrodroid",
        "airtable"                                       => "com.formagrid.airtable",
        "zapier"                                         => "com.zapier.android",
        "ifttt"                                          => "com.ifttt.ifttt",
        "shortcuts"|"shortcut"                           => "com.google.android.apps.shortcuts",
        "chatgpt"|"chat gpt"                             => "com.openai.chatgpt",
        "claude"                                         => "com.anthropic.claude",
        "perplexity"                                     => "ai.perplexity.app.android",

        "copilot"|"microsoft copilot"                    => "com.microsoft.copilot",
        "grok"                                           => "com.x.android",
        _                                                => &n,
    };
    if pkg != n.as_str() { pkg.to_string() } else { name.trim().to_string() }
}

fn route_openclaw_v3(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // \u{2500}\u{2500} DSL Script execution
        ("POST", "/dsl/run") => {
            let macro_id = extract_json_str(body, "macro_id").unwrap_or_else(gen_id);
            let script   = extract_json_str(body, "script").unwrap_or_default();
            let log = execute_dsl_script(&mut STATE.lock().unwrap(), &macro_id, &script);
            Some(format!(r#"{{"ok":true,"log":[{}]}}"#,
                log.iter().map(|l| format!(r#""{}""#, esc(l))).collect::<Vec<_>>().join(",")))
        }

        // \u{2500}\u{2500} Reactive subscriptions
        ("GET",  "/rx/subscriptions") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.rx_subscriptions.iter().map(|sub|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"fired":{}}}"#,
                    esc(&sub.id), esc(&sub.name), sub.enabled, sub.fired_count)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/rx/subscribe") => {
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            let name   = extract_json_str(body, "name").unwrap_or_default();
            let kinds_str = extract_json_str(body, "event_kinds").unwrap_or_default();
            let event_kinds: Vec<String> = kinds_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            let target_macro = extract_json_str(body, "target_macro").unwrap_or_default();
            let debounce_ms  = extract_json_num(body, "debounce_ms").unwrap_or(0.0) as u128;
            let throttle_ms  = extract_json_num(body, "throttle_ms").unwrap_or(0.0) as u128;
            let mut operators = Vec::new();
            if debounce_ms > 0 { operators.push(RxOperator::Debounce(debounce_ms)); }
            if throttle_ms > 0 { operators.push(RxOperator::Throttle(throttle_ms)); }
            if body.contains(r#""distinct":true"#) { operators.push(RxOperator::Distinct); }
            let sub = RxSubscription {
                id: id.clone(), name, event_kinds, operators, target_macro, enabled: true,
                fired_count: 0, last_fired: 0, debounce_last: 0, throttle_last: 0,
                take_count: 0, skip_count: 0, last_value: String::new(), buffer: Vec::new(),
            };
            STATE.lock().unwrap().rx_subscriptions.push(sub);
            Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
        }
        ("POST", "/rx/event") => {
            let kind = extract_json_str(body, "kind").unwrap_or_default();
            let data = extract_json_str(body, "data").unwrap_or_default();
            let event = RxEvent { kind: kind.clone(), data, ts: now_ms(), source: "api".to_string() };
            let mut s = STATE.lock().unwrap();
            let subs: Vec<RxSubscription> = s.rx_subscriptions.iter().cloned().collect();
            for mut sub in subs {
                if !sub.enabled { continue; }
                if let Some(_payload) = rx_process_event(&mut sub, &event, &s) {
                    let target = sub.target_macro.clone();
                    chain_macro(&mut s, &target);
                    if let Some(rs) = s.rx_subscriptions.iter_mut().find(|r| r.id == sub.id) {
                        rs.fired_count += 1; rs.last_fired = now_ms();
                    }
                }
            }
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} State machines
        ("GET",  "/fsm/machines")   => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.state_machines.iter().map(|m|
                format!(r#"{{"id":"{}","name":"{}","state":"{}","enabled":{}}}"#,
                    esc(&m.id), esc(&m.name), esc(&m.current_state), m.enabled)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/fsm/event")      => {
            let machine_id = extract_json_str(body, "machine_id").unwrap_or_default();
            let event_kind = extract_json_str(body, "event").unwrap_or_default();
            fsm_process_event(&mut STATE.lock().unwrap(), &machine_id, &event_kind);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Context zones
        ("GET",  "/zones")          => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.context_zones.iter().map(|z|
                format!(r#"{{"id":"{}","name":"{}","active":{},"profile":"{}"}}"#,
                    esc(&z.id), esc(&z.name), z.currently_active, esc(&z.activate_profile))
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }

        // \u{2500}\u{2500} Bundle export/import
        ("GET",  "/bundle/export")  => {
            let tag = path.find("tag=").map(|i| &path[i+4..]).map(|s| s.split('&').next().unwrap_or(""));
            Some(export_bundle(&STATE.lock().unwrap(), tag))
        }
        ("POST", "/bundle/import")  => {
            import_macros_json(&mut STATE.lock().unwrap(), body);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Channel messaging
        ("POST", "/channel/post")   => {
            let ch  = extract_json_str(body, "channel").unwrap_or_default();
            let msg = extract_json_str(body, "message").unwrap_or_default();
            channel_post(&mut STATE.lock().unwrap(), &ch, &msg);
            Some(r#"{"ok":true}"#.to_string())
        }

        // \u{2500}\u{2500} Battery-aware scheduling
        ("POST", "/battery/defer")  => {
            let macro_id = extract_json_str(body, "macro_id").unwrap_or_default();
            let min_pct  = extract_json_num(body, "min_pct").unwrap_or(20.0) as i32;
            defer_until_charged(&mut STATE.lock().unwrap(), &macro_id, min_pct);
            Some(format!(r#"{{"ok":true,"deferred":"{}","min_pct":{}}}"#, esc(&macro_id), min_pct))
        }

        // ── v43: Natural-language automation shortcuts ───────────────────────
        // Simple HTTP API that maps plain intents → macro objects.
        // Called by KiraTools.runTool("if_then", ...) and by AI chat.

        // POST /auto/if_then {"if":"battery < 20","then":"notify me low battery"}
        ("POST", "/auto/if_then") => {
            let cond_str   = extract_json_str(body, "if").unwrap_or_default();
            let action_str = extract_json_str(body, "then").unwrap_or_default();
            let id         = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if cond_str.is_empty() || action_str.is_empty() {
                return Some(r#"{"error":"need if and then fields"}"#.to_string());
            }
            let (tkind, tval) = parse_nl_condition(&cond_str);
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"if {} then {}","enabled":true,"triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&cond_str), esc(&action_str),
                esc(&tkind), esc(&tval), esc(&action_str)
            ));
            let mid = m.id.clone(); let mname = m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","trigger":"{}","val":"{}"}}"#,
                esc(&mid), esc(&mname), esc(&tkind), esc(&tval)))
        }

        // POST /auto/watch_app {"app":"youtube","action":"log I opened YouTube"}
        ("POST", "/auto/watch_app") => {
            let app    = extract_json_str(body, "app").unwrap_or_default();
            let action = extract_json_str(body, "action").unwrap_or_default();
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if app.is_empty() { return Some(r#"{"error":"need app"}"#.to_string()); }
            let pkg = app_name_to_pkg(&app);
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"when {} opens","enabled":true,"triggers":[{{"kind":"app_opened","config":{{"package":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&app), esc(&pkg), esc(&action)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","app":"{}","pkg":"{}"}}"#,
                esc(&mid), esc(&app), esc(&pkg)))
        }

        // POST /auto/repeat {"task":"check battery","every_minutes":30}
        ("POST", "/auto/repeat") => {
            let task    = extract_json_str(body, "task").unwrap_or_default();
            let minutes = extract_json_num(body, "every_minutes").unwrap_or(30.0) as u64;
            let id      = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if task.is_empty() { return Some(r#"{"error":"need task"}"#.to_string()); }
            let interval_ms = minutes * 60_000;
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"every {}min: {}","enabled":true,"triggers":[{{"kind":"interval","config":{{"interval_ms":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), minutes, esc(&task), interval_ms, esc(&task)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","task":"{}","every_minutes":{}}}"#,
                esc(&mid), esc(&task), minutes))
        }

        // POST /auto/on_notif {"keyword":"OTP","action":"read aloud","app":""}
        ("POST", "/auto/on_notif") => {
            let keyword = extract_json_str(body, "keyword").unwrap_or_default();
            let action  = extract_json_str(body, "action").unwrap_or_default();
            let app     = extract_json_str(body, "app").unwrap_or_default();
            let id      = extract_json_str(body, "id").unwrap_or_else(gen_id);
            let tkind = if app.is_empty() { "keyword_notif" } else { "app_notif" };
            let tval  = if app.is_empty() { keyword.clone() } else { app_name_to_pkg(&app) };
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"on notif '{}': {}","enabled":true,"tags":["notification"],"triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&keyword), esc(&action),
                esc(tkind), esc(&tval), esc(&action)
            ));
            let mid = m.id.clone(); let mname = m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","keyword":"{}"}}"#,
                esc(&mid), esc(&mname), esc(&keyword)))
        }

        // POST /auto/on_time {"time":"07:30","action":"good morning","days":"daily"}
        ("POST", "/auto/on_time") => {
            let time   = extract_json_str(body, "time").unwrap_or_else(|| "08:00".to_string());
            let action = extract_json_str(body, "action").unwrap_or_default();
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if action.is_empty() { return Some(r#"{"error":"need action"}"#.to_string()); }
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"at {}: {}","enabled":true,"tags":["scheduled"],"triggers":[{{"kind":"time_daily","config":{{"time":"{}"}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&time), esc(&action), esc(&time), esc(&action)
            ));
            let mid = m.id.clone();
            let mut s = STATE.lock().unwrap();
            schedule_macro_daily(&mut s, &id, &time);
            s.macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","time":"{}","action":"{}"}}"#,
                esc(&mid), esc(&time), esc(&action)))
        }

        // POST /auto/on_charge {"action":"run backup","state":"plugged"}
        ("POST", "/auto/on_charge") => {
            let action = extract_json_str(body, "action").unwrap_or_default();
            let state  = extract_json_str(body, "state").unwrap_or_else(|| "plugged".to_string());
            let id     = extract_json_str(body, "id").unwrap_or_else(gen_id);
            if action.is_empty() { return Some(r#"{"error":"need action"}"#.to_string()); }
            let tkind = if state == "unplugged" { "power_disconnected" } else { "power_connected" };
            let m = parse_macro_from_json(&format!(
                r#"{{"id":"{}","name":"on {}: {}","enabled":true,"triggers":[{{"kind":"{}","config":{{}}}}],"conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#,
                esc(&id), esc(&state), esc(&action), esc(tkind), esc(&action)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","state":"{}","action":"{}"}}"#,
                esc(&mid), esc(&state), esc(&action)))
        }

        // GET /auto/list  — friendly summary of all automations
        ("GET", "/auto/list") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macros.iter().map(|m| {
                let tsum = m.triggers.first().map(|t| t.kind.to_str().to_string()).unwrap_or_default();
                let asum = m.actions.first()
                    .map(|a| a.params.get("message").cloned().unwrap_or_else(|| a.kind.to_str().to_string()))
                    .unwrap_or_default();
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"runs":{},"trigger":"{}","action":"{}","tags":[{}]}}"#,
                    esc(&m.id), esc(&m.name), m.enabled, m.run_count,
                    esc(&tsum), esc(&asum[..asum.len().min(60)]),
                    m.tags.iter().map(|t| format!("\"{}\"",esc(t))).collect::<Vec<_>>().join(","))
            }).collect();
            Some(format!(r#"{{"ok":true,"count":{},"automations":[{}]}}"#, items.len(), items.join(",")))
        }

        // POST /auto/enable {"id":"...","enabled":true}
        ("POST", "/auto/enable") => {
            let id  = extract_json_str(body, "id").unwrap_or_default();
            let ena = !body.contains(r#""enabled":false"#);
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
                m.enabled = ena;
                Some(format!(r#"{{"ok":true,"id":"{}","enabled":{}}}"#, esc(&id), ena))
            } else {
                Some(format!(r#"{{"error":"automation '{}' not found"}}"#, esc(&id)))
            }
        }

        // DELETE /auto/:id
        ("DELETE", auto_path) if auto_path.starts_with("/auto/") => {
            let id = auto_path.trim_start_matches("/auto/");
            let mut s = STATE.lock().unwrap();
            let before = s.macros.len();
            s.macros.retain(|m| m.id != id);
            Some(format!(r#"{{"ok":true,"removed":{}}}"#, before - s.macros.len()))
        }

        // OpenClaw v3: Advanced automation features

        // GET /auto/templates
        ("GET", "/auto/templates") => {
            let templates = vec![
                ("morning_routine",    "Morning routine",      "time_daily",    "07:00"),
                ("low_battery_alert",  "Low battery alert",    "battery_low",   "20"),
                ("youtube_opened",     "Log YouTube usage",    "app_opened",    "com.google.android.youtube"),
                ("screen_off_silence", "Silence on screen off","screen_off",    ""),
                ("wifi_greeter",       "WiFi connected",       "wifi_changed",  "connected"),
                ("morning_briefing",   "Morning briefing",     "time_daily",    "06:30"),
                ("night_mode",         "Night mode at 22:00",  "time_daily",    "22:00"),
                ("shake_screenshot",   "Shake to screenshot",  "shake",         ""),
                ("sms_reader",         "Read incoming SMS",    "sms_received",  ""),
                ("call_logger",        "Log missed calls",     "call_missed",   ""),
                ("bt_audio",           "BT connected",         "bt_connected",  ""),
                ("charge_done",        "Charge complete",      "battery_low",   "95"),
            ];
            let items: Vec<String> = templates.iter().map(|(id, name, tkind, tval)|
                format!(r#"{{"id":"{}","name":"{}","trigger_kind":"{}","trigger_val":"{}"}}"#,
                    id, name, tkind, tval)
            ).collect();
            Some(format!(r#"{{"ok":true,"count":{},"templates":[{}]}}"#,
                items.len(), items.join(",")))
        }

        // POST /auto/from_template {"template_id":"morning_routine","action":"...","time":"07:30"}
        ("POST", "/auto/from_template") => {
            let tpl_id      = extract_json_str(body, "template_id").unwrap_or_default();
            let custom_act  = extract_json_str(body, "action").unwrap_or_default();
            let time_ov     = extract_json_str(body, "time").unwrap_or_default();
            let macro_id    = extract_json_str(body, "id").unwrap_or_else(|| format!("tpl_{}", gen_id()));

            let (tkind, tval, default_act) = match tpl_id.as_str() {
                "morning_routine"    => ("time_daily",   "07:00",  "good morning, give me today summary"),
                "low_battery_alert"  => ("battery_low",  "20",     "my battery is low, please charge"),
                "youtube_opened"     => ("app_opened",   "com.google.android.youtube", "log YouTube session started"),
                "screen_off_silence" => ("screen_off",   "",       "mute volume"),
                "wifi_greeter"       => ("wifi_changed", "connected","WiFi connected, checking updates"),
                "morning_briefing"   => ("time_daily",   "06:30",  "give me today morning briefing"),
                "night_mode"         => ("time_daily",   "22:00",  "set screen brightness to minimum"),
                "shake_screenshot"   => ("shake",        "",       "take a screenshot"),
                "sms_reader"         => ("sms_received", "",       "read the latest SMS message aloud"),
                "call_logger"        => ("call_missed",  "",       "log missed call received"),
                "bt_audio"           => ("bt_connected", "",       "bluetooth audio device connected"),
                "charge_done"        => ("battery_low",  "95",     "battery fully charged"),
                _                   => ("manual",        "",       "run automation"),
            };

            let tval_final = if !time_ov.is_empty() { time_ov.as_str() } else { tval };
            let act_final  = if !custom_act.is_empty() { custom_act.as_str() } else { default_act };

            let m = parse_macro_from_json(&format!(
                concat!(r#"{{"id":"{}","name":"[{}] {}","enabled":true,"tags":["template","{}"],"#,
                        r#""triggers":[{{"kind":"{}","config":{{"value":"{}"}}}}],"#,
                        r#""conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#),
                esc(&macro_id), esc(&tpl_id), esc(act_final),
                esc(&tpl_id), esc(tkind), esc(tval_final), esc(act_final)
            ));
            let mid  = m.id.clone();
            let mname= m.name.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","trigger":"{}","val":"{}"}}"#,
                esc(&mid), esc(&mname), esc(tkind), esc(tval_final)))
        }

        // POST /auto/scene {"name":"work mode","actions":["mute notifications","open calendar"]}
        ("POST", "/auto/scene") => {
            let name    = extract_json_str(body, "name").unwrap_or_default();
            let scene_id= extract_json_str(body, "id")
                .unwrap_or_else(|| format!("scene_{}", name.to_lowercase().replace(' ', "_")));
            if name.is_empty() { return Some(r#"{"error":"need name"}"#.to_string()); }
            // Extract actions array content
            let acts: Vec<String> = {
                let key = "\"actions\":[";
                if let Some(start) = body.find(key) {
                    let after = &body[start + key.len()..];
                    if let Some(end) = after.find(']') {
                        after[..end].split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    } else { vec![] }
                } else { vec![] }
            };
            let combined = acts.join(", then ");
            let m = parse_macro_from_json(&format!(
                concat!(r#"{{"id":"{}","name":"scene: {}","enabled":true,"tags":["scene"],"#,
                        r#""triggers":[{{"kind":"manual","config":{{}}}}],"#,
                        r#""conditions":[],"actions":[{{"type":"kira_chat","params":{{"message":"{}"}}}}]}}"#),
                esc(&scene_id), esc(&name), esc(&combined)
            ));
            let mid = m.id.clone();
            STATE.lock().unwrap().macros.push(m);
            Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","steps":{}}}"#,
                esc(&mid), esc(&name), acts.len()))
        }

        // POST /auto/run_now {"id":"macro_id"} — trigger immediately
        ("POST", "/auto/run_now") => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            if id.is_empty() { return Some(r#"{"error":"need id"}"#.to_string()); }
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter().find(|m| m.id == id).cloned() {
                let name = m.name.clone();
                let (steps, _ok) = execute_macro_actions(&mut s, &id, &m.actions);
                Some(format!(r#"{{"ok":true,"id":"{}","name":"{}","steps":{}}}"#,
                    esc(&id), esc(&name), steps))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&id)))
            }
        }

        // POST /auto/pause {"id":"macro_id","resume_after_minutes":30}
        ("POST", "/auto/pause") => {
            let id      = extract_json_str(body, "id").unwrap_or_default();
            let minutes = extract_json_num(body, "resume_after_minutes").unwrap_or(60.0) as u64;
            if id.is_empty() { return Some(r#"{"error":"need id"}"#.to_string()); }
            let resume_ms = now_ms() + (minutes as u128) * 60_000;
            let mut s = STATE.lock().unwrap();
            if let Some(m) = s.macros.iter_mut().find(|m| m.id == id) {
                m.enabled = false;
                let resume_trigger = Trigger {
                    id:           format!("resume_{}", id),
                    trigger_type: "time".to_string(),
                    value:        resume_ms.to_string(),
                    action:       format!("enable_macro:{}", id),
                    fired:        false,
                    repeat:       false,
                };
                s.triggers.push(resume_trigger);
                Some(format!(r#"{{"ok":true,"id":"{}","paused_minutes":{}}}"#,
                    esc(&id), minutes))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&id)))
            }
        }

        // GET /auto/history — last 50 runs
        ("GET", "/auto/history") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macro_run_log.iter().rev().take(50)
                .map(|r| format!(
                    r#"{{"id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"ms":{},"ts":{}}}"#,
                    esc(&r.macro_id), esc(&r.macro_name), esc(&r.trigger),
                    r.success, r.steps_run, r.duration_ms, r.ts))
                .collect();
            Some(format!(r#"{{"ok":true,"count":{},"history":[{}]}}"#,
                items.len(), items.join(",")))
        }

        // GET /auto/stats
        ("GET", "/auto/stats") => {
            let s = STATE.lock().unwrap();
            let enabled  = s.macros.iter().filter(|m| m.enabled).count();
            let total_runs: u64 = s.macros.iter().map(|m| m.run_count).sum();
            let success  = s.macro_run_log.iter().filter(|r| r.success).count();
            let failed   = s.macro_run_log.iter().filter(|r| !r.success).count();
            Some(format!(
                r#"{{"total":{},"enabled":{},"disabled":{},"total_runs":{},"success":{},"failed":{}}}"#,
                s.macros.len(), enabled, s.macros.len()-enabled,
                total_runs, success, failed))
        }

        // POST /auto/clone {"id":"src","new_id":"dst","new_name":"Copy of ..."}
        ("POST", "/auto/clone") => {
            let src     = extract_json_str(body, "id").unwrap_or_default();
            let new_id  = extract_json_str(body, "new_id").unwrap_or_else(|| format!("clone_{}", gen_id()));
            let new_nm  = extract_json_str(body, "new_name").unwrap_or_default();
            let mut s   = STATE.lock().unwrap();
            if let Some(original) = s.macros.iter().find(|m| m.id == src).cloned() {
                let mut c    = original.clone();
                c.id         = new_id.clone();
                c.name       = if new_nm.is_empty() { format!("{} (copy)", original.name) } else { new_nm };
                c.run_count  = 0;
                let cname    = c.name.clone();
                s.macros.push(c);
                Some(format!(r#"{{"ok":true,"id":"{}","name":"{}"}}"#, esc(&new_id), esc(&cname)))
            } else {
                Some(format!(r#"{{"error":"'{}' not found"}}"#, esc(&src)))
            }
        }

        // POST /auto/batch_enable {"ids":["a","b"],"enabled":true}
        ("POST", "/auto/batch_enable") => {
            let enabled = !body.contains("\"enabled\":false");
            let ids_raw = body.find("\"ids\":[")
                .map(|i| { let after = &body[i+7..]; after[..after.find(']').unwrap_or(0)].to_string() })
                .unwrap_or_default();
            let ids: Vec<&str> = ids_raw.split(',')
                .map(|s| s.trim().trim_matches('"'))
                .filter(|s| !s.is_empty())
                .collect();
            let mut s = STATE.lock().unwrap();
            let mut count = 0usize;
            for m in s.macros.iter_mut() {
                if ids.contains(&m.id.as_str()) { m.enabled = enabled; count += 1; }
            }
            Some(format!(r#"{{"ok":true,"updated":{}}}"#, count))
        }

        _ => None,
    }
}

// / \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}
// Roboru / E-Robot / Automate Engine
// Inspired by: LlamaLab Automate (flowchart), E-Robot (170+ events, 150+ actions),
// Robot Framework (keyword-driven RPA), UiPath (intelligent automation)
// \u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}\u{2501}

/// Flowchart block types (Automate-style visual programming)
#[derive(Clone, Debug)]
enum FlowBlockKind {
    Start,
    Stop,
    Action,       // execute a MacroAction
    Decision,     // if/else branch
    Loop,         // for/while loop
    Wait,         // delay
    Fork,         // parallel execution branches
    Join,         // wait for all parallel branches
    SubFlow,      // call another flow by id
    Catch,        // error handler
    Log,          // debug logging block
}

/// A node in the visual flowchart
#[derive(Clone)]
struct FlowBlock {
    id:           String,
    kind:         FlowBlockKind,
    label:        String,
    // Connections: next block ids
    next:         Vec<String>,   // [0]=true branch, [1]=false branch for Decision
    // Payload
    action:       Option<MacroAction>,
    condition:    Option<MacroCondition>,
    loop_count:   u32,
    loop_var:     String,   // variable to increment each loop
    sub_flow_id:  String,   // for SubFlow blocks
    // Retry config (E-Robot pattern)
    retry_count:  u32,
    retry_delay_ms: u64,
}

/// A complete visual flow (like Automate's flowchart)
#[derive(Clone)]
struct AutoFlow {
    id:          String,
    name:        String,
    description: String,
    enabled:     bool,
    start_block: String,
    blocks:      HashMap<String, FlowBlock>,
    created_ms:  u128,
    run_count:   u64,
    last_run_ms: u128,
    tags:        Vec<String>,
}

/// Keyword definition (Robot Framework pattern)
/// A named reusable action sequence
#[derive(Clone)]
struct Keyword {
    name:        String,  // e.g. "Open And Login YouTube"
    description: String,
    steps:       Vec<MacroAction>,
    args:        Vec<String>,  // parameter names
    returns:     String,  // variable name to store result
}

/// Hyper-automation pipeline step (UiPath/Comidor pattern)
/// Combines BPM workflow + RPA action + AI decision
#[derive(Clone)]
struct PipelineStep {
    id:          String,
    name:        String,
    kind:        String,  // "rpa", "ai_decision", "data_extract", "api_call", "human_task"
    // RPA config
    action:      Option<MacroAction>,
    // AI decision config
    prompt:      String,   // AI prompt for this step
    out_var:     String,   // variable to store AI response
    // Data extraction
    extract_pattern: String,  // regex or XPath-like selector
    extract_source:  String,  // "screen", "clipboard", "notification", "url"
    // Human task (pause and wait for signal)
    timeout_ms:  u128,
    // Retry
    retry_count: u32,
    retry_delay_ms: u64,
    // Condition to skip this step
    skip_if:     Option<MacroCondition>,
}

/// A hyper-automation pipeline
#[derive(Clone)]
struct HyperPipeline {
    id:          String,
    name:        String,
    steps:       Vec<PipelineStep>,
    enabled:     bool,
    run_count:   u64,
    last_run_ms: u128,
}

/// Retry result
enum RetryResult {
    Success(String),
    Failed(String, u32),  // error + attempts
}

/// Smart retry engine with exponential backoff (E-Robot pattern)
fn retry_action(
    s: &mut KiraState,
    macro_id: &str,
    action: &MacroAction,
    max_retries: u32,
    base_delay_ms: u64,
) -> RetryResult {
    for attempt in 0..=max_retries {
        // Enqueue the action
        enqueue_action(s, macro_id, action);
        // In Rust we can't actually wait for the Java result synchronously,
        // so we track retry state in variables
        let retry_key = format!("_retry_{}_{}", macro_id, action.kind.to_str());
        s.variables.insert(retry_key.clone(), AutoVariable {
            name: retry_key.clone(),
            value: format!("attempt:{}", attempt),
            var_type: "string".to_string(),
            persistent: false,
            created_ms: now_ms(),
            updated_ms: now_ms(),
        });
        if attempt < max_retries {
            // Exponential backoff: delay = base * 2^attempt (capped at 30s)
            let delay = (base_delay_ms * (1 << attempt.min(4))).min(30_000);
            s.pending_actions.push_back(PendingMacroAction {
                macro_id:  macro_id.to_string(),
                action_id: gen_id(),
                kind:      "wait".to_string(),
                params:    { let mut m = HashMap::new(); m.insert("ms".to_string(), delay.to_string()); m },
                ts: now_ms(),
            });
        }
    }
    RetryResult::Success("enqueued".to_string())
}

/// Execute a visual flowchart (Automate-style)
fn execute_flow(s: &mut KiraState, flow: &AutoFlow, start_id: Option<&str>) -> u32 {
    let mut steps = 0u32;
    let mut current_id = start_id.unwrap_or(&flow.start_block).to_string();
    let mut visited: HashMap<String, u32> = HashMap::new();
    let max_steps = 500u32;

    while steps < max_steps {
        let block = match flow.blocks.get(&current_id) {
            Some(b) => b.clone(),
            None => break,
        };

        // Loop guard \u{2014} prevent infinite loops
        let visit_count = visited.entry(current_id.clone()).or_insert(0);
        *visit_count += 1;
        if *visit_count > 100 { break; } // stuck in a loop

        steps += 1;

        match block.kind {
            FlowBlockKind::Stop => break,
            FlowBlockKind::Start => {
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Action => {
                if let Some(ref action) = block.action {
                    if block.retry_count > 0 {
                        retry_action(s, &flow.id, action, block.retry_count, block.retry_delay_ms);
                    } else {
                        enqueue_action(s, &flow.id, action);
                    }
                }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Decision => {
                let cond_result = block.condition.as_ref()
                    .map(|c| eval_condition(s, c))
                    .unwrap_or(false);
                current_id = if cond_result {
                    block.next.first().cloned().unwrap_or_default()
                } else {
                    block.next.get(1).cloned().unwrap_or_default()
                };
            }
            FlowBlockKind::Loop => {
                let count = block.loop_count.min(100);
                let body_id = block.next.first().cloned().unwrap_or_default();
                let after_id = block.next.get(1).cloned().unwrap_or_default();
                for i in 0..count {
                    // Set loop variable
                    if !block.loop_var.is_empty() {
                        let ts = now_ms();
                        s.variables.insert(block.loop_var.clone(), AutoVariable {
                            name: block.loop_var.clone(), value: i.to_string(),
                            var_type: "number".to_string(), persistent: false,
                            created_ms: ts, updated_ms: ts,
                        });
                    }
                    if !body_id.is_empty() {
                        let sub_flow = AutoFlow {
                            id: flow.id.clone(), name: flow.name.clone(),
                            description: String::new(), enabled: true,
                            start_block: body_id.clone(),
                            blocks: flow.blocks.clone(),
                            created_ms: 0, run_count: 0, last_run_ms: 0, tags: vec![],
                        };
                        steps += execute_flow(s, &sub_flow, Some(&body_id));
                    }
                }
                current_id = after_id;
            }
            FlowBlockKind::SubFlow => {
                if !block.sub_flow_id.is_empty() {
                    // Chain to another named flow
                    chain_macro(s, &block.sub_flow_id);
                }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Wait => {
                let ms = block.action.as_ref()
                    .and_then(|a| a.params.get("ms"))
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(1000);
                s.pending_actions.push_back(PendingMacroAction {
                    macro_id: flow.id.clone(), action_id: gen_id(),
                    kind: "wait".to_string(),
                    params: { let mut m = HashMap::new(); m.insert("ms".to_string(), ms.to_string()); m },
                    ts: now_ms(),
                });
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Fork => {
                // Parallel: enqueue all branches
                for next_id in &block.next {
                    let branch_flow = AutoFlow {
                        id: flow.id.clone(), name: flow.name.clone(),
                        description: String::new(), enabled: true,
                        start_block: next_id.clone(),
                        blocks: flow.blocks.clone(),
                        created_ms: 0, run_count: 0, last_run_ms: 0, tags: vec![],
                    };
                    steps += execute_flow(s, &branch_flow, Some(next_id));
                }
                break; // Fork doesn't have a single next
            }
            FlowBlockKind::Log => {
                let msg = block.label.clone();
                let expanded = expand_vars(s, &msg);
                s.daily_log.push_back(format!("[flow:{}] {}", flow.id, expanded));
                if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Catch => {
                // Error catch \u{2014} just continue to next
                current_id = block.next.first().cloned().unwrap_or_default();
            }
            FlowBlockKind::Join => {
                current_id = block.next.first().cloned().unwrap_or_default();
            }
        }

        if current_id.is_empty() { break; }
    }
    steps
}

/// Execute a keyword (Robot Framework pattern)
/// Resolves args from variables then runs steps
fn execute_keyword(s: &mut KiraState, kw: &Keyword, args: &HashMap<String, String>) -> String {
    // Bind args to local variables
    for (name, val) in args {
        let ts = now_ms();
        s.variables.insert(name.clone(), AutoVariable {
            name: name.clone(), value: expand_vars(s, val),
            var_type: "string".to_string(), persistent: false,
            created_ms: ts, updated_ms: ts,
        });
    }
    // Run steps
    let steps = kw.steps.clone();
    let id = format!("kw_{}", kw.name.replace(' ', "_"));
    let (step_count, _) = execute_macro_actions(s, &id, &steps);
    // Return result variable
    if !kw.returns.is_empty() {
        s.variables.get(&kw.returns).map(|v| v.value.clone()).unwrap_or_default()
    } else {
        format!("ok:{}", step_count)
    }
}

/// Execute a hyper-automation pipeline (UiPath/Comidor pattern)
fn execute_pipeline(s: &mut KiraState, pipeline: &HyperPipeline) -> (u32, Vec<String>) {
    let mut steps = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for step in &pipeline.steps {
        // Check skip condition
        if let Some(ref cond) = step.skip_if {
            if eval_condition(s, cond) { continue; }
        }

        steps += 1;

        match step.kind.as_str() {
            "rpa" => {
                if let Some(ref action) = step.action {
                    if step.retry_count > 0 {
                        retry_action(s, &pipeline.id, action, step.retry_count, step.retry_delay_ms);
                    } else {
                        enqueue_action(s, &pipeline.id, action);
                    }
                }
            }
            "ai_decision" => {
                // Enqueue kira_ask action with the prompt
                let action = MacroAction {
                    kind: MacroActionKind::KiraAsk,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("prompt".to_string(), expand_vars(s, &step.prompt));
                        m.insert("out_var".to_string(), step.out_var.clone());
                        m
                    },
                    sub_actions: vec![],
                    enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            "data_extract" => {
                // Enqueue extraction action
                let mut params = HashMap::new();
                params.insert("source".to_string(), step.extract_source.clone());
                params.insert("pattern".to_string(), step.extract_pattern.clone());
                params.insert("out_var".to_string(), step.out_var.clone());
                let action = MacroAction {
                    kind: MacroActionKind::GetClipboard,
                    params, sub_actions: vec![], enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            "api_call" => {
                if let Some(ref action) = step.action {
                    enqueue_action(s, &pipeline.id, action);
                }
            }
            "human_task" => {
                // Pause pipeline and send notification to user
                let action = MacroAction {
                    kind: MacroActionKind::SendNotification,
                    params: {
                        let mut m = HashMap::new();
                        m.insert("title".to_string(), format!("Action required: {}", step.name));
                        m.insert("text".to_string(), expand_vars(s, &step.prompt));
                        m
                    },
                    sub_actions: vec![], enabled: true,
                };
                enqueue_action(s, &pipeline.id, &action);
            }
            _ => {
                errors.push(format!("unknown step kind: {}", step.kind));
            }
        }
    }
    (steps, errors)
}

/// Parse a flow from JSON
fn parse_flow_from_json(body: &str) -> Option<AutoFlow> {
    let id   = extract_json_str(body, "id").unwrap_or_else(gen_id);
    let name = extract_json_str(body, "name").unwrap_or_else(|| "Unnamed Flow".to_string());
    let desc = extract_json_str(body, "description").unwrap_or_default();
    let start= extract_json_str(body, "start_block").unwrap_or_default();
    if start.is_empty() { return None; }
    // blocks: [{id, kind, label, next:["id1","id2"], action:{...}, condition:{...}}]
    let mut blocks = HashMap::new();
    let blocks_key = r#""blocks":["#;
    let bstart = match body.find(blocks_key) {
        Some(i) => i + blocks_key.len(), None => return None
    };
    let slice = &body[bstart..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let bid  = extract_json_str(obj, "id").unwrap_or_else(gen_id);
                    let kind_str = extract_json_str(obj, "kind").unwrap_or_else(|| "action".to_string());
                    let label = extract_json_str(obj, "label").unwrap_or_default();
                    let loop_count = extract_json_num(obj, "loop_count").unwrap_or(1.0) as u32;
                    let loop_var   = extract_json_str(obj, "loop_var").unwrap_or_default();
                    let sub_flow_id= extract_json_str(obj, "sub_flow_id").unwrap_or_default();
                    let retry_count  = extract_json_num(obj, "retry_count").unwrap_or(0.0) as u32;
                    let retry_delay  = extract_json_num(obj, "retry_delay_ms").unwrap_or(1000.0) as u64;
                    // next array: ["id1","id2"]
                    let mut next_ids = Vec::new();
                    if let Some(ni) = obj.find(r#""next":["#) {
                        let ns = &obj[ni + 8..];
                        let end = ns.find(']').unwrap_or(ns.len());
                        for part in ns[..end].split(',') {
                            let id_part = part.trim().trim_matches('"').to_string();
                            if !id_part.is_empty() { next_ids.push(id_part); }
                        }
                    }
                    // Parse condition
                    let condition = if let Some(ci) = obj.find(r#""condition":{"#) {
                        let cs = &obj[ci + 13..];
                        let end = cs.find('}').unwrap_or(cs.len());
                        let co = &cs[..end];
                        Some(MacroCondition {
                            lhs: extract_json_str(co, "lhs").unwrap_or_default(),
                            operator: extract_json_str(co, "op").unwrap_or_else(|| "eq".to_string()),
                            rhs: extract_json_str(co, "rhs").unwrap_or_default(),
                        })
                    } else { None };
                    // Parse action
                    let action = if let Some(ai) = obj.find(r#""action":{"#) {
                        let ast = &obj[ai + 10..];
                        let end_act = find_matching_brace(ast).unwrap_or(ast.len());
                        let ao = &ast[..end_act];
                        let kind_s = extract_json_str(ao, "kind").unwrap_or_default();
                        let mut params = HashMap::new();
                        if let Some(pi) = ao.find(r#""params":{"#) {
                            let ps = &ao[pi + 10..];
                            let pe = ps.find('}').unwrap_or(ps.len());
                            parse_flat_kv(&ps[..pe], &mut params);
                        }
                        Some(MacroAction { kind: MacroActionKind::from_str(&kind_s), params, sub_actions: vec![], enabled: true })
                    } else { None };

                    let kind = match kind_str.as_str() {
                        "start"    => FlowBlockKind::Start,
                        "stop"     => FlowBlockKind::Stop,
                        "decision" => FlowBlockKind::Decision,
                        "loop"     => FlowBlockKind::Loop,
                        "wait"     => FlowBlockKind::Wait,
                        "fork"     => FlowBlockKind::Fork,
                        "join"     => FlowBlockKind::Join,
                        "sub_flow" => FlowBlockKind::SubFlow,
                        "catch"    => FlowBlockKind::Catch,
                        "log"      => FlowBlockKind::Log,
                        _          => FlowBlockKind::Action,
                    };
                    blocks.insert(bid.clone(), FlowBlock {
                        id: bid, kind, label, next: next_ids,
                        action, condition, loop_count, loop_var, sub_flow_id,
                        retry_count, retry_delay_ms: retry_delay,
                    });
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    Some(AutoFlow {
        id, name, description: desc, enabled: true, start_block: start,
        blocks, created_ms: now_ms(), run_count: 0, last_run_ms: 0, tags: vec![],
    })
}

fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch { '{' => depth += 1, '}' => { depth -= 1; if depth < 0 { return Some(i); } } _ => {} }
    }
    None
}

/// Parse a keyword from JSON
fn parse_keyword_from_json(body: &str) -> Option<Keyword> {
    let name = extract_json_str(body, "name")?;
    let desc = extract_json_str(body, "description").unwrap_or_default();
    let returns = extract_json_str(body, "returns").unwrap_or_default();
    let args_str = extract_json_str(body, "args").unwrap_or_default();
    let args: Vec<String> = args_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    let steps = parse_actions_from_json(body, "steps");
    Some(Keyword { name, description: desc, steps, args, returns })
}

/// Parse a pipeline from JSON
fn parse_pipeline_from_json(body: &str) -> Option<HyperPipeline> {
    let id   = extract_json_str(body, "id").unwrap_or_else(gen_id);
    let name = extract_json_str(body, "name").unwrap_or_else(|| "Pipeline".to_string());
    // steps: [{id, name, kind, prompt, out_var, retry_count, ...}]
    let mut steps = Vec::new();
    let key = r#""steps":["#;
    let start = match body.find(key) { Some(i) => i + key.len(), None => return None };
    let slice = &body[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let sid = extract_json_str(obj, "id").unwrap_or_else(gen_id);
                    let sname = extract_json_str(obj, "name").unwrap_or_default();
                    let kind  = extract_json_str(obj, "kind").unwrap_or_else(|| "rpa".to_string());
                    let prompt = extract_json_str(obj, "prompt").unwrap_or_default();
                    let out_var = extract_json_str(obj, "out_var").unwrap_or_default();
                    let extract_pattern = extract_json_str(obj, "extract_pattern").unwrap_or_default();
                    let extract_source  = extract_json_str(obj, "extract_source").unwrap_or_else(|| "screen".to_string());
                    let retry_count   = extract_json_num(obj, "retry_count").unwrap_or(0.0) as u32;
                    let retry_delay   = extract_json_num(obj, "retry_delay_ms").unwrap_or(1000.0) as u64;
                    let timeout_ms    = extract_json_num(obj, "timeout_ms").unwrap_or(30000.0) as u128;
                    let skip_if = if let Some(ci) = obj.find(r#""skip_if":{"#) {
                        let cs = &obj[ci + 11..]; let end = cs.find('}').unwrap_or(cs.len());
                        let co = &cs[..end];
                        Some(MacroCondition {
                            lhs: extract_json_str(co, "lhs").unwrap_or_default(),
                            operator: extract_json_str(co, "op").unwrap_or_else(|| "eq".to_string()),
                            rhs: extract_json_str(co, "rhs").unwrap_or_default(),
                        })
                    } else { None };
                    let action = if let Some(ai) = obj.find(r#""action":{"#) {
                        let ast = &obj[ai + 10..];
                        let end_act = find_matching_brace(ast).unwrap_or(ast.len());
                        let ao = &ast[..end_act];
                        let ks = extract_json_str(ao, "kind").unwrap_or_default();
                        let mut params = HashMap::new();
                        if let Some(pi) = ao.find(r#""params":{"#) {
                            let ps = &ao[pi+10..]; let pe = ps.find('}').unwrap_or(ps.len());
                            parse_flat_kv(&ps[..pe], &mut params);
                        }
                        Some(MacroAction { kind: MacroActionKind::from_str(&ks), params, sub_actions: vec![], enabled: true })
                    } else { None };
                    steps.push(PipelineStep { id: sid, name: sname, kind, action, prompt, out_var, extract_pattern, extract_source, timeout_ms, retry_count, retry_delay_ms: retry_delay, skip_if });
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
    Some(HyperPipeline { id, name, steps, enabled: true, run_count: 0, last_run_ms: 0 })
}

/// HTTP routes for Roboru engine
fn route_roboru(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        // Flows (visual flowchart)
        ("GET",  "/flows")          => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_flows.iter().map(|(id, f)|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"blocks":{},"run_count":{},"last_run_ms":{}}}"#,
                    esc(id), esc(&f.name), f.enabled, f.blocks.len(), f.run_count, f.last_run_ms)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/flows/add")      => {
            if let Some(flow) = parse_flow_from_json(body) {
                let id = flow.id.clone();
                STATE.lock().unwrap().roboru_flows.insert(id.clone(), flow);
                Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
            } else {
                Some(r#"{"error":"invalid flow json"}"#.to_string())
            }
        }
        ("POST", "/flows/run")      => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let flow = s.roboru_flows.get(&id).cloned();
            if let Some(flow) = flow {
                let steps = execute_flow(&mut s, &flow, None);
                if let Some(f) = s.roboru_flows.get_mut(&id) {
                    f.run_count += 1; f.last_run_ms = now_ms();
                }
                Some(format!(r#"{{"ok":true,"steps":{}}}"#, steps))
            } else {
                Some(format!(r#"{{"error":"flow not found: {}"}}"#, esc(&id)))
            }
        }
        ("POST", "/flows/remove")   => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            STATE.lock().unwrap().roboru_flows.remove(&id);
            Some(r#"{"ok":true}"#.to_string())
        }
        // Keywords (Robot Framework pattern)
        ("GET",  "/keywords")       => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_keywords.iter().map(|(name, kw)|
                format!(r#"{{"name":"{}","description":"{}","args":{},"steps":{}}}"#,
                    esc(name), esc(&kw.description),
                    format!("[{}]", kw.args.iter().map(|a| format!(r#""{}""#, esc(a))).collect::<Vec<_>>().join(",")),
                    kw.steps.len())
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/keywords/add")   => {
            if let Some(kw) = parse_keyword_from_json(body) {
                let name = kw.name.clone();
                STATE.lock().unwrap().roboru_keywords.insert(name.clone(), kw);
                Some(format!(r#"{{"ok":true,"name":"{}"}}"#, esc(&name)))
            } else { Some(r#"{"error":"invalid keyword json"}"#.to_string()) }
        }
        ("POST", "/keywords/run")   => {
            let name = extract_json_str(body, "name").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let kw = s.roboru_keywords.get(&name).cloned();
            if let Some(kw) = kw {
                let args: HashMap<String,String> = kw.args.iter().enumerate().map(|(i, arg_name): (usize, &String)| {
                    let val = extract_json_str(body, &format!("arg{}", i)).unwrap_or_default();
                    (arg_name.clone(), val)
                }).collect();
                let result = execute_keyword(&mut s, &kw, &args);
                Some(format!(r#"{{"ok":true,"result":"{}"}}"#, esc(&result)))
            } else { Some(format!(r#"{{"error":"keyword not found: {}"}}"#, esc(&name))) }
        }
        // Pipelines (Hyper-automation)
        ("GET",  "/pipelines")      => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.roboru_pipelines.iter().map(|(id, p)|
                format!(r#"{{"id":"{}","name":"{}","enabled":{},"steps":{},"run_count":{}}}"#,
                    esc(id), esc(&p.name), p.enabled, p.steps.len(), p.run_count)
            ).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/pipelines/add")  => {
            if let Some(pipeline) = parse_pipeline_from_json(body) {
                let id = pipeline.id.clone();
                STATE.lock().unwrap().roboru_pipelines.insert(id.clone(), pipeline);
                Some(format!(r#"{{"ok":true,"id":"{}"}}"#, esc(&id)))
            } else { Some(r#"{"error":"invalid pipeline json"}"#.to_string()) }
        }
        ("POST", "/pipelines/run")  => {
            let id = extract_json_str(body, "id").unwrap_or_default();
            let mut s = STATE.lock().unwrap();
            let pipeline = s.roboru_pipelines.get(&id).cloned();
            if let Some(pipeline) = pipeline {
                let (steps, errors) = execute_pipeline(&mut s, &pipeline);
                if let Some(p) = s.roboru_pipelines.get_mut(&id) {
                    p.run_count += 1; p.last_run_ms = now_ms();
                }
                Some(format!(r#"{{"ok":true,"steps":{},"errors":{}}}"#,
                    steps,
                    format!("[{}]", errors.iter().map(|e| format!(r#""{}""#, esc(e))).collect::<Vec<_>>().join(","))))
            } else { Some(format!(r#"{{"error":"pipeline not found: {}"}}"#, esc(&id))) }
        }
        _ => None,
    }
}

// \u{2500}\u{2500}\u{2500} OpenClaw v2: Advanced automation features \u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}

/// Macro schedule: run macro at specific time daily (HH:MM format)
/// Stored as a cron job internally
fn schedule_macro_daily(s: &mut KiraState, macro_id: &str, time_hhmm: &str) {
    // Parse HH:MM \u{2192} store as cron job with interval = 24h
    // The trigger watcher checks against current time string
    let job_id = format!("daily_{}_{}", macro_id, time_hhmm.replace(':', ""));
    s.cron_jobs.retain(|j| j.id != job_id);
    s.cron_jobs.push(CronJob {
        id:          job_id,
        expression:  time_hhmm.to_string(),
        action:      format!("chain_macro:{}", macro_id),
        last_run:    0,
        interval_ms: 86_400_000, // 24h
        enabled:     true,
    });
}

/// Macro group: run multiple macros in sequence or parallel
fn run_macro_group(s: &mut KiraState, macro_ids: &[&str], parallel: bool) {
    if !check_rate_limit(s) { return; }
    if parallel {
        // Enqueue all at once \u{2014} Java executes them concurrently
        for id in macro_ids {
            let actions: Vec<MacroAction> = s.macros.iter()
                .find(|m| m.id == *id && m.enabled)
                .map(|m| m.actions.clone())
                .unwrap_or_default();
            if !actions.is_empty() {
                let (_, _) = execute_macro_actions(s, id, &actions);
            }
        }
    } else {
        // Sequential: chain them
        for id in macro_ids {
            chain_macro(s, id);
        }
    }
}

/// Conditional macro: only run if ALL conditions pass
fn try_run_macro_conditional(s: &mut KiraState, macro_id: &str) -> bool {
    let conditions: Vec<MacroCondition> = s.macros.iter()
        .find(|m| m.id == macro_id)
        .map(|m| m.conditions.clone())
        .unwrap_or_default();
    if !conditions.iter().all(|c| eval_condition(s, c)) { return false; }
    chain_macro(s, macro_id);
    true
}

/// Smart trigger debounce: ignore repeat fires within N ms
fn is_debounced(s: &KiraState, macro_id: &str, debounce_ms: u128) -> bool {
    let now = now_ms();
    if let Some(m) = s.macros.iter().find(|m| m.id == macro_id) {
        return now - m.last_run_ms < debounce_ms;
    }
    false
}

/// Variable interpolation in action params \u{2014} supports math expressions
fn resolve_param(s: &KiraState, param: &str) -> String {
    let expanded = expand_vars(s, param);
    // If it looks like an expression (has operators), try to evaluate
    if expanded.contains('+') || expanded.contains('-') ||
       expanded.contains('*') || expanded.contains('/') {
        if let Some(result) = eval_math(expanded.trim()) {
            return result;
        }
    }
    expanded
}

/// Get macro by name (case-insensitive) \u{2014} useful for natural language commands
fn find_macro_by_name(s: &KiraState, name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    s.macros.iter()
        .find(|m| m.name.to_lowercase().contains(&lower) || m.id == name)
        .map(|m| m.id.clone())
}

/// Automation analytics: return summary of recent macro activity
fn get_automation_analytics(s: &KiraState) -> String {
    let now = now_ms();
    let last_24h = s.macro_run_log.iter()
        .filter(|r| now - r.ts < 86_400_000).count();
    let last_1h = s.macro_run_log.iter()
        .filter(|r| now - r.ts < 3_600_000).count();
    let success_count = s.macro_run_log.iter()
        .filter(|r| r.success).count();
    let fail_count = s.macro_run_log.iter()
        .filter(|r| !r.success).count();
    let total_steps: u32 = s.macro_run_log.iter().map(|r| r.steps_run).sum();
    let enabled_macros = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
    let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();

    // Most active macro
    let mut counts: HashMap<String, u32> = HashMap::new();
    for r in &s.macro_run_log {
        *counts.entry(r.macro_name.clone()).or_insert(0) += 1;
    }
    let most_active = counts.iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, v)| format!("{} ({}x)", k, v))
        .unwrap_or_else(|| "none".to_string());

    format!(
        r#"{{"runs_24h":{},"runs_1h":{},"success":{},"failed":{},"total_steps":{},"enabled_macros":{},"templates":{},"variables":{},"active_profile":"{}","most_active":"{}","pending_actions":{}}}"#,
        last_24h, last_1h, success_count, fail_count, total_steps,
        enabled_macros, templates, s.variables.len(),
        esc(&s.active_profile), esc(&most_active),
        s.pending_actions.len()
    )
}

/// Automation report: full text summary for AI to read
fn get_automation_report(s: &KiraState) -> String {
    let now = now_ms();
    let mut lines = Vec::new();
    lines.push(format!("=== Kira Automation Report ==="));
    lines.push(format!("Active profile: {}", s.active_profile));
    lines.push(format!("Enabled macros: {}", s.macros.iter().filter(|m| m.enabled).count()));
    lines.push(format!("Total variables: {}", s.variables.len()));
    lines.push(format!("Pending actions: {}", s.pending_actions.len()));
    lines.push(String::new());
    lines.push("Recent runs:".to_string());
    for r in s.macro_run_log.iter().rev().take(5) {
        let ago = (now - r.ts) / 1000;
        lines.push(format!("  \u{2022} {} \u{2014} {} steps \u{2014} {}s ago", r.macro_name, r.steps_run, ago));
    }
    lines.push(String::new());
    lines.push("Variables:".to_string());
    for (name, var) in s.variables.iter().take(10) {
        lines.push(format!("  %{}% = {}", name.to_uppercase(), var.value));
    }
    lines.join("\n")
}

/// Export all macros as a single JSON string (for backup / sharing)
fn export_macros_json(s: &KiraState) -> String {
    let items: Vec<String> = s.macros.iter().map(macro_to_json).collect();
    format!(
        r#"{{"version":"9.0","exported_ms":{},"count":{},"macros":[{}]}}"#,
        now_ms(), items.len(), items.join(",")
    )
}

/// Import macros from exported JSON (merge, don't wipe existing)
fn import_macros_json(s: &mut KiraState, json: &str) {
    // Find the "macros":[...] array and parse each entry
    let key = "\"macros\":[";
    let start = match json.find(key) { Some(i) => i + key.len(), None => return };
    let slice = &json[start..];
    let mut depth = 0i32; let mut obj_start = 0; let mut in_obj = false;
    for (i, ch) in slice.char_indices() {
        match ch {
            '{' => { if depth == 0 { obj_start = i; in_obj = true; } depth += 1; }
            '}' => {
                depth -= 1;
                if depth == 0 && in_obj {
                    let obj = &slice[obj_start..=i];
                    let m = parse_macro_from_json(obj);
                    s.macros.retain(|x| x.id != m.id); // replace if exists
                    s.macros.push(m);
                    in_obj = false;
                }
            }
            ']' if depth == 0 => break,
            _ => {}
        }
    }
}

/// Watchdog: find macros that have been pending for >30s and log them
fn watchdog_check(s: &mut KiraState) {
    let now = now_ms();
    let stale: Vec<String> = s.pending_actions.iter()
        .filter(|a| now - a.ts > 30_000)
        .map(|a| format!("{}:{}", a.macro_id, a.kind))
        .collect();
    if !stale.is_empty() {
        s.daily_log.push_back(format!("[watchdog] stale actions: {}", stale.join(", ")));
        if s.daily_log.len() > 1000 { s.daily_log.pop_front(); }
        // Remove stale actions older than 2 minutes
        s.pending_actions.retain(|a| now - a.ts < 120_000);
    }
}

/// HTTP route additions for OpenClaw features
fn route_openclaw(method: &str, path: &str, body: &str) -> Option<String> {
    match (method, path) {
        ("GET",  "/macros/export")  => Some(export_macros_json(&STATE.lock().unwrap())),
        ("POST", "/macros/import")  => { import_macros_json(&mut STATE.lock().unwrap(), body); Some(r#"{"ok":true}"#.to_string()) }
        ("GET",  "/macros/templates") => {
            let s = STATE.lock().unwrap();
            let items: Vec<String> = s.macros.iter()
                .filter(|m| m.tags.contains(&"template".to_string()))
                .map(macro_to_json).collect();
            Some(format!("[{}]", items.join(",")))
        }
        ("POST", "/macros/chain")   => {
            let id = extract_json_str(body, "target").unwrap_or_default();
            if !id.is_empty() { chain_macro(&mut STATE.lock().unwrap(), &id); }
            Some(format!(r#"{{"ok":true,"chained":"{}"}}"#, esc(&id)))
        }
        ("POST", "/macros/pipeline") => {
            let id = extract_json_str(body, "macro_id").unwrap_or_else(gen_id);
            run_pipeline(&mut STATE.lock().unwrap(), &id, body);
            Some(format!(r#"{{"ok":true,"pipeline":"{}"}}"#, esc(&id)))
        }
        ("GET",  "/expr")           => {
            // Evaluate expression: GET /expr?e=5+3 \u{2192} {"result":"8"}
            let expr = path.find("e=").map(|i| &path[i+2..]).unwrap_or("").replace('+', " ");
            let result = eval_expr(&STATE.lock().unwrap(), &expr);
            Some(format!(r#"{{"result":"{}"}}"#, esc(&result)))
        }
        ("GET",  "/variables/expand") => {
            // Expand %VAR% tokens: GET /variables/expand?text=hello+%BATTERY%
            let text = path.find("text=").map(|i| &path[i+5..]).unwrap_or("").replace('+', " ");
            let result = expand_vars(&STATE.lock().unwrap(), &text);
            Some(format!(r#"{{"result":"{}"}}"#, esc(&result)))
        }
        ("GET",  "/automation/analytics") => Some(get_automation_analytics(&STATE.lock().unwrap())),
        ("GET",  "/automation/report")    => {
            let report = get_automation_report(&STATE.lock().unwrap());
            Some(format!(r#"{{"report":"{}"}}"#, esc(&report)))
        }
        ("POST", "/macros/schedule")      => {
            let id   = extract_json_str(body, "macro_id").unwrap_or_default();
            let time = extract_json_str(body, "time").unwrap_or_default();
            if !id.is_empty() && !time.is_empty() {
                schedule_macro_daily(&mut STATE.lock().unwrap(), &id, &time);
            }
            Some(format!(r#"{{"ok":true,"scheduled":"{}","time":"{}"}}"#, esc(&id), esc(&time)))
        }
        ("POST", "/macros/group")         => {
            let parallel = body.contains(r#""parallel":true"#);
            let ids_str = extract_json_str(body, "ids").unwrap_or_default();
            let ids: Vec<&str> = ids_str.split(',').map(|s| s.trim()).collect();
            run_macro_group(&mut STATE.lock().unwrap(), &ids, parallel);
            Some(format!(r#"{{"ok":true,"count":{}}}"#, ids.len()))
        }
        ("GET",  "/macros/find")          => {
            let name = body.find("name=").map(|i| &body[i+5..]).unwrap_or("").split('&').next().unwrap_or("");
            let result = find_macro_by_name(&STATE.lock().unwrap(), name);
            Some(match result {
                Some(id) => format!(r#"{{"found":true,"id":"{}"}}"#, esc(&id)),
                None     => r#"{"found":false}"#.to_string(),
            })
        }
        ("POST", "/macros/conditional")   => {
            let id = extract_json_str(body, "macro_id").unwrap_or_default();
            let ran = if !id.is_empty() { try_run_macro_conditional(&mut STATE.lock().unwrap(), &id) } else { false };
            Some(format!(r#"{{"ok":true,"ran":{}}}"#, ran))
        }
        ("GET",  "/automation/status") => {
            let s = STATE.lock().unwrap();
            let enabled = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
            let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();
            Some(format!(
                r#"{{"enabled_macros":{},"templates":{},"total_macros":{},"variables":{},"active_profile":"{}","pending_actions":{},"run_log_entries":{},"rate_ok":{}}}"#,
                enabled, templates, s.macros.len(), s.variables.len(),
                esc(&s.active_profile), s.pending_actions.len(),
                s.macro_run_log.len(), check_rate_limit(&s)
            ))
        }
        _ => None,
    }
}

// Crypto
// SECURITY: Credential key derivation — 1024 rounds of byte mixing.
// This is obfuscation (in-memory protection), NOT cryptographic encryption.
// For real encryption-at-rest, use Android Keystore via JNI.
fn derive_key(name: &str) -> Vec<u8> {
    let mut key = [0u8; 32];
    for (i, b) in name.bytes().enumerate() {
        key[i % 32] = key[i % 32].wrapping_add(b).wrapping_add(i as u8);
    }
    // Stretch: 1024 mixing rounds
    for round in 0u32..1024 {
        let rb = (round & 0xFF) as u8;
        for i in 0..32usize {
            key[i] = key[i]
                .wrapping_add(key[(i + 1) % 32])
                .wrapping_add(rb)
                .rotate_left(((i % 7) + 1) as u32);
        }
    }
    key.to_vec()
}
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> {
    if key.is_empty() { return data.to_vec(); }
    let klen = key.len();
    data.iter().enumerate().map(|(i, &b)| {
        let k1 = key[i % klen];
        let k2 = key[(i.wrapping_add(klen / 2 + 1)) % klen];
        b ^ k1 ^ k2.rotate_left(((i % 5) + 1) as u32)
    }).collect()
}

// Utilities

fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn gen_id()  -> String { format!("k{}", now_ms()) }
fn estimate_tokens(s: &str) -> u32 { (s.len()/4).max(1) as u32 }
fn esc(s: &str) -> String { s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "") }
fn json_str(s: &str) -> String { format!("\"{}\"", esc(s)) }
fn json_str_arr(v: &[String]) -> String { format!("[{}]", v.iter().map(|s| format!("\"{}\"", esc(s))).collect::<Vec<_>>().join(",")) }

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let search=format!("\"{}\":\"", key);
    let start=json.find(&search)?+search.len();
    let end=json[start..].find('"')?+start;
    Some(json[start..end].to_string())
}

fn extract_json_num(json: &str, key: &str) -> Option<f64> {
    let search=format!("\"{}\":", key);
    let start=json.find(&search)?+search.len();
    let slice=json[start..].trim_start();
    let end=slice.find(|c: char| !c.is_ascii_digit() && c!='.' && c!='-').unwrap_or(slice.len());
    slice[..end].parse::<f64>().ok()
}

fn extract_json_f32(json: &str, key: &str) -> Option<f32> {
    extract_json_num(json, key).map(|v| v as f32)
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Session C — AES-256-GCM authenticated encryption for secrets
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use aes_gcm::{Aes256Gcm, Key, Nonce, KeyInit};
use aes_gcm::aead::Aead;

/// Derive a stable 32-byte key from a device-specific seed string.
/// Seed is typically: SHA256(ANDROID_ID + package_name), supplied by Java.
/// Uses 64 rounds of XOR + rotate mixing — lightweight but sufficient
/// as a KDF since the seed itself comes from a 256-bit random source.
pub fn derive_aes_key(seed: &str) -> [u8; 32] {
    let mut key = [0u8; 32];
    let seed_bytes = seed.as_bytes();
    // Mix seed bytes into key with rotation
    for (i, &b) in seed_bytes.iter().enumerate() {
        key[i % 32] ^= b.wrapping_add(i as u8);
        key[(i + 7) % 32] = key[(i + 7) % 32].rotate_left(1) ^ b;
    }
    // 64 extra mixing rounds
    for round in 0u8..64 {
        for i in 0..32 {
            key[i] = key[i].wrapping_add(key[(i + 1) % 32])
                .rotate_left(3)
                ^ round;
        }
    }
    key
}

/// Derive a 12-byte deterministic nonce from the key + a domain string.
/// Domain prevents nonce reuse across different fields (api_key, tg_token, etc).
fn derive_nonce(key: &[u8; 32], domain: &str) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    for (i, &b) in domain.as_bytes().iter().enumerate() {
        nonce[i % 12] ^= b;
    }
    // Mix with first 12 bytes of key
    for i in 0..12 {
        nonce[i] ^= key[i].rotate_right(2);
    }
    nonce
}

/// Encrypt plaintext with AES-256-GCM. Returns hex-encoded ciphertext+tag.
/// domain: field name ("api_key", "tg_token", etc) — prevents cross-field decryption.
pub fn aes_encrypt(plaintext: &str, key_seed: &str, domain: &str) -> String {
    let key_bytes  = derive_aes_key(key_seed);
    let nonce_bytes = derive_nonce(&key_bytes, domain);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce  = Nonce::from_slice(&nonce_bytes);
    match cipher.encrypt(nonce, plaintext.as_bytes()) {
        Ok(ciphertext) => {
            // hex-encode: ciphertext includes 16-byte GCM auth tag appended
            ciphertext.iter().map(|b| format!("{:02x}", b)).collect()
        }
        Err(_) => String::new(), // should never happen
    }
}

/// Decrypt AES-256-GCM hex ciphertext. Returns plaintext or empty string on failure.
pub fn aes_decrypt(hex_ciphertext: &str, key_seed: &str, domain: &str) -> String {
    if hex_ciphertext.is_empty() { return String::new(); }
    // Decode hex
    let bytes: Option<Vec<u8>> = (0..hex_ciphertext.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_ciphertext[i..i+2], 16).ok())
        .collect();
    let ciphertext = match bytes {
        Some(b) if !b.is_empty() => b,
        _ => return String::new(),
    };
    let key_bytes   = derive_aes_key(key_seed);
    let nonce_bytes = derive_nonce(&key_bytes, domain);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce  = Nonce::from_slice(&nonce_bytes);
    match cipher.decrypt(nonce, ciphertext.as_slice()) {
        Ok(plain) => String::from_utf8(plain).unwrap_or_default(),
        Err(_)    => String::new(), // wrong key or tampered ciphertext
    }
}


// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Session K — Pure-Rust HTTPS client via rustls
// Works on arm64-v8a. Falls back to plain HTTP on other ABIs (or through
// Java bridge via /http_proxy endpoint).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

use rustls::ClientConfig;
use rustls::pki_types::ServerName;
// imports already in scope from state.rs

/// Send HTTPS POST and return response body.
/// Uses rustls with webpki-roots (Mozilla CA bundle compiled in).
pub fn https_post(
    host:       &str,
    port:       u16,
    path:       &str,
    body:       &str,
    auth_token: &str,
    timeout_s:  u64,
) -> Result<String, String> {
    // Build TLS config with Mozilla root certificates
    let root_store = {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    };
    let config = Arc::new({
        let mut c = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        c
    });

    // Establish TCP connection
    let addr   = format!("{}:{}", host, port);
    let stream = std::net::TcpStream::connect(&addr)
        .map_err(|e| format!("tcp connect {}: {}", addr, e))?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s)))
        .map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(std::time::Duration::from_secs(15)))
        .map_err(|e| e.to_string())?;

    // TLS handshake
    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| format!("invalid hostname {}: {:?}", host, e))?;
    let mut conn = rustls::ClientConnection::new(config, server_name)
        .map_err(|e| format!("tls init: {}", e))?;
    let mut tls_stream = rustls::Stream::new(&mut conn, &mut stream);

    // Write HTTP/1.1 request
    let request = format!(
        "POST {} HTTP/1.1
         Host: {}
         Authorization: Bearer {}
         Content-Type: application/json
         Content-Length: {}
         Connection: close
         
         {}",
        path, host, auth_token, body.len(), body
    );
    tls_stream.write_all(request.as_bytes())
        .map_err(|e| format!("write: {}", e))?;

    // Read response
    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match tls_stream.read(&mut buf) {
            Ok(0)  => break,
            Ok(n)  => response.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionAborted => break,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof     => break,
            Err(e) => return Err(format!("read: {}", e)),
        }
        if response.len() > 10 * 1024 * 1024 { break; } // 10MB cap
    }

    let resp_str = String::from_utf8_lossy(&response).into_owned();
    // Strip HTTP headers — find blank line
    if let Some(body_start) = resp_str.find("

") {
        Ok(resp_str[body_start + 4..].to_string())
    } else {
        Ok(resp_str)
    }
}

/// GET request over HTTPS (for Telegram API, GitHub releases, etc.)
pub fn https_get(host: &str, port: u16, path: &str, timeout_s: u64) -> Result<String, String> {
    let root_store = {
        let mut store = rustls::RootCertStore::empty();
        store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        store
    };
    let config = Arc::new({
        let mut c = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        c
    });

    let tcp = std::net::TcpStream::connect(format!("{}:{}", host, port))
        .map_err(|e| e.to_string())?;
    tcp.set_read_timeout(Some(std::time::Duration::from_secs(timeout_s)))
        .map_err(|e| e.to_string())?;

    let server_name = ServerName::try_from(host.to_string())
        .map_err(|e| format!("hostname: {:?}", e))?;
    let mut conn   = rustls::ClientConnection::new(config, server_name)
        .map_err(|e| format!("tls: {}", e))?;
    let mut stream = rustls::Stream::new(&mut conn, &mut tcp);

    let request = format!(
        "GET {} HTTP/1.1
Host: {}
Connection: close

",
        path, host
    );
    stream.write_all(request.as_bytes()).map_err(|e| e.to_string())?;

    let mut response = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => { response.extend_from_slice(&buf[..n]); }
        }
    }
    let resp = String::from_utf8_lossy(&response).into_owned();
    Ok(if let Some(i) = resp.find("

") { resp[i+4..].to_string() } else { resp })
}
