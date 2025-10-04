//! Circuit breakers and reliability patterns for MCP transport
//!
//! This module provides circuit breaker implementation, automatic failover,
//! retry logic with exponential backoff, and health monitoring for MCP servers.

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, error, info, warn};

// ============================================================================
// Circuit Breaker States
// ============================================================================

/// Circuit breaker states following the classic pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, all requests fail fast
    Open,
    /// Circuit is testing if service has recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "CLOSED"),
            CircuitState::Open => write!(f, "OPEN"),
            CircuitState::HalfOpen => write!(f, "HALF-OPEN"),
        }
    }
}

// ============================================================================
// Circuit Breaker Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening the circuit
    pub failure_threshold: u32,
    /// Time window for counting failures
    pub failure_window: Duration,
    /// How long to wait before trying half-open state
    pub recovery_timeout: Duration,
    /// Number of successful calls in half-open before closing
    pub success_threshold: u32,
    /// Timeout for individual requests
    pub request_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            failure_window: Duration::from_secs(60),
            recovery_timeout: Duration::from_secs(30),
            success_threshold: 3,
            request_timeout: Duration::from_secs(30),
        }
    }
}

// ============================================================================
// Circuit Breaker Implementation
// ============================================================================

/// Circuit breaker for protecting against cascading failures
pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<Mutex<u32>>,
    success_count: Arc<Mutex<u32>>,
    last_failure_time: Arc<Mutex<Option<Instant>>>,
    last_state_change: Arc<Mutex<Instant>>,
    total_requests: Arc<Mutex<u64>>,
    total_failures: Arc<Mutex<u64>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(Mutex::new(0)),
            success_count: Arc::new(Mutex::new(0)),
            last_failure_time: Arc::new(Mutex::new(None)),
            last_state_change: Arc::new(Mutex::new(Instant::now())),
            total_requests: Arc::new(Mutex::new(0)),
            total_failures: Arc::new(Mutex::new(0)),
        }
    }

    /// Execute a function through the circuit breaker
    pub async fn call<F, T>(&self, f: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        // Check circuit state
        let current_state = self.check_state().await;

        match current_state {
            CircuitState::Open => {
                // Fail fast when circuit is open
                return Err(anyhow::anyhow!(
                    "Circuit breaker is OPEN - service unavailable"
                ));
            }
            CircuitState::HalfOpen => {
                debug!("Circuit breaker is HALF-OPEN - testing service recovery");
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Increment request counter
        *self.total_requests.lock().await += 1;

        // Execute the function with timeout
        let result = tokio::time::timeout(self.config.request_timeout, f).await;

        match result {
            Ok(Ok(value)) => {
                self.on_success().await;
                Ok(value)
            }
            Ok(Err(e)) => {
                self.on_failure().await;
                Err(e)
            }
            Err(_) => {
                self.on_failure().await;
                Err(anyhow::anyhow!(
                    "Request timed out after {:?}",
                    self.config.request_timeout
                ))
            }
        }
    }

    /// Check and potentially update circuit state
    async fn check_state(&self) -> CircuitState {
        let mut state = self.state.write().await;
        let last_change = *self.last_state_change.lock().await;

        match *state {
            CircuitState::Open => {
                // Check if recovery timeout has elapsed
                if last_change.elapsed() >= self.config.recovery_timeout {
                    info!("Circuit breaker transitioning from OPEN to HALF-OPEN");
                    *state = CircuitState::HalfOpen;
                    *self.last_state_change.lock().await = Instant::now();
                    *self.success_count.lock().await = 0;
                }
            }
            CircuitState::Closed => {
                // Check if failures are within time window
                if let Some(last_failure) = *self.last_failure_time.lock().await {
                    if last_failure.elapsed() > self.config.failure_window {
                        // Reset failure count if outside window
                        *self.failure_count.lock().await = 0;
                    }
                }
            }
            _ => {}
        }

        *state
    }

    /// Handle successful call
    async fn on_success(&self) {
        let mut state = self.state.write().await;

        match *state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.lock().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    info!("Circuit breaker transitioning from HALF-OPEN to CLOSED");
                    *state = CircuitState::Closed;
                    *self.last_state_change.lock().await = Instant::now();
                    *self.failure_count.lock().await = 0;
                    *success_count = 0;
                }
            }
            CircuitState::Closed => {
                // Normal success, potentially reset failure count
                *self.failure_count.lock().await = 0;
            }
            _ => {}
        }
    }

    /// Handle failed call
    async fn on_failure(&self) {
        *self.total_failures.lock().await += 1;
        *self.last_failure_time.lock().await = Some(Instant::now());

        let mut state = self.state.write().await;

        match *state {
            CircuitState::Closed => {
                let mut failure_count = self.failure_count.lock().await;
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    error!(
                        "Circuit breaker transitioning from CLOSED to OPEN after {} failures",
                        *failure_count
                    );
                    *state = CircuitState::Open;
                    *self.last_state_change.lock().await = Instant::now();
                }
            }
            CircuitState::HalfOpen => {
                warn!("Circuit breaker transitioning from HALF-OPEN to OPEN due to failure");
                *state = CircuitState::Open;
                *self.last_state_change.lock().await = Instant::now();
                *self.success_count.lock().await = 0;
            }
            _ => {}
        }
    }

    /// Get current circuit state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get circuit breaker metrics
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            state: *self.state.read().await,
            total_requests: *self.total_requests.lock().await,
            total_failures: *self.total_failures.lock().await,
            current_failure_count: *self.failure_count.lock().await,
            current_success_count: *self.success_count.lock().await,
        }
    }

    /// Reset circuit breaker to closed state
    pub async fn reset(&self) {
        info!("Manually resetting circuit breaker to CLOSED");
        *self.state.write().await = CircuitState::Closed;
        *self.failure_count.lock().await = 0;
        *self.success_count.lock().await = 0;
        *self.last_state_change.lock().await = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub state: CircuitState,
    pub total_requests: u64,
    pub total_failures: u64,
    pub current_failure_count: u32,
    pub current_success_count: u32,
}

