# Monitoring & Observability Setup for mistral.rs

This directory contains comprehensive monitoring, logging, and alerting configurations for production deployment of mistral.rs.

## Overview

The monitoring stack provides:

- **Real-time metrics** via Prometheus
- **Visual dashboards** via Grafana
- **Alerting** for critical issues
- **Centralized logging** with multiple backend options
- **Performance tracking** against defined SLOs

## Key Performance Targets

| Metric           | Target     | Critical Threshold |
| ---------------- | ---------- | ------------------ |
| P95 Latency      | \<200ms    | >500ms             |
| Tool Call P95    | \<100ms    | >200ms             |
| Throughput       | 1000 req/s | \<500 req/s        |
| Memory Usage     | \<2GB      | >3GB               |
| Connection Reuse | >90%       | \<70%              |
| Error Rate       | \<1%       | >5%                |
| Startup Time     | \<300ms    | >1s                |
| Shutdown Time    | \<2s       | >5s                |

## Quick Start

### 1. Deploy Prometheus

```bash
# Start Prometheus with our configuration
docker run -d \
  --name prometheus \
  -p 9090:9090 \
  -v $(pwd)/prometheus.yml:/etc/prometheus/prometheus.yml \
  -v $(pwd)/alert-rules.yml:/etc/prometheus/alert-rules.yml \
  prom/prometheus

# Verify it's running
curl http://localhost:9090/-/healthy
```

### 2. Deploy Grafana

```bash
# Start Grafana
docker run -d \
  --name grafana \
  -p 3000:3000 \
  -e "GF_SECURITY_ADMIN_PASSWORD=admin" \
  grafana/grafana

# Import dashboards
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @grafana-dashboard-overview.json

curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @grafana-dashboard-performance.json
```

### 3. Deploy Alertmanager

```bash
# Create alertmanager.yml
cat > alertmanager.yml << EOF
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'severity']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'web.hook'

receivers:
- name: 'web.hook'
  webhook_configs:
  - url: 'http://localhost:5001/alerts'
    send_resolved: true
EOF

# Start Alertmanager
docker run -d \
  --name alertmanager \
  -p 9093:9093 \
  -v $(pwd)/alertmanager.yml:/etc/alertmanager/alertmanager.yml \
  prom/alertmanager
```

### 4. Configure mistral.rs Metrics

Add to your mistral.rs configuration:

```rust
// In mistralrs-server/src/main.rs or your metrics module
use prometheus::{register_histogram_vec, register_gauge_vec, register_counter_vec};

lazy_static! {
    // Request metrics
    static ref REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "mistralrs_request_duration_seconds",
        "Request duration in seconds",
        &["method", "endpoint", "status"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0]
    ).unwrap();

    // Tool call metrics
    static ref TOOL_CALL_DURATION: HistogramVec = register_histogram_vec!(
        "mistralrs_tool_call_duration_seconds",
        "Tool call duration in seconds",
        &["tool", "status"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.2, 0.5]
    ).unwrap();

    // Connection metrics
    static ref CONNECTION_REUSE: CounterVec = register_counter_vec!(
        "mistralrs_connection_reused_total",
        "Total reused connections",
        &["pool"]
    ).unwrap();

    // Circuit breaker metrics
    static ref CIRCUIT_BREAKER_STATE: GaugeVec = register_gauge_vec!(
        "mistralrs_circuit_breaker_state",
        "Circuit breaker state (0=closed, 1=half-open, 2=open)",
        &["name"]
    ).unwrap();

    // Resource metrics
    static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "mistralrs_active_connections",
        "Number of active connections"
    ).unwrap();
}

// Add metrics endpoint
async fn metrics_handler() -> impl Responder {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4")
        .body(buffer)
}
```

### 5. Setup Logging

Choose your logging backend:

#### Option A: ELK Stack

```bash
# Deploy Elasticsearch
docker run -d \
  --name elasticsearch \
  -p 9200:9200 \
  -e "discovery.type=single-node" \
  -e "xpack.security.enabled=false" \
  elasticsearch:8.11.0

# Deploy Kibana
docker run -d \
  --name kibana \
  -p 5601:5601 \
  -e "ELASTICSEARCH_HOSTS=http://elasticsearch:9200" \
  kibana:8.11.0

# Deploy Filebeat
docker run -d \
  --name filebeat \
  -v $(pwd)/logging-config.yaml:/usr/share/filebeat/filebeat.yml \
  -v /var/log/mistralrs:/var/log/mistralrs:ro \
  elastic/filebeat:8.11.0
```

#### Option B: Grafana Loki

```bash
# Deploy Loki
docker run -d \
  --name loki \
  -p 3100:3100 \
  grafana/loki

# Deploy Promtail
docker run -d \
  --name promtail \
  -v $(pwd)/logging-config.yaml:/etc/promtail/config.yml \
  -v /var/log/mistralrs:/var/log/mistralrs:ro \
  grafana/promtail
```

