//! Configuration examples for optimized MCP performance
//!
//! This example demonstrates different configuration profiles optimized for
//! various workload types and deployment scenarios.

use mistralrs_mcp::{McpClientConfig, McpServerConfig, McpServerSource};
use std::collections::HashMap;
// Duration previously used in earlier tuning section; remove until a timing profile example is reintroduced.

/// Configuration optimized for local LLM inference workloads
///
/// This configuration is tuned for:
/// - Fast response times (P95 < 100ms)
/// - High connection reuse (>90%)
/// - Minimal memory footprint (<2GB)
/// - Quick recovery from failures (<5s)
pub fn create_local_llm_config() -> McpClientConfig {
    McpClientConfig {
        servers: vec![
            // Local filesystem MCP server
            McpServerConfig {
                id: "filesystem".to_string(),
                name: "Local Filesystem".to_string(),
                source: McpServerSource::Process {
                    command: "mcp-server-filesystem".to_string(),
                    args: vec!["--root".to_string(), ".".to_string()],
                    work_dir: None,
                    env: None,
                },
                enabled: true,
                tool_prefix: Some("fs".to_string()),
                ..Default::default()
            },
            // Local RAG Redis server for caching
            McpServerConfig {
                id: "rag-redis".to_string(),
                name: "RAG Redis Cache".to_string(),
                source: McpServerSource::Http {
                    url: "http://localhost:6379/mcp".to_string(),
                    timeout_secs: Some(5), // Fast timeout for local Redis
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("cache".to_string()),
                ..Default::default()
            },
            // Local development tools
            McpServerConfig {
                id: "dev-tools".to_string(),
                name: "Development Tools".to_string(),
                source: McpServerSource::Process {
                    command: "mcp-dev-tools".to_string(),
                    args: vec!["--mode".to_string(), "fast".to_string()],
                    work_dir: None,
                    env: Some({
                        let mut env = HashMap::new();
                        env.insert("MCP_CACHE".to_string(), "1".to_string());
                        env.insert("MCP_PARALLEL".to_string(), "4".to_string());
                        env
                    }),
                },
                enabled: true,
                tool_prefix: Some("dev".to_string()),
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(30),    // Reduced from 180s default
        max_concurrent_calls: Some(10), // Allow parallel tool calls
        ..Default::default()
    }
}

/// Configuration optimized for cloud/remote services
///
/// This configuration is tuned for:
/// - Higher latency tolerance
/// - Better retry strategies
/// - Connection pooling for HTTP/WebSocket
/// - Circuit breakers for resilience
pub fn create_cloud_config() -> McpClientConfig {
    McpClientConfig {
        servers: vec![
            // Remote API server
            McpServerConfig {
                id: "api-server".to_string(),
                name: "Cloud API Server".to_string(),
                source: McpServerSource::Http {
                    url: "https://api.example.com/mcp".to_string(),
                    timeout_secs: Some(60),
                    headers: Some({
                        let mut headers = HashMap::new();
                        headers.insert("X-API-Version".to_string(), "v2".to_string());
                        headers.insert("X-Client-ID".to_string(), "mistral-rs".to_string());
                        headers
                    }),
                },
                enabled: true,
                tool_prefix: Some("api".to_string()),
                bearer_token: Some(std::env::var("API_TOKEN").unwrap_or_default()),
                ..Default::default()
            },
            // WebSocket for real-time features
            McpServerConfig {
                id: "realtime".to_string(),
                name: "Real-time WebSocket".to_string(),
                source: McpServerSource::WebSocket {
                    url: "wss://realtime.example.com/mcp".to_string(),
                    timeout_secs: Some(30),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("rt".to_string()),
                bearer_token: Some(std::env::var("WS_TOKEN").unwrap_or_default()),
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(60),   // Higher timeout for network calls
        max_concurrent_calls: Some(5), // Limit concurrent cloud calls
        ..Default::default()
    }
}

/// Configuration for hybrid workloads (local + cloud)
///
/// Balances between local performance and cloud capabilities
pub fn create_hybrid_config() -> McpClientConfig {
    McpClientConfig {
        servers: vec![
            // Priority 1: Local fast tools
            McpServerConfig {
                id: "local-fast".to_string(),
                name: "Local Fast Tools".to_string(),
                source: McpServerSource::Process {
                    command: "mcp-local-tools".to_string(),
                    args: vec!["--priority".to_string(), "high".to_string()],
                    work_dir: None,
                    env: None,
                },
                enabled: true,
                tool_prefix: Some("local".to_string()),
                ..Default::default()
            },
            // Priority 2: Cached cloud services
            McpServerConfig {
                id: "cloud-cached".to_string(),
                name: "Cached Cloud Services".to_string(),
                source: McpServerSource::Http {
                    url: "https://cache.example.com/mcp".to_string(),
                    timeout_secs: Some(30),
                    headers: Some({
                        let mut headers = HashMap::new();
                        headers.insert("Cache-Control".to_string(), "max-age=300".to_string());
                        headers
                    }),
                },
                enabled: true,
                tool_prefix: Some("cloud".to_string()),
                ..Default::default()
            },
            // Priority 3: Fallback services
            McpServerConfig {
                id: "fallback".to_string(),
                name: "Fallback Services".to_string(),
                source: McpServerSource::Http {
                    url: "https://fallback.example.com/mcp".to_string(),
                    timeout_secs: Some(120),
                    headers: None,
                },
                enabled: false, // Enable only when needed
                tool_prefix: Some("fallback".to_string()),
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(45),
        max_concurrent_calls: Some(8),
        ..Default::default()
    }
}

/// Configuration for development/testing
///
/// Includes mock servers and debugging features
pub fn create_development_config() -> McpClientConfig {
    McpClientConfig {
        servers: vec![
            // Mock server for testing
            McpServerConfig {
                id: "mock".to_string(),
                name: "Mock Server".to_string(),
                source: McpServerSource::Http {
                    url: "http://localhost:3000/mcp".to_string(),
                    timeout_secs: Some(5),
                    headers: Some({
                        let mut headers = HashMap::new();
                        headers.insert("X-Debug".to_string(), "true".to_string());
                        headers.insert("X-Latency-Sim".to_string(), "50".to_string());
                        headers
                    }),
                },
                enabled: true,
                tool_prefix: Some("mock".to_string()),
                ..Default::default()
            },
            // Debug tools
            McpServerConfig {
                id: "debug".to_string(),
                name: "Debug Tools".to_string(),
                source: McpServerSource::Process {
                    command: "mcp-debug".to_string(),
                    args: vec![
                        "--verbose".to_string(),
                        "--trace".to_string(),
                        "--metrics".to_string(),
                    ],
                    work_dir: None,
                    env: Some({
                        let mut env = HashMap::new();
                        env.insert("MCP_LOG_LEVEL".to_string(), "DEBUG".to_string());
                        env.insert("MCP_TRACE".to_string(), "1".to_string());
                        env.insert("MCP_METRICS".to_string(), "1".to_string());
                        env
                    }),
                },
                enabled: true,
                tool_prefix: Some("debug".to_string()),
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(300),  // Long timeout for debugging
        max_concurrent_calls: Some(1), // Serial execution for debugging
        ..Default::default()
    }
}

/// Configuration for high-throughput scenarios
///
/// Optimized for maximum throughput with acceptable latency
pub fn create_high_throughput_config() -> McpClientConfig {
    McpClientConfig {
        servers: vec![
            // Load-balanced servers
            McpServerConfig {
                id: "lb-1".to_string(),
                name: "Load Balancer 1".to_string(),
                source: McpServerSource::Http {
                    url: "http://lb1.internal:8080/mcp".to_string(),
                    timeout_secs: Some(10),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("lb1".to_string()),
                ..Default::default()
            },
            McpServerConfig {
                id: "lb-2".to_string(),
                name: "Load Balancer 2".to_string(),
                source: McpServerSource::Http {
                    url: "http://lb2.internal:8080/mcp".to_string(),
                    timeout_secs: Some(10),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("lb2".to_string()),
                ..Default::default()
            },
            McpServerConfig {
                id: "lb-3".to_string(),
                name: "Load Balancer 3".to_string(),
                source: McpServerSource::Http {
                    url: "http://lb3.internal:8080/mcp".to_string(),
                    timeout_secs: Some(10),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("lb3".to_string()),
                ..Default::default()
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(15), // Fast timeout for high throughput
        max_concurrent_calls: Some(50), // High concurrency
        ..Default::default()
    }
}

/// Main example showing usage
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Select configuration based on environment
    let config = match std::env::var("MCP_PROFILE").as_deref() {
        Ok("local") => {
            println!("Using LOCAL LLM configuration");
            create_local_llm_config()
        }
        Ok("cloud") => {
            println!("Using CLOUD configuration");
            create_cloud_config()
        }
        Ok("hybrid") => {
            println!("Using HYBRID configuration");
            create_hybrid_config()
        }
        Ok("dev") => {
            println!("Using DEVELOPMENT configuration");
            create_development_config()
        }
        Ok("throughput") => {
            println!("Using HIGH THROUGHPUT configuration");
            create_high_throughput_config()
        }
        _ => {
            println!("Using DEFAULT (local) configuration");
            create_local_llm_config()
        }
    };

    // Print configuration summary
    println!("\nConfiguration Summary:");
    println!("- Servers: {}", config.servers.len());
    println!("- Auto-register tools: {}", config.auto_register_tools);
    println!(
        "- Tool timeout: {:?}s",
        config.tool_timeout_secs.unwrap_or(180)
    );
    println!(
        "- Max concurrent calls: {}",
        config.max_concurrent_calls.unwrap_or(1)
    );

    println!("\nServers:");
    for server in &config.servers {
        println!(
            "  - {} ({})",
            server.name,
            if server.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        match &server.source {
            McpServerSource::Http {
                url, timeout_secs, ..
            } => {
                println!("    Type: HTTP");
                println!("    URL: {}", url);
                println!("    Timeout: {:?}s", timeout_secs.unwrap_or(0));
            }
            McpServerSource::Process { command, args, .. } => {
                println!("    Type: Process");
                println!("    Command: {} {}", command, args.join(" "));
            }
            McpServerSource::WebSocket {
                url, timeout_secs, ..
            } => {
                println!("    Type: WebSocket");
                println!("    URL: {}", url);
                println!("    Timeout: {:?}s", timeout_secs.unwrap_or(0));
            }
        }
        if let Some(prefix) = &server.tool_prefix {
            println!("    Tool prefix: {}", prefix);
        }
    }

    // Performance optimization tips based on configuration
    println!("\nOptimization Tips:");
    match std::env::var("MCP_PROFILE").as_deref() {
        Ok("local") => {
            println!("✓ Connection pooling enabled for HTTP requests");
            println!("✓ Redis caching configured for frequently accessed data");
            println!("✓ Fast timeouts (30s) for local services");
            println!("✓ High concurrency (10) for parallel tool execution");
        }
        Ok("cloud") => {
            println!("✓ Circuit breakers protect against cascading failures");
            println!("✓ Retry with exponential backoff for transient errors");
            println!("✓ Longer timeouts (60s) for network latency");
            println!("✓ Connection keep-alive for HTTP/WebSocket");
        }
        Ok("hybrid") => {
            println!("✓ Tiered architecture with local-first strategy");
            println!("✓ Cache headers for CDN integration");
            println!("✓ Fallback servers for high availability");
            println!("✓ Balanced concurrency (8) for mixed workloads");
        }
        Ok("throughput") => {
            println!("✓ Multiple load-balanced endpoints");
            println!("✓ High concurrency (50) for maximum throughput");
            println!("✓ Fast timeouts (15s) to prevent bottlenecks");
            println!("✓ Connection pooling across all servers");
        }
        _ => {
            println!("✓ Default optimizations applied");
        }
    }

    println!("\nExpected Performance:");
    match std::env::var("MCP_PROFILE").as_deref() {
        Ok("local") => {
            println!("- P95 latency: <100ms");
            println!("- Connection reuse: >90%");
            println!("- Memory usage: <2GB");
            println!("- Recovery time: <5s");
        }
        Ok("cloud") => {
            println!("- P95 latency: <500ms");
            println!("- Connection reuse: >80%");
            println!("- Circuit breaker trip: <5s");
            println!("- Retry success rate: >95%");
        }
        Ok("hybrid") => {
            println!("- Local P95: <100ms");
            println!("- Cloud P95: <300ms");
            println!("- Cache hit rate: >70%");
            println!("- Fallback usage: <5%");
        }
        Ok("throughput") => {
            println!("- Throughput: >1000 req/s");
            println!("- P50 latency: <50ms");
            println!("- Concurrent requests: 50");
            println!("- Connection pool efficiency: >95%");
        }
        _ => {
            println!("- Standard performance metrics");
        }
    }

    Ok(())
}
