//! Security policy system for agent tools.
//!
//! Provides a tiered security framework with multiple predefined levels
//! and granular configuration options for sandboxing, resource limits,
//! command restrictions, and network policies.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

/// Security policy level for agent operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityLevel {
    /// Maximum security: strict sandboxing, minimal permissions, no network
    Strict,
    /// Balanced security: moderate restrictions, common operations allowed
    Moderate,
    /// Minimal restrictions: most operations allowed, relaxed limits
    Permissive,
    /// No security enforcement (use with extreme caution)
    Disabled,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        Self::Moderate
    }
}

impl SecurityLevel {
    /// Returns a human-readable description of this security level
    pub fn description(&self) -> &'static str {
        match self {
            Self::Strict => "Maximum security with strict sandboxing and minimal permissions",
            Self::Moderate => "Balanced security with reasonable restrictions",
            Self::Permissive => "Minimal restrictions with most operations allowed",
            Self::Disabled => "No security enforcement (dangerous)",
        }
    }

    /// Returns whether this level allows network access
    pub fn allows_network(&self) -> bool {
        matches!(self, Self::Permissive | Self::Disabled)
    }

    /// Returns whether this level allows arbitrary command execution
    pub fn allows_arbitrary_commands(&self) -> bool {
        matches!(self, Self::Permissive | Self::Disabled)
    }

    /// Returns default resource limits for this level
    pub fn default_resource_limits(&self) -> ResourceLimits {
        match self {
            Self::Strict => ResourceLimits {
                max_file_size: 10 * 1024 * 1024, // 10MB
                max_batch_size: 100,
                max_memory_mb: 256,
                max_execution_time: Duration::from_secs(10),
                max_concurrent_operations: 5,
                max_output_size: 1 * 1024 * 1024, // 1MB
            },
            Self::Moderate => ResourceLimits {
                max_file_size: 100 * 1024 * 1024, // 100MB
                max_batch_size: 1000,
                max_memory_mb: 1024,
                max_execution_time: Duration::from_secs(60),
                max_concurrent_operations: 20,
                max_output_size: 10 * 1024 * 1024, // 10MB
            },
            Self::Permissive => ResourceLimits {
                max_file_size: 1024 * 1024 * 1024, // 1GB
                max_batch_size: 10000,
                max_memory_mb: 4096,
                max_execution_time: Duration::from_secs(300),
                max_concurrent_operations: 100,
                max_output_size: 100 * 1024 * 1024, // 100MB
            },
            Self::Disabled => ResourceLimits {
                max_file_size: usize::MAX,
                max_batch_size: usize::MAX,
                max_memory_mb: usize::MAX,
                max_execution_time: Duration::MAX,
                max_concurrent_operations: usize::MAX,
                max_output_size: usize::MAX,
            },
        }
    }

    /// Returns default sandbox policy for this level
    pub fn default_sandbox_policy(&self) -> SandboxPolicy {
        match self {
            Self::Strict => SandboxPolicy {
                enabled: true,
                allow_read_outside: false,
                allow_write_outside: false,
                allow_symlinks: false,
                allow_hidden_files: false,
                enforce_path_canonicalization: true,
                allowed_extensions: Some(HashSet::from([
                    "txt".to_string(),
                    "md".to_string(),
                    "json".to_string(),
                    "yaml".to_string(),
                    "toml".to_string(),
                ])),
                blocked_paths: HashSet::new(),
            },
            Self::Moderate => SandboxPolicy {
                enabled: true,
                allow_read_outside: true,
                allow_write_outside: false,
                allow_symlinks: true,
                allow_hidden_files: false,
                enforce_path_canonicalization: true,
                allowed_extensions: None, // All extensions allowed
                blocked_paths: HashSet::from([
                    PathBuf::from("/etc/shadow"),
                    PathBuf::from("/etc/passwd"),
                    PathBuf::from("C:\\Windows\\System32\\config"),
                ]),
            },
            Self::Permissive => SandboxPolicy {
                enabled: true,
                allow_read_outside: true,
                allow_write_outside: true,
                allow_symlinks: true,
                allow_hidden_files: true,
                enforce_path_canonicalization: false,
                allowed_extensions: None,
                blocked_paths: HashSet::new(),
            },
            Self::Disabled => SandboxPolicy {
                enabled: false,
                allow_read_outside: true,
                allow_write_outside: true,
                allow_symlinks: true,
                allow_hidden_files: true,
                enforce_path_canonicalization: false,
                allowed_extensions: None,
                blocked_paths: HashSet::new(),
            },
        }
    }

    /// Returns default command policy for this level
    pub fn default_command_policy(&self) -> CommandPolicy {
        match self {
            Self::Strict => CommandPolicy {
                enabled: true,
                allow_arbitrary_commands: false,
                allowed_commands: HashSet::from([
                    "cat".to_string(),
                    "ls".to_string(),
                    "head".to_string(),
                    "tail".to_string(),
                    "grep".to_string(),
                    "wc".to_string(),
                ]),
                blocked_commands: HashSet::new(),
                allow_shell_execution: false,
                max_command_length: 1024,
            },
            Self::Moderate => CommandPolicy {
                enabled: true,
                allow_arbitrary_commands: false,
                allowed_commands: HashSet::from([
                    "cat".to_string(),
                    "ls".to_string(),
                    "head".to_string(),
                    "tail".to_string(),
                    "grep".to_string(),
                    "wc".to_string(),
                    "find".to_string(),
                    "sort".to_string(),
                    "uniq".to_string(),
                ]),
                blocked_commands: HashSet::from([
                    "rm".to_string(),
                    "del".to_string(),
                    "format".to_string(),
                    "dd".to_string(),
                ]),
                allow_shell_execution: false,
                max_command_length: 4096,
            },
            Self::Permissive => CommandPolicy {
                enabled: true,
                allow_arbitrary_commands: true,
                allowed_commands: HashSet::new(), // Empty means all allowed
                blocked_commands: HashSet::from(["format".to_string(), "dd".to_string()]),
                allow_shell_execution: true,
                max_command_length: 16384,
            },
            Self::Disabled => CommandPolicy {
                enabled: false,
                allow_arbitrary_commands: true,
                allowed_commands: HashSet::new(),
                blocked_commands: HashSet::new(),
                allow_shell_execution: true,
                max_command_length: usize::MAX,
            },
        }
    }

    /// Returns default network policy for this level
    pub fn default_network_policy(&self) -> NetworkPolicy {
        match self {
            Self::Strict => NetworkPolicy {
                enabled: true,
                allow_outbound: false,
                allow_inbound: false,
                allowed_hosts: HashSet::new(),
                blocked_hosts: HashSet::new(),
                allowed_ports: HashSet::new(),
                max_connections: 0,
            },
            Self::Moderate => NetworkPolicy {
                enabled: true,
                allow_outbound: false,
                allow_inbound: false,
                allowed_hosts: HashSet::from(["localhost".to_string(), "127.0.0.1".to_string()]),
                blocked_hosts: HashSet::new(),
                allowed_ports: HashSet::from([80, 443]),
                max_connections: 10,
            },
            Self::Permissive => NetworkPolicy {
                enabled: true,
                allow_outbound: true,
                allow_inbound: false,
                allowed_hosts: HashSet::new(), // Empty means all allowed
                blocked_hosts: HashSet::new(),
                allowed_ports: HashSet::new(), // Empty means all allowed
                max_connections: 100,
            },
            Self::Disabled => NetworkPolicy {
                enabled: false,
                allow_outbound: true,
                allow_inbound: true,
                allowed_hosts: HashSet::new(),
                blocked_hosts: HashSet::new(),
                allowed_ports: HashSet::new(),
                max_connections: usize::MAX,
            },
        }
    }
}

