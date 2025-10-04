//! Example demonstrating secure MCP configuration with capability-based access control
//!
//! This example shows how to:
//! - Configure MCP servers with security policies
//! - Use different trust levels for different servers
//! - Validate and sanitize tool inputs
//! - Monitor security events through audit logging

use mistralrs_mcp::{
    McpClient, McpClientConfig, McpServerConfig, McpServerSource,
    SecurityPolicy, FilesystemPolicy, ProcessPolicy, NetworkPolicy,
    EnvironmentPolicy, RateLimitPolicy, AuditPolicy
};
use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::Result;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create a secure MCP configuration
    let config = create_secure_config()?;

    // Initialize the MCP client
    let mut client = McpClient::new(config);
    info!("Initializing secure MCP client...");
    client.initialize().await?;

    // Get registered tools
    let tools = client.get_tools();
    info!("Registered {} MCP tools with security policies", tools.len());

    // Get tool callbacks for integration with models
    let callbacks = client.get_tool_callbacks_with_tools();

    // Example: List available tools and their security constraints
    for (tool_name, tool_info) in tools.iter() {
        info!("Tool: {} from server: {}", tool_name, tool_info.server_name);
        if let Some(desc) = &tool_info.description {
            info!("  Description: {}", desc);
        }
    }

    // Example of using a tool (will be validated by security policy)
    if let Some(callback_with_tool) = callbacks.get("fs_read_file") {
        info!("Testing filesystem read tool with security validation...");

        // This would be blocked if the path is outside allowed directories
        let args = serde_json::json!({
            "path": "T:/projects/rust-mistral/mistral.rs/sandbox/test.txt"
        });

        // The security validator will check:
        // 1. Path is within allowed directories
        // 2. File extension is permitted
        // 3. No path traversal attempts
        // 4. Rate limits are not exceeded

        // Note: In real usage, this would be called by the model
        // and arguments would come from tool calling
    }

    Ok(())
}

