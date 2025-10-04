//! Resource monitoring for MCP servers
//!
//! Provides tracking and automatic cleanup of MCP server resources including
//! connections, active tool calls, and memory usage.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Resource monitor for MCP servers
///
/// Tracks resource usage per server and provides automatic cleanup of stale connections.
///
/// # Features
///
/// - **Connection Tracking**: Monitor open connections per server
/// - **Tool Call Tracking**: Track active tool calls and their duration
/// - **Memory Monitoring**: Monitor per-server memory usage
/// - **Automatic Cleanup**: Clean up stale connections and timed-out requests
/// - **Resource Limits**: Enforce per-server resource limits
///
/// # Example
///
/// ```rust,no_run
/// use mistralrs_mcp::resource_monitor::ResourceMonitor;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let monitor = ResourceMonitor::new(ResourceLimits {
///         max_connections_per_server: 100,
///         max_active_requests_per_server: 50,
///         connection_idle_timeout: Duration::from_secs(300),
///         request_timeout: Duration::from_secs(60),
///     });
///
///     // Start background cleanup task
///     monitor.start_cleanup_task(Duration::from_secs(30));
///
///     // Track resources
///     monitor.connection_opened("server1").await;
///     monitor.request_started("server1", "tool_call_1").await;
///
///     // ... work happens ...
///
///     monitor.request_completed("server1", "tool_call_1").await;
///     monitor.connection_closed("server1").await;
///
///     Ok(())
/// }
/// ```
pub struct ResourceMonitor {
    /// Resource statistics per server
    stats: Arc<RwLock<HashMap<String, ServerResourceStats>>>,
    /// Resource limits configuration
    limits: ResourceLimits,
}

/// Resource limits for MCP servers
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum number of connections per server
    pub max_connections_per_server: usize,
    /// Maximum number of active requests per server
    pub max_active_requests_per_server: usize,
    /// Idle timeout for connections
    pub connection_idle_timeout: Duration,
    /// Timeout for individual requests
    pub request_timeout: Duration,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_connections_per_server: 100,
            max_active_requests_per_server: 50,
            connection_idle_timeout: Duration::from_secs(300), // 5 minutes
            request_timeout: Duration::from_secs(60),          // 1 minute
        }
    }
}

/// Resource statistics for a single server
#[derive(Debug, Clone)]
struct ServerResourceStats {
    /// Server ID
    server_id: String,
    /// Number of open connections
    open_connections: usize,
    /// Active tool calls with their start times
    active_requests: HashMap<String, Instant>,
    /// Last activity timestamp
    last_activity: Instant,
    /// Connection opened timestamps (for idle detection)
    connection_timestamps: Vec<Instant>,
}

impl ServerResourceStats {
    fn new(server_id: String) -> Self {
        Self {
            server_id,
            open_connections: 0,
            active_requests: HashMap::new(),
            last_activity: Instant::now(),
            connection_timestamps: Vec::new(),
        }
    }
}