impl CircuitBreakerMetrics {
    pub fn failure_rate(&self) -> f64 {
        if self.total_requests > 0 {
            (self.total_failures as f64 / self.total_requests as f64) * 100.0
        } else {
            0.0
        }
    }
}

// ============================================================================
// Retry Policy with Exponential Backoff
// ============================================================================

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    /// Add jitter to prevent thundering herd
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

/// Execute a function with retry logic
pub async fn with_retry<F, Fut, T>(policy: &RetryPolicy, mut f: F) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempt = 0;
    let mut delay = policy.initial_delay;

    loop {
        attempt += 1;

        match f().await {
            Ok(result) => {
                if attempt > 1 {
                    debug!("Request succeeded after {} attempts", attempt);
                }
                return Ok(result);
            }
            Err(e) if attempt >= policy.max_attempts => {
                error!("Request failed after {} attempts: {}", attempt, e);
                return Err(e);
            }
            Err(e) => {
                warn!(
                    "Request failed (attempt {}/{}): {}",
                    attempt, policy.max_attempts, e
                );

                // Calculate next delay with exponential backoff
                let mut next_delay = delay.mul_f64(policy.backoff_multiplier);
                if next_delay > policy.max_delay {
                    next_delay = policy.max_delay;
                }

                // Add jitter if enabled
                if policy.jitter {
                    use rand::Rng;
                    let jitter_range = next_delay.as_millis() as u64 / 4;
                    let jitter = rand::thread_rng().gen_range(0..jitter_range);
                    next_delay += Duration::from_millis(jitter);
                }

                debug!("Retrying after {:?}", next_delay);
                tokio::time::sleep(next_delay).await;

                delay = next_delay;
            }
        }
    }
}

// ============================================================================
// Health Monitor
// ============================================================================

/// Health status of a service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Service is healthy and responding
    Healthy,
    /// Service is degraded but still operational
    Degraded,
    /// Service is unhealthy and not responding
    Unhealthy,
    /// Health status is unknown
    Unknown,
}

