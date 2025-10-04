//! MCP Client Tests
//!
//! Tests for the McpClient functionality including:
//! - Connection management
//! - Tool discovery
//! - Tool execution
//! - Resource operations
//! - Error handling

mod mock_server;

use anyhow::Result;
use mistralrs_mcp::{McpClient, McpClientConfig, McpServerConfig, McpServerSource};
use mock_server::{
    create_calculator_tool, create_echo_tool, /* create_failing_tool */ MockMcpServer,
    MockResource,
};
// use serde_json::json; // unused
use std::sync::Arc;

// ============================================================================
// Client Initialization Tests
// ============================================================================

#[tokio::test]
async fn test_client_creation() {
    let config = McpClientConfig::default();
    let client = McpClient::new(config);

    // Client should be created successfully
    assert_eq!(client.get_tools().len(), 0);
    assert_eq!(client.get_tool_callbacks().len(), 0);
}

#[tokio::test]
async fn test_client_with_custom_config() {
    let config = McpClientConfig {
        servers: vec![],
        auto_register_tools: false,
        tool_timeout_secs: Some(60),
        max_concurrent_calls: Some(20),
        global_security_policy: None,
    };

    let client = McpClient::new(config);
    assert_eq!(client.get_tools().len(), 0);
}

#[tokio::test]
async fn test_client_initialize_no_servers() -> Result<()> {
    let config = McpClientConfig::default();
    let mut client = McpClient::new(config);

    // Initialize should succeed even with no servers
    client.initialize().await?;

    Ok(())
}

// ============================================================================
// HTTP Server Connection Tests
// ============================================================================

#[tokio::test]
async fn test_client_connect_http_server() -> Result<()> {
    // Setup mock server
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Configure client
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
    client.initialize().await?;

    // Should have discovered tools
    assert!(client.get_tools().len() > 0);
    assert!(client.get_tool_callbacks().len() > 0);

    Ok(())
}

#[tokio::test]
async fn test_client_http_with_bearer_token() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "secure-server".to_string(),
            name: "Secure Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: Some("test-token-12345".to_string()),
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

    Ok(())
}

// ============================================================================
// WebSocket Server Connection Tests
// ============================================================================

#[tokio::test]
async fn test_client_connect_websocket_server() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "ws-server".to_string(),
            name: "WebSocket Server".to_string(),
            source: McpServerSource::WebSocket {
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

    Ok(())
}

// ============================================================================
// Multi-Server Tests
// ============================================================================

#[tokio::test]
async fn test_client_multiple_servers() -> Result<()> {
    // Setup two servers
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

    // Should have tools from both servers with prefixes
    let tools = client.get_tools();
    assert!(tools.contains_key("http_echo"));
    assert!(tools.contains_key("ws_calculate"));

    Ok(())
}

#[tokio::test]
async fn test_client_disabled_server_not_connected() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "disabled-server".to_string(),
            name: "Disabled Server".to_string(),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: false, // Disabled
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

    // Should have no tools since server is disabled
    assert_eq!(client.get_tools().len(), 0);

    Ok(())
}

// ============================================================================
// Tool Discovery Tests
// ============================================================================

#[tokio::test]
async fn test_client_auto_register_tools() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
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

    // Should have auto-registered both tools
    assert_eq!(client.get_tools().len(), 2);
    assert!(client.get_tools().contains_key("echo"));
    assert!(client.get_tools().contains_key("calculate"));

    Ok(())
}

