//! Optimized MCP client with connection pooling and reliability features
//!
//! This module extends the base MCP client with performance optimizations including:
//! - Connection pooling for HTTP/WebSocket/Redis
//! - Circuit breakers for fault tolerance
//! - Retry logic with exponential backoff
//! - Health monitoring and automatic recovery
//! - Multi-tier caching for reduced latency

use crate::client::{McpClient, McpServerConnection};
use crate::connection_pool::{ConnectionPoolManager, PoolConfig};
use crate::rag_integration::{AgentContextManager, RagMcpClient};
use crate::reliability::{
    CircuitBreaker, CircuitBreakerConfig, FailoverManager, HealthMonitor, RetryPolicy, with_retry
};
use crate::transport::{HttpTransport, McpTransport, ProcessTransport, WebSocketTransport};
use crate::{McpClientConfig, McpServerConfig, McpServerSource, McpToolInfo};
use anyhow::Result;
use async_trait::async_trait;
use rust_mcp_schema::Resource;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

// ============================================================================
// Optimized Configuration
// ============================================================================

/// Performance-optimized configuration for MCP client
#[derive(Debug, Clone)]
pub struct OptimizedConfig {
    /// Base MCP client configuration
    pub base_config: McpClientConfig,

    /// Connection pool configuration
    pub pool_config: PoolConfig,

    /// Circuit breaker configuration
    pub circuit_breaker_config: CircuitBreakerConfig,

    /// Retry policy configuration
    pub retry_policy: RetryPolicy,

    /// Health check interval
    pub health_check_interval: Duration,

    /// Enable connection pooling
    pub enable_pooling: bool,

    /// Enable circuit breakers
    pub enable_circuit_breakers: bool,

    /// Enable health monitoring
    pub enable_health_monitoring: bool,

    /// Redis URL for caching (optional)
    pub redis_url: Option<String>,
}

impl Default for OptimizedConfig {
    fn default() -> Self {
        Self {
            base_config: McpClientConfig::default(),
            pool_config: PoolConfig::default(),
            circuit_breaker_config: CircuitBreakerConfig {
                failure_threshold: 5,
                failure_window: Duration::from_secs(60),
                recovery_timeout: Duration::from_secs(5), // Fast recovery for local LLM
                success_threshold: 2,
                request_timeout: Duration::from_secs(30),
            },
            retry_policy: RetryPolicy {
                max_attempts: 3,
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(2),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            health_check_interval: Duration::from_secs(30),
            enable_pooling: true,
            enable_circuit_breakers: true,
            enable_health_monitoring: true,
            redis_url: None,
        }
    }
}

impl OptimizedConfig {
    /// Create a high-performance configuration for local LLM inference
    pub fn for_local_llm() -> Self {
        Self {
            circuit_breaker_config: CircuitBreakerConfig {
                failure_threshold: 3,
                failure_window: Duration::from_secs(30),
                recovery_timeout: Duration::from_secs(5),
                success_threshold: 1,
                request_timeout: Duration::from_secs(30),
            },
            retry_policy: RetryPolicy {
                max_attempts: 2,
                initial_delay: Duration::from_millis(50),
                max_delay: Duration::from_secs(1),
                backoff_multiplier: 2.0,
                jitter: false,
            },
            health_check_interval: Duration::from_secs(15),
            ..Default::default()
        }
    }

    /// Create a configuration for cloud/remote services
    pub fn for_cloud() -> Self {
        Self {
            circuit_breaker_config: CircuitBreakerConfig {
                failure_threshold: 5,
                failure_window: Duration::from_secs(60),
                recovery_timeout: Duration::from_secs(30),
                success_threshold: 3,
                request_timeout: Duration::from_secs(60),
            },
            retry_policy: RetryPolicy {
                max_attempts: 5,
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(10),
                backoff_multiplier: 2.0,
                jitter: true,
            },
            health_check_interval: Duration::from_secs(60),
            ..Default::default()
        }
    }
}

// ============================================================================
// Optimized MCP Connection
// ============================================================================

/// Wrapper around MCP connection with reliability features
struct OptimizedConnection {
    inner: Arc<dyn McpServerConnection>,
    circuit_breaker: Arc<CircuitBreaker>,
    health_monitor: Arc<HealthMonitor>,
    retry_policy: RetryPolicy,
}

impl OptimizedConnection {
    fn new(
        inner: Arc<dyn McpServerConnection>,
        circuit_config: CircuitBreakerConfig,
        retry_policy: RetryPolicy,
    ) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(circuit_config));
        let health_monitor = Arc::new(HealthMonitor::new(inner.server_name().to_string()));

