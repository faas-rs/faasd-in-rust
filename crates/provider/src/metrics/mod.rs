use lazy_static::lazy_static;
use prometheus::{self, register_histogram_vec, register_int_counter_vec};

lazy_static! {
    pub static ref HTTP_METRICS: HttpMetrics = HttpMetrics::new();
}

#[derive(Clone)]
pub struct HttpMetrics {
    pub request_duration: prometheus::HistogramVec,
    pub requests_total: prometheus::IntCounterVec,
}

impl HttpMetrics {
    pub fn new() -> Self {
        Self {
            request_duration: register_histogram_vec!(
                "http_request_duration_seconds",
                "Request duration in seconds",
                &["method", "path", "status"]
            )
            .unwrap(),
            requests_total: register_int_counter_vec!(
                "http_requests_total",
                "Total number of HTTP requests",
                &["method", "path", "status"]
            )
            .unwrap(),
        }
    }
}

pub const TEXT_CONTENT_TYPE: &str = "text/plain; version=0.0.4";
