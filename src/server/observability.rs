/// Observability for Glyph MCP
///
/// Prometheus metrics and OpenTelemetry tracing integration.

use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// ============================================================================
// Metrics Types
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_tools_called: u64,
    pub active_connections: u64,
    pub uptime_seconds: u64,
    pub tool_metrics: std::collections::HashMap<String, ToolMetrics>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub call_count: u64,
    pub error_count: u64,
    pub total_duration_ms: u64,
    pub avg_duration_ms: f64,
}

// ============================================================================
// Metrics Collector
// ============================================================================

pub struct MetricsCollector {
    metrics: Arc<RwLock<ServerMetrics>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(ServerMetrics::default())),
            start_time: Instant::now(),
        }
    }

    pub async fn record_request(&self, success: bool) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
    }

    pub async fn record_tool_call(
        &self,
        tool_name: &str,
        duration_ms: u64,
        success: bool,
    ) {
        let mut metrics = self.metrics.write().await;
        metrics.total_tools_called += 1;

        let tool_metrics = metrics
            .tool_metrics
            .entry(tool_name.to_string())
            .or_insert_with(ToolMetrics::default);

        tool_metrics.call_count += 1;
        if !success {
            tool_metrics.error_count += 1;
        }

        tool_metrics.total_duration_ms += duration_ms;
        tool_metrics.avg_duration_ms =
            tool_metrics.total_duration_ms as f64 / tool_metrics.call_count as f64;
    }

    pub async fn record_connection_change(&self, delta: i32) {
        let mut metrics = self.metrics.write().await;
        metrics.active_connections = (metrics.active_connections as i32 + delta).max(0) as u64;
    }

    pub async fn get_metrics(&self) -> ServerMetrics {
        let mut metrics = self.metrics.read().await.clone();
        metrics.uptime_seconds = self.start_time.elapsed().as_secs();
        metrics
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> String {
        let metrics = self.get_metrics().await;

        let mut output = String::new();

        // Total requests
        output.push_str(&format!(
            "# HELP glyph_requests_total Total number of requests\n\
             # TYPE glyph_requests_total counter\n\
             glyph_requests_total {}\n",
            metrics.total_requests
        ));

        // Successful requests
        output.push_str(&format!(
            "# HELP glyph_requests_successful Successful requests\n\
             # TYPE glyph_requests_successful counter\n\
             glyph_requests_successful {}\n",
            metrics.successful_requests
        ));

        // Failed requests
        output.push_str(&format!(
            "# HELP glyph_requests_failed Failed requests\n\
             # TYPE glyph_requests_failed counter\n\
             glyph_requests_failed {}\n",
            metrics.failed_requests
        ));

        // Active connections
        output.push_str(&format!(
            "# HELP glyph_active_connections Current active connections\n\
             # TYPE glyph_active_connections gauge\n\
             glyph_active_connections {}\n",
            metrics.active_connections
        ));

        // Uptime
        output.push_str(&format!(
            "# HELP glyph_uptime_seconds Server uptime in seconds\n\
             # TYPE glyph_uptime_seconds counter\n\
             glyph_uptime_seconds {}\n",
            metrics.uptime_seconds
        ));

        // Tool-specific metrics
        for (tool_name, tool_metrics) in metrics.tool_metrics.iter() {
            output.push_str(&format!(
                "# HELP glyph_tool_calls_total Total calls for tool {}\n\
                 # TYPE glyph_tool_calls_total counter\n\
                 glyph_tool_calls_total{{tool=\"{}\"}} {}\n",
                tool_name, tool_name, tool_metrics.call_count
            ));

            output.push_str(&format!(
                "# HELP glyph_tool_errors_total Total errors for tool {}\n\
                 # TYPE glyph_tool_errors_total counter\n\
                 glyph_tool_errors_total{{tool=\"{}\"}} {}\n",
                tool_name, tool_name, tool_metrics.error_count
            ));

            output.push_str(&format!(
                "# HELP glyph_tool_duration_ms_avg Average duration for tool {}\n\
                 # TYPE glyph_tool_duration_ms_avg gauge\n\
                 glyph_tool_duration_ms_avg{{tool=\"{}\"}} {}\n",
                tool_name, tool_name, tool_metrics.avg_duration_ms
            ));
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tracing Integration
// ============================================================================

use tracing::{Span, span, Level};

pub struct TracingContext {
    pub request_id: String,
    pub span: Span,
}

impl TracingContext {
    pub fn new(request_id: String, operation: &str) -> Self {
        let span = span!(
            Level::INFO,
            "mcp_operation",
            request_id = %request_id,
            operation = operation
        );

        Self { request_id, span }
    }

    pub fn record_field(&self, key: &str, value: &str) {
        self.span.record(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        collector.record_request(true).await;
        collector.record_request(false).await;
        collector.record_tool_call("test_tool", 100, true).await;

        let metrics = collector.get_metrics().await;
        assert_eq!(metrics.total_requests, 2);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 1);
        assert_eq!(metrics.total_tools_called, 1);
    }

    #[tokio::test]
    async fn test_prometheus_export() {
        let collector = MetricsCollector::new();
        collector.record_request(true).await;

        let prometheus_output = collector.export_prometheus().await;
        assert!(prometheus_output.contains("glyph_requests_total"));
        assert!(prometheus_output.contains("glyph_uptime_seconds"));
    }
}
