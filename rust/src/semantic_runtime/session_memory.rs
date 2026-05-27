use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::types::{ActionStatus, Subsystem, UnifiedAction};

/// In-memory session memory that records every action the semantic runtime performs.
/// This is the foundation for replay, provenance, and contextual awareness.
pub struct SessionMemory {
    actions: Arc<Mutex<VecDeque<UnifiedAction>>>,
    max_size: usize,
}

impl SessionMemory {
    pub fn new(max_size: usize) -> Self {
        Self {
            actions: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    pub fn record(&self, action: UnifiedAction) {
        let mut guard = self.actions.lock().unwrap();
        if guard.len() >= self.max_size {
            guard.pop_front();
        }
        guard.push_back(action);
    }

    pub fn get_all(&self) -> Vec<UnifiedAction> {
        self.actions.lock().unwrap().iter().cloned().collect()
    }

    pub fn get_last_n(&self, n: usize) -> Vec<UnifiedAction> {
        let guard = self.actions.lock().unwrap();
        guard.iter().rev().take(n).cloned().collect()
    }

    pub fn get_by_subsystem(&self, subsystem: Subsystem) -> Vec<UnifiedAction> {
        self.actions
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.subsystem == subsystem)
            .cloned()
            .collect()
    }

    pub fn get_by_status(&self, status: ActionStatus) -> Vec<UnifiedAction> {
        self.actions
            .lock()
            .unwrap()
            .iter()
            .filter(|a| a.status == status)
            .cloned()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.actions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.lock().unwrap().is_empty()
    }

    pub fn clear(&self) {
        self.actions.lock().unwrap().clear();
    }

    /// Generate a Merkle root over all recorded action receipt hashes.
    pub fn merkle_root(&self) -> Option<[u8; 32]> {
        let guard = self.actions.lock().unwrap();
        let hashes: Vec<[u8; 32]> = guard
            .iter()
            .filter_map(|a| a.receipt_hash)
            .collect();
        if hashes.is_empty() {
            return None;
        }
        Some(crate::protocol::merkle_root_from_hashes(&hashes))
    }

    /// Export session as canonical JSON for replay or archival.
    pub fn export(&self) -> serde_json::Value {
        let actions = self.get_all();
        serde_json::json!({
            "session_actions": actions,
            "action_count": actions.len(),
        })
    }
}

impl Default for SessionMemory {
    fn default() -> Self {
        Self::new(10_000)
    }
}

/// Convenience builder for UnifiedAction.
pub struct ActionBuilder {
    action: UnifiedAction,
}

impl ActionBuilder {
    pub fn new(intent: impl Into<String>) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        Self {
            action: UnifiedAction {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                intent: intent.into(),
                subsystem: Subsystem::SolanaQuery,
                status: ActionStatus::Pending,
                result_summary: String::new(),
                receipt_hash: None,
                metadata: Default::default(),
            },
        }
    }

    pub fn subsystem(mut self, s: Subsystem) -> Self {
        self.action.subsystem = s;
        self
    }

    pub fn status(mut self, s: ActionStatus) -> Self {
        self.action.status = s;
        self
    }

    pub fn result(mut self, r: impl Into<String>) -> Self {
        self.action.result_summary = r.into();
        self
    }

    pub fn receipt_hash(mut self, h: [u8; 32]) -> Self {
        self.action.receipt_hash = Some(h);
        self
    }

    pub fn metadata(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.action.metadata.insert(k.into(), v.into());
        self
    }

    pub fn build(self) -> UnifiedAction {
        self.action
    }
}
