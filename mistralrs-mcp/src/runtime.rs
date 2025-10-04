//! Tokio runtime configuration optimized for local LLM workloads
//!
//! Provides optimized Tokio runtime configuration for MCP servers
//! running alongside local LLM inference.

use std::time::Duration;

/// Tokio runtime configuration optimized for MCP servers
///
/// This configuration is tuned for the specific characteristics of MCP servers:
/// - I/O-bound operations (HTTP, WebSocket, stdin/stdout)
/// - Occasional CPU-intensive JSON parsing
/// - Long-running connections with sporadic activity
/// - Integration with compute-heavy LLM inference
///
/// # Example
///
/// ```rust,no_run
/// use mistralrs_mcp::runtime::RuntimeConfig;
///
/// #[tokio::main]
/// async fn main() {
///     // Use default optimized configuration
///     let config = RuntimeConfig::default_for_mcp();
///
///     println!("Worker threads: {}", config.worker_threads);
///     println!("Blocking threads: {}", config.max_blocking_threads);
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Number of worker threads for async tasks
    ///
    /// Defaults to: num_cpus * 2 for I/O-bound workloads
    pub worker_threads: usize,

    /// Maximum number of blocking threads
    ///
    /// Used for blocking operations like synchronous file I/O
    /// Defaults to: 512 (Tokio default)
    pub max_blocking_threads: usize,

    /// Thread stack size in bytes
    ///
    /// Defaults to: 2MB (sufficient for most async operations)
    pub thread_stack_size: usize,

    /// Thread keep-alive duration for blocking threads
    ///
    /// How long idle blocking threads are kept alive
    /// Defaults to: 10 seconds
    pub thread_keep_alive: Duration,

    /// Enable I/O driver
    ///
    /// Required for async I/O operations
    /// Defaults to: true
    pub enable_io: bool,

    /// Enable time driver
    ///
    /// Required for timeouts and intervals
    /// Defaults to: true
    pub enable_time: bool,

    /// Global queue interval for work-stealing
    ///
    /// How often workers check the global queue
    /// Lower = better fairness, Higher = better throughput
    /// Defaults to: 31 (Tokio default - good balance)
    pub global_queue_interval: u32,

    /// Event interval for I/O polling
    ///
    /// How many tasks to poll before checking I/O events
    /// Lower = more responsive I/O, Higher = better CPU efficiency
    /// Defaults to: 61 (Tokio default)
    pub event_interval: u32,
}

impl RuntimeConfig {
    /// Create a default configuration optimized for MCP servers
    ///
    /// This configuration balances I/O responsiveness with CPU efficiency,
    /// suitable for running MCP servers alongside local LLM inference.
    pub fn default_for_mcp() -> Self {
        let num_cpus = num_cpus::get();

        Self {
            // I/O-bound workload: use 2x CPU cores for better concurrency
            worker_threads: num_cpus * 2,

            // Standard blocking thread pool
            max_blocking_threads: 512,

            // 2MB stack size (sufficient for most async operations)
            thread_stack_size: 2 * 1024 * 1024,

            // Keep idle threads for 10 seconds (reduces thread churn)
            thread_keep_alive: Duration::from_secs(10),

            // Enable both I/O and time drivers
            enable_io: true,
            enable_time: true,

            // Standard Tokio work-stealing configuration
            global_queue_interval: 31,

            // Standard I/O polling configuration
            event_interval: 61,
        }
    }

    /// Create a minimal configuration for low-resource environments
    ///
    /// Reduces thread count and memory usage for systems with limited resources.
    pub fn minimal() -> Self {
        Self {
            worker_threads: 2, // Minimum for reasonable async performance
            max_blocking_threads: 16,
            thread_stack_size: 1 * 1024 * 1024, // 1MB
            thread_keep_alive: Duration::from_secs(5),
            enable_io: true,
            enable_time: true,
            global_queue_interval: 31,
            event_interval: 61,
        }
    }

    /// Create a high-throughput configuration for busy servers
    ///
    /// Optimizes for maximum throughput at the cost of higher resource usage.
    pub fn high_throughput() -> Self {
        let num_cpus = num_cpus::get();

        Self {
            worker_threads: num_cpus * 4, // Aggressive thread count
            max_blocking_threads: 1024,
            thread_stack_size: 4 * 1024 * 1024, // 4MB
            thread_keep_alive: Duration::from_secs(30),
            enable_io: true,
            enable_time: true,
            global_queue_interval: 15, // More frequent global queue checks
            event_interval: 31,        // More responsive I/O
        }
    }

