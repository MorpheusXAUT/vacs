mod manager;
pub use manager::*;

use crate::metrics::guards::{CallAttemptGuard, CallAttemptOutcome, CallGuard};
use std::collections::HashSet;
use vacs_protocol::vatsim::ClientId;
use vacs_protocol::ws::shared::{CallId, CallTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RingingCall {
    pub call_id: CallId,
    pub caller_id: ClientId,
    pub target: CallTarget,
    pub notified_clients: HashSet<ClientId>,
}

#[derive(Debug)]
struct RingingCallEntry {
    call_id: CallId,
    caller_id: ClientId,
    target: CallTarget,
    notified_clients: HashSet<ClientId>,
    rejected_clients: HashSet<ClientId>,
    errored_clients: HashSet<ClientId>,
    guard: CallAttemptGuard,
}

#[derive(Debug, Clone)]
pub struct ActiveCall {
    pub call_id: CallId,
    pub caller_id: ClientId,
    pub callee_id: ClientId,
}

#[derive(Debug)]
struct ActiveCallEntry {
    call_id: CallId,
    caller_id: ClientId,
    callee_id: ClientId,
    _guard: CallGuard,
}

impl RingingCallEntry {
    pub fn new(
        call_id: CallId,
        caller_id: ClientId,
        target: CallTarget,
        notified_clients: HashSet<ClientId>,
    ) -> Self {
        Self {
            call_id,
            caller_id,
            target,
            notified_clients,
            rejected_clients: HashSet::new(),
            errored_clients: HashSet::new(),
            guard: CallAttemptGuard::new(),
        }
    }

    pub fn has_notified_client(&self, client_id: &ClientId) -> bool {
        self.notified_clients.contains(client_id)
    }

    pub fn involves(&self, client_id: &ClientId) -> bool {
        self.caller_id == *client_id || self.notified_clients.contains(client_id)
    }

    pub fn mark_rejected(&mut self, client_id: &ClientId) -> bool {
        if !self.notified_clients.contains(client_id) {
            return false;
        }
        self.rejected_clients.insert(client_id.clone());
        self.all_rejected_or_errored()
    }

    pub fn mark_errored(&mut self, client_id: &ClientId) -> bool {
        if !self.notified_clients.contains(client_id) {
            return false;
        }
        self.errored_clients.insert(client_id.clone());
        self.all_rejected_or_errored()
    }

    pub fn set_outcome(&mut self, outcome: CallAttemptOutcome) {
        self.guard.set_outcome(outcome);
    }

    pub fn complete(mut self, outcome: CallAttemptOutcome) -> RingingCall {
        self.set_outcome(outcome);
        RingingCall::from(self)
    }

    pub fn to_ringing_call(&self) -> RingingCall {
        RingingCall::from(self)
    }

    fn all_rejected_or_errored(&self) -> bool {
        self.rejected_clients.len() + self.errored_clients.len() >= self.notified_clients.len()
    }
}

impl From<RingingCallEntry> for RingingCall {
    fn from(value: RingingCallEntry) -> Self {
        Self {
            call_id: value.call_id,
            caller_id: value.caller_id,
            target: value.target,
            notified_clients: value.notified_clients,
        }
    }
}

impl From<&RingingCallEntry> for RingingCall {
    fn from(value: &RingingCallEntry) -> Self {
        Self {
            call_id: value.call_id,
            caller_id: value.caller_id.clone(),
            target: value.target.clone(),
            notified_clients: value.notified_clients.clone(),
        }
    }
}

impl ActiveCall {
    pub fn peer(&self, client_id: &ClientId) -> Option<&ClientId> {
        if self.caller_id == *client_id {
            Some(&self.callee_id)
        } else if self.callee_id == *client_id {
            Some(&self.caller_id)
        } else {
            None
        }
    }

    pub fn involves(&self, client_id: &ClientId) -> bool {
        self.caller_id == *client_id || self.callee_id == *client_id
    }
}

impl ActiveCallEntry {
    pub fn new(call_id: CallId, caller_id: ClientId, callee_id: ClientId) -> Self {
        Self {
            call_id,
            caller_id,
            callee_id,
            _guard: CallGuard::new(),
        }
    }

    pub fn peer(&self, client_id: &ClientId) -> Option<&ClientId> {
        if self.caller_id == *client_id {
            Some(&self.callee_id)
        } else if self.callee_id == *client_id {
            Some(&self.caller_id)
        } else {
            None
        }
    }

    pub fn involves(&self, client_id: &ClientId) -> bool {
        self.caller_id == *client_id || self.callee_id == *client_id
    }
}

impl From<ActiveCallEntry> for ActiveCall {
    fn from(entry: ActiveCallEntry) -> Self {
        Self {
            call_id: entry.call_id,
            caller_id: entry.caller_id,
            callee_id: entry.callee_id,
        }
    }
}

impl From<&ActiveCallEntry> for ActiveCall {
    fn from(entry: &ActiveCallEntry) -> Self {
        Self {
            call_id: entry.call_id,
            caller_id: entry.caller_id.clone(),
            callee_id: entry.callee_id.clone(),
        }
    }
}
