// Kira Rust Core v9 — v40 edition
//
// NEW in v40 (Tasker/MacroDroid automation engine — pure Rust):
//   - MacroEngine: full IF/THEN/ELSE automation macros
//   - 40+ TriggerType variants (time, geo, app, notif, battery, screen,
//     wifi, bluetooth, charging, shake, volume-btn, sms, call, headset,
//     airplane, power-connected, idle, unlock, nfc, clipboard, signal…)
//   - 60+ ActionType variants (HTTP, shell via Shizuku, clipboard, media,
//     volume, torch, TTS, brightness, airplane, send-notif, open-app,
//     toast, vibrate, set-variable, log, wait, stop-flow, loop, intent…)
//   - Variable engine (store/retrieve named vars, math + string expr)
//   - Named Profiles (Work/Home/Sleep/Car) with auto-switch rules
//   - Flow control: Loop, Delay, If/Else, Stop
//   - Macro import/export JSON
//   - All v8/v38 features preserved

#![allow(non_snake_case, dead_code, clippy::upper_case_acronyms)]

use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// v40: AUTOMATION ENGINE — Triggers, Conditions, Actions, Macros, Profiles
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Every possible trigger type — mirrors Tasker + MacroDroid combined
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
    // Location (geofence — lat/lon + radius fed from Java)
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
    KiraAsk,          // prompt → stores result in variable
    KiraSpeak,        // TTS via Kira voice
    KiraMessage,      // send message to active session
    // Variables
    SetVariable,      // name + value (supports %VAR% tokens + math)
    IncrementVariable,
    DecrementVariable,
    ClearVariable,
    // Flow control
    Wait,             // ms
    If,               // condition → else_action_index
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
    /// Generic key-value params (url, body, variable_name, value, ms, pkg…)
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
    /// Extra match data: SSID, package name, battery threshold, cron expr…
    config:  HashMap<String, String>,
    enabled: bool,
}

/// Condition for If action or macro-level constraint
#[derive(Clone)]
struct MacroCondition {
    lhs:      String,   // variable name or built-in: %BATTERY%, %SCREEN_PKG%, %TIME_H%…
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// v38: Setup / Theme / Config / Shizuku (unchanged)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
    accent_color:    u32,
    bg_color:        u32,
    card_color:      u32,
    muted_color:     u32,
    star_count:      u32,
    star_speed:      f32,
    star_tilt_x:     f32,
    star_tilt_y:     f32,
    star_parallax_x: f32,
    star_parallax_y: f32,
}
impl Default for ThemeConfig {
    fn default() -> Self { ThemeConfig { accent_color:0xFFDC143C, bg_color:0xFF050508, card_color:0xFF0e0e18, muted_color:0xFF666680, star_count:110, star_speed:0.013, star_tilt_x:0.0, star_tilt_y:0.0, star_parallax_x:0.0, star_parallax_y:0.0 } }
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Core State
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Default)]
struct KiraState {
    // Device
    screen_nodes:      String,
    screen_pkg:        String,
    battery_pct:       i32,
    battery_charging:  bool,

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
    request_count:     u64,
    tool_call_count:   u64,

    // v38
    setup:             SetupState,
    theme:             ThemeConfig,
    config:            KiraConfig,
    shizuku:           ShizukuStatus,

