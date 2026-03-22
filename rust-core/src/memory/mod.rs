// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// kira-core :: memory
//
// Vector memory: add, search, MMR re-rank, temporal decay.
// Mirrors OpenClaw: src/memory/manager.ts, src/memory/mmr.ts,
//                   src/memory/temporal-decay.ts
//
// Session 1: types + cosine similarity.
// Session 5: embedding calls + full search pipeline.
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub mod index;
pub mod search;

pub use index::{MemoryStore, MemoryEntry};
pub use search::SearchResult;
