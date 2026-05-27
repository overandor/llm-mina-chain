use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{AgentId, CanonicalTimestamp, SemVer};

/// Canonical structured log entry. Every agent MUST emit logs in this format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalLogEntry {
    /// Unix timestamp in milliseconds.
    pub timestamp: CanonicalTimestamp,
    /// Log level.
    pub level: LogLevel,
    /// Which agent emitted this log.
    pub agent: AgentId,
    /// Module or component name.
    pub module: String,
    /// Short event name.
    pub event: String,
    /// Structured payload.
    pub data: serde_json::Value,
    /// Distributed trace identifier.
    pub trace_id: String,
    /// Protocol version.
    pub version: SemVer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl CanonicalLogEntry {
    pub fn new(
        level: LogLevel,
        agent: AgentId,
        module: impl Into<String>,
        event: impl Into<String>,
        data: serde_json::Value,
        trace_id: impl Into<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            timestamp,
            level,
            agent,
            module: module.into(),
            event: event.into(),
            data,
            trace_id: trace_id.into(),
            version: SemVer::CURRENT,
        }
    }

    /// Serialize to a canonical JSON string (newline-delimited).
    pub fn to_canonical_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Convenience macro-like functions for common log patterns.
pub fn log_info(agent: AgentId, module: &str, event: &str, data: serde_json::Value) {
    let entry = CanonicalLogEntry::new(
        LogLevel::Info,
        agent,
        module,
        event,
        data,
        uuid::Uuid::new_v4().to_string(),
    );
    println!("{}", entry.to_canonical_json());
}

pub fn log_error(agent: AgentId, module: &str, event: &str, data: serde_json::Value) {
    let entry = CanonicalLogEntry::new(
        LogLevel::Error,
        agent,
        module,
        event,
        data,
        uuid::Uuid::new_v4().to_string(),
    );
    eprintln!("{}", entry.to_canonical_json());
}

pub fn log_warn(agent: AgentId, module: &str, event: &str, data: serde_json::Value) {
    let entry = CanonicalLogEntry::new(
        LogLevel::Warn,
        agent,
        module,
        event,
        data,
        uuid::Uuid::new_v4().to_string(),
    );
    println!("{}", entry.to_canonical_json());
}
