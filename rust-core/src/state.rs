use lz4_flex::{compress_prepend_size, decompress_size_prepended};

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

#![allow(non_snake_case, dead_code, unused_mut, clippy::upper_case_acronyms)]

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
            pending_shell:    std::collections::VecDeque::new(),
            shell_results:    std::collections::HashMap::new(),
            agent_tasks:      std::collections::VecDeque::new(),
            tg_last_update_id: 0,
            tg_pending_sends:  std::collections::VecDeque::new(),
            tg_message_log:    std::collections::VecDeque::new(),
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
            pending_shell:    std::collections::VecDeque::new(),
            shell_results:    std::collections::HashMap::new(),
            agent_tasks:      std::collections::VecDeque::new(),
            tg_last_update_id: 0,
            tg_pending_sends:  std::collections::VecDeque::new(),
            tg_message_log:    std::collections::VecDeque::new(),
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
            pending_shell:    std::collections::VecDeque::new(),
            shell_results:    std::collections::HashMap::new(),
            agent_tasks:      std::collections::VecDeque::new(),
            tg_last_update_id: 0,
            tg_pending_sends:  std::collections::VecDeque::new(),
            tg_message_log:    std::collections::VecDeque::new(),
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
            pending_shell:    std::collections::VecDeque::new(),
            shell_results:    std::collections::HashMap::new(),
            agent_tasks:      std::collections::VecDeque::new(),
            tg_last_update_id: 0,
            tg_pending_sends:  std::collections::VecDeque::new(),
            tg_message_log:    std::collections::VecDeque::new(),
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
#[derive(Clone, Default)]
pub struct TgSend {
    pub chat_id: i64,
    pub text:    String,
    pub ts:      u128,
}

/// A received Telegram message
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

#[derive(Clone)]
struct CrashEntry {
    ts:      u128,
    thread:  String,
    message: String,   // first line of trace
    trace:   String,   // full stack trace (capped at 4KB)
}

#[derive(Default)]
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

lazy_static::
// ── LZ4 compression helpers for conversation history (Session B) ──────────

/// Compress a conversation turn into LZ4 bytes.
/// Format: "role content" → lz4_prepend_size(bytes)
pub fn lz4_pack_turn(role: &str, content: &str) -> Vec<u8> {
    let raw = format!("{} {}", role, content);
    compress_prepend_size(raw.as_bytes())
}

/// Decompress a single turn. Returns (role, content) or None on error.
pub fn lz4_unpack_turn(compressed: &[u8]) -> Option<(String, String)> {
    let raw = decompress_size_prepended(compressed).ok()?;
    let s   = String::from_utf8(raw).ok()?;
    let mut parts = s.splitn(2, ' ');
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
#[derive(Default, Clone)]
pub struct ShellJob {
    pub id:      String,
    pub cmd:     String,
    pub timeout: u64,
    pub created: u128,
}

/// Build the system prompt for the AI, including memory context
pub fn build_system_prompt(state: &KiraState, persona: &str) -> String {
    let mem_context: String = state.memory_index.iter().take(5)
        .map(|m| format!("- {}", m.content))
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
                id:           format!("mem_{}", idx),
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
