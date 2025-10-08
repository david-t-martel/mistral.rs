# Security Policy System

This document describes the security policy system for agent tools, providing a tiered framework for controlling access, resource usage, command execution, and network access.

## Overview

The security policy system provides four predefined security levels with granular configuration options:

- **Strict**: Maximum security with strict sandboxing and minimal permissions
- **Moderate**: Balanced security with reasonable restrictions (default)
- **Permissive**: Minimal restrictions with most operations allowed
- **Disabled**: No security enforcement (use with extreme caution)

## Security Levels

### Strict

Maximum security profile suitable for untrusted code execution:

- **Sandbox**: Enabled, no read/write outside sandbox
- **File Access**: Limited to text files (txt, md, json, yaml, toml)
- **Commands**: Only safe read-only commands (cat, ls, head, tail, grep, wc)
- **Network**: Completely disabled
- **Resource Limits**:
  - Max file size: 10MB
  - Max batch size: 100 files
  - Max memory: 256MB
  - Max execution time: 10 seconds
  - Max concurrent operations: 5

**Use Cases**: Running untrusted agent code, testing, production environments with strict security requirements.

### Moderate (Default)

Balanced security profile suitable for most use cases:

- **Sandbox**: Enabled, allow read outside (but not write)
- **File Access**: All file types allowed, sensitive paths blocked
- **Commands**: Common utilities allowed, destructive commands blocked
- **Network**: Localhost only, ports 80/443 allowed
- **Resource Limits**:
  - Max file size: 100MB
  - Max batch size: 1000 files
  - Max memory: 1024MB
  - Max execution time: 60 seconds
  - Max concurrent operations: 20

**Use Cases**: Development environments, CI/CD pipelines, general agent operations.

### Permissive

Minimal restrictions suitable for trusted environments:

- **Sandbox**: Enabled, allow read/write outside sandbox
- **File Access**: All file types allowed, including hidden files
- **Commands**: Most commands allowed, only dangerous ones blocked (format, dd)
- **Network**: Full outbound access allowed
- **Resource Limits**:
  - Max file size: 1GB
  - Max batch size: 10000 files
  - Max memory: 4096MB
  - Max execution time: 300 seconds
  - Max concurrent operations: 100

**Use Cases**: Development machines, trusted internal tools, administrative tasks.

### Disabled

No security enforcement - all operations allowed:

- **Sandbox**: Disabled
- **File Access**: Unrestricted
- **Commands**: Unrestricted
- **Network**: Unrestricted
- **Resource Limits**: Unlimited

**Use Cases**: Testing, debugging, special administrative tasks. **NOT RECOMMENDED** for production.

## Usage

### Basic Usage

```rust
use mistralrs_agent_tools::types::{SandboxConfig, SecurityPolicy, SecurityLevel};
use mistralrs_agent_tools::tools::sandbox::Sandbox;

// Use a predefined security level
let config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    SecurityPolicy::moderate()
);
let sandbox = Sandbox::new(config);

// Strict security
let strict_config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    SecurityPolicy::strict()
);
let strict_sandbox = Sandbox::new(strict_config);

// Permissive security
let permissive_config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    SecurityPolicy::permissive()
);
let permissive_sandbox = Sandbox::new(permissive_config);
```

### Custom Security Policies

You can customize any predefined policy:

```rust
use mistralrs_agent_tools::types::{
    SecurityPolicy, ResourceLimits, SandboxPolicy, CommandPolicy, NetworkPolicy
};
use std::collections::HashSet;
use std::time::Duration;

// Start with moderate, customize resource limits
let custom_policy = SecurityPolicy::moderate()
    .with_resource_limits(ResourceLimits {
        max_file_size: 50 * 1024 * 1024,  // 50MB
        max_batch_size: 500,
        max_memory_mb: 512,
        max_execution_time: Duration::from_secs(30),
        max_concurrent_operations: 10,
        max_output_size: 5 * 1024 * 1024,
    });

// Customize command policy
let policy_with_commands = SecurityPolicy::moderate()
    .with_command_policy(CommandPolicy {
        enabled: true,
        allow_arbitrary_commands: false,
        allowed_commands: HashSet::from([
            "cat".to_string(),
            "ls".to_string(),
            "grep".to_string(),
            "find".to_string(),
        ]),
        blocked_commands: HashSet::from([
            "rm".to_string(),
            "del".to_string(),
        ]),
        allow_shell_execution: false,
        max_command_length: 2048,
    });

// Customize sandbox policy
let policy_with_sandbox = SecurityPolicy::moderate()
    .with_sandbox_policy(SandboxPolicy {
        enabled: true,
        allow_read_outside: true,
        allow_write_outside: false,
        allow_symlinks: true,
        allow_hidden_files: false,
        enforce_path_canonicalization: true,
        allowed_extensions: Some(HashSet::from([
            "txt".to_string(),
            "md".to_string(),
            "json".to_string(),
        ])),
        blocked_paths: HashSet::from([
            PathBuf::from("/etc/shadow"),
            PathBuf::from("C:\\Windows\\System32"),
        ]),
    });

// Customize network policy
let policy_with_network = SecurityPolicy::moderate()
    .with_network_policy(NetworkPolicy {
        enabled: true,
        allow_outbound: true,
        allow_inbound: false,
        allowed_hosts: HashSet::from([
            "api.example.com".to_string(),
            "localhost".to_string(),
        ]),
        blocked_hosts: HashSet::new(),
        allowed_ports: HashSet::from([80, 443, 8080]),
        max_connections: 20,
    });
```

