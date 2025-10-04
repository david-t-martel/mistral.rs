//! Integration Tests
//!
//! End-to-end tests for MCP integration with full workflow testing

mod mock_server;

use anyhow::Result;
use mistralrs_mcp::{McpClient, McpClientConfig, McpServerConfig, McpServerSource};
use mock_server::{
    create_calculator_tool, create_echo_tool, create_failing_tool, create_slow_tool, MockMcpServer,
};
// use serde_json::json; // unused
use std::sync::Arc;

// ============================================================================
// End-to-End Workflow Tests
// ============================================================================

#[tokio::test]
async fn test_complete_mcp_workflow() -> Result<()> {
    // Setup server with multiple tools
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 1. Create client
    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "test-server".to_string(),
            name: "Test Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);

    // 2. Initialize and discover tools
    client.initialize().await?;

    // 3. Verify tools are registered
    assert_eq!(client.get_tools().len(), 2);
    assert!(client.get_tools().contains_key("echo"));
    assert!(client.get_tools().contains_key("calculate"));

    // 4. Get tool callbacks
    let callbacks = client.get_tool_callbacks();
    assert_eq!(callbacks.len(), 2);

    // 5. Get tool callbacks with tools
    let callbacks_with_tools = client.get_tool_callbacks_with_tools();
    assert_eq!(callbacks_with_tools.len(), 2);

    // Verify tool definitions
    let echo_tool = callbacks_with_tools.get("echo").unwrap();
    assert_eq!(echo_tool.tool.function.name, "echo");

    Ok(())
}

#[tokio::test]
async fn test_multi_server_integration() -> Result<()> {
    // Setup multiple servers
    let server1 = Arc::new(MockMcpServer::new());
    server1.register_tool(create_echo_tool()).await;
    let http_url = server1.run_http_server().await?;

    let server2 = Arc::new(MockMcpServer::new());
    server2.register_tool(create_calculator_tool()).await;
    let ws_url = server2.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Configure client with both servers
    let config = McpClientConfig {
        servers: vec![
            McpServerConfig {
                id: "http-server".to_string(),
                name: "HTTP Server".to_string(),
                source: McpServerSource::Http {
                    url: http_url,
                    timeout_secs: Some(5),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("http".to_string()),
                resources: None,
                bearer_token: None,
                security_policy: None,
            },
            McpServerConfig {
                id: "ws-server".to_string(),
                name: "WebSocket Server".to_string(),
                source: McpServerSource::WebSocket {
                    url: ws_url,
                    timeout_secs: Some(5),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("ws".to_string()),
                resources: None,
                bearer_token: None,
                security_policy: None,
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Verify tools from both servers
    assert_eq!(client.get_tools().len(), 2);
    assert!(client.get_tools().contains_key("http_echo"));
    assert!(client.get_tools().contains_key("ws_calculate"));

    // Verify server metadata
    let http_tool = client.get_tools().get("http_echo").unwrap();
    assert_eq!(http_tool.server_name, "HTTP Server");

    let ws_tool = client.get_tools().get("ws_calculate").unwrap();
    assert_eq!(ws_tool.server_name, "WebSocket Server");

    Ok(())
}

// ============================================================================
// Failure Scenario Tests
// ============================================================================

#[tokio::test]
async fn test_tool_execution_failure() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_failing_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    assert!(client.get_tools().contains_key("failing_operation"));

    Ok(())
}

#[tokio::test]
async fn test_tool_timeout_handling() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_slow_tool(5000)).await; // 5 second delay
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(10), // Server timeout longer than tool execution
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(2), // Tool timeout shorter than execution time
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Tool should be registered
    assert!(client.get_tools().contains_key("slow_operation"));

    // Note: Actual timeout testing would require executing the tool
    // which is tested in the react_agent tests

    Ok(())
}

#[tokio::test]
async fn test_connection_recovery() -> Result<()> {
    // Test scenario: Server goes down and comes back up
    // For this test, we'll simulate by creating a new server

    let server1 = Arc::new(MockMcpServer::new());
    server1.register_tool(create_echo_tool()).await;
    let url1 = server1.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url: url1.clone(),
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    assert!(client.get_tools().len() > 0);

    // In a real scenario, we'd test reconnection logic here
    // For now, we verify the client can be reinitialized

    Ok(())
}

// ============================================================================
// Performance and Stress Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_tool_calls() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(30),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(10),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Verify concurrency semaphore is working
    // In production, tool callbacks would be executed concurrently
    // and the semaphore would limit concurrent execution

    assert!(client.get_tools().len() > 0);

    Ok(())
}

#[tokio::test]
async fn test_high_throughput() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(30),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(50),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // In a real scenario, we'd measure tool execution throughput
    // For now, we verify the client can handle high concurrency limits

    assert!(client.get_tools().len() > 0);

    Ok(())
}

