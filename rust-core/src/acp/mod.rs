// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: acp
//
// Agent Control Protocol  -  session wire format + event bus.
// Mirrors OpenClaw: src/acp/
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod types;
pub mod bus;

pub use types::{
    AcpEvent, AcpCommand, ContentBlock, StopReason, ErrorCode,
    Usage, SessionPatch, SpawnConfig, Attachment,
    // ThinkingLevel, ExecSecurity, ResponseUsage removed — using plain strings (S1 refactor)
};
pub use bus::{AcpBus, AcpBusHandle, new_bus_handle};
