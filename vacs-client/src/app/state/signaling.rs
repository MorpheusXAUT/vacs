use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::app::state::{AppStateInner, sealed};
use crate::audio::manager::SourceType;
use crate::error::Error;
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter};
use vacs_signaling::client::State;
use vacs_signaling::error::SignalingError;
use vacs_signaling::protocol::ws::SignalingMessage;

pub trait AppStateSignalingExt: sealed::Sealed {
    async fn connect_signaling(&self) -> Result<(), Error>;
    async fn disconnect_signaling(&mut self, app: &AppHandle);
    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle);
    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error>;
    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<String>);
    fn remove_outgoing_call_peer_id(&mut self, peer_id: &str) -> bool;
    fn incoming_call_peer_ids_len(&self) -> usize;
    fn add_incoming_call_peer_id(&mut self, peer_id: &str);
    fn remove_incoming_call_peer_id(&mut self, peer_id: &str) -> bool;
    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &str, incoming: bool);
}

impl AppStateSignalingExt for AppStateInner {
    async fn connect_signaling(&self) -> Result<(), Error> {
        log::info!("Connecting to signaling server");

        if self.connection.state() != State::Disconnected {
            log::info!("Already connected and logged in with signaling server");
            return Err(Error::Signaling(Box::from(SignalingError::Other(
                "Already connected".to_string(),
            ))));
        }

        log::debug!("Connecting to signaling server");
        self.connection.connect().await?;

        log::info!("Successfully connected to signaling server");
        Ok(())
    }

    async fn disconnect_signaling(&mut self, app: &AppHandle) {
        log::info!("Disconnecting from signaling server");

        if self.connection.state() == State::Disconnected {
            log::info!("Tried to disconnect from signaling server, but not connected");
            return;
        }

        self.cleanup_signaling(app).await;

        self.connection.disconnect().await;
        app.emit("signaling:disconnected", Value::Null).ok();
        log::debug!("Successfully disconnected from signaling server");
    }

    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle) {
        log::info!("Handling signaling server connection closed");

        self.cleanup_signaling(app).await;

        app.emit("signaling:disconnected", Value::Null).ok();
        log::debug!("Successfully handled closed signaling server connection");
    }

    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error> {
        log::trace!("Sending signaling message: {msg:?}");

        if let Err(err) = self.connection.send(msg).await {
            log::warn!("Failed to send signaling message: {err:?}");
            return Err(err.into());
        }

        log::trace!("Successfully sent signaling message");
        Ok(())
    }

    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<String>) {
        self.outgoing_call_peer_id = peer_id;
    }

    fn remove_outgoing_call_peer_id(&mut self, peer_id: &str) -> bool {
        if let Some(id) = &self.outgoing_call_peer_id
            && id == peer_id
        {
            self.outgoing_call_peer_id = None;
            self.audio_manager.stop(SourceType::Ringback);
            true
        } else {
            false
        }
    }

    fn incoming_call_peer_ids_len(&self) -> usize {
        self.incoming_call_peer_ids.len()
    }

    fn add_incoming_call_peer_id(&mut self, peer_id: &str) {
        self.incoming_call_peer_ids.insert(peer_id.to_string());
    }

    fn remove_incoming_call_peer_id(&mut self, peer_id: &str) -> bool {
        let found = self.incoming_call_peer_ids.remove(peer_id);
        if self.incoming_call_peer_ids.is_empty() {
            self.audio_manager.stop(SourceType::Ring);
        }
        found
    }

    fn add_call_to_call_list(&mut self, app: &AppHandle, peer_id: &str, incoming: bool) {
        #[derive(Clone, Serialize)]
        #[serde(rename_all = "camelCase")]
        struct CallListEntry<'a> {
            peer_id: &'a str,
            incoming: bool,
        }

        app.emit(
            "signaling:add-to-call-list",
            CallListEntry { peer_id, incoming },
        )
        .ok();
    }
}

impl AppStateInner {
    async fn cleanup_signaling(&mut self, app: &AppHandle) {
        self.incoming_call_peer_ids.clear();
        self.outgoing_call_peer_id = None;

        self.audio_manager.stop(SourceType::Ring);
        self.audio_manager.stop(SourceType::Ringback);

        self.audio_manager.detach_call_output();
        self.audio_manager.detach_input_device();

        if let Some(peer_id) = self.active_call_peer_id().cloned() {
            self.end_call(&peer_id).await;
        };
        let peer_ids = self.held_calls.keys().cloned().collect::<Vec<_>>();
        for peer_id in peer_ids {
            self.end_call(&peer_id).await;
            app.emit("signaling:call-end", &peer_id).ok();
        }
    }
}