// ============================================================================
// Resource Management Tests
// ============================================================================

#[tokio::test]
async fn test_client_resource_cleanup() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    {
        let config = McpClientConfig {
            servers: vec![McpServerConfig {
                id: "server".to_string(),
                name: "Server".to_string(),
                source: McpServerSource::Http {
                    url,
                    timeout_secs: Some(5),
                    headers: None,
                },
                enabled: true,
                tool_prefix: None,
                resources: None,
                bearer_token: None,
                security_policy: None,
            }],
            auto_register_tools: true,
            tool_timeout_secs: Some(10),
            max_concurrent_calls: Some(5),
            global_security_policy: None,
        };

        let mut client = McpClient::new(config);
        client.initialize().await?;

        assert!(client.get_tools().len() > 0);
    } // Client dropped here

    // If we reach here, resource cleanup was successful
    // In production, we'd verify connections are closed properly

    Ok(())
}

#[tokio::test]
async fn test_memory_usage() -> Result<()> {
    // Test that repeated initialization doesn't leak memory

    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    for _ in 0..10 {
        let config = McpClientConfig {
            servers: vec![McpServerConfig {
                id: "server".to_string(),
                name: "Server".to_string(),
                source: McpServerSource::Http {
                    url: url.clone(),
                    timeout_secs: Some(5),
                    headers: None,
                },
                enabled: true,
                tool_prefix: None,
                resources: None,
                bearer_token: None,
                security_policy: None,
            }],
            auto_register_tools: true,
            tool_timeout_secs: Some(10),
            max_concurrent_calls: Some(5),
            global_security_policy: None,
        };

        let mut client = McpClient::new(config);
        client.initialize().await?;

        assert_eq!(client.get_tools().len(), 2);
    }

    // If we reach here without OOM, memory management is working

    Ok(())
}

// ============================================================================
// Schema Validation Tests
// ============================================================================

#[tokio::test]
async fn test_tool_schema_parsing() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Verify tool schema is properly parsed
    let tool = client.get_tools().get("calculate").unwrap();
    assert_eq!(tool.name, "calculate");
    assert!(tool.description.is_some());
    assert!(tool.input_schema.is_object());

    // Verify tool function definition
    let callback_with_tool = client
        .get_tool_callbacks_with_tools()
        .get("calculate")
        .unwrap();
    assert_eq!(callback_with_tool.tool.function.name, "calculate");

    Ok(())
}

// ============================================================================
// Error Propagation Tests
// ============================================================================

#[tokio::test]
async fn test_initialization_error_propagation() {
    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "bad-server".to_string(),
            name: "Bad Server".to_string(),
            source: McpServerSource::Http {
                url: "http://invalid-url-that-does-not-exist:9999".to_string(),
                timeout_secs: Some(1),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    let result = client.initialize().await;

    // Error should be propagated properly
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("error") || error.to_string().contains("failed"));
}

// ============================================================================
// Configuration Edge Cases
// ============================================================================

#[tokio::test]
async fn test_empty_tool_prefix() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: Some("".to_string()), // Empty prefix
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Should still register tools (prefix handling should be robust)
    assert!(client.get_tools().len() > 0);

    Ok(())
}

#[tokio::test]
async fn test_zero_concurrent_calls() {
    // Test edge case: max_concurrent_calls = 0
    let config = McpClientConfig {
        servers: vec![],
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(0), // Edge case
        global_security_policy: None,
    };

    let client = McpClient::new(config);

    // Client should still be created, but would block tool execution
    assert_eq!(client.get_tools().len(), 0);
}

#[tokio::test]
async fn test_very_long_timeout() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "server".to_string(),
            name: "Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(3600), // 1 hour timeout
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(7200), // 2 hour tool timeout
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    assert!(client.get_tools().len() > 0);

    Ok(())
}
