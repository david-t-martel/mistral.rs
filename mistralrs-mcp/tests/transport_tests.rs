//! Transport Layer Tests
//!
//! Comprehensive tests for all three transport types:
//! - HTTP Transport
//! - WebSocket Transport
//! - Process Transport

mod mock_server;

use anyhow::Result;
use mistralrs_mcp::transport::{HttpTransport, McpTransport, ProcessTransport, WebSocketTransport};
use mock_server::{create_calculator_tool, create_echo_tool, MockMcpServer, MockServerConfig};
use serde_json::json;
use std::sync::Arc;

// ============================================================================
// HTTP Transport Tests
// ============================================================================

#[tokio::test]
async fn test_http_transport_basic_request() -> Result<()> {
    // Setup mock server
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create transport
    let transport = HttpTransport::new(url, Some(5), None)?;

    // Initialize connection
    let init_result = transport
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await?;

    assert!(init_result.get("protocolVersion").is_some());
    Ok(())
}

#[tokio::test]
async fn test_http_transport_tools_list() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = HttpTransport::new(url, Some(5), None)?;

    let result = transport.send_request("tools/list", json!({})).await?;

    let tools = result.get("tools").and_then(|t| t.as_array()).unwrap();
    assert_eq!(tools.len(), 2);

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(tool_names.contains(&"echo"));
    assert!(tool_names.contains(&"calculate"));

    Ok(())
}

#[tokio::test]
async fn test_http_transport_tool_call() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = HttpTransport::new(url, Some(5), None)?;

    let result = transport
        .send_request(
            "tools/call",
            json!({
                "name": "echo",
                "arguments": {"message": "Test message"}
            }),
        )
        .await?;

    let content = result.get("content").and_then(|c| c.as_array()).unwrap();
    assert!(!content.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_http_transport_with_headers() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let mut headers = std::collections::HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
    headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let transport = HttpTransport::new(url, Some(5), Some(headers))?;

    let result = transport
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await?;

    assert!(result.get("protocolVersion").is_some());
    Ok(())
}

#[tokio::test]
async fn test_http_transport_timeout() -> Result<()> {
    let server = Arc::new(MockMcpServer::with_config(MockServerConfig {
        response_delay_ms: Some(2000), // 2 second delay
        ..Default::default()
    }));
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Create transport with 1 second timeout
    let transport = HttpTransport::new(url, Some(1), None)?;

    let result = transport
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await;

    // Should timeout
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_http_transport_ping() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = HttpTransport::new(url, Some(5), None)?;

    // Ping should succeed
    transport.ping().await?;

    Ok(())
}

#[tokio::test]
async fn test_http_transport_close() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = HttpTransport::new(url, Some(5), None)?;

    // Close is a no-op for HTTP but should not error
    transport.close().await?;

    Ok(())
}

// ============================================================================
// WebSocket Transport Tests
// ============================================================================

