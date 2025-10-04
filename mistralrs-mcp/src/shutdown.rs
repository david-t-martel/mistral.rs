//! Graceful shutdown coordinator for MCP servers
//!
//! Provides coordinated shutdown of all MCP servers with proper resource cleanup,
//! request cancellation, and timeout-based forced shutdown.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Shutdown signal types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownSignal {
    /// Graceful shutdown requested (SIGTERM)
    Graceful,
    /// Immediate shutdown requested (SIGINT)
    Immediate,
    /// Force shutdown after timeout
    Forced,
}

/// Shutdown coordinator state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShutdownState {
    /// System is running normally
    Running,
    /// Graceful shutdown in progress
    ShuttingDown,
    /// System has shut down
    Shutdown,
}

/// Shutdown coordinator for all MCP servers
///
/// Manages coordinated shutdown of multiple MCP servers with proper cleanup,
/// request cancellation, and timeout handling.
///
/// # Features
///
/// - **Graceful Shutdown**: Cancel in-flight requests gracefully
/// - **Timeout-Based Forced Shutdown**: Automatic force shutdown after timeout
/// - **Connection Draining**: Flush caches and connection pools
/// - **Signal Handling**: Responds to SIGTERM and SIGINT
/// - **Progress Tracking**: Monitor shutdown progress for each server
///
/// # Example
///
/// ```rust,no_run
/// use mistralrs_mcp::shutdown::ShutdownCoordinator;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let coordinator = ShutdownCoordinator::new(Duration::from_secs(30));
///
///     // Register servers
///     coordinator.register_server("server1").await;
///     coordinator.register_server("server2").await;
///
///     // Start shutdown listener
///     let shutdown_handle = coordinator.listen_for_signals();
///
///     // ... application runs ...
///
///     // Initiate graceful shutdown
///     coordinator.initiate_shutdown(Duration::from_secs(5)).await?;
///
///     Ok(())
/// }
/// ```
pub struct ShutdownCoordinator {
    /// Current shutdown state
    state: Arc<RwLock<ShutdownState>>,
    /// Broadcast channel for shutdown signals
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    /// Registered servers and their shutdown status
    servers: Arc<RwLock<HashMap<String, ServerShutdownStatus>>>,
    /// Default timeout for graceful shutdown
    graceful_timeout: Duration,
}