    /// Apply this configuration to a Tokio runtime builder
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use tokio::runtime::Builder;
    /// use mistralrs_mcp::runtime::RuntimeConfig;
    ///
    /// let config = RuntimeConfig::default_for_mcp();
    /// let runtime = config.apply_to_builder(Builder::new_multi_thread())
    ///     .enable_all()
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn apply_to_builder(
        &self,
        mut builder: tokio::runtime::Builder,
    ) -> tokio::runtime::Builder {
        builder
            .worker_threads(self.worker_threads)
            .max_blocking_threads(self.max_blocking_threads)
            .thread_stack_size(self.thread_stack_size)
            .thread_keep_alive(self.thread_keep_alive)
            .global_queue_interval(self.global_queue_interval)
            .event_interval(self.event_interval);

        if self.enable_io {
            builder.enable_io();
        }

        if self.enable_time {
            builder.enable_time();
        }

        builder
    }

    /// Build a configured Tokio runtime
    ///
    /// Creates a multi-threaded runtime with this configuration applied.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mistralrs_mcp::runtime::RuntimeConfig;
    ///
    /// let config = RuntimeConfig::default_for_mcp();
    /// let runtime = config.build().unwrap();
    ///
    /// runtime.block_on(async {
    ///     println!("Runtime is running!");
    /// });
    /// ```
    pub fn build(self) -> std::io::Result<tokio::runtime::Runtime> {
        self.apply_to_builder(tokio::runtime::Builder::new_multi_thread())
            .build()
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::default_for_mcp()
    }
}

/// Calculate optimal worker thread count for the system
///
/// Takes into account:
/// - Available CPU cores
/// - System memory
/// - Workload type (I/O-bound vs CPU-bound)
pub fn calculate_optimal_threads(workload_type: WorkloadType) -> usize {
    let num_cpus = num_cpus::get();

    match workload_type {
        WorkloadType::IoBound => {
            // I/O-bound: use 2-4x CPU cores
            // More threads = better I/O concurrency
            (num_cpus * 2).min(32) // Cap at 32 to avoid excessive overhead
        }
        WorkloadType::CpuBound => {
            // CPU-bound: use 1x CPU cores
            // More threads = more contention
            num_cpus
        }
        WorkloadType::Mixed => {
            // Mixed workload: use 1.5x CPU cores
            ((num_cpus * 3) / 2).max(2)
        }
    }
}

/// Workload type for thread calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadType {
    /// I/O-bound workload (network, file I/O)
    IoBound,
    /// CPU-bound workload (computation, parsing)
    CpuBound,
    /// Mixed workload
    Mixed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default_for_mcp();
        let num_cpus = num_cpus::get();

        assert_eq!(config.worker_threads, num_cpus * 2);
        assert!(config.enable_io);
        assert!(config.enable_time);
    }

    #[test]
    fn test_minimal_config() {
        let config = RuntimeConfig::minimal();

        assert_eq!(config.worker_threads, 2);
        assert_eq!(config.max_blocking_threads, 16);
    }

    #[test]
    fn test_high_throughput_config() {
        let config = RuntimeConfig::high_throughput();
        let num_cpus = num_cpus::get();

        assert_eq!(config.worker_threads, num_cpus * 4);
        assert_eq!(config.max_blocking_threads, 1024);
    }

    #[test]
    fn test_thread_calculation() {
        let io_threads = calculate_optimal_threads(WorkloadType::IoBound);
        let cpu_threads = calculate_optimal_threads(WorkloadType::CpuBound);
        let mixed_threads = calculate_optimal_threads(WorkloadType::Mixed);

        assert!(io_threads >= cpu_threads);
        assert!(mixed_threads > cpu_threads);
        assert!(io_threads <= 32); // Should be capped
    }

    #[test]
    fn test_runtime_building() {
        let config = RuntimeConfig::default_for_mcp();
        let result = config.build();

        assert!(result.is_ok());
    }
}
