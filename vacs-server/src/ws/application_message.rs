use crate::metrics::ErrorMetrics;
use crate::metrics::guards::CallAttemptOutcome;
use crate::state::AppState;
use crate::state::clients::session::ClientSession;
use std::collections::HashSet;
use std::ops::ControlFlow;
use std::sync::Arc;
use vacs_protocol::ws::client::{CallReject, ClientMessage};
use vacs_protocol::ws::server;
use vacs_protocol::ws::server::CallCancelReason;
use vacs_protocol::ws::shared::{
    CallAccept, CallEnd, CallError, CallErrorReason, CallId, CallInvite, CallTarget, ErrorReason,
    WebrtcAnswer, WebrtcIceCandidate, WebrtcOffer,
};

#[tracing::instrument(level = "trace", skip(state))]
pub async fn handle_application_message(
    state: &Arc<AppState>,
    client: &ClientSession,
    message: ClientMessage,
) -> ControlFlow<(), ()> {
    tracing::trace!("Handling application message");

    match message {
        ClientMessage::ListClients => {
            tracing::trace!("Returning list of clients");
            let clients = state.list_clients(Some(client.id())).await;
            if let Err(err) = client.send_message(server::ClientList { clients }).await {
                tracing::warn!(?err, "Failed to send client list");
            }
        }
        ClientMessage::ListStations => {
            tracing::trace!("Returning list of stations");
            let stations = state
                .clients
                .list_stations(client.active_profile(), client.position_id())
                .await;
            if let Err(err) = client.send_message(server::StationList { stations }).await {
                tracing::warn!(?err, "Failed to send station list");
            }
        }
        ClientMessage::Logout => {
            tracing::trace!("Logging out client");
            // TODO move higher to client manager?
            handle_client_disconnect(state, client).await;
            return ControlFlow::Break(());
        }
        ClientMessage::Disconnect => {
            tracing::trace!("Disconnecting client");
            // TODO move higher to client manager?
            handle_client_disconnect(state, client).await;
            return ControlFlow::Break(());
        }
        ClientMessage::CallInvite(call_invite) => {
            handle_call_invite(state, client, call_invite).await;
        }
        ClientMessage::CallAccept(call_accept) => {
            handle_call_accept(state, client, call_accept).await;
        }
        ClientMessage::CallReject(call_reject) => {
            handle_call_reject(state, client, call_reject).await;
        }
        ClientMessage::CallEnd(call_end) => {
            handle_call_end(state, client, call_end).await;
        }
        ClientMessage::CallError(call_error) => {
            handle_call_error(state, client, call_error).await;
        }
        ClientMessage::WebrtcOffer(webrtc_offer) => {
            handle_webrtc_offer(state, client, webrtc_offer).await;
        }
        ClientMessage::WebrtcAnswer(webrtc_answer) => {
            handle_webrtc_answer(state, client, webrtc_answer).await;
        }
        ClientMessage::WebrtcIceCandidate(webrtc_ice_candidate) => {
            handle_webrtc_ice_candidate(state, client, webrtc_ice_candidate).await;
        }
        ClientMessage::Login(_) => {}
        ClientMessage::Error(_) => {}
    };
    ControlFlow::Continue(())
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_call_invite(state: &AppState, client: &ClientSession, invite: CallInvite) {
    tracing::trace!("Handling call invite");
    let caller_id = client.id();
    let call_id = &invite.call_id;

    if let Err(until) = state.rate_limiters().check_call_invite(caller_id) {
        tracing::debug!(?until, "Rate limit exceeded, rejecting call invite");
        let reason = ErrorReason::RateLimited {
            retry_after_secs: until.as_secs(),
        };
        ErrorMetrics::error(&reason);
        client.send_error(reason).await;
        return;
    }

    if invite.source.client_id != *caller_id {
        tracing::debug!("Source client ID mismatch, rejecting call invite");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Source client ID mismatch"),
        )
        .await;
        return;
    }

    let target_clients = match &invite.target {
        CallTarget::Client(client_id) => {
            if state.clients.is_client_connected(client_id).await {
                HashSet::from([client_id.clone()])
            } else {
                HashSet::new()
            }
        }
        CallTarget::Position(position_id) => state.clients.clients_for_position(position_id).await,
        CallTarget::Station(station_id) => state.clients.clients_for_station(station_id).await,
    }
    .into_iter()
    .filter(|client_id| client_id != client.id())
    .collect::<HashSet<_>>();

    if target_clients.is_empty() {
        tracing::trace!("No clients found for call invite, returning target not found error");
        // TODO error metrics
        send_call_error(client, call_id, CallErrorReason::TargetNotFound, None).await;
        return;
    }

    if !state
        .calls
        .start_call_attempt(call_id, client.id(), &invite.target, &target_clients)
    {
        tracing::debug!("Client already has an outgoing call, rejecting call invite");
        // TODO error metrics
        send_call_error(client, call_id, CallErrorReason::CallActive, None).await;
        return;
    }

    for callee_id in target_clients {
        tracing::trace!(?callee_id, "Sending call invite to target");
        if let Err(err) = state.send_message(&callee_id, invite.clone()).await {
            tracing::warn!(?err, ?callee_id, "Failed to send call invite to target");
            // TODO error metrics
            if state
                .calls
                .mark_errored(call_id, &callee_id)
                .unwrap_or(true)
            {
                tracing::trace!(?callee_id, "All call attempts failed, returning call error");
                // TODO error metrics
                // TODO other call error reason?
                send_call_error(client, call_id, CallErrorReason::CallFailure, None).await;
                return;
            }
        }
    }
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_call_accept(state: &AppState, client: &ClientSession, accept: CallAccept) {
    tracing::trace!("Handling call acceptance");
    let answerer_id = client.id();
    let call_id = &accept.call_id;

    if accept.accepting_client_id != *answerer_id {
        tracing::debug!("Accepting client ID mismatch, rejecting call acceptance");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Accepting client ID mismatch"),
        )
        .await;
        return;
    }

    let Some(ringing) = state.calls.accept_call(call_id, answerer_id) else {
        tracing::warn!("No ringing call found, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::CallFailure, None).await;
        return;
    };

    tracing::trace!("Sending call accept to source client");
    if let Err(err) = state.send_message(&ringing.caller_id, accept.clone()).await {
        tracing::warn!(?err, "Failed to send call accept to source client");
        // TODO error metrics
        // TODO other error?
        client.send_error(err).await;
        return;
    }

    if ringing.notified_clients.len() > 1 {
        let cancelled = server::CallCancelled::new(
            *call_id,
            CallCancelReason::AnsweredElsewhere(answerer_id.clone()),
        );

        for callee_id in ringing.notified_clients {
            if callee_id == *answerer_id {
                continue;
            }

            tracing::trace!(
                ?callee_id,
                "Sending call cancelled to other notified client"
            );
            if let Err(err) = state.send_message(&callee_id, cancelled.clone()).await {
                // TODO error metrics
                tracing::warn!(
                    ?err,
                    ?callee_id,
                    "Failed to send call cancelled to other notified client"
                );
            }
        }
    }
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_call_reject(state: &AppState, client: &ClientSession, reject: CallReject) {
    tracing::trace!("Handling call rejection");
    let rejecter_id = client.id();
    let call_id = &reject.call_id;

    if reject.rejecting_client_id != *rejecter_id {
        tracing::debug!("Rejecting client ID mismatch, rejecting call rejection");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Rejecting client ID mismatch"),
        )
        .await;
        return;
    }

    let Some(failed) = state.calls.mark_rejected(call_id, rejecter_id) else {
        tracing::warn!("No ringing call found, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::CallFailure, None).await;
        return;
    };

    if failed
        && let Some(ringing) =
            state
                .calls
                .cancel_ringing_call(call_id, rejecter_id, CallAttemptOutcome::Rejected)
    {
        tracing::trace!(
            "All notified clients either rejected or errored, call failed, sending call error to source client"
        );
        // TODO error metrics
        // TODO other call error reason?
        // TODO send CallCancelled to all notified, just in case?
        if let Err(err) = state
            .send_message(
                &ringing.caller_id,
                server::CallCancelled::new(*call_id, CallCancelReason::AllFailed),
            )
            .await
        {
            tracing::warn!(?err, "Failed to send call error to source client");
        }
        return;
    }
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_call_end(state: &AppState, client: &ClientSession, end: CallEnd) {
    tracing::trace!("Handling call end");
    let ender_id = client.id();
    let call_id = &end.call_id;

    if end.ending_client_id != *ender_id {
        tracing::debug!("Ending client ID mismatch, rejecting call end");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Ending client ID mismatch"),
        )
        .await;
        return;
    }

    if let Some(ringing) = state.calls.end_ringing_call(call_id, ender_id) {
        tracing::trace!("Ringing call found, canceling");
        let cancelled = server::CallCancelled::new(*call_id, CallCancelReason::CallerCancelled);

        for callee_id in ringing.notified_clients {
            tracing::trace!(?callee_id, "Sending call cancelled to notified client");
            if let Err(err) = state.send_message(&callee_id, cancelled.clone()).await {
                // TODO error metrics
                tracing::warn!(
                    ?err,
                    ?callee_id,
                    "Failed to send call cancelled to notified client"
                );
            }
        }
    } else if let Some(active) = state.calls.end_active_call(call_id, ender_id) {
        tracing::trace!("Active call found, ending");
        if let Some(peer_id) = active.peer(ender_id) {
            tracing::trace!(?peer_id, "Sending call end to peer");
            if let Err(err) = state.send_message(peer_id, end.clone()).await {
                tracing::warn!(?err, ?peer_id, "Failed to send call end to peer");
                // TODO error metrics
                // TODO other call error reason?
                send_call_error(client, call_id, CallErrorReason::WebrtcFailure, None).await;
            }
        } else {
            tracing::warn!("No peer found for active call, returning call error");
            // TODO error metrics
            send_call_error(client, call_id, CallErrorReason::TargetNotFound, None).await;
            return;
        }
    } else {
        tracing::trace!("No ringing or active call found, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::TargetNotFound, None).await;
        return;
    }
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_call_error(state: &AppState, client: &ClientSession, error: CallError) {
    tracing::trace!("Handling call error");
    let erroring_id = client.id();
    let call_id = &error.call_id;

    let Some(failed) = state.calls.mark_errored(call_id, erroring_id) else {
        tracing::warn!("No ringing call found, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::CallFailure, None).await;
        return;
    };

    if failed
        && let Some(ringing) = state.calls.cancel_ringing_call(
            call_id,
            erroring_id,
            CallAttemptOutcome::Error(error.reason),
        )
    {
        tracing::trace!(
            "All notified clients either rejected or errored, call failed, sending call error to source client"
        );
        // TODO error metrics
        // TODO other call error reason?
        // TODO send CallCancelled to all notified, just in case?
        if let Err(err) = state
            .send_message(
                &ringing.caller_id,
                server::CallCancelled::new(*call_id, CallCancelReason::AllFailed),
            )
            .await
        {
            tracing::warn!(?err, "Failed to send call error to source client");
        }
        return;
    }
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_webrtc_offer(state: &AppState, client: &ClientSession, offer: WebrtcOffer) {
    tracing::trace!("Handling WebRTC offer");
    let client_id = client.id();
    let call_id = &offer.call_id;

    if offer.from_client_id != *client_id {
        tracing::debug!("Source client ID mismatch, rejecting WebRTC offer");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Source client ID mismatch"),
        )
        .await;
        return;
    }

    if !state.calls.has_active_call(call_id, client_id) {
        tracing::debug!("No active call found for WebRTC offer, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::SignalingFailure, None).await;
        return;
    }

    state
        .send_peer_message(client, &offer.to_client_id, offer.clone())
        .await;
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_webrtc_answer(state: &AppState, client: &ClientSession, answer: WebrtcAnswer) {
    tracing::trace!("Handling WebRTC answer");
    let client_id = client.id();
    let call_id = &answer.call_id;

    if answer.from_client_id != *client_id {
        tracing::debug!("Source client ID mismatch, rejecting WebRTC answer");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Source client ID mismatch"),
        )
        .await;
        return;
    }

    if !state.calls.has_active_call(call_id, client_id) {
        tracing::debug!("No active call found for WebRTC answer, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::SignalingFailure, None).await;
        return;
    }

    state
        .send_peer_message(client, &answer.to_client_id, answer.clone())
        .await;
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_webrtc_ice_candidate(
    state: &AppState,
    client: &ClientSession,
    ice_candidate: WebrtcIceCandidate,
) {
    tracing::trace!("Handling WebRTC ice candidate");
    let client_id = client.id();
    let call_id = &ice_candidate.call_id;

    if ice_candidate.from_client_id != *client_id {
        tracing::debug!("Source client ID mismatch, rejecting WebRTC ice candidate");
        // TODO error metrics
        send_call_error(
            client,
            call_id,
            CallErrorReason::Other,
            Some("Source client ID mismatch"),
        )
        .await;
        return;
    }

    if !state.calls.has_active_call(call_id, client_id) {
        tracing::debug!("No active call found for WebRTC ice candidate, returning call error");
        // TODO error metrics
        // TODO other call error reason?
        send_call_error(client, call_id, CallErrorReason::SignalingFailure, None).await;
        return;
    }

    state
        .send_peer_message(client, &ice_candidate.to_client_id, ice_candidate.clone())
        .await;
}

#[tracing::instrument(level = "trace", skip(state, client))]
async fn handle_client_disconnect(state: &AppState, client: &ClientSession) {
    tracing::trace!("Handling client disconnect");
    let client_id = client.id();

    let (ringing_calls, active) = state.calls.cleanup_client_calls(client_id);

    for ringing in ringing_calls {
        if ringing.caller_id == *client_id {
            let cancelled =
                server::CallCancelled::new(ringing.call_id, CallCancelReason::CallerCancelled);
            for callee_id in ringing.notified_clients {
                tracing::trace!(?callee_id, "Sending call cancelled to notified client");
                if let Err(err) = state.send_message(&callee_id, cancelled.clone()).await {
                    // TODO error metrics
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
            // TODO error metrics
            // TODO other call error reason?
            // TODO send CallCancelled to all notified, just in case?
            if let Err(err) = state
                .send_message(
                    &ringing.caller_id,
                    server::CallCancelled::new(ringing.call_id, CallCancelReason::AllFailed),
                )
                .await
            {
                tracing::warn!(?err, "Failed to send call error to source client");
            }
        }
    }

    if let Some(active) = active
        && let Some(peer_id) = active.peer(client_id)
    {
        tracing::trace!(?peer_id, "Sending call end to peer");
        if let Err(err) = state
            .send_message(peer_id, CallEnd::new(active.call_id, peer_id.clone()))
            .await
        {
            tracing::warn!(?err, ?peer_id, "Failed to send call end to peer");
            // TODO error metrics
        } else {
            tracing::warn!("No peer found for active call");
            // TODO error metrics
        }
    }
}

async fn send_call_error(
    client: &ClientSession,
    call_id: &CallId,
    reason: CallErrorReason,
    message: Option<&str>,
) {
    if let Err(err) = client
        .send_message(CallError {
            call_id: *call_id,
            reason,
            message: message.map(|m| m.to_string()),
        })
        .await
    {
        tracing::warn!(?err, "Failed to send call error message");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ws::test_util::{TestSetup, create_client_info};
    use axum::extract::ws;
    use axum::extract::ws::Utf8Bytes;
    use pretty_assertions::assert_eq;
    use std::ops::Deref;
    use test_log::test;
    use vacs_protocol::ws::LoginFailureReason;

    #[test(tokio::test)]
    async fn handle_application_message_list_clients_without_self() {
        let mut setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::ListClients,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = setup
            .take_last_websocket_message()
            .await
            .expect("No message received");
        assert_eq!(
            message,
            ws::Message::Text(Utf8Bytes::from_static(
                r#"{"type":"ClientList","clients":[]}"#
            ))
        );
    }

    #[test(tokio::test)]
    async fn handle_application_message_list_stations() {
        let mut setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::ListStations,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = setup
            .take_last_websocket_message()
            .await
            .expect("No message received");
        assert_eq!(
            message,
            ws::Message::Text(Utf8Bytes::from_static(
                r#"{"type":"StationList","stations":[]}"#
            ))
        );
    }

    #[test(tokio::test)]
    async fn handle_application_message_list_clients() {
        let mut setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;
        setup.register_client(create_client_info(2)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::ListClients,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = setup
            .take_last_websocket_message()
            .await
            .expect("No message received");
        assert_eq!(
            message,
            ws::Message::Text(Utf8Bytes::from_static(
                r#"{"type":"ClientList","clients":[{"id":"client2","positionId":"POSITION2","displayName":"Client 2","frequency":"200.000"}]}"#
            ))
        );
    }

    #[test(tokio::test)]
    async fn handle_application_message_logout() {
        let setup = TestSetup::new();
        setup.register_client(create_client_info(1)).await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::Logout,
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Break(()));
    }

    #[test(tokio::test)]
    async fn handle_application_message_call_offer() {
        let setup = TestSetup::new();
        let client_info_1 = create_client_info(1);
        let client_info_2 = create_client_info(2);
        let mut clients = setup
            .register_clients(vec![client_info_1, client_info_2])
            .await;

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::CallOffer {
                client_id: ClientId::from("client2"),
                sdp: "sdp1".to_string(),
            },
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));

        let message = clients
            .get_mut("client2")
            .unwrap()
            .1
            .recv()
            .await
            .expect("Failed to receive message");
        assert_eq!(
            message,
            SignalingMessage::CallOffer {
                client_id: ClientId::from("client1"),
                sdp: "sdp1".to_string()
            }
        );
    }

    #[test(tokio::test)]
    async fn handle_application_message_unknown() {
        let setup = TestSetup::new();

        let control_flow = handle_application_message(
            &setup.app_state,
            &setup.session,
            setup.websocket_tx.lock().await.deref(),
            SignalingMessage::LoginFailure {
                reason: LoginFailureReason::DuplicateId,
            },
        )
        .await;
        assert_eq!(control_flow, ControlFlow::Continue(()));
    }

    #[test(tokio::test)]
    async fn check_self_message_allows_regular_message() {
        let setup = TestSetup::new();

        let is_self_message = check_self_message(
            setup.websocket_tx.lock().await.deref(),
            &setup.session,
            &ClientId::from("client2"),
        )
        .await;
        assert_eq!(is_self_message, false);
    }

    #[test(tokio::test)]
    async fn check_self_message_rejects_message_to_self() {
        let setup = TestSetup::new();

        let is_self_message = check_self_message(
            setup.websocket_tx.lock().await.deref(),
            &setup.session,
            &ClientId::from("client1"),
        )
        .await;
        assert_eq!(is_self_message, true);
    }
}