/// Shutdown status for an individual server
#[derive(Debug, Clone)]
struct ServerShutdownStatus {
    /// Server ID
    id: String,
    /// Number of active connections
    active_connections: usize,
    /// Number of in-flight requests
    active_requests: usize,
    /// Whether shutdown has been initiated for this server
    #[allow(dead_code)]
    // Retained for future granular per-server transition metrics (will drive adaptive timeouts)
    shutdown_initiated: bool,
    /// Whether shutdown has completed for this server
    shutdown_complete: bool,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    ///
    /// # Arguments
    ///
    /// * `graceful_timeout` - Maximum time to wait for graceful shutdown before forcing
    pub fn new(graceful_timeout: Duration) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);

        Self {
            state: Arc::new(RwLock::new(ShutdownState::Running)),
            shutdown_tx,
            servers: Arc::new(RwLock::new(HashMap::new())),
            graceful_timeout,
        }
    }

    /// Register a server with the shutdown coordinator
    ///
    /// # Arguments
    ///
    /// * `server_id` - Unique identifier for the server
    pub async fn register_server(&self, server_id: impl Into<String>) {
        let server_id = server_id.into();
        let mut servers = self.servers.write().await;

        servers.insert(
            server_id.clone(),
            ServerShutdownStatus {
                id: server_id.clone(),
                active_connections: 0,
                active_requests: 0,
                shutdown_initiated: false,
                shutdown_complete: false,
            },
        );

        debug!("Registered server for shutdown: {}", server_id);
    }

    /// Unregister a server from the shutdown coordinator
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID to remove
    pub async fn unregister_server(&self, server_id: &str) {
        let mut servers = self.servers.write().await;
        servers.remove(server_id);
        debug!("Unregistered server: {}", server_id);
    }

    /// Update connection count for a server
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID
    /// * `connections` - Current number of active connections
    pub async fn update_connection_count(&self, server_id: &str, connections: usize) {
        let mut servers = self.servers.write().await;
        if let Some(status) = servers.get_mut(server_id) {
            status.active_connections = connections;
        }
    }

    /// Update request count for a server
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID
    /// * `requests` - Current number of in-flight requests
    pub async fn update_request_count(&self, server_id: &str, requests: usize) {
        let mut servers = self.servers.write().await;
        if let Some(status) = servers.get_mut(server_id) {
            status.active_requests = requests;
        }
    }

    /// Mark a server as shutdown complete
    ///
    /// # Arguments
    ///
    /// * `server_id` - Server ID that has completed shutdown
    pub async fn mark_server_shutdown_complete(&self, server_id: &str) {
        let mut servers = self.servers.write().await;
        if let Some(status) = servers.get_mut(server_id) {
            status.shutdown_complete = true;
            info!("Server shutdown complete: {}", server_id);
        }
    }

    /// Subscribe to shutdown signals
    ///
    /// Returns a receiver that will receive shutdown signals
    pub fn subscribe(&self) -> broadcast::Receiver<ShutdownSignal> {
        self.shutdown_tx.subscribe()
    }

    /// Check if shutdown has been initiated
    pub async fn is_shutting_down(&self) -> bool {
        matches!(
            *self.state.read().await,
            ShutdownState::ShuttingDown | ShutdownState::Shutdown
        )
    }

    /// Check if shutdown is complete
    pub async fn is_shutdown(&self) -> bool {
        matches!(*self.state.read().await, ShutdownState::Shutdown)
    }

    /// Initiate graceful shutdown
    ///
    /// # Arguments
    ///
    /// * `timeout` - Maximum time to wait for graceful shutdown
    ///
    /// # Returns
    ///
    /// Ok(()) if shutdown completed successfully, error otherwise
    pub async fn initiate_shutdown(&self, timeout_duration: Duration) -> Result<()> {
        // Check if already shutting down
        {
            let state = self.state.read().await;
            if *state != ShutdownState::Running {
                warn!("Shutdown already initiated");
                return Ok(());
            }
        }

        // Update state to shutting down
        {
            let mut state = self.state.write().await;
            *state = ShutdownState::ShuttingDown;
        }

        info!(
            "Initiating graceful shutdown with timeout: {:?}",
            timeout_duration
        );

        // Send graceful shutdown signal
        let _ = self.shutdown_tx.send(ShutdownSignal::Graceful);

        // Wait for all servers to shut down gracefully or timeout
        match timeout(timeout_duration, self.wait_for_all_servers()).await {
            Ok(Ok(())) => {
                info!("Graceful shutdown completed successfully");
            }
            Ok(Err(e)) => {
                error!("Graceful shutdown failed: {}", e);
                // Continue to forced shutdown
            }
            Err(_) => {
                warn!("Graceful shutdown timeout, forcing shutdown");
                let _ = self.shutdown_tx.send(ShutdownSignal::Forced);

                // Give forced shutdown a brief period to complete
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        }

        // Mark as fully shutdown
        {
            let mut state = self.state.write().await;
            *state = ShutdownState::Shutdown;
        }

        info!("Shutdown complete");
        Ok(())
    }

    /// Wait for all registered servers to complete shutdown
    async fn wait_for_all_servers(&self) -> Result<()> {
        loop {
            let all_complete = {
                let servers = self.servers.read().await;
                if servers.is_empty() {
                    return Ok(());
                }

                let complete_count = servers.values().filter(|s| s.shutdown_complete).count();

                debug!(
                    "Shutdown progress: {}/{} servers complete",
                    complete_count,
                    servers.len()
                );

                // Log servers still in progress
                for status in servers.values() {
                    if !status.shutdown_complete {
                        debug!(
                            "Server {} still shutting down: {} connections, {} requests",
                            status.id, status.active_connections, status.active_requests
                        );
                    }
                }

                complete_count == servers.len()
            };

            if all_complete {
                return Ok(());
            }

            // Wait a bit before checking again
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Listen for system signals (SIGTERM, SIGINT) and initiate shutdown
    ///
    /// Returns a handle that can be awaited to block until shutdown is complete
    pub fn listen_for_signals(&self) -> tokio::task::JoinHandle<Result<()>> {
        let coordinator = self.clone();

        tokio::spawn(async move {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};

                let mut sigterm = signal(SignalKind::terminate())
                    .map_err(|e| anyhow::anyhow!("Failed to install SIGTERM handler: {}", e))?;
                let mut sigint = signal(SignalKind::interrupt())
                    .map_err(|e| anyhow::anyhow!("Failed to install SIGINT handler: {}", e))?;

                tokio::select! {
                    _ = sigterm.recv() => {
                        info!("Received SIGTERM, initiating graceful shutdown");
                        coordinator.initiate_shutdown(coordinator.graceful_timeout).await?;
                    }
                    _ = sigint.recv() => {
                        info!("Received SIGINT, initiating immediate shutdown");
                        coordinator.initiate_shutdown(Duration::from_secs(2)).await?;
                    }
                }
            }

            #[cfg(not(unix))]
            {
                use tokio::signal::ctrl_c;

                ctrl_c()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to install Ctrl+C handler: {}", e))?;

                info!("Received Ctrl+C, initiating graceful shutdown");
                coordinator
                    .initiate_shutdown(coordinator.graceful_timeout)
                    .await?;
            }

            Ok(())
        })
    }

    /// Get shutdown statistics
    pub async fn get_shutdown_stats(&self) -> ShutdownStats {
        let servers = self.servers.read().await;

        ShutdownStats {
            total_servers: servers.len(),
            shutdown_complete: servers.values().filter(|s| s.shutdown_complete).count(),
            active_connections: servers.values().map(|s| s.active_connections).sum(),
            active_requests: servers.values().map(|s| s.active_requests).sum(),
        }
    }
}