/// Health monitor for tracking service health
pub struct HealthMonitor {
    name: String,
    health_check_interval: Duration,
    health_check_timeout: Duration,
    consecutive_failures_for_unhealthy: u32,
    consecutive_successes_for_healthy: u32,
    status: Arc<RwLock<HealthStatus>>,
    consecutive_failures: Arc<Mutex<u32>>,
    consecutive_successes: Arc<Mutex<u32>>,
    last_check: Arc<Mutex<Option<Instant>>>,
    metrics: Arc<Mutex<HealthMetrics>>,
}

impl HealthMonitor {
    pub fn new(name: String) -> Self {
        Self {
            name,
            health_check_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            consecutive_failures_for_unhealthy: 3,
            consecutive_successes_for_healthy: 2,
            status: Arc::new(RwLock::new(HealthStatus::Unknown)),
            consecutive_failures: Arc::new(Mutex::new(0)),
            consecutive_successes: Arc::new(Mutex::new(0)),
            last_check: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(HealthMetrics::default())),
        }
    }

    /// Start health monitoring with the provided check function
    pub fn start<F, Fut>(&self, check_fn: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let status = Arc::clone(&self.status);
        let consecutive_failures = Arc::clone(&self.consecutive_failures);
        let consecutive_successes = Arc::clone(&self.consecutive_successes);
        let last_check = Arc::clone(&self.last_check);
        let metrics = Arc::clone(&self.metrics);
        let interval = self.health_check_interval;
        let timeout = self.health_check_timeout;
        let failures_threshold = self.consecutive_failures_for_unhealthy;
        let success_threshold = self.consecutive_successes_for_healthy;
        let name = self.name.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let start = Instant::now();
                let result = tokio::time::timeout(timeout, check_fn()).await;
                let duration = start.elapsed();

                *last_check.lock().await = Some(Instant::now());

                let mut metrics_guard = metrics.lock().await;
                metrics_guard.total_checks += 1;
                metrics_guard.last_check_duration = duration;

                match result {
                    Ok(Ok(())) => {
                        let mut successes = consecutive_successes.lock().await;
                        let mut failures = consecutive_failures.lock().await;

                        *successes += 1;
                        *failures = 0;
                        metrics_guard.successful_checks += 1;

                        let mut current_status = status.write().await;

                        if *successes >= success_threshold
                            && *current_status != HealthStatus::Healthy
                        {
                            info!("{}: Health status changed to HEALTHY", name);
                            *current_status = HealthStatus::Healthy;
                        } else if *successes == 1 && *current_status == HealthStatus::Unhealthy {
                            info!("{}: Health status changed to DEGRADED", name);
                            *current_status = HealthStatus::Degraded;
                        }
                    }
                    Ok(Err(e)) => {
                        warn!("{}: Health check failed: {}", name, e);
                        let mut successes = consecutive_successes.lock().await;
                        let mut failures = consecutive_failures.lock().await;

                        *failures += 1;
                        *successes = 0;
                        metrics_guard.failed_checks += 1;

                        let mut current_status = status.write().await;

                        if *failures >= failures_threshold
                            && *current_status != HealthStatus::Unhealthy
                        {
                            error!("{}: Health status changed to UNHEALTHY", name);
                            *current_status = HealthStatus::Unhealthy;
                        } else if *failures > 0 && *current_status == HealthStatus::Healthy {
                            warn!("{}: Health status changed to DEGRADED", name);
                            *current_status = HealthStatus::Degraded;
                        }
                    }
                    Err(_) => {
                        warn!("{}: Health check timed out", name);
                        let mut successes = consecutive_successes.lock().await;
                        let mut failures = consecutive_failures.lock().await;

                        *failures += 1;
                        *successes = 0;
                        metrics_guard.failed_checks += 1;

                        let mut current_status = status.write().await;

                        if *failures >= failures_threshold
                            && *current_status != HealthStatus::Unhealthy
                        {
                            error!("{}: Health status changed to UNHEALTHY after timeout", name);
                            *current_status = HealthStatus::Unhealthy;
                        } else if *failures > 0 && *current_status == HealthStatus::Healthy {
                            warn!("{}: Health status changed to DEGRADED after timeout", name);
                            *current_status = HealthStatus::Degraded;
                        }
                    }
                }
            }
        });
    }

    /// Get current health status
    pub async fn status(&self) -> HealthStatus {
        *self.status.read().await
    }

    /// Check if service is healthy
    pub async fn is_healthy(&self) -> bool {
        matches!(
            *self.status.read().await,
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// Get health metrics
    pub async fn metrics(&self) -> HealthMetrics {
        self.metrics.lock().await.clone()
    }
}

#[derive(Debug, Clone, Default)]
pub struct HealthMetrics {
    pub total_checks: u64,
    pub successful_checks: u64,
    pub failed_checks: u64,
    pub last_check_duration: Duration,
}

impl HealthMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_checks > 0 {
            (self.successful_checks as f64 / self.total_checks as f64) * 100.0
        } else {
            0.0
        }
    }
}

