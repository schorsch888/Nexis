//! Prometheus metrics for Nexis Gateway
//!
//! Exposes metrics for monitoring gateway performance and health.

use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_gauge_vec, register_histogram,
    register_histogram_vec, Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec,
};

lazy_static! {
    // ============================================================================
    // Connection Metrics
    // ============================================================================

    /// Total number of active WebSocket connections
    pub static ref CONNECTIONS_ACTIVE: Gauge =
        register_gauge!("nexis_connections_active", "Number of active WebSocket connections").unwrap();

    /// Total number of connections ever established
    pub static ref CONNECTIONS_TOTAL: Counter =
        register_counter!("nexis_connections_total", "Total number of connections established").unwrap();

    /// Connection errors by type
    pub static ref CONNECTION_ERRORS: CounterVec =
        register_counter_vec!("nexis_connection_errors", "Connection errors by type", &["error_type"]).unwrap();

    // ============================================================================
    // Message Metrics
    // ============================================================================

    /// Total messages received
    pub static ref MESSAGES_RECEIVED: Counter =
        register_counter!("nexis_messages_received_total", "Total messages received").unwrap();

    /// Total messages sent
    pub static ref MESSAGES_SENT: Counter =
        register_counter!("nexis_messages_sent_total", "Total messages sent").unwrap();

    /// Message processing latency
    pub static ref MESSAGE_LATENCY: Histogram = register_histogram!(
        "nexis_message_latency_seconds",
        "Message processing latency in seconds",
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).unwrap();

    /// Messages by type
    pub static ref MESSAGES_BY_TYPE: CounterVec =
        register_counter_vec!("nexis_messages_by_type", "Messages by type", &["type"]).unwrap();

    /// Message size distribution
    pub static ref MESSAGE_SIZE: Histogram = register_histogram!(
        "nexis_message_size_bytes",
        "Message size in bytes",
        vec![64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0]
    ).unwrap();

    // ============================================================================
    // AI Provider Metrics
    // ============================================================================

    /// AI provider requests
    pub static ref AI_REQUESTS_TOTAL: CounterVec =
        register_counter_vec!("nexis_ai_requests_total", "Total AI provider requests", &["provider"]).unwrap();

    /// AI provider errors
    pub static ref AI_ERRORS: CounterVec =
        register_counter_vec!("nexis_ai_errors_total", "AI provider errors", &["provider", "error_type"]).unwrap();

    /// AI request latency
    pub static ref AI_LATENCY: HistogramVec = register_histogram_vec!(
        "nexis_ai_latency_seconds",
        "AI request latency in seconds",
        &["provider"],
        vec![0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0, 60.0, 120.0]
    ).unwrap();

    /// AI tokens used
    pub static ref AI_TOKENS_TOTAL: CounterVec =
        register_counter_vec!("nexis_ai_tokens_total", "Total AI tokens used", &["provider", "type"]).unwrap();

    // ============================================================================
    // Room Metrics
    // ============================================================================

    /// Active rooms
    pub static ref ROOMS_ACTIVE: Gauge =
        register_gauge!("nexis_rooms_active", "Number of active rooms").unwrap();

    /// Room members
    pub static ref ROOM_MEMBERS: GaugeVec =
        register_gauge_vec!("nexis_room_members", "Number of members per room", &["room_id"]).unwrap();

    // ============================================================================
    // HTTP Metrics
    // ============================================================================

    /// HTTP requests by method and path
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec =
        register_counter_vec!("nexis_http_requests_total", "Total HTTP requests", &["method", "path"]).unwrap();

    /// HTTP request latency
    pub static ref HTTP_LATENCY: HistogramVec = register_histogram_vec!(
        "nexis_http_latency_seconds",
        "HTTP request latency in seconds",
        &["method", "path"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]
    ).unwrap();

    /// HTTP response status codes
    pub static ref HTTP_RESPONSES: CounterVec =
        register_counter_vec!("nexis_http_responses_total", "HTTP responses by status code", &["method", "path", "status"]).unwrap();

    // ============================================================================
    // System Metrics
    // ============================================================================

    /// Build info
    pub static ref BUILD_INFO: GaugeVec =
        register_gauge_vec!("nexis_build_info", "Build information", &["version", "commit"]).unwrap();
}

/// Initialize metrics with build info
pub fn init_metrics() {
    // Set build info
    BUILD_INFO
        .with_label_values(&[
            env!("CARGO_PKG_VERSION"),
            option_env!("GIT_COMMIT_SHA").unwrap_or("unknown"),
        ])
        .set(1.0);
}

/// Export metrics in Prometheus format
pub fn export() -> String {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_can_be_exported() {
        // Increment some counters
        MESSAGES_RECEIVED.inc();
        CONNECTIONS_TOTAL.inc();

        // Export should not panic
        let exported = export();
        assert!(exported.contains("nexis_messages_received_total"));
        assert!(exported.contains("nexis_connections_total"));
    }
}