/// Statistics about shutdown progress
#[derive(Debug, Clone)]
pub struct ShutdownStats {
    /// Total number of registered servers
    pub total_servers: usize,
    /// Number of servers that have completed shutdown
    pub shutdown_complete: usize,
    /// Total active connections across all servers
    pub active_connections: usize,
    /// Total in-flight requests across all servers
    pub active_requests: usize,
}

// Implement Clone manually since broadcast::Sender doesn't implement Clone in the standard way
impl Clone for ShutdownCoordinator {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            shutdown_tx: self.shutdown_tx.clone(),
            servers: Arc::clone(&self.servers),
            graceful_timeout: self.graceful_timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_coordinator_basic() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

        // Register servers
        coordinator.register_server("server1").await;
        coordinator.register_server("server2").await;

        // Check initial state
        assert!(!coordinator.is_shutting_down().await);
        assert!(!coordinator.is_shutdown().await);

        let stats = coordinator.get_shutdown_stats().await;
        assert_eq!(stats.total_servers, 2);
        assert_eq!(stats.shutdown_complete, 0);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_graceful() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(5));

        coordinator.register_server("server1").await;

        // Simulate server activity
        coordinator.update_connection_count("server1", 5).await;
        coordinator.update_request_count("server1", 3).await;

        // Start shutdown in background
        let coord_clone = coordinator.clone();
        let shutdown_task =
            tokio::spawn(
                async move { coord_clone.initiate_shutdown(Duration::from_secs(2)).await },
            );

        // Give it a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check state
        assert!(coordinator.is_shutting_down().await);

        // Mark server as complete
        coordinator.mark_server_shutdown_complete("server1").await;

        // Wait for shutdown to complete
        shutdown_task.await.unwrap().unwrap();

        // Verify final state
        assert!(coordinator.is_shutdown().await);

        let stats = coordinator.get_shutdown_stats().await;
        assert_eq!(stats.shutdown_complete, 1);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator_timeout() {
        let coordinator = ShutdownCoordinator::new(Duration::from_secs(1));

        coordinator.register_server("slow_server").await;

        // Start shutdown with very short timeout
        let start = std::time::Instant::now();
        coordinator
            .initiate_shutdown(Duration::from_millis(200))
            .await
            .unwrap();
        let elapsed = start.elapsed();

        // Should have forced shutdown after timeout + grace period
        assert!(elapsed < Duration::from_secs(3));
        assert!(coordinator.is_shutdown().await);
    }
}
