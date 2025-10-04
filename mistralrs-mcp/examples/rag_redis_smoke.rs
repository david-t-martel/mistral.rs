//! Example binary to exercise RAG-Redis MCP server tools outside test harness.
//! Usage (PowerShell):
//! ```pwsh
//! $env:RAG_REDIS_SERVER = "C:/users/david/bin/rag-redis-mcp-server.exe"
//! $env:REDIS_URL = "redis://127.0.0.1:6379"
//! cargo run -p mistralrs-mcp --example rag_redis_smoke
//! ```

use anyhow::Result;
use mistralrs_mcp::{McpClient, McpClientConfig, McpServerConfig, McpServerSource};
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    let server = std::env::var("RAG_REDIS_SERVER")?;
    let redis_url = std::env::var("REDIS_URL")?;

    let mut env_map = HashMap::new();
    env_map.insert("REDIS_URL".into(), redis_url);
    if let Ok(data_dir) = std::env::var("RAG_DATA_DIR") {
        env_map.insert("RAG_DATA_DIR".into(), data_dir);
    }
    if let Ok(cache_dir) = std::env::var("EMBEDDING_CACHE_DIR") {
        env_map.insert("EMBEDDING_CACHE_DIR".into(), cache_dir);
    }
    env_map.insert("RUST_LOG".into(), "warn".into());

    let mut client = McpClient::new(McpClientConfig {
        servers: vec![McpServerConfig {
            id: "rag-redis".into(),
            name: "RAG Redis".into(),
            source: McpServerSource::Process {
                command: server,
                args: vec![],
                work_dir: None,
                env: Some(env_map),
            },
            enabled: true,
            tool_prefix: None,
            resources: None,
            bearer_token: None,
            security_policy: None,
        }],
        auto_register_tools: true,
        tool_timeout_secs: Some(45),
        max_concurrent_calls: Some(4),
        global_security_policy: None,
    });

    client.initialize().await?;
    println!(
        "Discovered {} tools",
        client.get_tool_callbacks_with_tools().len()
    );

    let tool_map = client.get_tool_callbacks_with_tools();
    let ingest = tool_map
        .keys()
        .find(|k| k.contains("ingest") && k.contains("document"))
        .ok_or_else(|| anyhow::anyhow!("No ingest_document tool found"))?;
    let search = tool_map
        .keys()
        .find(|k| k.contains("search"))
        .ok_or_else(|| anyhow::anyhow!("No search tool found"))?;

    let ingest_cb = &tool_map.get(ingest).unwrap().callback;
    let ingest_args =
        json!({"content":"Example RAG Redis doc for smoke example.","document_id":"example_doc"})
            .to_string();
    let ingest_called = mistralrs_mcp::CalledFunction {
        name: ingest.clone(),
        arguments: ingest_args,
    };
    let ingest_res = (ingest_cb)(&ingest_called)?;
    println!("Ingest result: {}", ingest_res);

    let search_cb = &tool_map.get(search).unwrap().callback;
    let search_args = json!({"query":"example"}).to_string();
    let search_called = mistralrs_mcp::CalledFunction {
        name: search.clone(),
        arguments: search_args,
    };
    let search_res = (search_cb)(&search_called)?;
    println!("Search result: {}", search_res);

    println!("RAG Redis smoke example complete.");
    Ok(())
}
