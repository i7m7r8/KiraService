//! KiraService Rust core — v0.0.6
//! Session A: lib.rs split into logical files via include!()
//! 
//! Files are textually included so all items share one namespace —
//! zero behavioral change, no import rewiring needed.
//! 
//! Modules:
//!   state.rs        — structs, enums, lazy_static STATE (L1-1627)
//!   jni_bridge.rs   — JNI exports (L1628-2878)
//!   http.rs         — HTTP server + all route_http endpoints (L2879-5646)
//!   app_packages.rs — app_name_to_pkg() 280+ entries (L5647-7408)
//!   utils.rs        — now_ms, esc, json_str, extract_json* (L7409-end)

include!("state.rs");
include!("jni_bridge.rs");
include!("http.rs");
include!("app_packages.rs");
include!("utils.rs");