// ============================================================================
// Failover Manager
// ============================================================================

/// Manages failover between multiple service endpoints
pub struct FailoverManager<T> {
    endpoints: Vec<(String, T)>,
    current_index: Arc<Mutex<usize>>,
    circuit_breakers: Vec<Arc<CircuitBreaker>>,
    health_monitors: Vec<Arc<HealthMonitor>>,
}

impl<T: Clone> FailoverManager<T> {
    pub fn new(endpoints: Vec<(String, T)>) -> Self {
        let num_endpoints = endpoints.len();

        let circuit_breakers = (0..num_endpoints)
            .map(|_| Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default())))
            .collect();

        let health_monitors = endpoints
            .iter()
            .map(|(name, _)| Arc::new(HealthMonitor::new(name.clone())))
            .collect();

        Self {
            endpoints,
            current_index: Arc::new(Mutex::new(0)),
            circuit_breakers,
            health_monitors,
        }
    }

    /// Get the current active endpoint with circuit breaker
    pub async fn get_active(&self) -> Option<(T, Arc<CircuitBreaker>)> {
        let mut index = self.current_index.lock().await;

        // Try to find a healthy endpoint
        for _ in 0..self.endpoints.len() {
            let current = *index;
            let circuit_breaker = &self.circuit_breakers[current];
            let health_monitor = &self.health_monitors[current];

            // Check if endpoint is available
            let circuit_state = circuit_breaker.state().await;
            let _health_status = health_monitor.status().await;

            if circuit_state != CircuitState::Open && health_monitor.is_healthy().await {
                return Some((
                    self.endpoints[current].1.clone(),
                    Arc::clone(circuit_breaker),
                ));
            }

            // Try next endpoint
            *index = (*index + 1) % self.endpoints.len();
        }

        // All endpoints are unavailable
        None
    }

    /// Manually failover to next endpoint
    pub async fn failover(&self) {
        let mut index = self.current_index.lock().await;
        let old_index = *index;
        *index = (*index + 1) % self.endpoints.len();

        info!(
            "Failing over from {} to {}",
            self.endpoints[old_index].0, self.endpoints[*index].0
        );
    }

    /// Get all endpoint statuses
    pub async fn endpoint_statuses(&self) -> Vec<EndpointStatus> {
        let mut statuses = Vec::new();

        for i in 0..self.endpoints.len() {
            let circuit_state = self.circuit_breakers[i].state().await;
            let health_status = self.health_monitors[i].status().await;
            let is_active = *self.current_index.lock().await == i;

            statuses.push(EndpointStatus {
                name: self.endpoints[i].0.clone(),
                circuit_state,
                health_status,
                is_active,
            });
        }

        statuses
    }
}

#[derive(Debug, Clone)]
pub struct EndpointStatus {
    pub name: String,
    pub circuit_state: CircuitState,
    pub health_status: HealthStatus,
    pub is_active: bool,
}

// ============================================================================
// Reliability Wrapper for MCP Connections
// ============================================================================

