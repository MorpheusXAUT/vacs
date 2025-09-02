use crate::app::state::http::AppStateHttpExt;
use crate::app::state::webrtc::AppStateWebrtcExt;
use crate::app::state::{sealed, AppState, AppStateInner};
use crate::audio::manager::SourceType;
use crate::config::BackendEndpoint;
use crate::error::{Error, FrontendError};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;
use vacs_protocol::http::ws::WebSocketToken;
use vacs_protocol::ws::{LoginFailureReason, SignalingMessage};
use vacs_signaling::error::SignalingError;

pub trait AppStateSignalingExt: sealed::Sealed {
    async fn connect_signaling(&mut self, app: &AppHandle) -> Result<(), Error>;
    async fn disconnect_signaling(&mut self, app: &AppHandle);
    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error>;
    fn set_outgoing_call_peer_id(&mut self, peer_id: Option<String>);
    fn remove_outgoing_call_peer_id(&mut self, peer_id: &str) -> bool;
    fn incoming_call_peer_ids_len(&self) -> usize;
    fn add_incoming_call_peer_id(&mut self, peer_id: &str);
    fn remove_incoming_call_peer_id(&mut self, peer_id: &str) -> bool;
}

impl AppStateSignalingExt for AppStateInner {
    async fn connect_signaling(&mut self, app: &AppHandle) -> Result<(), Error> {
        log::info!("Connecting to signaling server");

        if self.connection.is_logged_in() {
            log::info!("Already connected and logged in with signaling server");
            return Err(Error::Signaling(Box::from(SignalingError::LoginError(
                LoginFailureReason::DuplicateId,
            ))));
        }

        log::debug!("Retrieving WebSocket auth token");
        let token = self
            .http_get::<WebSocketToken>(BackendEndpoint::WsToken, None)
            .await?
            .token;

        log::debug!("Connecting to signaling server");
        let (disconnect_tx, disconnect_rx) = oneshot::channel();
        self.connection
            .connect(
                app.clone(),
                self.config.backend.ws_url.as_str(),
                token.as_str(),
                disconnect_tx,
            )
            .await?;

        let app_clone = app.clone();
        tauri::async_runtime::spawn(async move {
            let requested = disconnect_rx.await.unwrap_or(false);

            log::debug!("Signaling connection task ended, cleaning up state");
            app_clone
                .state::<AppState>()
                .lock()
                .await
                .handle_signaling_connection_closed(&app_clone, requested)
                .await;
            log::debug!("Finished cleaning up state after signaling connection task ended");
        });

        log::info!("Successfully connected to signaling server");
        Ok(())
    }

    async fn disconnect_signaling(&mut self, app: &AppHandle) {
        log::info!("Disconnecting from signaling server");

        if !self.connection.is_connected() {
            log::info!("Tried to disconnection from signaling server, but not connected");
            return;
        }

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

        self.connection.disconnect();
        app.emit("signaling:disconnected", Value::Null).ok();
        log::debug!("Successfully disconnected from signaling server");
    }

    async fn send_signaling_message(&mut self, msg: SignalingMessage) -> Result<(), Error> {
        log::trace!("Sending signaling message: {msg:?}");

        if !self.connection.is_logged_in() {
            log::warn!("Not logged in with signaling server, cannot send message");
            return Err(Error::Network("Not connected".to_string()));
        };

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
}

impl AppStateInner {
    async fn handle_signaling_connection_closed(&mut self, app: &AppHandle, requested: bool) {
        log::info!("Handling closed signaling server connection, requested: {requested}");

        app.emit("signaling:disconnected", Value::Null).ok();
        if !requested {
            app.emit::<FrontendError>(
                "error",
                Error::Network("Disconnected from websocket connection".to_string()).into(),
            )
            .ok();
        }
        log::debug!("Successfully handled closed signaling server connection");
    }
}
