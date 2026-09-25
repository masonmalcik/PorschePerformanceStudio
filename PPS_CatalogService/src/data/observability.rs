use crate::application::{MetricsSink, RequestMetric};

pub struct TracingMetrics;

impl MetricsSink for TracingMetrics {
    fn record_request(&self, metric: RequestMetric<'_>) {
        tracing::info!(
            event = "application_metric",
            metric_name = "http_request",
            method = metric.method,
            path = metric.path,
            status = metric.status,
            duration_ms = metric.duration_ms as u64,
            count = 1_u64,
        );
    }

    fn increment(&self, name: &'static str) {
        tracing::info!(
            event = "application_metric",
            metric_name = name,
            count = 1_u64
        );
    }
}