/// Resource limits for agent operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    /// Maximum file size to read (in bytes)
    pub max_file_size: usize,
    /// Maximum number of files to process in batch operations
    pub max_batch_size: usize,
    /// Maximum memory usage (in MB)
    pub max_memory_mb: usize,
    /// Maximum execution time for operations
    pub max_execution_time: Duration,
    /// Maximum concurrent operations
    pub max_concurrent_operations: usize,
    /// Maximum output size (in bytes)
    pub max_output_size: usize,
}

/// Sandbox policy configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxPolicy {
    /// Whether sandbox is enabled
    pub enabled: bool,
    /// Allow reading outside sandbox
    pub allow_read_outside: bool,
    /// Allow writing outside sandbox
    pub allow_write_outside: bool,
    /// Allow following symlinks
    pub allow_symlinks: bool,
    /// Allow accessing hidden files
    pub allow_hidden_files: bool,
    /// Enforce path canonicalization (resolve .. and symlinks)
    pub enforce_path_canonicalization: bool,
    /// Allowed file extensions (None means all allowed)
    pub allowed_extensions: Option<HashSet<String>>,
    /// Explicitly blocked paths
    pub blocked_paths: HashSet<PathBuf>,
}

/// Command execution policy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPolicy {
    /// Whether command policy is enabled
    pub enabled: bool,
    /// Allow arbitrary commands (if false, only allowed_commands are permitted)
    pub allow_arbitrary_commands: bool,
    /// Explicitly allowed commands
    pub allowed_commands: HashSet<String>,
    /// Explicitly blocked commands
    pub blocked_commands: HashSet<String>,
    /// Allow shell execution (PowerShell, bash, etc.)
    pub allow_shell_execution: bool,
    /// Maximum command length (characters)
    pub max_command_length: usize,
}

