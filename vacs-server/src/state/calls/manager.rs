use crate::metrics::ErrorMetrics;
use crate::metrics::guards::CallAttemptOutcome;
use crate::state::AppState;
use crate::state::calls::{ActiveCall, ActiveCallEntry, RingingCall, RingingCallEntry};
use parking_lot::RwLock;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use tracing::instrument;
use vacs_protocol::vatsim::ClientId;
use vacs_protocol::ws::server;
use vacs_protocol::ws::server::CallCancelReason;
use vacs_protocol::ws::shared::{CallEnd, CallId, CallTarget};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartCallError {
    CallerBusy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallTerminationOutcome {
    CallNotFound,
    ClientNotNotified,
    Continued,
    Failed(RingingCall),
}

pub struct CallManager {
    ringing_calls: RwLock<HashMap<CallId, RingingCallEntry>>,
    active_calls: RwLock<HashMap<CallId, ActiveCallEntry>>,
    client_incoming_calls: RwLock<HashMap<ClientId, HashSet<CallId>>>,
    client_outgoing_calls: RwLock<HashMap<ClientId, CallId>>,
}

impl Default for CallManager {
    fn default() -> Self {
        CallManager::new()
    }
}

impl std::fmt::Debug for CallManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallStateManager")
            .field("ringing_calls", &self.ringing_calls.read().len())
            .field("active_calls", &self.active_calls.read().len())
            .finish()
    }
}

impl CallManager {
    pub fn new() -> Self {
        Self {
            ringing_calls: RwLock::new(HashMap::new()),
            active_calls: RwLock::new(HashMap::new()),
            client_incoming_calls: RwLock::new(HashMap::new()),
            client_outgoing_calls: RwLock::new(HashMap::new()),
        }
    }

    pub fn has_outgoing_call(&self, client_id: &ClientId) -> bool {
        self.client_outgoing_calls.read().contains_key(client_id)
    }

    pub fn has_active_call(&self, call_id: &CallId, client_id: &ClientId) -> bool {
        self.active_calls
            .read()
            .get(call_id)
            .is_some_and(|active| active.involves(client_id))
    }

    pub fn ringing_call(&self, call_id: &CallId) -> Option<RingingCall> {
        self.ringing_calls.read().get(call_id).map(Into::into)
    }

    pub fn active_call(&self, call_id: &CallId) -> Option<ActiveCall> {
        self.active_calls.read().get(call_id).map(Into::into)
    }

    /// Number of currently active calls. This is the value the `vacs_calls_active` gauge tracks,
    /// since every entry holds a [`CallGuard`](crate::metrics::guards::CallGuard).
    pub fn active_call_count(&self) -> usize {
        self.active_calls.read().len()
    }

    pub fn start_call_attempt(
        &self,
        call_id: &CallId,
        caller_id: &ClientId,
        target: &CallTarget,
        notified_clients: &HashSet<ClientId>,
    ) -> Result<(), StartCallError> {
        if self.has_outgoing_call(caller_id) {
            tracing::warn!("Client already has outgoing call");
            return Err(StartCallError::CallerBusy);
        }

        let ringing = RingingCallEntry::new(
            *call_id,
            caller_id.clone(),
            target.clone(),
            notified_clients.clone(),
        );

        self.ringing_calls.write().insert(*call_id, ringing);
        self.client_outgoing_calls
            .write()
            .insert(caller_id.clone(), *call_id);

        let mut client_incoming_calls = self.client_incoming_calls.write();
        for client_id in notified_clients {
            client_incoming_calls
                .entry(client_id.clone())
                .or_default()
                .insert(*call_id);
        }

        Ok(())
    }

    pub fn reject_call(
        &self,
        call_id: &CallId,
        rejecting_client_id: &ClientId,
    ) -> CallTerminationOutcome {
        self.remove_client_incoming_call(call_id, rejecting_client_id);

        let mut ringing_calls = self.ringing_calls.write();
        match ringing_calls.entry(*call_id) {
            Entry::Occupied(mut entry) => {
                if !entry.get().has_notified_client(rejecting_client_id) {
                    return CallTerminationOutcome::ClientNotNotified;
                }

                if entry.get_mut().mark_rejected(rejecting_client_id) {
                    let ringing = entry.remove();
                    drop(ringing_calls);
                    self.cleanup_ringing_call(&ringing);
                    CallTerminationOutcome::Failed(ringing.complete(CallAttemptOutcome::Rejected))
                } else {
                    CallTerminationOutcome::Continued
                }
            }
            Entry::Vacant(_) => CallTerminationOutcome::CallNotFound,
        }
    }