### Legacy Mode (Backward Compatibility)

The security policy system is backward compatible with existing code:

```rust
// Legacy mode - uses old API without security policies
let legacy_config = SandboxConfig::new(PathBuf::from("/workspace"))
    .allow_read_outside(true)
    .max_read_size(100 * 1024 * 1024)
    .max_batch_size(1000);

let sandbox = Sandbox::new(legacy_config);
```

When no `SecurityPolicy` is set, the sandbox uses the legacy configuration values directly.

### Mixed Mode

You can also use the modern security policy alongside legacy setters:

```rust
// Create config with security policy
let mut config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    SecurityPolicy::moderate()
);

// Legacy setters are ignored when security policy is present
// (but you can still use them for backward compatibility)
config = config
    .allow_read_outside(false)  // Ignored - policy takes precedence
    .max_read_size(50 * 1024 * 1024);  // Ignored - policy takes precedence

// Or set policy on existing legacy config
let config = SandboxConfig::new(PathBuf::from("/workspace"))
    .allow_read_outside(true)
    .with_policy(SecurityPolicy::strict());  // Policy overrides legacy settings
```

## Security Policy Override

**⚠️ WARNING**: The override mechanism completely bypasses all security checks. Use with extreme caution!

### Enabling Override

The override capability must be explicitly enabled in the security policy:

```rust
// Create policy with override capability
let policy = SecurityPolicy::moderate()
    .with_override_enabled();

let config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    policy
);

// Create sandbox and enable override
let sandbox = Sandbox::new(config)
    .with_override(true);

// Now all security checks are bypassed
let path = sandbox.validate_read(Path::new("/etc/passwd"))?;  // Allowed!
```

### Override Behavior

When override is enabled:

- ✅ All path validation is skipped
- ✅ File size limits are ignored
- ✅ Batch size limits are ignored
- ✅ Extension restrictions are bypassed
- ✅ Blocked path checks are skipped
- ✅ Sandbox boundaries are not enforced

### When to Use Override

Override should **ONLY** be used in these specific scenarios:

1. **Emergency Access**: Recovery operations requiring temporary elevated permissions
1. **Administrative Tasks**: Maintenance operations by trusted administrators
1. **Testing**: Controlled testing environments where security is explicitly disabled
1. **Migration**: Temporary bypass during system migrations

### Best Practices for Override

```rust
// ❌ BAD: Override enabled by default
let policy = SecurityPolicy::moderate()
    .with_override_enabled();

// ✅ GOOD: Override enabled only when explicitly needed
fn create_policy(allow_override: bool) -> SecurityPolicy {
    let mut policy = SecurityPolicy::moderate();
    if allow_override {
        policy = policy.with_override_enabled();
    }
    policy
}

// ✅ GOOD: Override controlled by environment variable
let allow_override = std::env::var("ALLOW_SECURITY_OVERRIDE")
    .map(|v| v == "true")
    .unwrap_or(false);

let policy = if allow_override {
    SecurityPolicy::moderate().with_override_enabled()
} else {
    SecurityPolicy::moderate()
};

// ✅ GOOD: Log when override is used
if sandbox.is_override_enabled() {
    eprintln!("WARNING: Security policy override is active!");
}
```

## Validation Methods

The `SecurityPolicy` provides several validation methods:

```rust
let policy = SecurityPolicy::moderate();

// Validate file size
policy.validate_file_size(1024)?;  // Ok
policy.validate_file_size(1_000_000_000)?;  // Error: exceeds limit

// Validate batch size
policy.validate_batch_size(100)?;  // Ok
policy.validate_batch_size(10000)?;  // Error: exceeds limit

// Validate command
policy.validate_command("cat file.txt")?;  // Ok
policy.validate_command("rm -rf /")?;  // Error: blocked command

// Validate file extension
policy.validate_file_extension("txt")?;  // Ok (if policy restricts extensions)

// Validate path
policy.validate_path(Path::new("/etc/shadow"))?;  // Error: blocked path

// Check network access
assert!(policy.is_host_allowed("localhost"));
assert!(!policy.is_host_allowed("evil.com"));

assert!(policy.is_port_allowed(443));
```

## Migration Guide

### From Legacy to Security Policies

If you're using the old `SandboxConfig` API:

```rust
// Old code
let config = SandboxConfig::new(PathBuf::from("/workspace"))
    .allow_read_outside(true)
    .max_read_size(100 * 1024 * 1024)
    .max_batch_size(1000);
```

Migrate to security policies:

```rust
// New code
let config = SandboxConfig::with_security_policy(
    PathBuf::from("/workspace"),
    SecurityPolicy::moderate()  // Matches old defaults
);
```

Or keep your code unchanged - it will continue to work in legacy mode!

### Gradual Migration

You can adopt security policies gradually:

1. **Phase 1**: Keep existing code, no changes needed
1. **Phase 2**: Add security policies to new code only
1. **Phase 3**: Migrate critical paths to security policies
1. **Phase 4**: Complete migration, deprecate legacy API

## Security Considerations

### Defense in Depth

Security policies provide multiple layers of protection:

1. **Sandbox boundary** - Path-based access control
1. **File extension filtering** - Prevent access to sensitive file types
1. **Blocked path list** - Explicitly deny critical system paths
1. **Resource limits** - Prevent DoS attacks
1. **Command filtering** - Prevent dangerous operations
1. **Network restrictions** - Control external access

### Audit and Logging

Always log security-relevant events:

```rust
use log::{info, warn, error};

// Log policy violations
match sandbox.validate_write(path) {
    Ok(p) => {
        info!("Write access granted: {}", p.display());
    }
    Err(AgentError::SandboxViolation(msg)) => {
        error!("Sandbox violation: {}", msg);
        return Err(AgentError::SandboxViolation(msg));
    }
    Err(e) => return Err(e),
}

// Log override usage
if sandbox.is_override_enabled() {
    warn!("Security override is active - all checks bypassed!");
}
```

### Production Recommendations

For production environments:

1. **Use Strict or Moderate** - Never use Disabled in production
1. **Disable Override** - Never enable `allow_override` in production
1. **Audit Regularly** - Review security logs for violations
1. **Principle of Least Privilege** - Grant minimum necessary permissions
1. **Update Blocklists** - Keep blocked paths current with system changes

## Examples

### Example 1: File Processing Agent

```rust
use mistralrs_agent_tools::types::{SandboxConfig, SecurityPolicy};
use mistralrs_agent_tools::tools::sandbox::Sandbox;

// Strict policy for processing untrusted files
let policy = SecurityPolicy::strict();
let config = SandboxConfig::with_security_policy(
    PathBuf::from("/data/uploads"),
    policy
);
let sandbox = Sandbox::new(config);

// Process files safely
for file in files {
    match sandbox.validate_read(&file) {
        Ok(validated_path) => {
            // Safe to process
            process_file(&validated_path)?;
        }
        Err(e) => {
            eprintln!("Skipping {}: {}", file.display(), e);
        }
    }
}
```

### Example 2: Development Tool

```rust
// Permissive policy for developer tool
let policy = SecurityPolicy::permissive();
let config = SandboxConfig::with_security_policy(
    PathBuf::from("/home/user/projects"),
    policy
);
let sandbox = Sandbox::new(config);

// Developers can work freely within their workspace
let file = sandbox.validate_write(&project_file)?;
std::fs::write(file, content)?;
```

### Example 3: CI/CD Pipeline

```rust
// Moderate policy for CI/CD
let policy = SecurityPolicy::moderate()
    .with_resource_limits(ResourceLimits {
        max_file_size: 500 * 1024 * 1024,  // 500MB for artifacts
        max_batch_size: 5000,
        max_memory_mb: 2048,
        max_execution_time: Duration::from_secs(300),  // 5 minutes
        max_concurrent_operations: 50,
        max_output_size: 100 * 1024 * 1024,
    });

let config = SandboxConfig::with_security_policy(
    PathBuf::from("/ci/workspace"),
    policy
);
let sandbox = Sandbox::new(config);
```

## Testing

Test security policies thoroughly:

```rust
#[test]
fn test_security_policy() {
    let policy = SecurityPolicy::strict();
    
    // Should reject large files
    assert!(policy.validate_file_size(100 * 1024 * 1024).is_err());
    
    // Should reject blocked commands
    assert!(policy.validate_command("rm -rf /").is_err());
    
    // Should allow safe commands
    assert!(policy.validate_command("cat file.txt").is_ok());
    
    // Should block network access
    assert!(!policy.is_host_allowed("example.com"));
}

#[test]
fn test_sandbox_with_policy() {
    let policy = SecurityPolicy::strict();
    let config = SandboxConfig::with_security_policy(
        PathBuf::from("/tmp/test"),
        policy
    );
    let sandbox = Sandbox::new(config);
    
    // Test path validation
    let outside_path = PathBuf::from("/etc/passwd");
    assert!(sandbox.validate_read(&outside_path).is_err());
    
    let inside_path = PathBuf::from("/tmp/test/file.txt");
    assert!(sandbox.validate_read(&inside_path).is_ok());
}
```

## Support

For questions or issues related to the security policy system:

1. Check this documentation
1. Review the code in `src/types/security.rs` and `src/tools/sandbox.rs`
1. Open an issue on GitHub with the `security` label

## Version History

- **v1.0**: Initial security policy system implementation
  - Four predefined security levels
  - Granular policy configuration
  - Backward compatibility with legacy API
  - Security override mechanism