        // Start health monitoring
        let inner_clone = Arc::clone(&inner);
        let monitor_clone = Arc::clone(&health_monitor);
        monitor_clone.start(move || {
            let inner = Arc::clone(&inner_clone);
            async move { inner.ping().await }
        });

        Self {
            inner,
            circuit_breaker,
            health_monitor,
            retry_policy,
        }
    }
}

#[async_trait]
impl McpServerConnection for OptimizedConnection {
    fn server_id(&self) -> &str {
        self.inner.server_id()
    }

    fn server_name(&self) -> &str {
        self.inner.server_name()
    }

    async fn list_tools(&self) -> Result<Vec<McpToolInfo>> {
        // Use circuit breaker and retry logic
        self.circuit_breaker
            .call(async {
                with_retry(&self.retry_policy, || self.inner.list_tools()).await
            })
            .await
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<String> {
        // Use circuit breaker and retry logic
        self.circuit_breaker
            .call(async {
                with_retry(&self.retry_policy, || {
                    self.inner.call_tool(name, arguments.clone())
                })
                .await
            })
            .await
    }

    async fn list_resources(&self) -> Result<Vec<Resource>> {
        self.circuit_breaker
            .call(async {
                with_retry(&self.retry_policy, || self.inner.list_resources()).await
            })
            .await
    }

    async fn read_resource(&self, uri: &str) -> Result<String> {
        self.circuit_breaker
            .call(async {
                with_retry(&self.retry_policy, || self.inner.read_resource(uri)).await
            })
            .await
    }

    async fn ping(&self) -> Result<()> {
        self.inner.ping().await
    }
}

// ============================================================================
// Optimized MCP Client
// ============================================================================

/// High-performance MCP client with connection pooling and reliability
pub struct OptimizedMcpClient {
    /// Base MCP client
    base_client: Arc<RwLock<McpClient>>,

    /// Connection pool manager
    pool_manager: Arc<ConnectionPoolManager>,

    /// RAG/context manager
    rag_client: Option<Arc<RagMcpClient>>,

    /// Agent context manager
    context_manager: Option<Arc<AgentContextManager>>,

    /// Optimized connections with reliability wrappers
    optimized_connections: Arc<RwLock<HashMap<String, Arc<OptimizedConnection>>>>,

    /// Configuration
    config: OptimizedConfig,

    /// Metrics collector
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

impl OptimizedMcpClient {
    /// Create a new optimized MCP client
    pub async fn new(config: OptimizedConfig) -> Result<Self> {
        // Create base client
        let base_client = Arc::new(RwLock::new(McpClient::new(config.base_config.clone())));

        // Create connection pool manager
        let pool_manager = Arc::new(
            ConnectionPoolManager::new(config.pool_config.clone(), config.redis_url.as_deref())
                .await?,
        );

        // Create RAG client if configured
        let rag_client = if config.redis_url.is_some() {
            Some(Arc::new(RagMcpClient::new(Arc::clone(&base_client.read().await) as Arc<McpClient>)))
        } else {
            None
        };

        // Create context manager if RAG is available
        let context_manager = rag_client
            .as_ref()
            .map(|rag| Arc::new(AgentContextManager::new(Arc::clone(rag))));

        Ok(Self {
            base_client,
            pool_manager,
            rag_client,
            context_manager,
            optimized_connections: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        })
    }

