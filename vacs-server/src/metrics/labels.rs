use crate::metrics::guards::CallAttemptOutcome;
use crate::release::catalog::BundleType;
use vacs_protocol::http::version::ReleaseChannel;
use vacs_protocol::ws::{
    CallErrorReason, DisconnectReason, ErrorReason, LoginFailureReason, SignalingMessage,
};

pub trait AsMetricLabel {
    fn as_metric_label(&self) -> &'static str;
}

impl AsMetricLabel for DisconnectReason {
    fn as_metric_label(&self) -> &'static str {
        match self {
            DisconnectReason::Terminated => "terminated",
            DisconnectReason::NoActiveVatsimConnection => "no_active_vatsim_connection",
            DisconnectReason::AmbiguousVatsimPosition(_) => "ambiguous_vatsim_position",
        }
    }
}

impl AsMetricLabel for Option<DisconnectReason> {
    fn as_metric_label(&self) -> &'static str {
        match self {
            Some(reason) => reason.as_metric_label(),
            None => "graceful",
        }
    }
}

impl AsMetricLabel for LoginFailureReason {
    fn as_metric_label(&self) -> &'static str {
        match self {
            LoginFailureReason::Unauthorized => "unauthorized",
            LoginFailureReason::DuplicateId => "duplicate_id",
            LoginFailureReason::InvalidCredentials => "invalid_credentials",
            LoginFailureReason::NoActiveVatsimConnection => "no_active_vatsim_connection",
            LoginFailureReason::AmbiguousVatsimPosition(_) => "ambiguous_vatsim_position",
            LoginFailureReason::InvalidVatsimPosition => "invalid_vatsim_position",
            LoginFailureReason::Timeout => "timeout",
            LoginFailureReason::IncompatibleProtocolVersion => "incompatible_protocol_version",
        }
    }
}

impl AsMetricLabel for CallAttemptOutcome {
    fn as_metric_label(&self) -> &'static str {
        match self {
            CallAttemptOutcome::Accepted => "accepted",
            CallAttemptOutcome::Rejected => "rejected",
            CallAttemptOutcome::Cancelled => "cancelled",
            CallAttemptOutcome::Aborted => "aborted",
            CallAttemptOutcome::Error(CallErrorReason::AudioFailure) => "error_audio_failure",
            CallAttemptOutcome::Error(CallErrorReason::AutoHangup) => "error_auto_hangup",
            CallAttemptOutcome::Error(CallErrorReason::WebrtcFailure) => "error_webrtc_failure",
            CallAttemptOutcome::Error(CallErrorReason::CallFailure) => "error_call_failure",
            CallAttemptOutcome::Error(CallErrorReason::SignalingFailure) => {
                "error_signaling_failure"
            }
            CallAttemptOutcome::Error(CallErrorReason::Other) => "error_other",
        }
    }
}

impl AsMetricLabel for Option<CallAttemptOutcome> {
    fn as_metric_label(&self) -> &'static str {
        match self {
            Some(outcome) => outcome.as_metric_label(),
            None => "aborted",
        }
    }
}

impl AsMetricLabel for ReleaseChannel {
    fn as_metric_label(&self) -> &'static str {
        self.as_str()
    }
}

impl AsMetricLabel for BundleType {
    fn as_metric_label(&self) -> &'static str {
        self.as_str()
    }
}

impl AsMetricLabel for SignalingMessage {
    fn as_metric_label(&self) -> &'static str {
        match self {
            SignalingMessage::Login { .. } => "login",
            SignalingMessage::LoginFailure { .. } => "login_failure",
            SignalingMessage::Logout => "logout",
            SignalingMessage::CallInvite { .. } => "call_invite",
            SignalingMessage::ClientInfo { .. } => "client_info",
            SignalingMessage::SessionInfo { .. } => "session_info",
            SignalingMessage::CallAccept { .. } => "call_accept",
            SignalingMessage::CallReject { .. } => "call_reject",
            SignalingMessage::CallOffer { .. } => "call_offer",
            SignalingMessage::CallAnswer { .. } => "call_answer",
            SignalingMessage::CallEnd { .. } => "call_end",
            SignalingMessage::CallError { .. } => "call_error",
            SignalingMessage::CallIceCandidate { .. } => "call_ice_candidate",
            SignalingMessage::PeerNotFound { .. } => "peer_not_found",
            SignalingMessage::ClientConnected { .. } => "client_connected",
            SignalingMessage::ClientDisconnected { .. } => "client_disconnected",
            SignalingMessage::ListClients => "list_clients",
            SignalingMessage::ClientList { .. } => "client_list",
            SignalingMessage::Error { .. } => "error",
            SignalingMessage::Disconnected { .. } => "disconnected",
            SignalingMessage::StationChanges { .. } => "station_changes",
            SignalingMessage::ListStations => "list_stations",
            SignalingMessage::StationList { .. } => "station_list",
        }
    }
}

impl AsMetricLabel for ErrorReason {
    fn as_metric_label(&self) -> &'static str {
        match self {
            ErrorReason::MalformedMessage => "malformed_message",
            ErrorReason::Internal(_) => "internal",
            ErrorReason::PeerConnection => "peer_connection",
            ErrorReason::UnexpectedMessage(_) => "unexpected_message",
            ErrorReason::RateLimited { .. } => "rate_limited",
        }
    }
}