/// Create a secure MCP configuration with different trust levels
fn create_secure_config() -> Result<McpClientConfig> {
    // Global restrictive policy for all servers (default)
    let global_policy = SecurityPolicy {
        id: "global-default".to_string(),
        description: Some("Default restrictive global policy".to_string()),
        filesystem: FilesystemPolicy {
            allowed_paths: vec![],
            blocked_paths: vec![
                PathBuf::from("/etc"),
                PathBuf::from("/sys"),
                PathBuf::from("/proc"),
                PathBuf::from("C:\\Windows"),
                PathBuf::from("C:\\Program Files"),
            ],
            allowed_extensions: Some(vec![
                ".txt".to_string(),
                ".json".to_string(),
                ".md".to_string(),
            ]),
            blocked_extensions: vec![
                ".exe".to_string(),
                ".dll".to_string(),
                ".so".to_string(),
                ".sh".to_string(),
                ".ps1".to_string(),
            ],
            max_file_size: Some(10 * 1024 * 1024), // 10MB
            allow_hidden: false,
            allow_symlinks: false,
            allow_write: false,
            allow_delete: false,
        },
        process: ProcessPolicy {
            allowed_commands: vec![],
            blocked_commands: vec![
                "rm".to_string(),
                "del".to_string(),
                "format".to_string(),
            ],
            allowed_args_patterns: vec![],
            blocked_args_patterns: vec![
                r".*[;&|`$].*".to_string(),
            ],
            max_args: Some(10),
            max_arg_length: Some(1000),
            allow_shell: false,
        },
        network: NetworkPolicy {
            allowed_urls: vec![],
            blocked_urls: vec![],
            allowed_protocols: vec!["https".to_string()],
            allowed_ports: Some(vec![443]),
            block_private_ips: true,
            block_loopback: true,
        },
        environment: EnvironmentPolicy {
            allowed_vars: vec![
                "PATH".to_string(),
                "HOME".to_string(),
                "USER".to_string(),
            ],
            blocked_vars: vec![
                "LD_PRELOAD".to_string(),
                "LD_LIBRARY_PATH".to_string(),
            ],
            sanitize_vars: vec![],
            allow_passthrough: false,
        },
        rate_limits: RateLimitPolicy {
            max_requests_per_minute: Some(60),
            max_concurrent: Some(5),
            max_total_operations: Some(1000),
        },
        audit: AuditPolicy {
            log_all_operations: false,
            log_failures: true,
            log_sensitive_access: true,
            include_arguments: false,
        },
        strict_mode: true,
    };

    // Filesystem server with sandboxed access
    let filesystem_policy = SecurityPolicy {
        id: "filesystem-sandbox".to_string(),
        description: Some("Sandboxed filesystem access".to_string()),
        filesystem: FilesystemPolicy {
            allowed_paths: vec![
                PathBuf::from("T:/projects/rust-mistral/mistral.rs/sandbox"),
            ],
            blocked_paths: vec![],
            allowed_extensions: Some(vec![
                ".txt".to_string(),
                ".json".to_string(),
                ".md".to_string(),
                ".rs".to_string(),
            ]),
            blocked_extensions: vec![
                ".exe".to_string(),
                ".dll".to_string(),
            ],
            max_file_size: Some(5 * 1024 * 1024), // 5MB
            allow_hidden: false,
            allow_symlinks: false,
            allow_write: true,
            allow_delete: false,
        },
        ..global_policy.clone()
    };

    // Time server with minimal permissions
    let time_policy = SecurityPolicy {
        id: "time-minimal".to_string(),
        description: Some("Minimal permissions for time operations".to_string()),
        filesystem: FilesystemPolicy {
            allowed_paths: vec![],
            blocked_paths: vec![],
            allowed_extensions: None,
            blocked_extensions: vec![],
            max_file_size: Some(0),
            allow_hidden: false,
            allow_symlinks: false,
            allow_write: false,
            allow_delete: false,
        },
        process: ProcessPolicy {
            allowed_commands: vec![],
            blocked_commands: vec![],
            allowed_args_patterns: vec![],
            blocked_args_patterns: vec![],
            max_args: Some(0),
            max_arg_length: Some(0),
            allow_shell: false,
        },
        network: NetworkPolicy {
            allowed_urls: vec![],
            blocked_urls: vec![],
            allowed_protocols: vec![],
            allowed_ports: None,
            block_private_ips: true,
            block_loopback: true,
        },
        environment: EnvironmentPolicy {
            allowed_vars: vec!["TZ".to_string()],
            blocked_vars: vec![],
            sanitize_vars: vec![],
            allow_passthrough: false,
        },
        rate_limits: RateLimitPolicy {
            max_requests_per_minute: Some(120),
            max_concurrent: Some(10),
            max_total_operations: None,
        },
        audit: AuditPolicy {
            log_all_operations: false,
            log_failures: false,
            log_sensitive_access: false,
            include_arguments: false,
        },
        strict_mode: false,
    };

    // Create the configuration
    let config = McpClientConfig {
        servers: vec![
            // Filesystem server with sandboxed access
            McpServerConfig {
                id: "filesystem".to_string(),
                name: "Filesystem (Sandboxed)".to_string(),
                source: McpServerSource::Process {
                    command: "npx".to_string(),
                    args: vec![
                        "-y".to_string(),
                        "@modelcontextprotocol/server-filesystem@latest".to_string(),
                        "T:/projects/rust-mistral/mistral.rs/sandbox".to_string(),
                    ],
                    work_dir: None,
                    env: Some(HashMap::from([
                        ("MCP_PROTOCOL_VERSION".to_string(), "2025-06-18".to_string()),
                    ])),
                },
                enabled: true,
                tool_prefix: Some("fs".to_string()),
                resources: Some(vec!["file://**".to_string()]),
                bearer_token: None,
                security_policy: Some(filesystem_policy),
            },
            // Time server with minimal permissions
            McpServerConfig {
                id: "time".to_string(),
                name: "Time Server".to_string(),
                source: McpServerSource::Process {
                    command: "npx".to_string(),
                    args: vec![
                        "-y".to_string(),
                        "@modelcontextprotocol/server-time@latest".to_string(),
                    ],
                    work_dir: None,
                    env: Some(HashMap::from([
                        ("MCP_PROTOCOL_VERSION".to_string(), "2025-06-18".to_string()),
                    ])),
                },
                enabled: true,
                tool_prefix: Some("time".to_string()),
                resources: None,
                bearer_token: None,
                security_policy: Some(time_policy),
            },
        ],
        auto_register_tools: true,
        tool_timeout_secs: Some(30),
        max_concurrent_calls: Some(3),
        global_security_policy: Some(global_policy),
    };

    Ok(config)
}

/// Example of handling security violations
fn handle_security_violation(error: &anyhow::Error) {
    warn!("Security violation detected: {}", error);

    // In production, you might want to:
    // 1. Send alert to security team
    // 2. Log to SIEM system
    // 3. Block the requesting client
    // 4. Trigger incident response workflow
}

/// Example of monitoring security events
async fn monitor_security_events() {
    // In a real implementation, this would:
    // 1. Subscribe to audit log events
    // 2. Detect patterns of abuse
    // 3. Update security policies dynamically
    // 4. Generate security reports
    info!("Security monitoring active");
}