    /// Initialize all configured servers with optimizations
    pub async fn initialize(&mut self) -> Result<()> {
        let start = std::time::Instant::now();

        // Initialize base client
        self.base_client.write().await.initialize().await?;

        // Wrap connections with optimization layers
        if self.config.enable_circuit_breakers || self.config.enable_health_monitoring {
            self.wrap_connections_with_optimization().await?;
        }

        // Ingest documentation if RAG is available
        if let Some(context_mgr) = &self.context_manager {
            tokio::spawn({
                let context_mgr = Arc::clone(context_mgr);
                async move {
                    if let Err(e) = context_mgr.ingest_project_docs().await {
                        warn!("Failed to ingest project documentation: {}", e);
                    }
                }
            });
        }

        let duration = start.elapsed();
        info!("Optimized MCP client initialized in {:?}", duration);

        self.metrics.write().await.initialization_time = duration;

        Ok(())
    }

    /// Wrap base connections with optimization layers
    async fn wrap_connections_with_optimization(&mut self) -> Result<()> {
        let base_client = self.base_client.read().await;
        let servers = base_client.get_servers();

        let mut optimized = self.optimized_connections.write().await;

        for (server_id, connection) in servers {
            let optimized_conn = OptimizedConnection::new(
                Arc::clone(connection),
                self.config.circuit_breaker_config.clone(),
                self.config.retry_policy.clone(),
            );

            optimized.insert(server_id.clone(), Arc::new(optimized_conn));
        }

        Ok(())
    }

    /// Get tool callbacks with performance optimizations
    pub async fn get_optimized_tool_callbacks(&self) -> HashMap<String, Arc<dyn Fn(&str) -> Result<String> + Send + Sync>> {
        // This would return optimized callbacks that use the pooled connections
        // For now, delegate to base client
        HashMap::new() // Placeholder
    }

    /// Execute a tool call with all optimizations
    pub async fn call_tool_optimized(&self, server_id: &str, name: &str, arguments: Value) -> Result<String> {
        let start = std::time::Instant::now();

        // Try optimized connection first
        let result = if let Some(conn) = self.optimized_connections.read().await.get(server_id) {
            conn.call_tool(name, arguments).await
        } else {
            // Fallback to base client
            if let Some(conn) = self.base_client.read().await.get_servers().get(server_id) {
                conn.call_tool(name, arguments).await
            } else {
                return Err(anyhow::anyhow!("Server {} not found", server_id));
            }
        };

        // Update metrics
        let duration = start.elapsed();
        let mut metrics = self.metrics.write().await;
        metrics.total_tool_calls += 1;
        metrics.total_latency += duration;

        if result.is_ok() {
            metrics.successful_calls += 1;
            if duration < Duration::from_millis(100) {
                metrics.calls_under_100ms += 1;
            }
        } else {
            metrics.failed_calls += 1;
        }

        result
    }

    /// Get build context using RAG if available
    pub async fn get_build_context(&self, target: &str) -> Result<String> {
        if let Some(context_mgr) = &self.context_manager {
            context_mgr.get_build_context(target).await
        } else {
            Ok(String::new())
        }
    }

    /// Get API documentation using RAG if available
    pub async fn get_api_docs(&self, module: &str, function: Option<&str>) -> Result<String> {
        if let Some(context_mgr) = &self.context_manager {
            context_mgr.get_api_docs(module, function).await
        } else {
            Ok(String::new())
        }
    }

    /// Get performance metrics
    pub async fn get_metrics(&self) -> PerformanceReport {
        let metrics = self.metrics.read().await;
        let pool_metrics = self.pool_manager.aggregate_metrics().await;

        // Collect circuit breaker metrics
        let mut circuit_states = HashMap::new();
        for (id, conn) in self.optimized_connections.read().await.iter() {
            let cb_metrics = conn.circuit_breaker.metrics().await;
            circuit_states.insert(id.clone(), cb_metrics);
        }

        PerformanceReport {
            initialization_time: metrics.initialization_time,
            total_tool_calls: metrics.total_tool_calls,
            successful_calls: metrics.successful_calls,
            failed_calls: metrics.failed_calls,
            average_latency: if metrics.total_tool_calls > 0 {
                metrics.total_latency / metrics.total_tool_calls as u32
            } else {
                Duration::from_secs(0)
            },
            p95_latency: metrics.calculate_p95_latency(),
            calls_under_100ms: metrics.calls_under_100ms,
            connection_reuse_rate: pool_metrics.total_reuse_percentage(),
            circuit_breaker_states: circuit_states,
            memory_usage_mb: self.estimate_memory_usage(),
        }
    }