    pub fn call_error(
        &self,
        call_id: &CallId,
        erroring_client_id: &ClientId,
    ) -> CallTerminationOutcome {
        self.remove_client_incoming_call(call_id, erroring_client_id);

        let mut ringing_calls = self.ringing_calls.write();
        match ringing_calls.entry(*call_id) {
            Entry::Occupied(mut entry) => {
                if !entry.get().has_notified_client(erroring_client_id) {
                    return CallTerminationOutcome::ClientNotNotified;
                }

                if entry.get_mut().mark_errored(erroring_client_id) {
                    let ringing = entry.remove();
                    drop(ringing_calls);
                    self.cleanup_ringing_call(&ringing);
                    // TODO: should we allow passing strict error reason here?
                    CallTerminationOutcome::Failed(ringing.complete(CallAttemptOutcome::Error(
                        vacs_protocol::ws::shared::CallErrorReason::CallFailure,
                    )))
                } else {
                    CallTerminationOutcome::Continued
                }
            }
            Entry::Vacant(_) => CallTerminationOutcome::CallNotFound,
        }
    }

    pub fn accept_call(
        &self,
        call_id: &CallId,
        accepting_client_id: &ClientId,
    ) -> Option<RingingCall> {
        let ringing = {
            let mut ringing_calls = self.ringing_calls.write();
            match ringing_calls.entry(*call_id) {
                Entry::Occupied(entry) if entry.get().has_notified_client(accepting_client_id) => {
                    Some(entry.remove())
                }
                _ => None,
            }
        }?;

        self.cleanup_ringing_call(&ringing);

        let active = ActiveCallEntry::new(
            *call_id,
            ringing.caller_id.clone(),
            accepting_client_id.clone(),
        );

        self.active_calls.write().insert(*call_id, active);

        Some(ringing.complete(CallAttemptOutcome::Accepted))
    }

    pub fn cancel_ringing_call(
        &self,
        call_id: &CallId,
        cancelling_client_id: &ClientId,
        outcome: CallAttemptOutcome,
    ) -> Option<RingingCall> {
        let ringing = {
            let mut ringing_calls = self.ringing_calls.write();
            match ringing_calls.entry(*call_id) {
                Entry::Occupied(entry) if entry.get().involves(cancelling_client_id) => {
                    Some(entry.remove())
                }
                _ => None,
            }
        }?;

        self.cleanup_ringing_call(&ringing);

        Some(ringing.complete(outcome))
    }

    pub fn end_ringing_call(
        &self,
        call_id: &CallId,
        cancelling_client_id: &ClientId,
    ) -> Option<RingingCall> {
        let ringing = {
            let mut ringing_calls = self.ringing_calls.write();
            match ringing_calls.entry(*call_id) {
                Entry::Occupied(entry) if entry.get().caller_id == *cancelling_client_id => {
                    Some(entry.remove())
                }
                _ => None,
            }
        }?;

        self.cleanup_ringing_call(&ringing);

        Some(ringing.complete(CallAttemptOutcome::Cancelled))
    }

    pub fn end_active_call(
        &self,
        call_id: &CallId,
        ending_client_id: &ClientId,
    ) -> Option<ActiveCall> {
        let active = {
            let mut active_calls = self.active_calls.write();
            match active_calls.entry(*call_id) {
                Entry::Occupied(entry) if entry.get().involves(ending_client_id) => {
                    Some(entry.remove())
                }
                _ => None,
            }
        }?;

        Some(ActiveCall::from(active))
    }