impl ResourceMonitor {
    /// Create a new resource monitor with the given limits
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            stats: Arc::new(RwLock::new(HashMap::new())),
            limits,
        }
    }

    /// Record a connection being opened for a server
    ///
    /// # Returns
    ///
    /// Ok(()) if the connection is allowed, error if limit exceeded
    pub async fn connection_opened(&self, server_id: &str) -> anyhow::Result<()> {
        let mut stats = self.stats.write().await;
        let server_stats = stats
            .entry(server_id.to_string())
            .or_insert_with(|| ServerResourceStats::new(server_id.to_string()));

        // Check connection limit
        if server_stats.open_connections >= self.limits.max_connections_per_server {
            return Err(anyhow::anyhow!(
                "Connection limit exceeded for server {}: {}/{}",
                server_id,
                server_stats.open_connections,
                self.limits.max_connections_per_server
            ));
        }

        server_stats.open_connections += 1;
        server_stats.connection_timestamps.push(Instant::now());
        server_stats.last_activity = Instant::now();

        debug!(
            "Connection opened for {}: {} total connections",
            server_id, server_stats.open_connections
        );

        Ok(())
    }

    /// Record a connection being closed for a server
    pub async fn connection_closed(&self, server_id: &str) {
        let mut stats = self.stats.write().await;
        if let Some(server_stats) = stats.get_mut(server_id) {
            if server_stats.open_connections > 0 {
                server_stats.open_connections -= 1;

                // Remove oldest connection timestamp
                if !server_stats.connection_timestamps.is_empty() {
                    server_stats.connection_timestamps.remove(0);
                }

                debug!(
                    "Connection closed for {}: {} remaining",
                    server_id, server_stats.open_connections
                );
            } else {
                warn!(
                    "Connection close called but no connections tracked for {}",
                    server_id
                );
            }
        }
    }

    /// Record a request starting for a server
    ///
    /// # Returns
    ///
    /// Ok(()) if the request is allowed, error if limit exceeded
    pub async fn request_started(&self, server_id: &str, request_id: &str) -> anyhow::Result<()> {
        let mut stats = self.stats.write().await;
        let server_stats = stats
            .entry(server_id.to_string())
            .or_insert_with(|| ServerResourceStats::new(server_id.to_string()));

        // Check request limit
        if server_stats.active_requests.len() >= self.limits.max_active_requests_per_server {
            return Err(anyhow::anyhow!(
                "Request limit exceeded for server {}: {}/{}",
                server_id,
                server_stats.active_requests.len(),
                self.limits.max_active_requests_per_server
            ));
        }

        server_stats
            .active_requests
            .insert(request_id.to_string(), Instant::now());
        server_stats.last_activity = Instant::now();

        debug!(
            "Request started for {}: {} active requests",
            server_id,
            server_stats.active_requests.len()
        );

        Ok(())
    }

    /// Record a request completing for a server
    pub async fn request_completed(&self, server_id: &str, request_id: &str) {
        let mut stats = self.stats.write().await;
        if let Some(server_stats) = stats.get_mut(server_id) {
            if let Some(start_time) = server_stats.active_requests.remove(request_id) {
                let duration = start_time.elapsed();
                debug!(
                    "Request completed for {}: {} ms, {} active remaining",
                    server_id,
                    duration.as_millis(),
                    server_stats.active_requests.len()
                );
            } else {
                warn!(
                    "Request completion called but request not tracked: {}",
                    request_id
                );
            }
        }
    }

    /// Get current resource statistics for a server
    pub async fn get_stats(&self, server_id: &str) -> Option<ResourceStats> {
        let stats = self.stats.read().await;
        stats.get(server_id).map(|s| ResourceStats {
            server_id: s.server_id.clone(),
            open_connections: s.open_connections,
            active_requests: s.active_requests.len(),
            last_activity_seconds_ago: s.last_activity.elapsed().as_secs(),
        })
    }

    /// Get statistics for all servers
    pub async fn get_all_stats(&self) -> Vec<ResourceStats> {
        let stats = self.stats.read().await;
        stats
            .values()
            .map(|s| ResourceStats {
                server_id: s.server_id.clone(),
                open_connections: s.open_connections,
                active_requests: s.active_requests.len(),
                last_activity_seconds_ago: s.last_activity.elapsed().as_secs(),
            })
            .collect()
    }

    /// Clean up stale connections and timed-out requests
    ///
    /// Returns the number of resources cleaned up
    pub async fn cleanup_stale_resources(&self) -> CleanupStats {
        let mut stats = self.stats.write().await;
        let now = Instant::now();

        let mut stale_connections = 0;
        let mut timed_out_requests = 0;

        for server_stats in stats.values_mut() {
            // Check for idle connections
            server_stats.connection_timestamps.retain(|timestamp| {
                let is_stale = now.duration_since(*timestamp) > self.limits.connection_idle_timeout;
                if is_stale {
                    stale_connections += 1;
                    if server_stats.open_connections > 0 {
                        server_stats.open_connections -= 1;
                    }
                }
                !is_stale
            });

            // Check for timed-out requests
            server_stats
                .active_requests
                .retain(|request_id, start_time| {
                    let is_timed_out =
                        now.duration_since(*start_time) > self.limits.request_timeout;
                    if is_timed_out {
                        warn!(
                            "Request timed out for server {}: {} ({}s)",
                            server_stats.server_id,
                            request_id,
                            now.duration_since(*start_time).as_secs()
                        );
                        timed_out_requests += 1;
                    }
                    !is_timed_out
                });
        }

        if stale_connections > 0 || timed_out_requests > 0 {
            info!(
                "Cleaned up {} stale connections and {} timed-out requests",
                stale_connections, timed_out_requests
            );
        }

        CleanupStats {
            stale_connections,
            timed_out_requests,
        }
    }

    /// Start a background task that periodically cleans up stale resources
    ///
    /// Returns a handle that can be used to stop the cleanup task
    pub fn start_cleanup_task(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let monitor = self.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                let cleanup_stats = monitor.cleanup_stale_resources().await;

                if cleanup_stats.stale_connections > 0 || cleanup_stats.timed_out_requests > 0 {
                    debug!(
                        "Cleanup cycle: {} stale connections, {} timed-out requests",
                        cleanup_stats.stale_connections, cleanup_stats.timed_out_requests
                    );
                }
            }
        })
    }

    /// Remove all tracking for a server (e.g., when server is unregistered)
    pub async fn remove_server(&self, server_id: &str) {
        let mut stats = self.stats.write().await;
        stats.remove(server_id);
        debug!("Removed resource tracking for server: {}", server_id);
    }
}