    /// Estimate memory usage in MB
    fn estimate_memory_usage(&self) -> f64 {
        // This is a rough estimate based on typical sizes
        // In production, you'd use actual memory profiling

        // Base overhead ~50MB
        let base = 50.0;

        // Connections ~1MB each
        let connections = self.config.base_config.servers.len() as f64;

        // Cache entries ~10KB each (assuming 100 entries)
        let cache = 1.0;

        base + connections + cache
    }

    /// Perform health check on all servers
    pub async fn health_check(&self) -> HealthReport {
        let mut report = HealthReport {
            healthy_servers: Vec::new(),
            unhealthy_servers: Vec::new(),
            degraded_servers: Vec::new(),
        };

        for (id, conn) in self.optimized_connections.read().await.iter() {
            let health = conn.health_monitor.status().await;
            let circuit = conn.circuit_breaker.state().await;

            let status = ServerHealthStatus {
                server_id: id.clone(),
                health_status: format!("{:?}", health),
                circuit_state: format!("{}", circuit),
            };

            match health {
                crate::reliability::HealthStatus::Healthy => report.healthy_servers.push(status),
                crate::reliability::HealthStatus::Unhealthy => report.unhealthy_servers.push(status),
                crate::reliability::HealthStatus::Degraded => report.degraded_servers.push(status),
                _ => {}
            }
        }

        report
    }
}

// ============================================================================
// Performance Metrics
// ============================================================================

#[derive(Debug, Default)]
struct PerformanceMetrics {
    initialization_time: Duration,
    total_tool_calls: u64,
    successful_calls: u64,
    failed_calls: u64,
    total_latency: Duration,
    calls_under_100ms: u64,
    latency_samples: Vec<Duration>,
}

impl PerformanceMetrics {
    fn calculate_p95_latency(&self) -> Duration {
        if self.latency_samples.is_empty() {
            return Duration::from_secs(0);
        }

        let mut sorted = self.latency_samples.clone();
        sorted.sort();

        let p95_index = (sorted.len() as f64 * 0.95) as usize;
        sorted[p95_index.min(sorted.len() - 1)]
    }
}

#[derive(Debug)]
pub struct PerformanceReport {
    pub initialization_time: Duration,
    pub total_tool_calls: u64,
    pub successful_calls: u64,
    pub failed_calls: u64,
    pub average_latency: Duration,
    pub p95_latency: Duration,
    pub calls_under_100ms: u64,
    pub connection_reuse_rate: f64,
    pub circuit_breaker_states: HashMap<String, crate::reliability::CircuitBreakerMetrics>,
    pub memory_usage_mb: f64,
}

#[derive(Debug)]
pub struct HealthReport {
    pub healthy_servers: Vec<ServerHealthStatus>,
    pub unhealthy_servers: Vec<ServerHealthStatus>,
    pub degraded_servers: Vec<ServerHealthStatus>,
}

#[derive(Debug)]
pub struct ServerHealthStatus {
    pub server_id: String,
    pub health_status: String,
    pub circuit_state: String,
}

// ============================================================================
// Extension Methods for Base Client
// ============================================================================

impl McpClient {
    /// Get reference to internal servers map
    pub fn get_servers(&self) -> &HashMap<String, Arc<dyn McpServerConnection>> {
        &self.servers
    }
}

// ============================================================================
// Usage Example
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_optimized_client_creation() {
        let config = OptimizedConfig::for_local_llm();
        let client = OptimizedMcpClient::new(config).await;
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let config = OptimizedConfig::default();
        let client = OptimizedMcpClient::new(config).await.unwrap();

        let report = client.get_metrics().await;
        assert_eq!(report.total_tool_calls, 0);
        assert_eq!(report.successful_calls, 0);
    }
}