/// Wraps an MCP connection with reliability features
pub struct ReliableConnection<C> {
    inner: C,
    circuit_breaker: Arc<CircuitBreaker>,
    #[allow(dead_code)]
    // Retained for potential dynamic retry tuning / metrics not yet implemented
    retry_policy: RetryPolicy,
    health_monitor: Arc<HealthMonitor>,
}

impl<C> ReliableConnection<C> {
    pub fn new(inner: C, name: String) -> Self {
        let circuit_breaker = Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        let health_monitor = Arc::new(HealthMonitor::new(name));

        Self {
            inner,
            circuit_breaker,
            retry_policy: RetryPolicy::default(),
            health_monitor,
        }
    }

    /// Get reference to inner connection
    pub fn inner(&self) -> &C {
        &self.inner
    }

    /// Get circuit breaker metrics
    pub async fn circuit_metrics(&self) -> CircuitBreakerMetrics {
        self.circuit_breaker.metrics().await
    }

    /// Get health metrics
    pub async fn health_metrics(&self) -> HealthMetrics {
        self.health_monitor.metrics().await
    }

    /// Check if connection is available
    pub async fn is_available(&self) -> bool {
        self.circuit_breaker.state().await != CircuitState::Open
            && self.health_monitor.is_healthy().await
    }
}

// Implement MCP-specific reliability wrapper
#[async_trait]
pub trait ReliableTransport {
    async fn send_request_with_reliability(&self, method: &str, params: Value) -> Result<Value>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_state_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            recovery_timeout: Duration::from_millis(100),
            success_threshold: 2,
            ..Default::default()
        };

        let cb = CircuitBreaker::new(config);

        // Initial state should be closed
        assert_eq!(cb.state().await, CircuitState::Closed);

        // First failure
        let _ = cb
            .call(async { Err::<(), _>(anyhow::anyhow!("error")) })
            .await;
        assert_eq!(cb.state().await, CircuitState::Closed);

        // Second failure - should open
        let _ = cb
            .call(async { Err::<(), _>(anyhow::anyhow!("error")) })
            .await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Wait for recovery timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should be half-open now
        let _ = cb.check_state().await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Success in half-open
        let _ = cb.call(async { Ok::<_, anyhow::Error>(()) }).await;
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Second success - should close
        let _ = cb.call(async { Ok::<_, anyhow::Error>(()) }).await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_retry_with_exponential_backoff() {
        let policy = RetryPolicy {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            jitter: false,
        };

        let mut attempt_count = 0;
        let start = Instant::now();

        let result = with_retry(&policy, || {
            attempt_count += 1;
            async move {
                if attempt_count < 3 {
                    Err(anyhow::anyhow!("error"))
                } else {
                    Ok(42)
                }
            }
        })
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count, 3);

        // Should have taken at least initial + initial*2 = 30ms
        assert!(start.elapsed() >= Duration::from_millis(30));
    }

    #[tokio::test]
    async fn test_health_monitor() {
        let monitor = HealthMonitor::new("test".to_string());

        // Initial status should be unknown
        assert_eq!(monitor.status().await, HealthStatus::Unknown);

        // Simulate health checks
        *monitor.consecutive_successes.lock().await = 2;
        *monitor.status.write().await = HealthStatus::Healthy;
        assert!(monitor.is_healthy().await);

        *monitor.status.write().await = HealthStatus::Unhealthy;
        assert!(!monitor.is_healthy().await);
    }

    #[tokio::test]
    async fn test_failover_manager() {
        let endpoints = vec![
            ("endpoint1".to_string(), "url1"),
            ("endpoint2".to_string(), "url2"),
            ("endpoint3".to_string(), "url3"),
        ];

        let manager = FailoverManager::new(endpoints);

        // Should get first endpoint initially
        let active = manager.get_active().await;
        assert!(active.is_some());

        // Manual failover
        manager.failover().await;

        // Check endpoint statuses
        let statuses = manager.endpoint_statuses().await;
        assert_eq!(statuses.len(), 3);
        assert!(statuses.iter().any(|s| s.is_active));
    }
}