## Integration with mistral.rs

### 1. Add Dependencies

```toml
# In Cargo.toml
[dependencies]
prometheus = "0.13"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
metrics = "0.21"
metrics-exporter-prometheus = "0.12"
```

### 2. Initialize Tracing

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_target(false)
        )
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
```

### 3. Add Middleware for Metrics

```rust
use actix_web::{middleware, web, App, HttpServer};
use std::time::Instant;

async fn request_metrics_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.path().to_string();

    let response = next.call(req).await?;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    REQUEST_DURATION
        .with_label_values(&[&method, &path, &status])
        .observe(duration);

    Ok(response)
}
```

### 4. Add Health Checks

```rust
async fn health_check() -> impl Responder {
    // Check system health
    let health = json!({
        "status": "healthy",
        "timestamp": Utc::now().to_rfc3339(),
        "checks": {
            "memory": check_memory_usage(),
            "connections": check_connection_pool(),
            "mcp_servers": check_mcp_servers(),
        }
    });

    HttpResponse::Ok().json(health)
}

async fn readiness_check() -> impl Responder {
    // Check if ready to serve
    if is_ready() {
        HttpResponse::Ok().json(json!({"ready": true}))
    } else {
        HttpResponse::ServiceUnavailable().json(json!({"ready": false}))
    }
}

async fn liveness_check() -> impl Responder {
    // Simple liveness check
    HttpResponse::Ok().json(json!({"alive": true}))
}
```

## Dashboard Access

After deployment:

1. **Prometheus**: http://localhost:9090
1. **Grafana**: http://localhost:3000 (admin/admin)
1. **Alertmanager**: http://localhost:9093
1. **Kibana** (if using ELK): http://localhost:5601

## Alert Examples

### Critical Alerts

```yaml
# High latency
alert: HighLatencyP95Critical
expr: histogram_quantile(0.95, rate(mistralrs_request_duration_seconds_bucket[5m])) > 0.5
for: 2m
labels:
  severity: critical
annotations:
  summary: "P95 latency exceeds 500ms"
```

### Warning Alerts

```yaml
# Memory pressure
alert: HighMemoryUsageWarning
expr: process_resident_memory_bytes{job="mistralrs"} > 2684354560
for: 10m
labels:
  severity: warning
annotations:
  summary: "Memory usage exceeds 2.5GB"
```

## Monitoring Queries

Useful Prometheus queries:

```promql
# P95 latency
histogram_quantile(0.95, rate(mistralrs_request_duration_seconds_bucket[5m]))

# Request rate
sum(rate(mistralrs_requests_total[5m]))

# Error rate
sum(rate(mistralrs_requests_failed_total[5m])) / sum(rate(mistralrs_requests_total[5m]))

# Memory usage
process_resident_memory_bytes{job="mistralrs"} / 1024 / 1024

# Connection reuse rate
sum(rate(mistralrs_connection_reused_total[5m])) / sum(rate(mistralrs_connection_created_total[5m]))

# Tool call P95 latency
histogram_quantile(0.95, rate(mistralrs_tool_call_duration_seconds_bucket[5m]))

# Circuit breaker trips
increase(mistralrs_circuit_breaker_trips_total[1h])
```

## Troubleshooting

### Metrics not appearing

1. Check mistralrs is exposing `/metrics` endpoint
1. Verify Prometheus can reach the endpoint
1. Check scrape configuration in `prometheus.yml`

### High memory usage alerts

1. Check for memory leaks in connection pools
1. Verify circuit breakers are working
1. Review cache sizes and TTLs

### Slow queries in dashboards

1. Reduce time range
1. Add recording rules for expensive queries
1. Optimize PromQL expressions

## Performance Optimization Tips

1. **Use recording rules** for frequently-used expensive queries
1. **Set appropriate retention** in Prometheus (15d recommended)
1. **Use sampling** for high-volume logs
1. **Implement log rotation** to prevent disk exhaustion
1. **Use compression** for log shipping
1. **Set up dashboard caching** in Grafana

## Production Checklist

- [ ] Prometheus deployed and scraping metrics
- [ ] Grafana dashboards imported and working
- [ ] Alertmanager configured with notification channels
- [ ] Logging backend deployed (ELK or Loki)
- [ ] Log rotation configured
- [ ] Health check endpoints working
- [ ] Metrics endpoint exposed
- [ ] Alert rules deployed and tested
- [ ] Dashboard permissions configured
- [ ] Backup strategy for metrics/logs
- [ ] Monitoring the monitoring (meta-monitoring)

## Support

For issues or questions:

1. Check logs: `docker logs <container_name>`
1. Verify connectivity: `curl http://localhost:9090/-/healthy`
1. Review configurations in this directory
1. Check mistral.rs metrics implementation