    #[instrument(level = "trace", skip(self, state))]
    pub async fn cleanup_client_calls(&self, state: &AppState, client_id: &ClientId) {
        tracing::trace!("Cleaning up client calls");

        let mut cleaned_ringing_calls: Vec<RingingCall> = Vec::new();

        let outgoing_call_id = { self.client_outgoing_calls.write().remove(client_id) };
        if let Some(outgoing_call_id) = outgoing_call_id {
            let ringing = { self.ringing_calls.write().remove(&outgoing_call_id) };
            if let Some(ringing) = ringing {
                {
                    let mut client_incoming_calls = self.client_incoming_calls.write();
                    for callee_id in ringing.notified_clients.iter() {
                        if let Some(calls) = client_incoming_calls.get_mut(callee_id) {
                            calls.remove(&outgoing_call_id);
                            if calls.is_empty() {
                                client_incoming_calls.remove(callee_id);
                            }
                        }
                    }
                }

                tracing::trace!(?outgoing_call_id, "Aborting outgoing ringing call");
                cleaned_ringing_calls.push(ringing.complete(CallAttemptOutcome::Aborted)); // TODO other outcome?
            }
        }

        let incoming_call_ids = { self.client_incoming_calls.write().remove(client_id) };
        if let Some(incoming_call_ids) = incoming_call_ids {
            let mut ringing_calls = self.ringing_calls.write();
            let mut removed_call_ids = Vec::new();

            for call_id in incoming_call_ids {
                if let Some(ringing) = ringing_calls.get_mut(&call_id) {
                    ringing.notified_clients.remove(client_id);
                    ringing.rejected_clients.remove(client_id);
                    ringing.errored_clients.remove(client_id);

                    tracing::trace!(
                        ?call_id,
                        ?ringing,
                        "Removing client from incoming ringing call"
                    );

                    if ringing.all_rejected_or_errored() {
                        tracing::trace!(?call_id, "Aborting incoming ringing call");
                        ringing.set_outcome(CallAttemptOutcome::Aborted); // TODO other outcome?
                        cleaned_ringing_calls.push(ringing.to_ringing_call());
                        removed_call_ids.push(call_id);
                    }
                }
            }

            for call_id in removed_call_ids {
                ringing_calls.remove(&call_id);
            }
        }

        let cleaned_active_calls = self.remove_active_calls_of(client_id);

        for ringing in cleaned_ringing_calls {
            self.client_outgoing_calls
                .write()
                .remove(&ringing.caller_id);

            if ringing.caller_id == *client_id {
                let cancelled =
                    server::CallCancelled::new(ringing.call_id, CallCancelReason::CallerCancelled);
                for callee_id in ringing.notified_clients {
                    tracing::trace!(?callee_id, "Sending call cancelled to notified client");
                    if let Err(err) = state.send_message(&callee_id, cancelled.clone()).await {
                        tracing::warn!(
                            ?err,
                            ?callee_id,
                            "Failed to send call cancelled to notified client"
                        );
                    }
                }
            } else {
                tracing::trace!(
                    "All notified clients either rejected or errored, call failed, sending call error to source client"
                );
                // TODO send CallCancelled to all notified, just in case?
                if let Err(err) = state
                    .send_message(
                        &ringing.caller_id,
                        server::CallCancelled::new(ringing.call_id, CallCancelReason::Disconnected),
                    )
                    .await
                {
                    tracing::warn!(?err, "Failed to send call error to source client");
                }
            }
        }

        for active in cleaned_active_calls {
            // Unreachable: the calls collected above all involve this client, so they all have a
            // peer. Kept as an invariant tripwire rather than an unwrap
            let Some(peer_id) = active.peer(client_id) else {
                ErrorMetrics::peer_not_found();
                tracing::warn!(call_id = ?active.call_id, "No peer found for active call");
                continue;
            };

            tracing::trace!(?peer_id, "Sending call end to peer");
            if let Err(err) = state
                .send_message(peer_id, CallEnd::new(active.call_id, peer_id.clone()))
                .await
            {
                tracing::warn!(?err, ?peer_id, "Failed to send call end to peer");
            }
        }
    }

    /// Removes every active call the client takes part in.
    ///
    /// A client is meant to be in one call at a time, but the server does not enforce it:
    /// `start_call_attempt` only rejects a second *ringing* outgoing call, and acceptance is not
    /// checked against the calls already running. Draining every match keeps the bookkeeping
    /// honest if a client ends up in two anyway, because a call left behind here can never be
    /// reached again once its other party is gone too, and its
    /// [`CallGuard`](crate::metrics::guards::CallGuard) then keeps `vacs_calls_active` above zero
    /// for the rest of the server's lifetime.
    fn remove_active_calls_of(&self, client_id: &ClientId) -> Vec<ActiveCall> {
        let mut active_calls = self.active_calls.write();
        let call_ids: Vec<CallId> = active_calls
            .iter()
            .filter(|(_, active)| active.involves(client_id))
            .map(|(call_id, _)| *call_id)
            .collect();

        call_ids
            .into_iter()
            .filter_map(|call_id| active_calls.remove(&call_id))
            .map(ActiveCall::from)
            .collect()
    }

