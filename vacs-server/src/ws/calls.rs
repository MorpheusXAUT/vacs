use crate::metrics::guards::{CallAttemptGuard, CallAttemptOutcome, CallGuard};
use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Call(String, String);

impl Call {
    pub fn new(peer1_id: impl Into<String>, peer2_id: impl Into<String>) -> Self {
        let peer1_id = peer1_id.into();
        let peer2_id = peer2_id.into();

        if peer1_id <= peer2_id {
            Self(peer1_id, peer2_id)
        } else {
            Self(peer2_id, peer1_id)
        }
    }
}

impl From<(String, String)> for Call {
    fn from((peer1_id, peer2_id): (String, String)) -> Self {
        Self::new(peer1_id, peer2_id)
    }
}

pub struct CallStateManager {
    call_attempts: RwLock<HashMap<Call, CallAttemptGuard>>,
    active_calls: RwLock<HashMap<Call, CallGuard>>,
}

impl CallStateManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_call_attempt(&self, peer1_id: impl Into<String>, peer2_id: impl Into<String>) {
        self.call_attempts
            .write()
            .insert(Call::new(peer1_id, peer2_id), CallAttemptGuard::new());
    }

    pub fn complete_call_attempt(
        &self,
        peer1_id: impl Into<String>,
        peer2_id: impl Into<String>,
        outcome: CallAttemptOutcome,
    ) {
        if let Some(mut guard) = self
            .call_attempts
            .write()
            .remove(&Call::new(peer1_id, peer2_id))
        {
            guard.set_outcome(outcome);
        }
    }

    pub fn start_call(&self, peer1_id: impl Into<String>, peer2_id: impl Into<String>) {
        self.active_calls
            .write()
            .insert(Call::new(peer1_id, peer2_id), CallGuard::new());
    }

    pub fn end_call(&self, peer1_id: impl Into<String>, peer2_id: impl Into<String>) {
        self.active_calls
            .write()
            .remove(&Call::new(peer1_id, peer2_id));
    }

    pub fn cleanup_client_calls(&self, peer_id: impl Into<String>) {
        let peer_id = peer_id.into();

        self.call_attempts.write().retain(|call, guard| {
            if call.0 == peer_id || call.1 == peer_id {
                guard.set_outcome(CallAttemptOutcome::Aborted);
                false
            } else {
                true
            }
        });

        self.active_calls
            .write()
            .retain(|call, _| call.0 != peer_id && call.1 != peer_id)
    }
}

impl Default for CallStateManager {
    fn default() -> Self {
        Self {
            call_attempts: RwLock::new(HashMap::new()),
            active_calls: RwLock::new(HashMap::new()),
        }
    }
}