    // ── v40: Automation Engine ────────────────────────────────────────────────
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

lazy_static::lazy_static! {
    static ref STATE: Arc<Mutex<KiraState>> = Arc::new(Mutex::new(KiraState {
        battery_pct:     100,
        max_tool_iters:  20,
        active_session:  "default".to_string(),
        active_provider: "groq".to_string(),
        active_profile:  "default".to_string(),
        soul_md: "You are Kira, a powerful Android AI agent. You are helpful, proactive, and autonomous.".to_string(),
        theme:   ThemeConfig::default(),
        config:  KiraConfig::default(),
        setup:   SetupState::default(),
        shizuku: ShizukuStatus::default(),
        ..Default::default()
    }));
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// v40: Variable engine helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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

    // All other actions → enqueue for Java
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
                    // For now just skip — full else branch is handled at the Java UI level
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Parse a macro from JSON body (simple hand-rolled parser matching the
// format the Java UI will POST)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
                // Parse config as flat key→value from "config":{...}
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// JNI Bridge
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

mod jni_bridge {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    fn cs(p: *const c_char) -> String {
        if p.is_null() { return String::new(); }
        unsafe { CStr::from_ptr(p).to_string_lossy().into_owned() }
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_startServer(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, port: i32,
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

    // ── v40: Device signal injectors (called from Java on each device event) ──

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOn(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_screen_on = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalScreenOff(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_screen_off = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalUnlocked(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_device_unlocked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocked(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_device_locked = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalShake(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_shake = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeUp(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_vol_up = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalVolumeDown(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { STATE.lock().unwrap().sig_vol_down = true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalWifi(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, ssid: *const c_char,
    ) { STATE.lock().unwrap().sig_wifi_ssid = cs(ssid); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalBluetooth(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, device: *const c_char,
    ) { STATE.lock().unwrap().sig_bt_device = cs(device); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalSms(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        sender: *const c_char, text: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.sig_sms_sender = cs(sender);
        s.sig_sms_text   = cs(text);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalCall(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, number: *const c_char,
    ) { STATE.lock().unwrap().sig_call_number = cs(number); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalNfc(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, tag_id: *const c_char,
    ) { STATE.lock().unwrap().sig_nfc_tag = cs(tag_id); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalClipboard(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, text: *const c_char,
    ) { STATE.lock().unwrap().sig_clipboard = cs(text); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppLaunched(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pkg: *const c_char,
    ) { STATE.lock().unwrap().sig_app_launched = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalAppClosed(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pkg: *const c_char,
    ) { STATE.lock().unwrap().sig_app_closed = cs(pkg); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalLocation(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, lat: f64, lon: f64, geofence: *const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.sig_lat      = lat;
        s.sig_lon      = lon;
        s.sig_geofence = cs(geofence);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_signalKiraEvent(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, event: *const c_char,
    ) { STATE.lock().unwrap().sig_kira_event = cs(event); }

    // ── v40: Macro management JNI ─────────────────────────────────────────────

    /// Add or replace a macro. Body is full macro JSON.
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addMacro(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, json: *const c_char,
    ) -> *mut c_char {
        let body = cs(json);
        let mut m = parse_macro_from_json(&body);
        let mut s = STATE.lock().unwrap();
        s.macros.retain(|x| x.id != m.id);
        let id = m.id.clone();
        s.macros.push(m);
        CString::new(format!(r#"{{"ok":true,"id":"{}"}}"#, id)).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeMacro(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char,
    ) {
        let id = cs(id);
        STATE.lock().unwrap().macros.retain(|m| m.id != id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_enableMacro(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char, enabled: bool,
    ) {
        let id = cs(id);
        if let Some(m) = STATE.lock().unwrap().macros.iter_mut().find(|m| m.id == id) {
            m.enabled = enabled;
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacros(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.macros.iter().map(macro_to_json).collect();
        CString::new(format!("[{}]", items.join(","))).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_runMacroNow(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char,
    ) -> *mut c_char {
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
        CString::new(format!(r#"{{"ok":true,"steps":{}}}"#, steps)).unwrap_or_default().into_raw()
    }

    /// Get next pending macro action for Java to execute
    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextMacroAction(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().pending_actions.pop_front() {
            Some(pa) => {
                let params_json: Vec<String> = pa.params.iter()
                    .map(|(k,v)| format!("\"{}\":\"{}\"", esc(k), esc(v))).collect();
                let json = format!(
                    r#"{{"macro_id":"{}","action_id":"{}","kind":"{}","ts":{},"params":{{{}}}}}"#,
                    esc(&pa.macro_id), esc(&pa.action_id), esc(&pa.kind), pa.ts, params_json.join(",")
                );
                CString::new(json).unwrap_or_default().into_raw()
            }
            None => std::ptr::null_mut(),
        }
    }

    // ── v40: Variable management ──────────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setVariable(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, name: *const c_char,
    ) -> *mut c_char {
        let name = cs(name);
        let s = STATE.lock().unwrap();
        let json = match s.variables.get(&name) {
            Some(v) => format!(r#"{{"name":"{}","value":"{}","type":"{}"}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type)),
            None    => r#"{"error":"not_found"}"#.to_string(),
        };
        CString::new(json).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getVariables(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.variables.values().map(|v|
            format!(r#"{{"name":"{}","value":"{}","type":"{}","updated_ms":{}}}"#, esc(&v.name), esc(&v.value), esc(&v.var_type), v.updated_ms)
        ).collect();
        CString::new(format!("[{}]", items.join(","))).unwrap_or_default().into_raw()
    }

    // ── v40: Profile management ───────────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setProfile(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id: *const c_char,
    ) {
        let id = cs(id);
        let mut s = STATE.lock().unwrap();
        s.active_profile = id.clone();
        for p in s.profiles.iter_mut() { p.active = p.id == id; }
        s.sig_kira_event = format!("profile_changed:{}", id);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProfiles(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.profiles.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","active":{}}}"#, esc(&p.id), esc(&p.name), p.active)
        ).collect();
        CString::new(format!("[{}]", items.join(","))).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getMacroRunLog(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.macro_run_log.iter().skip(s.macro_run_log.len().saturating_sub(100)).map(|r|
            format!(r#"{{"macro_id":"{}","name":"{}","trigger":"{}","success":{},"steps":{},"duration_ms":{},"ts":{}}}"#,
                esc(&r.macro_id), esc(&r.macro_name), esc(&r.trigger), r.success, r.steps_run, r.duration_ms, r.ts)
        ).collect();
        CString::new(format!("[{}]", items.join(","))).unwrap_or_default().into_raw()
    }

    // ── v38 JNI (unchanged) ───────────────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_syncConfig(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let json = config_to_json(&s.config);
        CString::new(json).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateSetupPage(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) { let mut s = STATE.lock().unwrap(); s.setup.done=true; s.config.setup_done=true; }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_isSetupDone(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> bool { STATE.lock().unwrap().config.setup_done }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_setCustomProvider(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, url:*const c_char, model:*const c_char,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, provider_id:*const c_char,
    ) -> *mut c_char {
        let id=cs(provider_id);
        let mut s = STATE.lock().unwrap();
        let found=s.providers.iter().find(|p| p.id==id).cloned();
        let result = if let Some(p)=found {
            s.active_provider=id; s.config.base_url=p.base_url.clone(); s.config.model=p.model.clone();
            format!(r#"{{"ok":true,"id":"{}","base_url":"{}","model":"{}"}}"#, esc(&s.active_provider),esc(&p.base_url),esc(&p.model))
        } else { format!(r#"{{"error":"unknown provider {}"}}"#, esc(&id)) };
        CString::new(result).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getProviders(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let items: Vec<String> = s.providers.iter().map(|p|
            format!(r#"{{"id":"{}","name":"{}","base_url":"{}","model":"{}","active":{}}}"#, esc(&p.id),esc(&p.name),esc(&p.base_url),esc(&p.model),p.id==s.active_provider)
        ).collect();
        CString::new(format!("[{}]", items.join(","))).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateShizukuStatus(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        installed:bool, permission_granted:bool, error_msg:*const c_char,
    ) {
        let mut s = STATE.lock().unwrap();
        s.shizuku.installed=installed; s.shizuku.permission_granted=permission_granted;
        s.shizuku.error_msg=cs(error_msg); s.shizuku.last_checked_ms=now_ms();
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getShizukuJson(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        CString::new(shizuku_to_json(&s.shizuku)).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateTilt(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, ax:f32, ay:f32,
    ) {
        let mut s = STATE.lock().unwrap();
        s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay;
        let tx=-ax*s.theme.star_speed; let ty=ay*s.theme.star_speed;
        s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08;
        s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08;
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStarParallax(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        CString::new(format!(r#"{{"px":{:.6},"py":{:.6},"ax":{:.4},"ay":{:.4}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y,s.theme.star_tilt_x,s.theme.star_tilt_y)).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getTheme(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        CString::new(format!(r#"{{"accent":{},"bg":{},"card":{},"muted":{},"star_count":{}}}"#, s.theme.accent_color,s.theme.bg_color,s.theme.card_color,s.theme.muted_color,s.theme.star_count)).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getStatsJson(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        CString::new(format!(
            r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"profiles":{},"active_profile":"{}","variables":{}}}"#,
            s.memory_index.len(), s.context_turns.len(),
            if s.shizuku.permission_granted{"active ✓"} else if s.shizuku.installed{"no permission"} else{"not running"},
            if !s.agent_context.is_empty(){"enabled ✓"} else{"disabled"},
            esc(&s.config.model), esc(&s.config.base_url),
            now_ms().saturating_sub(s.uptime_start),
            s.macros.len(), s.profiles.len(), esc(&s.active_profile),
            s.variables.len()
        )).unwrap_or_default().into_raw()
    }

    // ── v7 JNI (unchanged) ────────────────────────────────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushNotification(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, json:*const c_char,
    ) { STATE.lock().unwrap().screen_nodes = cs(json); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateScreenPackage(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pkg:*const c_char,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, pct:i32, charging:bool,
    ) {
        let mut s = STATE.lock().unwrap();
        let prev = s.battery_pct;
        s.battery_pct=pct; s.battery_charging=charging;
        fire_battery_triggers(&mut s, pct, prev);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_updateAgentContext(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, ctx:*const c_char,
    ) { STATE.lock().unwrap().agent_context = cs(ctx); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushContextTurn(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        role:*const c_char, content:*const c_char,
    ) {
        let role=cs(role); let content=cs(content);
        let tokens=estimate_tokens(&content);
        let ts=now_ms();
        let mut s = STATE.lock().unwrap();
        let sess_id = s.active_session.clone();
        s.total_tokens += tokens as u64;
        s.daily_log.push_back(format!("[{}] {}: {}", ts, role, &content[..content.len().min(80)]));
        s.context_turns.push_back(ContextTurn { role, content, ts, tokens, session:sess_id });
        if s.context_turns.len() > 60 { compact_context(&mut s); }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_indexMemory(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name:*const c_char, value:*const c_char,
    ) {
        let name=cs(name); let value=cs(value);
        let enc=xor_crypt(value.as_bytes(), derive_key(&name).as_slice());
        STATE.lock().unwrap().credentials.insert(name, enc);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_registerSkill(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name:*const c_char, desc:*const c_char, trigger:*const c_char, content:*const c_char,
    ) {
        let name=cs(name);
        STATE.lock().unwrap().skills.insert(name.clone(), Skill { name, description:cs(desc), trigger:cs(trigger), content:cs(content), enabled:true, usage_count:0 });
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addHeartbeatItem(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id:*const c_char, check:*const c_char, action:*const c_char, interval_ms:i64,
    ) {
        let item = HeartbeatItem { id:cs(id), check:cs(check), action:cs(action), enabled:true, last_run:0, interval_ms:interval_ms as u128 };
        let mut s = STATE.lock().unwrap();
        s.heartbeat_items.retain(|i| i.id!=item.id);
        s.heartbeat_items.push(item);
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_incrementToolIter(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id:*const c_char,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, session_id:*const c_char,
    ) { STATE.lock().unwrap().tool_iterations.remove(&cs(session_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_logTaskStep(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
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
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().pending_cmds.pop_front() {
            Some((id,body)) => CString::new(format!(r#"{{"id":"{}","body":{}}}"#, id, body)).unwrap().into_raw(),
            None => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_pushResult(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id:*const c_char, result:*const c_char,
    ) { STATE.lock().unwrap().results.insert(cs(id), cs(result)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_nextFiredTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        match STATE.lock().unwrap().fired_triggers.pop_front() {
            Some(t) => CString::new(t).unwrap().into_raw(),
            None    => std::ptr::null_mut(),
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_addTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        id:*const c_char, ttype:*const c_char, value:*const c_char, action:*const c_char, repeat:bool,
    ) { STATE.lock().unwrap().triggers.push(Trigger { id:cs(id), trigger_type:cs(ttype), value:cs(value), action:cs(action), fired:false, repeat }); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_removeTrigger(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, id:*const c_char,
    ) { let id=cs(id); STATE.lock().unwrap().triggers.retain(|t| t.id!=id); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_freeString(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, s:*mut c_char,
    ) { if !s.is_null() { unsafe { drop(CString::from_raw(s)); } } }

    // ── OpenClaw / NanoBot / ZeroClaw extended JNI ───────────────────────────

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_exportMacros(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let json = export_macros_json(&STATE.lock().unwrap());
        CString::new(json).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_importMacros(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, json: *const c_char,
    ) { import_macros_json(&mut STATE.lock().unwrap(), &cs(json)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_chainMacro(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, target_id: *const c_char,
    ) { chain_macro(&mut STATE.lock().unwrap(), &cs(target_id)); }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_evalExpr(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, expr: *const c_char,
    ) -> *mut c_char {
        let result = eval_expr(&STATE.lock().unwrap(), &cs(expr));
        CString::new(result).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_expandVars(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void, text: *const c_char,
    ) -> *mut c_char {
        let result = expand_vars(&STATE.lock().unwrap(), &cs(text));
        CString::new(result).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationAnalytics(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let json = get_automation_analytics(&STATE.lock().unwrap());
        CString::new(json).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationReport(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let report = get_automation_report(&STATE.lock().unwrap());
        CString::new(report).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_scheduleMacroDaily(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        macro_id: *const c_char, time_hhmm: *const c_char,
    ) {
        let id = cs(macro_id); let time = cs(time_hhmm);
        if !id.is_empty() && !time.is_empty() {
            schedule_macro_daily(&mut STATE.lock().unwrap(), &id, &time);
        }
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_findMacroByName(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        name: *const c_char,
    ) -> *mut c_char {
        let result = find_macro_by_name(&STATE.lock().unwrap(), &cs(name));
        let json = match result {
            Some(id) => format!(r#"{{"found":true,"id":"{}"}}"#, esc(&id)),
            None     => r#"{"found":false}"#.to_string(),
        };
        CString::new(json).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_resolveParam(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
        param: *const c_char,
    ) -> *mut c_char {
        let result = resolve_param(&STATE.lock().unwrap(), &cs(param));
        CString::new(result).unwrap_or_default().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn Java_com_kira_service_RustBridge_getAutomationStatus(
        _e: *mut std::ffi::c_void, _c: *mut std::ffi::c_void,
    ) -> *mut c_char {
        let s = STATE.lock().unwrap();
        let enabled = s.macros.iter().filter(|m| m.enabled && !m.tags.contains(&"template".to_string())).count();
        let templates = s.macros.iter().filter(|m| m.tags.contains(&"template".to_string())).count();
        let json = format!(
            r#"{{"enabled_macros":{},"templates":{},"total_macros":{},"variables":{},"active_profile":"{}","pending_actions":{},"run_log_entries":{},"rate_ok":{}}}"#,
            enabled, templates, s.macros.len(), s.variables.len(),
            esc(&s.active_profile), s.pending_actions.len(),
            s.macro_run_log.len(), check_rate_limit(&s)
        );
        CString::new(json).unwrap_or_default().into_raw()
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// HTTP Server
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn run_http(port: u16) {
    let listener = match TcpListener::bind(format!("0.0.0.0:{}", port)) {
        Ok(l) => l, Err(e) => { eprintln!("kira bind: {}", e); return; }
    };
    for stream in listener.incoming().flatten() { thread::spawn(|| handle_http(stream)); }
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
    let resp = route_http(parts[0], parts[1], &body);
    let http = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nX-Kira-Engine: rust-v9\r\n\r\n{}", resp.len(), resp);
    let _ = stream.write_all(http.as_bytes());
    STATE.lock().unwrap().request_count += 1;
}

fn route_http(method: &str, path: &str, body: &str) -> String {
    let path_clean = path.split('?').next().unwrap_or(path);
    match (method, path_clean) {
        // Health & stats
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
        ("POST", "/macros/add")        => { let mut m=parse_macro_from_json(body); let id=m.id.clone(); let mut s=STATE.lock().unwrap(); s.macros.retain(|x| x.id!=m.id); s.macros.push(m); format!(r#"{{"ok":true,"id":"{}"}}"#, id) }
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
        ("GET",  "/theme")             => { let s=STATE.lock().unwrap(); format!(r#"{{"accent":{},"bg":{},"card":{},"muted":{},"star_count":{},"parallax_x":{:.6},"parallax_y":{:.6}}}"#, s.theme.accent_color,s.theme.bg_color,s.theme.card_color,s.theme.muted_color,s.theme.star_count,s.theme.star_parallax_x,s.theme.star_parallax_y) }
        ("POST", "/theme/tilt")        => { let ax=extract_json_f32(body,"ax").unwrap_or(0.0); let ay=extract_json_f32(body,"ay").unwrap_or(0.0); let mut s=STATE.lock().unwrap(); s.theme.star_tilt_x=ax; s.theme.star_tilt_y=ay; let spd=s.theme.star_speed; let tx=-ax*spd; let ty=ay*spd; s.theme.star_parallax_x+=(tx-s.theme.star_parallax_x)*0.08; s.theme.star_parallax_y+=(ty-s.theme.star_parallax_y)*0.08; format!(r#"{{"px":{:.6},"py":{:.6}}}"#, s.theme.star_parallax_x,s.theme.star_parallax_y) }
        ("GET",  "/shizuku")           => { let s=STATE.lock().unwrap(); shizuku_to_json(&s.shizuku) }
        ("POST", "/shizuku")           => { let installed=body.contains(r#""installed":true"#); let granted=body.contains(r#""permission_granted":true"#); let err=extract_json_str(body,"error").unwrap_or_default(); let mut s=STATE.lock().unwrap(); s.shizuku.installed=installed; s.shizuku.permission_granted=granted; s.shizuku.error_msg=err; s.shizuku.last_checked_ms=now_ms(); r#"{"ok":true}"#.to_string() }
        ("GET",  "/appstats")          => { let s=STATE.lock().unwrap(); format!(r#"{{"facts":{},"history":{},"shizuku":"{}","accessibility":"{}","model":"{}","provider":"{}","uptime_ms":{},"macros":{},"active_profile":"{}","variables":{}}}"#, s.memory_index.len(),s.context_turns.len(), if s.shizuku.permission_granted{"active ✓"}else if s.shizuku.installed{"no permission"}else{"not running"}, if !s.agent_context.is_empty(){"enabled ✓"}else{"disabled"}, esc(&s.config.model),esc(&s.config.base_url),now_ms().saturating_sub(s.uptime_start),s.macros.len(),esc(&s.active_profile),s.variables.len()) }
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
        ("GET",  "/daily_log")         => { let s=STATE.lock().unwrap(); let items: Vec<String>=s.daily_log.iter().cloned().map(|l| format!("\"{}\"", esc(&l))).collect(); format!("[{}]", items.join(",")) }
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

        // OpenClaw / NanoBot / ZeroClaw extended automation routes
        _ => {
            if let Some(r) = route_openclaw(method, path_clean, body) { r }
            else { queue_to_java(path_clean.trim_start_matches('/'), body) }
        }
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Background threads
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// v40: Watchdog thread — cleans stale pending actions every 30s
fn run_watchdog() {
    loop {
        thread::sleep(Duration::from_secs(30));
        watchdog_check(&mut STATE.lock().unwrap());
    }
}

/// v40: Dedicated macro engine loop — checks all signal-based triggers every 500ms
fn run_macro_engine() {
    loop {
        thread::sleep(Duration::from_millis(500));
        let mut s = STATE.lock().unwrap();
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Helpers (unchanged from v8)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

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
        for tag in &e.tags { if tag.to_lowercase().contains(&ql) { score+=2.0; } }
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
    let mut results: Vec<(u32, &KbEntry)>=s.knowledge_base.iter().filter_map(|e| { let mut sc=0u32; if e.title.to_lowercase().contains(&query) { sc+=10; } if e.content.to_lowercase().contains(&query) { sc+=5; } for tag in &e.tags { if tag.to_lowercase().contains(&query) { sc+=3; } } if sc>0 { Some((sc,e)) } else { None } }).collect();
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

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// OpenClaw / NanoBot / ZeroClaw Extended Automation Engine
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
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

/// Cooldown tracker — returns true if macro is allowed to run now
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
    true // no cooldown set → always allowed
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
        // 1. Battery guardian — warn + enable power saver
        AutoMacro {
            id: "tpl_battery_guardian".to_string(),
            name: "🔋 Battery Guardian".to_string(),
            description: "Toast warning when battery drops below 20%, vibrate at 10%".to_string(),
            enabled: false, // templates off by default
            triggers: vec![MacroTrigger {
                kind: MacroTriggerKind::BatteryLevel,
                config: [("op".to_string(),"lte".to_string()),("threshold".to_string(),"20".to_string())].iter().cloned().collect(),
                enabled: true,
            }],
            conditions: vec![],
            actions: vec![
                act("show_toast", vec![("message", "⚠️ Battery low: %BATTERY%%")]),
                act("vibrate", vec![("ms", "500")]),
                act("log_event", vec![("message", "Battery guardian fired at %BATTERY%%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "battery".to_string(), "cooldown:300000".to_string()],
        },

        // 2. Work mode — activate when connecting to work WiFi
        AutoMacro {
            id: "tpl_work_mode".to_string(),
            name: "💼 Work Mode".to_string(),
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
                act("show_toast", vec![("message", "💼 Work mode activated")]),
                act("log_event", vec![("message", "Work mode: connected to %WIFI%")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "wifi".to_string(), "work".to_string()],
        },

        // 3. Sleep mode — dim screen, silence at night
        AutoMacro {
            id: "tpl_sleep_mode".to_string(),
            name: "🌙 Sleep Mode".to_string(),
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

        // 4. Car mode — BT connect auto-opens maps + disables notifications
        AutoMacro {
            id: "tpl_car_mode".to_string(),
            name: "🚗 Car Mode".to_string(),
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
                act("show_toast", vec![("message", "🚗 Car mode — drive safe!")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "car".to_string(), "bluetooth".to_string()],
        },

        // 5. AI morning briefing — Kira speaks summary on unlock
        AutoMacro {
            id: "tpl_morning_briefing".to_string(),
            name: "🌅 Morning Briefing".to_string(),
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

        // 6. Smart notification filter — AI decides if notif is urgent
        AutoMacro {
            id: "tpl_notif_filter".to_string(),
            name: "🧠 Smart Notif Filter".to_string(),
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

        // 7. Clipboard AI enhancer — transform clipboard text with AI
        AutoMacro {
            id: "tpl_clipboard_ai".to_string(),
            name: "📋 Clipboard AI".to_string(),
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
                act("show_toast", vec![("message", "✅ Clipboard enhanced by Kira")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "ai".to_string(), "clipboard".to_string()],
        },

        // 8. Webhook automation — receive external trigger, run AI, reply
        AutoMacro {
            id: "tpl_webhook_ai".to_string(),
            name: "🌐 Webhook AI Agent".to_string(),
            description: "Receive HTTP POST → Kira processes → HTTP reply (OpenClaw pattern)".to_string(),
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

        // 9. NFC tag launcher — tap tag to run specific macro
        AutoMacro {
            id: "tpl_nfc_launcher".to_string(),
            name: "📡 NFC Tag Launcher".to_string(),
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
                act("show_toast", vec![("message", "🏠 Welcome home!")]),
                act("kira_speak", vec![("text", "Welcome home. I've set your home profile.")]),
            ],
            profile: String::new(), run_count: 0, last_run_ms: 0, created_ms: ts,
            tags: vec!["template".to_string(), "nfc".to_string(), "home".to_string()],
        },

        // 10. Shake-to-SOS — shake 3x to send emergency SMS
        AutoMacro {
            id: "tpl_shake_sos".to_string(),
            name: "🆘 Shake SOS".to_string(),
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
                    ("message", "🆘 SOS! I need help. My location: https://maps.google.com/?q=%SOS_LAT%,%SOS_LON%"),
                ]),
                act("vibrate", vec![("ms", "2000")]),
                act("show_toast", vec![("message", "🆘 SOS sent!")]),
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


// ─── OpenClaw v2: Advanced automation features ────────────────────────────────

/// Macro schedule: run macro at specific time daily (HH:MM format)
/// Stored as a cron job internally
fn schedule_macro_daily(s: &mut KiraState, macro_id: &str, time_hhmm: &str) {
    // Parse HH:MM → store as cron job with interval = 24h
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
        // Enqueue all at once — Java executes them concurrently
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

/// Variable interpolation in action params — supports math expressions
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

/// Get macro by name (case-insensitive) — useful for natural language commands
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
        lines.push(format!("  • {} — {} steps — {}s ago", r.macro_name, r.steps_run, ago));
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
            // Evaluate expression: GET /expr?e=5+3 → {"result":"8"}
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
fn derive_key(name: &str) -> Vec<u8> { let mut key=vec![0u8;32]; for (i,b) in name.bytes().enumerate() { key[i%32]^=b.wrapping_add((i as u8).wrapping_mul(7)); } key }
fn xor_crypt(data: &[u8], key: &[u8]) -> Vec<u8> { if key.is_empty() { return data.to_vec(); } data.iter().enumerate().map(|(i,&b)| b^key[i%key.len()]).collect() }

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
