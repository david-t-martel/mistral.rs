//! Mock MCP Server Implementation for Testing
//!
//! This module provides a mock MCP server that simulates real MCP server behavior
//! for testing purposes. It supports all three transport types (HTTP, WebSocket, Process)
//! and implements the core MCP protocol methods.

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use mistralrs_mcp::{McpClientConfig, McpServerConfig, McpServerSource};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio_tungstenite::{accept_async, tungstenite::Message};

/// Mock MCP server that can run in different modes
pub struct MockMcpServer {
    /// Server configuration
    config: MockServerConfig,
    /// In-memory tool registry
    tools: Arc<Mutex<HashMap<String, MockTool>>>,
    /// In-memory resource registry
    resources: Arc<Mutex<HashMap<String, MockResource>>>,
    /// Request counter for testing
    request_count: Arc<Mutex<usize>>,
}

#[derive(Clone, Debug)]
pub struct MockServerConfig {
    /// Port for HTTP/WebSocket server
    pub port: u16,
    /// Simulate slow responses
    pub response_delay_ms: Option<u64>,
    /// Simulate errors
    pub error_rate: f32,
    /// Simulate timeouts
    pub timeout_rate: f32,
}

impl Default for MockServerConfig {
    fn default() -> Self {
        Self {
            port: 0, // Random port
            response_delay_ms: None,
            error_rate: 0.0,
            timeout_rate: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct MockTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Function to generate response based on arguments
    pub response_fn: Arc<dyn Fn(Value) -> Result<Value> + Send + Sync>,
}

impl std::fmt::Debug for MockTool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockTool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("input_schema", &self.input_schema)
            .field("response_fn", &"<fn>")
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct MockResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
    pub content: String,
}

impl MockMcpServer {
    /// Create a new mock server with default configuration
    pub fn new() -> Self {
        Self::with_config(MockServerConfig::default())
    }

    /// Create a new mock server with custom configuration
    pub fn with_config(config: MockServerConfig) -> Self {
        Self {
            config,
            tools: Arc::new(Mutex::new(HashMap::new())),
            resources: Arc::new(Mutex::new(HashMap::new())),
            request_count: Arc::new(Mutex::new(0)),
        }
    }

    /// Register a mock tool
    pub async fn register_tool(&self, tool: MockTool) {
        self.tools.lock().await.insert(tool.name.clone(), tool);
    }

    /// Register a mock resource
    #[allow(dead_code)] // Used by future resource-centric tests; keep implementation ready
    pub async fn register_resource(&self, resource: MockResource) {
        self.resources
            .lock()
            .await
            .insert(resource.uri.clone(), resource);
    }

    /// Get the number of requests received
    pub async fn request_count(&self) -> usize {
        *self.request_count.lock().await
    }

    /// Handle a JSON-RPC request
    async fn handle_request(&self, request: Value) -> Result<Value> {
        // Increment request counter
        *self.request_count.lock().await += 1;

        // Simulate response delay
        if let Some(delay_ms) = self.config.response_delay_ms {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }

        // Simulate random errors
        if self.config.error_rate > 0.0 && rand::random::<f32>() < self.config.error_rate {
            return Ok(json!({
                "jsonrpc": "2.0",
                "id": request.get("id"),
                "error": {
                    "code": -32000,
                    "message": "Simulated server error"
                }
            }));
        }

        // Simulate timeouts (just hang)
        if self.config.timeout_rate > 0.0 && rand::random::<f32>() < self.config.timeout_rate {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        }

        let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or(Value::Null);

        let result = match method {
            "initialize" => self.handle_initialize(params).await?,
            "tools/list" => self.handle_tools_list().await?,
            "tools/call" => self.handle_tools_call(params).await?,
            "resources/list" => self.handle_resources_list().await?,
            "resources/read" => self.handle_resources_read(params).await?,
            "ping" => json!({"status": "ok"}),
            "notifications/initialized" => {
                // Notifications don't need a response
                return Ok(Value::Null);
            }
            _ => json!({
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                }
            }),
        };

        Ok(json!({
            "jsonrpc": "2.0",
            "id": request.get("id"),
            "result": result
        }))
    }

