// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: tools
//
// Concrete tool implementations registered into ai::tools::ToolRegistry.
// Mirrors OpenClaw: src/agents/tools/, src/agents/openclaw-tools.ts
//
// Session 1: module skeleton + registration helper.
// Session 2: wire into the AI loop.
// Session 15: device tools (location, SMS, contacts, calendar).
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod system;
pub mod memory_tools;
pub mod device;

pub use system::register_system_tools;
pub use memory_tools::register_memory_tools;
pub use device::register_device_tools;

use crate::ai::tools::ToolRegistry;

/// Register all built-in tools into a registry.
/// Called once at startup.
pub fn register_all(registry: &mut ToolRegistry) {
    register_system_tools(registry);
    register_memory_tools(registry);
    register_device_tools(registry);
}