impl Clone for ResourceMonitor {
    fn clone(&self) -> Self {
        Self {
            stats: Arc::clone(&self.stats),
            limits: self.limits.clone(),
        }
    }
}

/// Public resource statistics for a server
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Server ID
    pub server_id: String,
    /// Number of open connections
    pub open_connections: usize,
    /// Number of active requests
    pub active_requests: usize,
    /// Seconds since last activity
    pub last_activity_seconds_ago: u64,
}

/// Statistics from a cleanup cycle
#[derive(Debug, Clone, Copy)]
pub struct CleanupStats {
    /// Number of stale connections cleaned up
    pub stale_connections: usize,
    /// Number of timed-out requests cleaned up
    pub timed_out_requests: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_tracking() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());

        monitor.connection_opened("server1").await.unwrap();
        monitor.connection_opened("server1").await.unwrap();

        let stats = monitor.get_stats("server1").await.unwrap();
        assert_eq!(stats.open_connections, 2);

        monitor.connection_closed("server1").await;

        let stats = monitor.get_stats("server1").await.unwrap();
        assert_eq!(stats.open_connections, 1);
    }

    #[tokio::test]
    async fn test_request_tracking() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());

        monitor.request_started("server1", "req1").await.unwrap();
        monitor.request_started("server1", "req2").await.unwrap();

        let stats = monitor.get_stats("server1").await.unwrap();
        assert_eq!(stats.active_requests, 2);

        monitor.request_completed("server1", "req1").await;

        let stats = monitor.get_stats("server1").await.unwrap();
        assert_eq!(stats.active_requests, 1);
    }

    #[tokio::test]
    async fn test_connection_limit() {
        let limits = ResourceLimits {
            max_connections_per_server: 2,
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        monitor.connection_opened("server1").await.unwrap();
        monitor.connection_opened("server1").await.unwrap();

        // Third connection should fail
        let result = monitor.connection_opened("server1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_timeout() {
        let limits = ResourceLimits {
            request_timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        monitor.request_started("server1", "req1").await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        let cleanup_stats = monitor.cleanup_stale_resources().await;
        assert_eq!(cleanup_stats.timed_out_requests, 1);

        let stats = monitor.get_stats("server1").await.unwrap();
        assert_eq!(stats.active_requests, 0);
    }
}