#[tokio::test]
async fn test_client_no_auto_register() -> Result<()> {
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
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: false, // Don't auto-register
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Should have no tools registered
    assert_eq!(client.get_tools().len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_client_tool_prefix() -> Result<()> {
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
            tool_prefix: Some("test_prefix".to_string()),
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

    // Tool should be registered with prefix
    assert!(client.get_tools().contains_key("test_prefix_echo"));
    assert!(!client.get_tools().contains_key("echo"));

    Ok(())
}

// ============================================================================
// Tool Callback Tests
// ============================================================================

#[tokio::test]
async fn test_client_tool_callbacks_available() -> Result<()> {
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

    // Tool callbacks should be available
    assert!(client.get_tool_callbacks().contains_key("echo"));

    // Tool callbacks with tools should also be available
    assert!(client.get_tool_callbacks_with_tools().contains_key("echo"));

    Ok(())
}

// ============================================================================
// Resource Operations Tests
// ============================================================================

#[tokio::test]
async fn test_client_list_resources() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server
        .register_resource(MockResource {
            uri: "file:///test.txt".to_string(),
            name: "Test File".to_string(),
            description: "A test file".to_string(),
            mime_type: "text/plain".to_string(),
            content: "Hello, World!".to_string(),
        })
        .await;
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
        auto_register_tools: false,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Note: We'd need to add a method to list resources on the client
    // For now, this test demonstrates the concept

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_client_connection_failure() {
    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "nonexistent-server".to_string(),
            name: "Nonexistent Server".to_string(),
            source: McpServerSource::Http {
                url: "http://127.0.0.1:9999".to_string(),
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

    // Should fail to connect
    assert!(result.is_err());
}

#[tokio::test]
async fn test_client_partial_server_failure() -> Result<()> {
    // One valid server, one invalid
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let config = McpClientConfig {
        servers: vec![
            McpServerConfig {
                id: "good-server".to_string(),
                name: "Good Server".to_string(),
                source: McpServerSource::Http {
                    url,
                    timeout_secs: Some(5),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("good".to_string()),
                resources: None,
                bearer_token: None,
                security_policy: None,
            },
            McpServerConfig {
                id: "bad-server".to_string(),
                name: "Bad Server".to_string(),
                source: McpServerSource::Http {
                    url: "http://127.0.0.1:9999".to_string(),
                    timeout_secs: Some(1),
                    headers: None,
                },
                enabled: true,
                tool_prefix: Some("bad".to_string()),
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
    let result = client.initialize().await;

    // Should fail because one server failed
    // In production, you might want partial failures to be handled differently
    assert!(result.is_err());

    Ok(())
}

// ============================================================================
// Concurrency Tests
// ============================================================================

#[tokio::test]
async fn test_client_concurrent_tool_discovery() -> Result<()> {
    // Setup multiple servers
    let servers: Vec<_> = (0..3)
        .map(|_i| {
            let server = Arc::new(MockMcpServer::new());
            let tool = create_echo_tool();
            (server, tool)
        })
        .collect();

    let mut server_configs = vec![];

    for (_i, (server, tool)) in servers.iter().enumerate() {
        tokio::spawn({
            let server = Arc::clone(server);
            let tool = tool.clone();
            async move {
                server.register_tool(tool).await;
            }
        })
        .await?;

        let url = server.clone().run_http_server().await?;
        server_configs.push(McpServerConfig {
            id: format!("server-{}", _i),
            name: format!("Server {}", _i),
            source: McpServerSource::Http {
                url,
                timeout_secs: Some(5),
                headers: None,
            },
            enabled: true,
            tool_prefix: Some(format!("s{}", _i)),
            resources: None,
            bearer_token: None,
            security_policy: None,
        });
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let config = McpClientConfig {
        servers: server_configs,
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(10),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    // Should have tools from all servers
    assert_eq!(client.get_tools().len(), 3);

    Ok(())
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[tokio::test]
async fn test_client_default_config() {
    let config = McpClientConfig::default();

    assert_eq!(config.servers.len(), 0);
    assert!(config.auto_register_tools);
    assert_eq!(config.tool_timeout_secs, Some(30));
    assert_eq!(config.max_concurrent_calls, Some(10));
}

#[tokio::test]
async fn test_client_custom_timeouts() -> Result<()> {
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
                timeout_secs: Some(60), // 60 second timeout
                headers: None,
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(120), // 2 minute tool timeout
        max_concurrent_calls: Some(50),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client.initialize().await?;

    assert!(client.get_tools().len() > 0);

    Ok(())
}
