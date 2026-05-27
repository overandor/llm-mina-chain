//! Semantic Runtime — AI-Native Blockchain Operating System
//!
//! This module unifies all agent subsystems under a single query router,
//! session memory, contextual awareness, and orchestration layer.
//!
//! # Architecture
//!
//! ```text
//! User Input → QueryRouter → ParsedIntent → Orchestrator
//!                                          │
//!              ┌───────────────────────────┼───────────────────────────┐
//!              │                           │                           │
//!         SolanaQuery              KnowledgeBase              LocalFilesystem
//!              │                           │                           │
//!         SessionMemory ←─────────────────┘                           │
//!              │                                                       │
//!         CanonicalReceipts ←─────────────────────────────────────────┘
//! ```
//!
//! Every action produces a `CanonicalReceipt`. Every query is recorded in
//! `SessionMemory`. Every intent is resolved through the `QueryRouter`.

pub mod context;
pub mod orchestrator;
pub mod query_router;
pub mod session_memory;
pub mod types;

pub use context::RuntimeContext;
pub use orchestrator::Orchestrator;
pub use query_router::QueryRouter;
pub use session_memory::{ActionBuilder, SessionMemory};
pub use types::{
    ActionStatus, Entity, EntityType, ExecutionPlan, IntentType, ParsedIntent, PlanStep,
    Subsystem, UnifiedAction,
};