/// Network access policy
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkPolicy {
    /// Whether network policy is enabled
    pub enabled: bool,
    /// Allow outbound connections
    pub allow_outbound: bool,
    /// Allow inbound connections
    pub allow_inbound: bool,
    /// Allowed hosts (empty means all allowed if allow_outbound is true)
    pub allowed_hosts: HashSet<String>,
    /// Blocked hosts
    pub blocked_hosts: HashSet<String>,
    /// Allowed ports (empty means all allowed)
    pub allowed_ports: HashSet<u16>,
    /// Maximum concurrent connections
    pub max_connections: usize,
}

/// Complete security policy configuration
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityPolicy {
    /// Security level
    pub level: SecurityLevel,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Sandbox policy
    pub sandbox_policy: SandboxPolicy,
    /// Command execution policy
    pub command_policy: CommandPolicy,
    /// Network access policy
    pub network_policy: NetworkPolicy,
    /// Whether to allow policy override (dangerous)
    pub allow_override: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        let level = SecurityLevel::default();
        Self {
            level,
            resource_limits: level.default_resource_limits(),
            sandbox_policy: level.default_sandbox_policy(),
            command_policy: level.default_command_policy(),
            network_policy: level.default_network_policy(),
            allow_override: false,
        }
    }
}

impl SecurityPolicy {
    /// Creates a new security policy from a level
    pub fn from_level(level: SecurityLevel) -> Self {
        Self {
            level,
            resource_limits: level.default_resource_limits(),
            sandbox_policy: level.default_sandbox_policy(),
            command_policy: level.default_command_policy(),
            network_policy: level.default_network_policy(),
            allow_override: false,
        }
    }

    /// Creates a strict security policy
    pub fn strict() -> Self {
        Self::from_level(SecurityLevel::Strict)
    }

    /// Creates a moderate security policy (default)
    pub fn moderate() -> Self {
        Self::from_level(SecurityLevel::Moderate)
    }

    /// Creates a permissive security policy
    pub fn permissive() -> Self {
        Self::from_level(SecurityLevel::Permissive)
    }

    /// Creates a disabled security policy (no enforcement)
    pub fn disabled() -> Self {
        Self::from_level(SecurityLevel::Disabled)
    }

    /// Enables policy override capability
    pub fn with_override_enabled(mut self) -> Self {
        self.allow_override = true;
        self
    }