    async fn handle_initialize(&self, _params: Value) -> Result<Value> {
        Ok(json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {},
                "resources": {}
            },
            "serverInfo": {
                "name": "mock-mcp-server",
                "version": "1.0.0"
            }
        }))
    }

    async fn handle_tools_list(&self) -> Result<Value> {
        let tools = self.tools.lock().await;
        let tool_list: Vec<Value> = tools
            .values()
            .map(|t| {
                json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": t.input_schema
                })
            })
            .collect();

        Ok(json!({
            "tools": tool_list
        }))
    }

    async fn handle_tools_call(&self, params: Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
        let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

        let tools = self.tools.lock().await;
        let tool = tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        let result = (tool.response_fn)(arguments)?;

        Ok(json!({
            "content": [
                {
                    "type": "text",
                    "text": result.to_string()
                }
            ]
        }))
    }

    async fn handle_resources_list(&self) -> Result<Value> {
        let resources = self.resources.lock().await;
        let resource_list: Vec<Value> = resources
            .values()
            .map(|r| {
                json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mimeType": r.mime_type
                })
            })
            .collect();

        Ok(json!({
            "resources": resource_list
        }))
    }

    async fn handle_resources_read(&self, params: Value) -> Result<Value> {
        let uri = params
            .get("uri")
            .and_then(|u| u.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing resource URI"))?;

        let resources = self.resources.lock().await;
        let resource = resources
            .get(uri)
            .ok_or_else(|| anyhow::anyhow!("Resource not found: {}", uri))?;

        Ok(json!({
            "contents": [
                {
                    "uri": resource.uri,
                    "mimeType": resource.mime_type,
                    "text": resource.content
                }
            ]
        }))
    }

    /// Run HTTP server (for HTTP transport testing)
    pub async fn run_http_server(self: Arc<Self>) -> Result<String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port)).await?;
        let addr = listener.local_addr()?;
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let server = Arc::clone(&self);
                tokio::spawn(async move {
                    if let Err(e) = server.handle_http_connection(stream).await {
                        eprintln!("HTTP connection error: {}", e);
                    }
                });
            }
        });

        Ok(url)
    }

    async fn handle_http_connection(&self, stream: tokio::net::TcpStream) -> Result<()> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let mut buffer = vec![0u8; 4096];
        let mut stream = stream;
        let n = stream.read(&mut buffer).await?;

        // Parse HTTP request to extract JSON body
        let request_str = String::from_utf8_lossy(&buffer[..n]);
        if let Some(body_start) = request_str.find("\r\n\r\n") {
            let body = &request_str[body_start + 4..];
            if let Ok(request) = serde_json::from_str::<Value>(body) {
                let response = self.handle_request(request).await?;
                let response_json = serde_json::to_string(&response)?;

                let http_response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_json.len(),
                    response_json
                );

                stream.write_all(http_response.as_bytes()).await?;
            }
        }

        Ok(())
    }

    /// Run WebSocket server (for WebSocket transport testing)
    pub async fn run_websocket_server(self: Arc<Self>) -> Result<String> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", self.config.port)).await?;
        let addr = listener.local_addr()?;
        let url = format!("ws://{}", addr);

        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let server = Arc::clone(&self);
                tokio::spawn(async move {
                    if let Err(e) = server.handle_websocket_connection(stream).await {
                        eprintln!("WebSocket connection error: {}", e);
                    }
                });
            }
        });

        Ok(url)
    }

    async fn handle_websocket_connection(&self, stream: tokio::net::TcpStream) -> Result<()> {
        let ws_stream = accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Ok(request) = serde_json::from_str::<Value>(&text) {
                    let response = self.handle_request(request).await?;

                    // Only send response if not null (for notifications)
                    if !response.is_null() {
                        let response_text = serde_json::to_string(&response)?;
                        write.send(Message::Text(response_text)).await?;
                    }
                }
            } else if let Message::Close(_) = msg {
                break;
            }
        }

        Ok(())
    }

    /// Run as a process (for stdio testing)
    #[allow(dead_code)] // Stdio transport path retained for parity; enable when stdio server tests are added
    pub async fn run_stdio_server(self: Arc<Self>) -> Result<()> {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut writer = stdout;
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;
            if n == 0 {
                break; // EOF
            }

            if let Ok(request) = serde_json::from_str::<Value>(line.trim()) {
                let response = self.handle_request(request).await?;

                // Only send response if not null (for notifications)
                if !response.is_null() {
                    let response_line = format!("{}\n", serde_json::to_string(&response)?);
                    writer.write_all(response_line.as_bytes()).await?;
                    writer.flush().await?;
                }
            }
        }

        Ok(())
    }
}

