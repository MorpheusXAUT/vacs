pub mod guards;
mod labels;

use crate::metrics::labels::AsMetricLabel;
use crate::release::catalog::BundleType;
use metrics::{counter, describe_counter, describe_gauge, describe_histogram, histogram};
use semver::Version;
use vacs_protocol::http::version::ReleaseChannel;
use vacs_protocol::ws::LoginFailureReason;

pub fn register_metrics() {
    ClientMetrics::register();
    CallMetrics::register();
    MessageMetrics::register();
    ErrorMetrics::register();
    VersionMetrics::register();
}

pub struct ClientMetrics;

impl ClientMetrics {
    pub fn login_attempt(success: bool) {
        let label = if success { "success" } else { "failure" };
        counter!("vacs_clients_login_attempts_total", "status" => label).increment(1);
    }

    pub fn login_failure(reason: LoginFailureReason) {
        let label = reason.as_metric_label();
        counter!("vacs_clients_login_failures_total", "reason" => label).increment(1);
    }

    fn register() {
        describe_gauge!(
            "vacs_clients_connected",
            "Number of currently connected clients"
        );
        describe_counter!(
            "vacs_clients_total",
            "Total number of client connections established"
        );
        describe_counter!(
            "vacs_clients_login_attempts_total",
            "Total login attempts, labeled by success/failure"
        );
        describe_counter!(
            "vacs_clients_login_failures_total",
            "Login failures by reason"
        );
        describe_counter!(
            "vacs_clients_disconnects_total",
            "Client disconnects by reason (graceful vs forced)"
        );
        describe_histogram!(
            "vacs_clients_session_duration_seconds",
            "Duration of client sessions in seconds"
        );
    }
}

struct CallMetrics;

impl CallMetrics {
    fn register() {
        describe_gauge!("vacs_calls_active", "Number of currently active calls");
        describe_counter!(
            "vacs_calls_attempts_total",
            "Total number of calls initiated, labeled by outcome (accepted, error, cancelled, no_answer, aborted)"
        );
        describe_counter!("vacs_calls_total", "Total number of calls established");
        describe_histogram!(
            "vacs_calls_duration_seconds",
            "Duration of completed calls in seconds"
        );
        describe_histogram!(
            "vacs_calls_answer_duration_seconds",
            "Time from invite to answer in seconds"
        );
    }
}

pub struct MessageMetrics;

impl MessageMetrics {
    pub fn sent(message_type: &impl AsMetricLabel, size_bytes: usize) {
        counter!("vacs_messages_sent_total", "type" => message_type.as_metric_label()).increment(1);
        histogram!("vacs_message_size_bytes", "direction" => "sent").record(size_bytes as f64);
    }

    pub fn received(message_type: &impl AsMetricLabel, size_bytes: usize) {
        counter!("vacs_messages_received_total", "type" => message_type.as_metric_label())
            .increment(1);
        histogram!("vacs_message_size_bytes", "direction" => "received").record(size_bytes as f64);
    }

    pub fn malformed() {
        counter!("vacs_messages_malformed_total").increment(1);
    }

    fn register() {
        describe_counter!(
            "vacs_messages_sent_total",
            "Total messages sent to clients, by message type"
        );
        describe_counter!(
            "vacs_messages_received_total",
            "Total messages received from clients, by message type"
        );
        describe_counter!(
            "vacs_messages_malformed_total",
            "Number of malformed messages received"
        );
        describe_histogram!(
            "vacs_message_size_bytes",
            "Size of WebSocket messages in bytes"
        );
    }
}

pub struct ErrorMetrics;

impl ErrorMetrics {
    pub fn error(error_type: &impl AsMetricLabel) {
        counter!("vacs_errors_total", "type" => error_type.as_metric_label()).increment(1);
    }

    pub fn peer_not_found() {
        counter!("vacs_errors_peer_not_found_total").increment(1);
    }

    pub fn rate_limits_hit() {
        counter!("vacs_errors_rate_limits_hit_total").increment(1);
    }

    fn register() {
        describe_counter!(
            "vacs_errors_total",
            "Errors encountered by the server, labeled by error type"
        );
        describe_counter!(
            "vacs_errors_peer_not_found_total",
            "Number of times a peer was not found"
        );
        describe_counter!(
            "vacs_errors_rate_limits_hit_total",
            "Number of times rate limiting was triggered"
        );
    }
}

pub struct VersionMetrics;

impl VersionMetrics {
    pub fn check(
        channel: &ReleaseChannel,
        client_version: &Version,
        target: impl Into<String>,
        arch: impl Into<String>,
        bundle_type: &BundleType,
        update_available: bool,
    ) {
        counter!(
            "vacs_version_checks_total",
            "channel" => channel.as_metric_label(),
            "client_version" => client_version.to_string(),
            "target" => target.into(),
            "arch" => arch.into(),
            "bundle_type" => bundle_type.as_metric_label(),
            "result" => if update_available { "update_available" } else { "up_to_date" }
        )
        .increment(1);
    }

    fn register() {
        describe_counter!(
            "vacs_version_checks_total",
            "Version checks labeled by version, channel, platform, architecture, bundle, and update availability"
        );
    }
}