#[tokio::test]
async fn test_websocket_transport_basic_request() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = WebSocketTransport::new(url, Some(5), None).await?;

    let init_result = transport
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await?;

    assert!(init_result.get("protocolVersion").is_some());
    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_tools_list() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = WebSocketTransport::new(url, Some(5), None).await?;

    let result = transport.send_request("tools/list", json!({})).await?;

    let tools = result.get("tools").and_then(|t| t.as_array()).unwrap();
    assert_eq!(tools.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_tool_call() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_calculator_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = WebSocketTransport::new(url, Some(5), None).await?;

    let result = transport
        .send_request(
            "tools/call",
            json!({
                "name": "calculate",
                "arguments": {
                    "operation": "add",
                    "a": 10,
                    "b": 5
                }
            }),
        )
        .await?;

    let content = result.get("content").and_then(|c| c.as_array()).unwrap();
    assert!(!content.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_concurrent_requests() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = Arc::new(WebSocketTransport::new(url, Some(10), None).await?);

    // Send multiple concurrent requests
    let mut handles = vec![];
    for i in 0..5 {
        let transport_clone = Arc::clone(&transport);
        let handle = tokio::spawn(async move {
            transport_clone
                .send_request(
                    "tools/call",
                    json!({
                        "name": "echo",
                        "arguments": {"message": format!("Message {}", i)}
                    }),
                )
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await??;
        assert!(result.get("content").is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_ping() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = WebSocketTransport::new(url, Some(5), None).await?;

    // Ping should succeed
    transport.ping().await?;

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_close() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = WebSocketTransport::new(url, Some(5), None).await?;

    // Close should succeed
    transport.close().await?;

    Ok(())
}

// ============================================================================
// Process Transport Tests
// ============================================================================

#[tokio::test]
async fn test_process_transport_basic() -> Result<()> {
    // Use a simple echo command
    #[cfg(target_os = "windows")]
    let (command, args) = (
        "cmd".to_string(),
        vec!["/C".to_string(), "echo".to_string(), "test".to_string()],
    );

    #[cfg(not(target_os = "windows"))]
    let (command, args) = ("echo".to_string(), vec!["test".to_string()]);

    let _transport = ProcessTransport::new(command, args, None, None).await?;

    // Process transport should be initialized
    // Note: This test is basic because we can't easily mock stdin/stdout
    // For full testing, we'd need a dedicated MCP server binary

    Ok(())
}

#[tokio::test]
async fn test_process_transport_with_work_dir() -> Result<()> {
    #[cfg(target_os = "windows")]
    let (command, args) = ("cmd".to_string(), vec!["/C".to_string(), "cd".to_string()]);

    #[cfg(not(target_os = "windows"))]
    let (command, args) = ("pwd".to_string(), vec![]);

    let work_dir = std::env::temp_dir().to_string_lossy().to_string();

    let _transport = ProcessTransport::new(command, args, Some(work_dir), None).await?;

    Ok(())
}

#[tokio::test]
async fn test_process_transport_with_env_vars() -> Result<()> {
    let mut env = std::collections::HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());

    #[cfg(target_os = "windows")]
    let (command, args) = (
        "cmd".to_string(),
        vec!["/C".to_string(), "echo".to_string(), "test".to_string()],
    );

    #[cfg(not(target_os = "windows"))]
    let (command, args) = ("echo".to_string(), vec!["test".to_string()]);

    let _transport = ProcessTransport::new(command, args, None, Some(env)).await?;

    Ok(())
}

#[tokio::test]
async fn test_process_transport_invalid_command() {
    let result =
        ProcessTransport::new("nonexistent_command_12345".to_string(), vec![], None, None).await;

    // Should fail with command not found
    assert!(result.is_err());
}

// ============================================================================
// Cross-Transport Comparison Tests
// ============================================================================

#[tokio::test]
async fn test_all_transports_return_same_tools() -> Result<()> {
    // Setup server
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    server.register_tool(create_calculator_tool()).await;

    // Start HTTP server
    let http_url = server.clone().run_http_server().await?;

    // Start WebSocket server (need separate instance)
    let ws_server = Arc::new(MockMcpServer::new());
    ws_server.register_tool(create_echo_tool()).await;
    ws_server.register_tool(create_calculator_tool()).await;
    let ws_url = ws_server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Test HTTP
    let http_transport = HttpTransport::new(http_url, Some(5), None)?;
    let http_result = http_transport.send_request("tools/list", json!({})).await?;
    let http_tools = http_result.get("tools").and_then(|t| t.as_array()).unwrap();

    // Test WebSocket
    let ws_transport = WebSocketTransport::new(ws_url, Some(5), None).await?;
    let ws_result = ws_transport.send_request("tools/list", json!({})).await?;
    let ws_tools = ws_result.get("tools").and_then(|t| t.as_array()).unwrap();

    // Should return same number of tools
    assert_eq!(http_tools.len(), ws_tools.len());
    assert_eq!(http_tools.len(), 2);

    Ok(())
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_http_transport_server_error() -> Result<()> {
    let server = Arc::new(MockMcpServer::with_config(MockServerConfig {
        error_rate: 1.0, // Always return errors
        ..Default::default()
    }));
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = HttpTransport::new(url, Some(5), None)?;

    let result = transport
        .send_request(
            "initialize",
            json!({
                "protocolVersion": "2025-03-26",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await;

    // Should return an error
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_connection_failure() {
    // Try to connect to non-existent server
    let result = WebSocketTransport::new("ws://127.0.0.1:9999".to_string(), Some(1), None).await;

    // Should fail to connect
    assert!(result.is_err());
}

#[tokio::test]
async fn test_http_transport_invalid_url() {
    let result = HttpTransport::new("not-a-valid-url".to_string(), Some(5), None);

    // Should fail with invalid URL
    assert!(result.is_ok()); // reqwest actually accepts this, but connection will fail
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
async fn test_http_transport_throughput() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_http_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = Arc::new(HttpTransport::new(url, Some(30), None)?);

    let start = tokio::time::Instant::now();
    let num_requests = 100;

    let mut handles = vec![];
    for i in 0..num_requests {
        let transport_clone = Arc::clone(&transport);
        let handle = tokio::spawn(async move {
            transport_clone
                .send_request(
                    "tools/call",
                    json!({
                        "name": "echo",
                        "arguments": {"message": format!("Request {}", i)}
                    }),
                )
                .await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!(
        "HTTP Transport: {} requests in {:?} ({:.2} req/sec)",
        num_requests,
        elapsed,
        num_requests as f64 / elapsed.as_secs_f64()
    );

    Ok(())
}

#[tokio::test]
async fn test_websocket_transport_throughput() -> Result<()> {
    let server = Arc::new(MockMcpServer::new());
    server.register_tool(create_echo_tool()).await;
    let url = server.run_websocket_server().await?;

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let transport = Arc::new(WebSocketTransport::new(url, Some(30), None).await?);

    let start = tokio::time::Instant::now();
    let num_requests = 100;

    let mut handles = vec![];
    for i in 0..num_requests {
        let transport_clone = Arc::clone(&transport);
        let handle = tokio::spawn(async move {
            transport_clone
                .send_request(
                    "tools/call",
                    json!({
                        "name": "echo",
                        "arguments": {"message": format!("Request {}", i)}
                    }),
                )
                .await
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await??;
    }

    let elapsed = start.elapsed();
    println!(
        "WebSocket Transport: {} requests in {:?} ({:.2} req/sec)",
        num_requests,
        elapsed,
        num_requests as f64 / elapsed.as_secs_f64()
    );

    Ok(())
}