impl Default for MockMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper function to create a simple echo tool for testing
pub fn create_echo_tool() -> MockTool {
    MockTool {
        name: "echo".to_string(),
        description: "Echoes back the input".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message to echo"
                }
            },
            "required": ["message"]
        }),
        response_fn: Arc::new(|args| {
            let message = args
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("(no message)");
            Ok(json!(format!("Echo: {}", message)))
        }),
    }
}

/// Helper function to create a calculator tool for testing
pub fn create_calculator_tool() -> MockTool {
    MockTool {
        name: "calculate".to_string(),
        description: "Performs basic arithmetic".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "a": {
                    "type": "number"
                },
                "b": {
                    "type": "number"
                }
            },
            "required": ["operation", "a", "b"]
        }),
        response_fn: Arc::new(|args| {
            let operation = args
                .get("operation")
                .and_then(|o| o.as_str())
                .unwrap_or("add");
            let a = args.get("a").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let b = args.get("b").and_then(|v| v.as_f64()).unwrap_or(0.0);

            let result = match operation {
                "add" => a + b,
                "subtract" => a - b,
                "multiply" => a * b,
                "divide" => {
                    if b == 0.0 {
                        return Err(anyhow::anyhow!("Division by zero"));
                    }
                    a / b
                }
                _ => return Err(anyhow::anyhow!("Unknown operation")),
            };

            Ok(json!(result))
        }),
    }
}

/// Helper function to create a slow tool for timeout testing
#[allow(dead_code)] // Reserved for timeout behavior tests
pub fn create_slow_tool(delay_ms: u64) -> MockTool {
    MockTool {
        name: "slow_operation".to_string(),
        description: "An operation that takes a long time".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
        response_fn: Arc::new(move |_args| {
            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            Ok(json!("Done after slow operation"))
        }),
    }
}

/// Helper function to create a failing tool for error testing
#[allow(dead_code)] // Reserved for error propagation tests
pub fn create_failing_tool() -> MockTool {
    MockTool {
        name: "failing_operation".to_string(),
        description: "An operation that always fails".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {}
        }),
        response_fn: Arc::new(|_args| Err(anyhow::anyhow!("This tool always fails"))),
    }
}

/// Helper function to create a test MCP server config with default values
#[allow(dead_code)] // Convenience constructor for concise test declarations
pub fn create_test_server_config(
    id: String,
    name: String,
    source: McpServerSource,
) -> McpServerConfig {
    McpServerConfig {
        id,
        name,
        source,
        enabled: true,
        tool_prefix: None,
        resources: None,
        bearer_token: None,
        security_policy: None,
    }
}

/// Helper function to create a test MCP client config with default values
#[allow(dead_code)] // Convenience constructor for concise test declarations
pub fn create_test_client_config(servers: Vec<McpServerConfig>) -> McpClientConfig {
    McpClientConfig {
        servers,
        auto_register_tools: true,
        tool_timeout_secs: Some(10),
        max_concurrent_calls: Some(5),
        global_security_policy: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_server_creation() {
        let server = MockMcpServer::new();
        assert_eq!(server.request_count().await, 0);
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let server = MockMcpServer::new();
        let tool = create_echo_tool();
        server.register_tool(tool).await;

        let tools = server.tools.lock().await;
        assert!(tools.contains_key("echo"));
    }

    #[tokio::test]
    async fn test_echo_tool_execution() {
        let tool = create_echo_tool();
        let args = json!({"message": "Hello, World!"});
        let result = (tool.response_fn)(args).unwrap();
        assert_eq!(result, json!("Echo: Hello, World!"));
    }

    #[tokio::test]
    async fn test_calculator_tool_execution() {
        let tool = create_calculator_tool();

        // Test addition
        let args = json!({"operation": "add", "a": 5, "b": 3});
        let result = (tool.response_fn)(args).unwrap();
        assert_eq!(result, json!(8.0));

        // Test division by zero
        let args = json!({"operation": "divide", "a": 5, "b": 0});
        assert!((tool.response_fn)(args).is_err());
    }
}
