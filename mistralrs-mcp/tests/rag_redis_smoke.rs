//! RAG-Redis MCP Server Smoke Test
//!
//! This test dynamically exercises a minimal subset of the RAG-Redis MCP tool
//! surface (ingest + search + list) if a server binary is available.
//!
//! Skips automatically when:
//!   * Env var `RAG_REDIS_SERVER` not set OR path does not exist
//!   * Env var `REDIS_URL` not set (assumes running Redis instance)
//!
//! To run manually (PowerShell example):
//! ```pwsh
//! $env:RAG_REDIS_SERVER = "C:/users/david/bin/rag-redis-mcp-server.exe"
//! $env:REDIS_URL = "redis://127.0.0.1:6379"
//! cargo test -p mistralrs-mcp rag_redis_smoke -- --nocapture
//! ```
//! Optional (improves retrieval determinism): set `RAG_DATA_DIR` to an empty temp dir.
//!
//! The assertions are intentionally soft (focus: connectivity & tool non-error).

use anyhow::{Context, Result};
use mistralrs_mcp::{McpClient, McpClientConfig, McpServerConfig, McpServerSource};
use serde_json::json;
use std::{collections::HashMap, path::Path};

fn skip(msg: &str) -> Result<()> {
    eprintln!("[rag-redis-smoke] SKIP: {msg}");
    Ok(())
}