    fn remove_client_incoming_call(&self, call_id: &CallId, client_id: &ClientId) {
        let mut client_incoming_calls = self.client_incoming_calls.write();
        if let Some(calls) = client_incoming_calls.get_mut(client_id) {
            calls.remove(call_id);
            if calls.is_empty() {
                client_incoming_calls.remove(client_id);
            }
        }
    }

    fn cleanup_ringing_call(&self, ringing: &RingingCallEntry) {
        self.client_outgoing_calls
            .write()
            .remove(&ringing.caller_id);

        let mut client_incoming_calls = self.client_incoming_calls.write();
        for callee_id in ringing.notified_clients.iter() {
            if let Some(calls) = client_incoming_calls.get_mut(callee_id) {
                calls.remove(&ringing.call_id);
                if calls.is_empty() {
                    client_incoming_calls.remove(callee_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::{TestSetup, create_client_info};
    use pretty_assertions::assert_eq;
    use test_log::test;
    use vacs_protocol::ws::server::ServerMessage;

    /// Establishes an active call from `caller` to `callee` and returns its id.
    fn establish_call(calls: &CallManager, caller: &ClientId, callee: &ClientId) -> CallId {
        let call_id = CallId::new();
        calls
            .start_call_attempt(
                &call_id,
                caller,
                &CallTarget::Client(callee.clone()),
                &HashSet::from([callee.clone()]),
            )
            .expect("Failed to start call attempt");
        calls
            .accept_call(&call_id, callee)
            .expect("Failed to accept call");
        call_id
    }

    #[test(tokio::test)]
    async fn cleanup_client_calls_removes_every_active_call_of_the_client() {
        let setup = TestSetup::new();
        let state = setup.app_state.clone();
        let (client1, _rx1) = setup.register_client(create_client_info(1)).await;
        let (client2, mut rx2) = setup.register_client(create_client_info(2)).await;
        let (client3, mut rx3) = setup.register_client(create_client_info(3)).await;

        // The desktop client never gets here, but the server accepts a second call for a client
        // that already has one, so the cleanup has to cope with it
        establish_call(&state.calls, client1.id(), client2.id());
        establish_call(&state.calls, client1.id(), client3.id());
        assert_eq!(state.calls.active_call_count(), 2);

        state.calls.cleanup_client_calls(&state, client1.id()).await;

        assert_eq!(
            state.calls.active_call_count(),
            0,
            "Disconnecting a client must not leave any of its calls active"
        );
        assert!(
            matches!(rx2.try_recv(), Ok(ServerMessage::CallEnd(_))),
            "Peer of the first call should be told the call ended"
        );
        assert!(
            matches!(rx3.try_recv(), Ok(ServerMessage::CallEnd(_))),
            "Peer of the second call should be told the call ended"
        );
    }

    #[test(tokio::test)]
    async fn ending_one_call_keeps_the_clients_other_call_cleanable() {
        let setup = TestSetup::new();
        let state = setup.app_state.clone();
        let (client1, _rx1) = setup.register_client(create_client_info(1)).await;
        let (client2, _rx2) = setup.register_client(create_client_info(2)).await;
        let (client3, _rx3) = setup.register_client(create_client_info(3)).await;

        let call_1 = establish_call(&state.calls, client1.id(), client2.id());
        establish_call(&state.calls, client1.id(), client3.id());

        state
            .calls
            .end_active_call(&call_1, client2.id())
            .expect("Failed to end first call");
        assert_eq!(state.calls.active_call_count(), 1);

        state.calls.cleanup_client_calls(&state, client1.id()).await;

        assert_eq!(
            state.calls.active_call_count(),
            0,
            "Ending one call must not make the client's other call uncleanable"
        );
    }

    #[test(tokio::test)]
    async fn cleanup_client_calls_is_idempotent() {
        let setup = TestSetup::new();
        let state = setup.app_state.clone();
        let (client1, _rx1) = setup.register_client(create_client_info(1)).await;
        let (client2, _rx2) = setup.register_client(create_client_info(2)).await;

        establish_call(&state.calls, client1.id(), client2.id());

        state.calls.cleanup_client_calls(&state, client1.id()).await;
        state.calls.cleanup_client_calls(&state, client2.id()).await;

        assert_eq!(state.calls.active_call_count(), 0);
    }
}
