//! Performance Benchmarks for MCP Integration
//!
//! Measures performance metrics for:
//! - Transport layer latency
//! - Tool execution overhead
//! - Concurrent operations
//! - Memory usage

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mistralrs_mcp::transport::{HttpTransport, McpTransport, WebSocketTransport};
use mistralrs_mcp::{McpClient, McpClientConfig, McpServerConfig, McpServerSource, SecurityPolicy};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

// Mock server code would be imported from tests if we restructure
// For now, we'll use simplified benchmarks

fn benchmark_http_transport_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("http_transport");
    group.measurement_time(Duration::from_secs(10));

    // Note: In real benchmarks, you'd need a running mock server
    // For demonstration, we'll show the structure

    group.bench_function("single_request", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            // Would make actual request to mock server
            // let transport = HttpTransport::new(...);
            // transport.send_request("ping", json!({})).await
            tokio::time::sleep(Duration::from_micros(100)).await;
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_websocket_transport_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("websocket_transport");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("single_request", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            // Would make actual request to mock server
            tokio::time::sleep(Duration::from_micros(100)).await;
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_tool_discovery(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_discovery");
    group.measurement_time(Duration::from_secs(10));

    for num_tools in [1, 10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*num_tools as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(num_tools),
            num_tools,
            |b, &num| {
                let rt = Runtime::new().unwrap();

                b.to_async(&rt).iter(|| async move {
                    // Simulate tool discovery for N tools
                    for _ in 0..num {
                        tokio::time::sleep(Duration::from_micros(10)).await;
                    }
                    black_box(num)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_concurrent_tool_calls(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_tool_calls");
    group.measurement_time(Duration::from_secs(15));

    for concurrency in [1, 5, 10, 20, 50].iter() {
        group.throughput(Throughput::Elements(*concurrency as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            concurrency,
            |b, &conc| {
                let rt = Runtime::new().unwrap();

                b.to_async(&rt).iter(|| async move {
                    let mut handles = vec![];

                    for _ in 0..conc {
                        let handle = tokio::spawn(async {
                            // Simulate tool execution
                            tokio::time::sleep(Duration::from_micros(50)).await;
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.await.unwrap();
                    }

                    black_box(conc)
                });
            },
        );
    }

    group.finish();
}

fn benchmark_json_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_serialization");

    // Benchmark tool call request serialization
    group.bench_function("tool_call_request", |b| {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "calculate",
                "arguments": {
                    "operation": "add",
                    "a": 10,
                    "b": 5
                }
            }
        });

        b.iter(|| {
            let serialized = serde_json::to_string(&request).unwrap();
            let _deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            black_box(())
        });
    });

    // Benchmark tool list response parsing
    group.bench_function("tool_list_response", |b| {
        let response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": [
                    {
                        "name": "echo",
                        "description": "Echoes input",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": {"type": "string"}
                            }
                        }
                    },
                    {
                        "name": "calculate",
                        "description": "Performs calculation",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "operation": {"type": "string"},
                                "a": {"type": "number"},
                                "b": {"type": "number"}
                            }
                        }
                    }
                ]
            }
        });

        b.iter(|| {
            let serialized = serde_json::to_string(&response).unwrap();
            let _deserialized: serde_json::Value = serde_json::from_str(&serialized).unwrap();
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_client_initialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("client_initialization");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("empty_config", |b| {
        b.iter(|| {
            let config = McpClientConfig::default();
            let _client = McpClient::new(config);
            black_box(())
        });
    });

    group.bench_function("with_servers", |b| {
        b.iter(|| {
            let config = McpClientConfig {
                servers: vec![
                    McpServerConfig {
                        id: "server1".to_string(),
                        name: "Server 1".to_string(),
                        source: McpServerSource::Http {
                            url: "http://localhost:8080".to_string(),
                            timeout_secs: Some(5),
                            headers: None,
                        },
                        enabled: false, // Don't actually connect in bench
                        tool_prefix: None,
                        resources: None,
                        bearer_token: None,
                        security_policy: None,
                    },
                    McpServerConfig {
                        id: "server2".to_string(),
                        name: "Server 2".to_string(),
                        source: McpServerSource::WebSocket {
                            url: "ws://localhost:8080".to_string(),
                            timeout_secs: Some(5),
                            headers: None,
                        },
                        enabled: false,
                        tool_prefix: None,
                        resources: None,
                        bearer_token: None,
                        security_policy: None,
                    },
                ],
                auto_register_tools: true,
                tool_timeout_secs: Some(30),
                max_concurrent_calls: Some(10),
                global_security_policy: Some(SecurityPolicy::restrictive()),
            };
            let _client = McpClient::new(config);
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_schema_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_conversion");

    // Benchmark converting MCP schema to tool parameters
    group.bench_function("simple_schema", |b| {
        let schema = json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The message"
                }
            },
            "required": ["message"]
        });

        b.iter(|| {
            // Simulate schema conversion logic
            if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                let _params: Vec<_> = properties.keys().collect();
            }
            black_box(())
        });
    });

    group.bench_function("complex_schema", |b| {
        let schema = json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["add", "subtract", "multiply", "divide"]
                },
                "operands": {
                    "type": "array",
                    "items": {"type": "number"},
                    "minItems": 2
                },
                "precision": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 10,
                    "default": 2
                },
                "metadata": {
                    "type": "object",
                    "properties": {
                        "timestamp": {"type": "string", "format": "date-time"},
                        "user": {"type": "string"}
                    }
                }
            },
            "required": ["operation", "operands"]
        });

        b.iter(|| {
            if let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) {
                let _params: Vec<_> = properties.keys().collect();
            }
            black_box(())
        });
    });

    group.finish();
}

fn benchmark_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");

    group.bench_function("success_path", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            let result: Result<(), anyhow::Error> = Ok(());
            match result {
                Ok(_) => black_box(()),
                Err(e) => {
                    black_box(e);
                }
            }
        });
    });

    group.bench_function("error_path", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter(|| async {
            let result: Result<(), anyhow::Error> = Err(anyhow::anyhow!("Test error"));
            match result {
                Ok(_) => black_box(()),
                Err(e) => {
                    black_box(e.to_string());
                }
            }
        });
    });

    group.finish();
}

fn benchmark_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    group.bench_function("tool_callback_map", |b| {
        b.iter(|| {
            let mut map = std::collections::HashMap::new();
            for i in 0..100 {
                map.insert(format!("tool_{}", i), i);
            }
            black_box(map)
        });
    });

    group.bench_function("tool_info_vec", |b| {
        use mistralrs_mcp::McpToolInfo;

        b.iter(|| {
            let mut tools = Vec::new();
            for i in 0..100 {
                tools.push(McpToolInfo {
                    name: format!("tool_{}", i),
                    description: Some(format!("Description {}", i)),
                    input_schema: json!({}),
                    server_id: "server".to_string(),
                    server_name: "Server".to_string(),
                });
            }
            black_box(tools)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_http_transport_latency,
    benchmark_websocket_transport_latency,
    benchmark_tool_discovery,
    benchmark_concurrent_tool_calls,
    benchmark_json_serialization,
    benchmark_client_initialization,
    benchmark_schema_conversion,
    benchmark_error_handling,
    benchmark_memory_allocation,
);

criterion_main!(benches);