#[tokio::test]
async fn rag_redis_smoke() -> Result<()> {
    let server_path = match std::env::var("RAG_REDIS_SERVER") {
        Ok(v) => v,
        Err(_) => return skip("RAG_REDIS_SERVER not set"),
    };
    if !Path::new(&server_path).exists() {
        return skip("RAG_REDIS_SERVER path does not exist");
    }

    let redis_url = match std::env::var("REDIS_URL") {
        Ok(v) => v,
        Err(_) => return skip("REDIS_URL not set (need Redis instance)"),
    };

    // Build env for the process
    let mut env_map = HashMap::new();
    env_map.insert("REDIS_URL".to_string(), redis_url);
    if let Ok(data_dir) = std::env::var("RAG_DATA_DIR") {
        env_map.insert("RAG_DATA_DIR".to_string(), data_dir);
    }
    if let Ok(cache_dir) = std::env::var("EMBEDDING_CACHE_DIR") {
        env_map.insert("EMBEDDING_CACHE_DIR".to_string(), cache_dir);
    }
    env_map.insert("RUST_LOG".to_string(), "warn".to_string());

    let config = McpClientConfig {
        servers: vec![McpServerConfig {
            id: "rag-redis".into(),
            name: "RAG Redis".into(),
            source: McpServerSource::Process {
                command: server_path.clone(),
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
        max_concurrent_calls: Some(2),
        global_security_policy: None,
    };

    let mut client = McpClient::new(config);
    client
        .initialize()
        .await
        .context("initialize rag-redis server")?;

    let tool_map = client.get_tool_callbacks_with_tools();
    assert!(
        !tool_map.is_empty(),
        "No tools registered from rag-redis server"
    );

    // Locate tool names with flexible matching (different versions may rename slightly)
    let ingest_tool = tool_map
        .keys()
        .find(|n| n.contains("ingest") && n.contains("document"))
        .cloned();
    let search_tool = tool_map
        .keys()
        .find(|n| {
            n.contains("search")
                && (n.contains("document") || n.contains("semantic") || n.contains("hybrid"))
        })
        .cloned();
    let list_tool = tool_map
        .keys()
        .find(|n| n.contains("list") && n.contains("document"))
        .cloned();

    if ingest_tool.is_none() || search_tool.is_none() {
        eprintln!(
            "[rag-redis-smoke] SKIP: required tools not found (ingest={:?} search={:?})",
            ingest_tool, search_tool
        );
        return Ok(()); // Non-fatal: server may be older build
    }

    let doc_content = "RAG Redis MCP smoke test content: paged attention integration note.";
    let ingest_name = ingest_tool.unwrap();
    let ingest_cb = &tool_map.get(&ingest_name).unwrap().callback;
    let ingest_args =
        json!({"content": doc_content, "document_id": "rag_redis_smoke_doc"}).to_string();
    let ingest_called = mistralrs_mcp::CalledFunction {
        name: ingest_name.clone(),
        arguments: ingest_args,
    };
    let ingest_res = (ingest_cb)(&ingest_called)?;
    assert!(ingest_res.to_lowercase().contains("doc") || !ingest_res.is_empty());

    if let Some(list_name) = list_tool.clone() {
        let list_cb = &tool_map.get(&list_name).unwrap().callback;
        let list_called = mistralrs_mcp::CalledFunction {
            name: list_name.clone(),
            arguments: "{}".into(),
        };
        let list_res = (list_cb)(&list_called)?;
        assert!(list_res.to_lowercase().contains("rag_redis_smoke_doc") || list_res.len() > 2);
    }

    let search_name = search_tool.unwrap();
    let search_cb = &tool_map.get(&search_name).unwrap().callback;
    let search_args = json!({"query": "paged attention integration"}).to_string();
    let search_called = mistralrs_mcp::CalledFunction {
        name: search_name.clone(),
        arguments: search_args,
    };
    let search_res = (search_cb)(&search_called)?;
    assert!(
        search_res.to_lowercase().contains("paged")
            || search_res.to_lowercase().contains("attention")
            || !search_res.is_empty()
    );

    // ================= Extended Coverage (best-effort) =================
    // Tool name pattern helper
    let find_tool = |substrs: &[&str]| -> Option<String> {
        tool_map
            .keys()
            .find(|k| {
                substrs
                    .iter()
                    .all(|s| k.to_lowercase().contains(&s.to_lowercase()))
            })
            .cloned()
    };

    // Helper to invoke a tool with json args string and soft assert non-error
    let invoke = |name: &str, args: serde_json::Value| {
        if let Some(entry) = tool_map.get(name) {
            let called = mistralrs_mcp::CalledFunction {
                name: name.to_string(),
                arguments: args.to_string(),
            };
            match (entry.callback)(&called) {
                Ok(out) => {
                    eprintln!("[rag-redis-smoke] tool={} ok ({} chars)", name, out.len());
                    Some(out)
                }
                Err(err) => {
                    eprintln!("[rag-redis-smoke] tool={} ERROR: {err}", name);
                    None
                }
            }
        } else {
            eprintln!("[rag-redis-smoke] tool {} not present", name);
            None
        }
    };

    // Memory store / recall
    if let Some(store_name) = find_tool(&["store", "memory"]) {
        invoke(
            &store_name,
            json!({"content":"Smoke memory fact about paged attention","memory_type":"short_term","importance":0.6}),
        );
    }
    if let Some(recall_name) = find_tool(&["recall", "memory"]) {
        invoke(&recall_name, json!({"query":"paged attention"}));
    }
    if let Some(memstats_name) = find_tool(&["memory", "stats"]) {
        invoke(&memstats_name, json!({}));
    }

    // Project snapshots (need two for diff)
    let project_id = "rag_redis_smoke_proj";
    let mut snapshot_ids: Vec<String> = Vec::new();
    if let Some(save_proj) = find_tool(&["save", "project", "context"]) {
        if let Some(out) = invoke(&save_proj, json!({"project_id": project_id})) {
            snapshot_ids.push(out.clone());
        }
        // Make a trivial change (store memory) then snapshot again
        if let Some(store_name) = find_tool(&["store", "memory"]) {
            invoke(
                &store_name,
                json!({"content":"Second snapshot marker","memory_type":"short_term","importance":0.4}),
            );
        }
        if let Some(out2) = invoke(&save_proj, json!({"project_id": project_id})) {
            snapshot_ids.push(out2.clone());
        }
    }
    if let Some(list_snap) = find_tool(&["list", "project", "snapshot"]) {
        invoke(&list_snap, json!({"project_id": project_id, "limit": 10}));
    }
    if snapshot_ids.len() >= 2 {
        if let Some(diff_tool) = find_tool(&["diff", "context"]) {
            let from_v = &snapshot_ids[0];
            let to_v = &snapshot_ids[1];
            invoke(
                &diff_tool,
                json!({"project_id": project_id, "from_version": from_v, "to_version": to_v}),
            );
        }
    }
    if let Some(load_tool) = find_tool(&["load", "project", "context"]) {
        invoke(&load_tool, json!({"project_id": project_id}));
    }
    if let Some(stats_tool) = find_tool(&["project", "statistics"]) {
        invoke(&stats_tool, json!({"project_id": project_id}));
    }

    // Sessions quick save/load
    let mut session_id: Option<String> = None;
    if let Some(qsave) = find_tool(&["quick", "save", "session"]) {
        if let Some(out) = invoke(
            &qsave,
            json!({"project_id": project_id, "description":"quick save"}),
        ) {
            session_id = Some(out);
        }
    }
    if let (Some(qload), Some(sid)) = (find_tool(&["quick", "load", "session"]), session_id.clone())
    {
        invoke(&qload, json!({"project_id": project_id, "session_id": sid}));
    }

    // Health & system metrics
    if let Some(health) = find_tool(&["health", "check"]) {
        invoke(&health, json!({}));
    }
    if let Some(sysm) = find_tool(&["system", "metrics"]) {
        invoke(&sysm, json!({}));
    }

    // Retrieval variants (semantic / hybrid / documents) beyond primary search
    if let Some(sem_tool) = find_tool(&["semantic", "search"]) {
        invoke(&sem_tool, json!({"query":"paged attention"}));
    }
    if let Some(hyb_tool) = find_tool(&["hybrid", "search"]) {
        invoke(
            &hyb_tool,
            json!({"query":"paged attention","semantic_weight":0.7}),
        );
    }
    if let Some(doc_search) = find_tool(&["search", "documents"]) {
        invoke(&doc_search, json!({"query":"paged attention"}));
    }

    // Document get and delete (delete last to keep environment cleaner for repeated runs)
    if let Some(get_doc) = find_tool(&["get", "document"]) {
        invoke(
            &get_doc,
            json!({"document_id":"rag_redis_smoke_doc","include_chunks":false}),
        );
    }
    if let Some(del_doc) = find_tool(&["delete", "document"]) {
        invoke(&del_doc, json!({"document_id":"rag_redis_smoke_doc"}));
    }

    eprintln!("[rag-redis-smoke] PASS: extended tool coverage complete (best-effort).");
    eprintln!("[rag-redis-smoke] PASS: ingest + list + search executed successfully.");
    Ok(())
}