    /// Sets custom resource limits
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.resource_limits = limits;
        self
    }

    /// Sets custom sandbox policy
    pub fn with_sandbox_policy(mut self, policy: SandboxPolicy) -> Self {
        self.sandbox_policy = policy;
        self
    }

    /// Sets custom command policy
    pub fn with_command_policy(mut self, policy: CommandPolicy) -> Self {
        self.command_policy = policy;
        self
    }

    /// Sets custom network policy
    pub fn with_network_policy(mut self, policy: NetworkPolicy) -> Self {
        self.network_policy = policy;
        self
    }

    /// Validates a file size against resource limits
    pub fn validate_file_size(&self, size: u64) -> Result<(), String> {
        if size > self.resource_limits.max_file_size as u64 {
            return Err(format!(
                "File size {} exceeds maximum allowed {}",
                size, self.resource_limits.max_file_size
            ));
        }
        Ok(())
    }

    /// Validates a batch size against resource limits
    pub fn validate_batch_size(&self, size: usize) -> Result<(), String> {
        if size > self.resource_limits.max_batch_size {
            return Err(format!(
                "Batch size {} exceeds maximum allowed {}",
                size, self.resource_limits.max_batch_size
            ));
        }
        Ok(())
    }

    /// Validates a command against command policy
    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        if !self.command_policy.enabled {
            return Ok(());
        }

        if command.len() > self.command_policy.max_command_length {
            return Err(format!(
                "Command length {} exceeds maximum allowed {}",
                command.len(),
                self.command_policy.max_command_length
            ));
        }

        // Extract command name (first word)
        let cmd_name = command.split_whitespace().next().unwrap_or("");

        // Check blocked commands
        if self.command_policy.blocked_commands.contains(cmd_name) {
            return Err(format!("Command '{}' is explicitly blocked", cmd_name));
        }

        // If arbitrary commands not allowed, check allowed list
        if !self.command_policy.allow_arbitrary_commands {
            if !self.command_policy.allowed_commands.is_empty()
                && !self.command_policy.allowed_commands.contains(cmd_name)
            {
                return Err(format!("Command '{}' is not in allowed list", cmd_name));
            }
        }

        Ok(())
    }

    /// Validates a file extension against sandbox policy
    pub fn validate_file_extension(&self, extension: &str) -> Result<(), String> {
        if !self.sandbox_policy.enabled {
            return Ok(());
        }

        if let Some(allowed) = &self.sandbox_policy.allowed_extensions {
            if !allowed.contains(extension) {
                return Err(format!(
                    "File extension '{}' is not in allowed list",
                    extension
                ));
            }
        }

        Ok(())
    }

    /// Validates a path against blocked paths
    pub fn validate_path(&self, path: &std::path::Path) -> Result<(), String> {
        if !self.sandbox_policy.enabled {
            return Ok(());
        }

        for blocked in &self.sandbox_policy.blocked_paths {
            if path.starts_with(blocked) {
                return Err(format!("Path is in blocked list: {}", path.display()));
            }
        }

        Ok(())
    }

    /// Checks if a host is allowed for network access
    pub fn is_host_allowed(&self, host: &str) -> bool {
        if !self.network_policy.enabled {
            return true;
        }

        if !self.network_policy.allow_outbound {
            return false;
        }

        if self.network_policy.blocked_hosts.contains(host) {
            return false;
        }

        if !self.network_policy.allowed_hosts.is_empty() {
            return self.network_policy.allowed_hosts.contains(host);
        }

        true
    }

    /// Checks if a port is allowed for network access
    pub fn is_port_allowed(&self, port: u16) -> bool {
        if !self.network_policy.enabled {
            return true;
        }

        if !self.network_policy.allowed_ports.is_empty() {
            return self.network_policy.allowed_ports.contains(&port);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_levels() {
        assert_eq!(SecurityLevel::default(), SecurityLevel::Moderate);
        assert!(!SecurityLevel::Strict.allows_network());
        assert!(SecurityLevel::Permissive.allows_network());
        assert!(SecurityLevel::Disabled.allows_arbitrary_commands());
    }

    #[test]
    fn test_policy_creation() {
        let strict = SecurityPolicy::strict();
        assert_eq!(strict.level, SecurityLevel::Strict);
        assert!(strict.sandbox_policy.enabled);
        assert!(!strict.sandbox_policy.allow_write_outside);

        let disabled = SecurityPolicy::disabled();
        assert_eq!(disabled.level, SecurityLevel::Disabled);
        assert!(!disabled.sandbox_policy.enabled);
    }

    #[test]
    fn test_file_size_validation() {
        let policy = SecurityPolicy::strict();
        assert!(policy.validate_file_size(1024).is_ok());
        assert!(policy.validate_file_size(100 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_command_validation() {
        let policy = SecurityPolicy::strict();
        assert!(policy.validate_command("cat file.txt").is_ok());
        assert!(policy.validate_command("rm file.txt").is_err());

        let permissive = SecurityPolicy::permissive();
        assert!(permissive.validate_command("rm file.txt").is_ok());
    }

    #[test]
    fn test_network_access() {
        let strict = SecurityPolicy::strict();
        assert!(!strict.is_host_allowed("example.com"));
        assert!(!strict.is_host_allowed("localhost")); // Strict blocks all

        let moderate = SecurityPolicy::moderate();
        // Moderate has allow_outbound=false, so even localhost is blocked
        // unless explicitly in allowed_hosts AND allow_outbound is true
        assert!(!moderate.is_host_allowed("localhost"));
        assert!(!moderate.is_host_allowed("example.com"));

        let permissive = SecurityPolicy::permissive();
        assert!(permissive.is_host_allowed("example.com"));
        assert!(permissive.is_host_allowed("localhost"));
    }

    #[test]
    fn test_custom_policy() {
        let policy = SecurityPolicy::moderate()
            .with_override_enabled()
            .with_resource_limits(ResourceLimits {
                max_file_size: 50 * 1024 * 1024,
                max_batch_size: 500,
                max_memory_mb: 512,
                max_execution_time: Duration::from_secs(30),
                max_concurrent_operations: 10,
                max_output_size: 5 * 1024 * 1024,
            });

        assert!(policy.allow_override);
        assert_eq!(policy.resource_limits.max_file_size, 50 * 1024 * 1024);
    }
}
