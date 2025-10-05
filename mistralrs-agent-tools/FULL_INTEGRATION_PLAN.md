# WinUtils FULL Integration Plan - All 90+ Utilities

## Revised Scope

**Original Plan**: 15-20 essential utilities\
**Updated Plan**: ALL 90+ utilities + shell executors

**Rationale**: Provide complete GNU coreutils compatibility for maximum agent flexibility, plus shell execution capabilities for advanced automation.

## Complete Utility Inventory

### File Operations (12 utilities)

1. **cat** - Concatenate and display files
1. **cp** - Copy files and directories
1. **dd** - Convert and copy files
1. **dir** - List directory contents (Windows-style)
1. **ln** - Create links
1. **ls** - List directory contents
1. **mkdir** - Create directories
1. **mv** - Move/rename files
1. **rm** - Remove files
1. **rmdir** - Remove directories
1. **touch** - Update file timestamps
1. **vdir** - List directory contents (verbose)

### Text Processing (25 utilities)

13. **base32** - Base32 encode/decode
01. **base64** - Base64 encode/decode
01. **basenc** - Multi-base encoding
01. **comm** - Compare sorted files
01. **csplit** - Split file by context
01. **cut** - Extract columns
01. **expand** - Convert tabs to spaces
01. **fold** - Wrap text lines
01. **fmt** - Format text
01. **head** - Output first part of files
01. **join** - Join lines of two files
01. **nl** - Number lines
01. **od** - Octal dump
01. **paste** - Merge lines
01. **pr** - Paginate or columnate files
01. **ptx** - Permuted index
01. **shuf** - Shuffle lines
01. **sort** - Sort lines
01. **split** - Split file into pieces
01. **tac** - Reverse concatenate
01. **tail** - Output last part of files
01. **tr** - Translate characters
01. **tsort** - Topological sort
01. **unexpand** - Convert spaces to tabs
01. **uniq** - Remove duplicate lines

### File Analysis (5 utilities)

38. **cksum** - Checksum and count bytes
01. **hashsum** - Compute hash sums (MD5, SHA, etc.)
01. **sum** - Checksum and count blocks
01. **wc** - Word, line, character count
01. **du** - Disk usage

### File Attributes (3 utilities)

43. **dircolors** - Color setup for ls
01. **link** - Create hard link
01. **unlink** - Remove file via unlink

### System Information (10 utilities)

46. **arch** - Print architecture
01. **date** - Display/set date and time
01. **df** - Disk free space
01. **env** - Run program in modified environment
01. **hostname** - Show/set hostname
01. **nproc** - Number of processors
01. **printenv** - Print environment variables
01. **pwd** - Print working directory
01. **sync** - Synchronize cached writes
01. **whoami** - Print username

### Path Manipulation (4 utilities)

56. **basename** - Strip directory from filename
01. **dirname** - Strip filename from path
01. **readlink** - Print resolved symbolic links
01. **realpath** - Print resolved absolute path

### Text Output (5 utilities)

60. **echo** - Display text
01. **printf** - Formatted output
01. **yes** - Repeatedly output string
01. **true** - Return success
01. **false** - Return failure

### File Security (3 utilities)

65. **shred** - Securely delete files
01. **truncate** - Shrink or extend file size
01. **mktemp** - Create temporary file/directory

### Numeric/Math (3 utilities)

68. **expr** - Evaluate expressions
01. **factor** - Factor numbers
01. **numfmt** - Number formatting
01. **seq** - Sequence of numbers

### Testing (2 utilities)

72. **test** - Check file types and compare values
01. **sleep** - Delay for specified time

### Advanced Text (1 utility)

74. **more** - Page through text

### Shell Executors (3 wrappers + enhanced)

75. **pwsh-wrapper** - PowerShell executor with winpath integration
01. **cmd-wrapper** - Command Prompt executor with winpath integration
01. **bash-wrapper** - Bash executor with winpath integration

### Enhanced Search Tools (5 wrappers)

78. **find-wrapper** - Enhanced find (fd-powered) with winpath
01. **grep-wrapper** - Enhanced grep (ripgrep-powered) with winpath
01. **tree** - Directory tree visualization
01. **where** - Windows command search
01. **which** - Unix command search

### Shared Libraries (2 core)

83. **winpath** - Path normalization library
01. **winutils-core** - Common utilities (help, version, testing)

**Total**: 84+ utilities/components

## Shell Executor Architecture

### Design Philosophy

The shell executors are **critical** for LLM agents because they enable:

1. **Command execution** - Run system commands programmatically
1. **Script execution** - Execute PowerShell/Bash/Batch scripts
1. **Pipeline support** - Chain multiple commands
1. **Environment control** - Manage environment variables
1. **Path translation** - Convert between Git Bash/WSL/Windows paths

### Shell Executor API

```rust
/// Shell execution context
pub struct ShellExecutor {
    shell_type: ShellType,
    working_dir: Option<PathBuf>,
    env_vars: HashMap<String, String>,
    timeout: Option<Duration>,
    sandbox: SandboxConfig,
}

pub enum ShellType {
    PowerShell,      // pwsh.exe or powershell.exe
    Cmd,             // cmd.exe
    Bash,            // bash.exe (Git Bash, WSL, MSYS2)
    GitBash,         // Specific Git Bash configuration
    Wsl,             // Windows Subsystem for Linux
}

pub struct ShellResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration: Duration,
}

impl ShellExecutor {
    pub fn new(shell_type: ShellType, sandbox: SandboxConfig) -> Self;
    
    /// Execute a single command
    pub fn execute_command(
        &self,
        command: &str,
        args: &[String],
    ) -> Result<ShellResult>;
    
    /// Execute a script
    pub fn execute_script(
        &self,
        script: &str,
    ) -> Result<ShellResult>;
    
    /// Execute with input piped to stdin
    pub fn execute_with_input(
        &self,
        command: &str,
        input: &str,
    ) -> Result<ShellResult>;
    
    /// Execute interactively (streaming output)
    pub fn execute_interactive(
        &self,
        command: &str,
        callback: Box<dyn Fn(String)>,
    ) -> Result<ShellResult>;
    
    /// Set environment variable
    pub fn set_env(&mut self, key: String, value: String);
    
    /// Set working directory
    pub fn set_working_dir(&mut self, path: PathBuf) -> Result<()>;
    
    /// Validate command before execution (sandbox check)
    pub fn validate_command(&self, command: &str) -> Result<()>;
}
```

### Security Considerations for Shell Execution

**Sandbox Enforcement**:

```rust
pub struct ShellSandboxConfig {
    /// Allowed commands (whitelist)
    pub allowed_commands: Option<Vec<String>>,
    
    /// Blocked commands (blacklist)
    pub blocked_commands: Vec<String>,
    
    /// Allow only commands in PATH
    pub restrict_to_path: bool,
    
    /// Maximum execution time
    pub timeout: Duration,
    
    /// Working directory restrictions
    pub allowed_dirs: Vec<PathBuf>,
    
    /// Environment variable restrictions
    pub allowed_env_vars: Option<Vec<String>>,
    
    /// Disable dangerous operations
    pub disable_dangerous: bool,  // rm -rf, format, etc.
}
```

**Default Blocked Commands**:

- `rm -rf /` or `rm -rf C:\`
- `format`
- `del /f /s /q` (recursive force delete)
- `rd /s /q` (recursive directory removal)
- PowerShell: `Remove-Item -Recurse -Force C:\`
- Any command with `sudo` or `runas`
- Network commands: `ssh`, `scp`, `ftp` (unless explicitly allowed)

## Complete API Design

### Core AgentToolkit Structure

```rust
pub struct AgentToolkit {
    sandbox: AgentTools,         // Existing sandbox
    shell_executors: HashMap<ShellType, ShellExecutor>,
    config: ToolkitConfig,
}

pub struct ToolkitConfig {
    pub sandbox_config: SandboxConfig,
    pub shell_config: ShellSandboxConfig,
    pub enable_shell_execution: bool,
    pub enable_network_tools: bool,
    pub verbose_logging: bool,
}

impl AgentToolkit {
    pub fn new(config: ToolkitConfig) -> Self;
    pub fn with_defaults() -> Self;
    
    // ========== FILE OPERATIONS ==========
    
    // Core file I/O
    pub fn cat(&self, files: &[String], options: CatOptions) -> Result<String>;
    pub fn read_file(&self, path: &str) -> Result<String>;
    pub fn read_lines(&self, path: &str) -> Result<Vec<String>>;
    pub fn read_bytes(&self, path: &str, max: Option<usize>) -> Result<Vec<u8>>;
    
    // File manipulation
    pub fn cp(&self, src: &str, dst: &str, options: CpOptions) -> Result<()>;
    pub fn mv(&self, src: &str, dst: &str, options: MvOptions) -> Result<()>;
    pub fn rm(&self, paths: &[String], options: RmOptions) -> Result<()>;
    pub fn ln(&self, target: &str, link: &str, symbolic: bool) -> Result<()>;
    pub fn touch(&self, paths: &[String], options: TouchOptions) -> Result<()>;
    pub fn dd(&self, options: DdOptions) -> Result<DdResult>;
    
    // Directory operations
    pub fn ls(&self, paths: &[String], options: LsOptions) -> Result<Vec<FileEntry>>;
    pub fn dir(&self, path: &str, options: DirOptions) -> Result<Vec<FileEntry>>;
    pub fn vdir(&self, path: &str) -> Result<Vec<FileEntry>>;
    pub fn mkdir(&self, paths: &[String], options: MkdirOptions) -> Result<()>;
    pub fn rmdir(&self, paths: &[String], options: RmdirOptions) -> Result<()>;
    pub fn tree(&self, root: &str, options: TreeOptions) -> Result<TreeNode>;
    
    // ========== TEXT PROCESSING ==========
    
    // Encoding
    pub fn base32(&self, data: &[u8], decode: bool) -> Result<Vec<u8>>;
    pub fn base64(&self, data: &[u8], decode: bool) -> Result<Vec<u8>>;
    pub fn basenc(&self, data: &[u8], encoding: BaseEncoding) -> Result<Vec<u8>>;
    
    // Text manipulation
    pub fn head(&self, files: &[String], lines: usize) -> Result<Vec<String>>;
    pub fn tail(&self, files: &[String], lines: usize) -> Result<Vec<String>>;
    pub fn tac(&self, files: &[String]) -> Result<Vec<String>>;
    pub fn cut(&self, files: &[String], options: CutOptions) -> Result<Vec<String>>;
    pub fn paste(&self, files: &[String], options: PasteOptions) -> Result<String>;
    pub fn join(&self, file1: &str, file2: &str, options: JoinOptions) -> Result<Vec<String>>;
    pub fn comm(&self, file1: &str, file2: &str, options: CommOptions) -> Result<CommResult>;
    
    // Text formatting
    pub fn fmt(&self, files: &[String], options: FmtOptions) -> Result<String>;
    pub fn fold(&self, files: &[String], width: usize) -> Result<Vec<String>>;
    pub fn expand(&self, files: &[String], tab_width: usize) -> Result<String>;
    pub fn unexpand(&self, files: &[String], tab_width: usize) -> Result<String>;
    pub fn nl(&self, files: &[String], options: NlOptions) -> Result<Vec<String>>;
    pub fn pr(&self, files: &[String], options: PrOptions) -> Result<String>;
    
    // Text analysis
    pub fn wc(&self, files: &[String], options: WcOptions) -> Result<WcResult>;
    pub fn sort(&self, files: &[String], options: SortOptions) -> Result<Vec<String>>;
    pub fn uniq(&self, file: &str, options: UniqOptions) -> Result<Vec<String>>;
    pub fn shuf(&self, files: &[String], options: ShufOptions) -> Result<Vec<String>>;
    pub fn tr(&self, set1: &str, set2: &str, input: &str) -> Result<String>;
    
    // Text splitting
    pub fn split(&self, file: &str, options: SplitOptions) -> Result<Vec<String>>;
    pub fn csplit(&self, file: &str, patterns: &[String]) -> Result<Vec<String>>;
    
    // Advanced text
    pub fn od(&self, files: &[String], options: OdOptions) -> Result<String>;
    pub fn ptx(&self, files: &[String], options: PtxOptions) -> Result<String>;
    pub fn tsort(&self, file: &str) -> Result<Vec<String>>;
    pub fn more(&self, file: &str, page_size: usize) -> Result<MoreReader>;
    
    // ========== FILE ANALYSIS ==========
    
    pub fn cksum(&self, files: &[String]) -> Result<Vec<ChecksumResult>>;
    pub fn sum(&self, files: &[String], sysv: bool) -> Result<Vec<SumResult>>;
    pub fn hashsum(&self, files: &[String], algorithm: HashAlgorithm) -> Result<Vec<HashResult>>;
    pub fn du(&self, paths: &[String], options: DuOptions) -> Result<Vec<DuResult>>;
    
    // ========== PATH OPERATIONS ==========
    
    pub fn basename(&self, path: &str, suffix: Option<&str>) -> Result<String>;
    pub fn dirname(&self, path: &str) -> Result<String>;
    pub fn readlink(&self, path: &str, canonical: bool) -> Result<String>;
    pub fn realpath(&self, path: &str) -> Result<String>;
    pub fn pwd(&self) -> Result<String>;
    
    // ========== SYSTEM INFORMATION ==========
    
    pub fn arch(&self) -> Result<String>;
    pub fn date(&self, format: Option<&str>) -> Result<String>;
    pub fn df(&self, paths: &[String], options: DfOptions) -> Result<Vec<DfResult>>;
    pub fn env(&self, command: Option<Vec<String>>) -> Result<HashMap<String, String>>;
    pub fn hostname(&self, fqdn: bool) -> Result<String>;
    pub fn nproc(&self, all: bool) -> Result<usize>;
    pub fn printenv(&self, vars: &[String]) -> Result<HashMap<String, String>>;
    pub fn sync(&self) -> Result<()>;
    pub fn whoami(&self) -> Result<String>;
    
    // ========== OUTPUT UTILITIES ==========
    
    pub fn echo(&self, text: &str, options: EchoOptions) -> Result<String>;
    pub fn printf(&self, format: &str, args: &[String]) -> Result<String>;
    pub fn yes(&self, text: &str, count: Option<usize>) -> Result<Vec<String>>;
    pub fn r#true(&self) -> Result<i32>;  // Returns 0
    pub fn r#false(&self) -> Result<i32>; // Returns 1
    
    // ========== FILE SECURITY ==========
    
    pub fn shred(&self, files: &[String], options: ShredOptions) -> Result<()>;
    pub fn truncate(&self, file: &str, size: u64) -> Result<()>;
    pub fn mktemp(&self, template: Option<&str>, directory: bool) -> Result<String>;
    pub fn link(&self, target: &str, link_name: &str) -> Result<()>;
    pub fn unlink(&self, file: &str) -> Result<()>;
    
    // ========== NUMERIC/MATH ==========
    
    pub fn expr(&self, expression: &str) -> Result<ExprResult>;
    pub fn factor(&self, numbers: &[u64]) -> Result<Vec<FactorResult>>;
    pub fn numfmt(&self, numbers: &[String], options: NumfmtOptions) -> Result<Vec<String>>;
    pub fn seq(&self, first: i64, increment: i64, last: i64) -> Result<Vec<i64>>;
    
    // ========== TESTING ==========
    
    pub fn test(&self, expression: &str) -> Result<bool>;
    pub fn sleep(&self, duration: Duration) -> Result<()>;
    
    // ========== SEARCH TOOLS ==========
    
    pub fn find(&self, paths: &[String], options: FindOptions) -> Result<Vec<String>>;
    pub fn grep(&self, pattern: &str, files: &[String], options: GrepOptions) -> Result<Vec<Match>>;
    pub fn which(&self, command: &str, all: bool) -> Result<Vec<PathBuf>>;
    pub fn r#where(&self, pattern: &str, options: WhereOptions) -> Result<Vec<PathBuf>>;
    
    // ========== SHELL EXECUTION ==========
    
    /// Execute command in PowerShell
    pub fn pwsh(&self, command: &str, options: ShellOptions) -> Result<ShellResult>;
    
    /// Execute command in Command Prompt
    pub fn cmd(&self, command: &str, options: ShellOptions) -> Result<ShellResult>;
    
    /// Execute command in Bash (Git Bash/WSL)
    pub fn bash(&self, command: &str, options: ShellOptions) -> Result<ShellResult>;
    
    /// Execute in preferred shell
    pub fn execute(&self, command: &str) -> Result<ShellResult>;
    
    /// Execute script file
    pub fn execute_script(&self, path: &str, shell: ShellType) -> Result<ShellResult>;
    
    // ========== ADVANCED ==========
    
    /// Get dircolors configuration
    pub fn dircolors(&self, file: Option<&str>) -> Result<String>;
}
```

## Module Structure (Expanded)

```
mistralrs-agent-tools/src/
├── lib.rs                    # Main API exports, AgentToolkit
├── error.rs                  # Unified error types
├── sandbox.rs                # Core sandbox (existing)
├── pathlib.rs                # Path utilities (from winpath)
├── config.rs                 # Configuration structures
│
├── tools/                    # All utility implementations
│   ├── mod.rs                # Tool registry
│   │
│   ├── file/                 # File operations (12 utils)
│   │   ├── mod.rs
│   │   ├── cat.rs
│   │   ├── cp.rs
│   │   ├── dd.rs
│   │   ├── dir.rs
│   │   ├── ln.rs
│   │   ├── ls.rs
│   │   ├── mkdir.rs
│   │   ├── mv.rs
│   │   ├── rm.rs
│   │   ├── rmdir.rs
│   │   ├── touch.rs
│   │   └── vdir.rs
│   │
│   ├── text/                 # Text processing (25 utils)
│   │   ├── mod.rs
│   │   ├── base32.rs
│   │   ├── base64.rs
│   │   ├── basenc.rs
│   │   ├── comm.rs
│   │   ├── csplit.rs
│   │   ├── cut.rs
│   │   ├── expand.rs
│   │   ├── fold.rs
│   │   ├── fmt.rs
│   │   ├── head.rs
│   │   ├── join.rs
│   │   ├── nl.rs
│   │   ├── od.rs
│   │   ├── paste.rs
│   │   ├── pr.rs
│   │   ├── ptx.rs
│   │   ├── shuf.rs
│   │   ├── sort.rs
│   │   ├── split.rs
│   │   ├── tac.rs
│   │   ├── tail.rs
│   │   ├── tr.rs
│   │   ├── tsort.rs
│   │   ├── unexpand.rs
│   │   ├── uniq.rs
│   │   └── more.rs
│   │
│   ├── analysis/             # File analysis (5 utils)
│   │   ├── mod.rs
│   │   ├── cksum.rs
│   │   ├── hashsum.rs
│   │   ├── sum.rs
│   │   ├── wc.rs
│   │   └── du.rs
│   │
│   ├── path/                 # Path operations (4 utils)
│   │   ├── mod.rs
│   │   ├── basename.rs
│   │   ├── dirname.rs
│   │   ├── readlink.rs
│   │   └── realpath.rs
│   │
│   ├── system/               # System info (10 utils)
│   │   ├── mod.rs
│   │   ├── arch.rs
│   │   ├── date.rs
│   │   ├── df.rs
│   │   ├── env.rs
│   │   ├── hostname.rs
│   │   ├── nproc.rs
│   │   ├── printenv.rs
│   │   ├── pwd.rs
│   │   ├── sync.rs
│   │   └── whoami.rs
│   │
│   ├── output/               # Output utilities (5 utils)
│   │   ├── mod.rs
│   │   ├── echo.rs
│   │   ├── printf.rs
│   │   ├── yes.rs
│   │   ├── true.rs
│   │   └── false.rs
│   │
│   ├── security/             # File security (5 utils)
│   │   ├── mod.rs
│   │   ├── shred.rs
│   │   ├── truncate.rs
│   │   ├── mktemp.rs
│   │   ├── link.rs
│   │   ├── unlink.rs
│   │   └── dircolors.rs
│   │
│   ├── numeric/              # Numeric/Math (4 utils)
│   │   ├── mod.rs
│   │   ├── expr.rs
│   │   ├── factor.rs
│   │   ├── numfmt.rs
│   │   └── seq.rs
│   │
│   ├── testing/              # Testing (2 utils)
│   │   ├── mod.rs
│   │   ├── test.rs
│   │   └── sleep.rs
│   │
│   ├── search/               # Search tools (5 utils)
│   │   ├── mod.rs
│   │   ├── find.rs           # Enhanced with fd
│   │   ├── grep.rs           # Enhanced with ripgrep
│   │   ├── tree.rs
│   │   ├── which.rs
│   │   └── where.rs
│   │
│   └── shell/                # Shell executors (NEW)
│       ├── mod.rs
│       ├── executor.rs       # Core execution engine
│       ├── pwsh.rs           # PowerShell wrapper
│       ├── cmd.rs            # Command Prompt wrapper
│       ├── bash.rs           # Bash wrapper
│       ├── sandbox.rs        # Shell-specific sandboxing
│       └── path_translation.rs  # Git Bash/WSL path conversion
│
├── schemas/                  # Tool schemas for LLM
│   ├── mod.rs
│   ├── generator.rs          # Schema generation
│   ├── file_tools.json
│   ├── text_tools.json
│   ├── system_tools.json
│   └── shell_tools.json
│
└── types/                    # Shared types
    ├── mod.rs
    ├── options.rs            # All *Options structs
    ├── results.rs            # All *Result structs
    └── enums.rs              # Shared enums
```

## Enhanced Cargo.toml

```toml
[package]
name = "mistralrs-agent-tools"
version = "0.3.0"  # Major version bump
edition = "2021"

[dependencies]
# Core
anyhow = "1.0"
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"

# Path handling
camino = "1.1"  # UTF-8 paths
dunce = "1.0"   # Windows canonical paths

# File operations
walkdir = "2.4"
glob = "0.3"
filetime = "0.2"
memmap2 = { version = "0.9", optional = true }

# Text processing
regex = "1.10"
encoding_rs = "0.8"  # Character encoding

# Shell execution
tokio = { version = "1.35", features = ["process", "io-util", "time"] }

# Hashing
sha2 = "0.10"
md-5 = "0.10"
blake2 = "0.10"

# Compression (for advanced features)
flat2 = { version = "1.0", optional = true }

# Math
num-bigint = "0.4"

# System
sysinfo = "0.30"

# Windows-specific
[target.'cfg(windows)'.dependencies]
windows = { version = "0.60", features = [
    "Win32_Foundation",
    "Win32_Storage_FileSystem",
    "Win32_System_Console",
    "Win32_System_Threading",
    "Win32_System_SystemInformation",
]}

[dev-dependencies]
tempfile = "3.8"
proptest = "1.4"
criterion = "0.5"

[features]
default = ["file-ops", "text-proc", "search", "shell-exec"]

# Core feature groups
file-ops = []           # File operations (cat, cp, mv, etc.)
text-proc = ["regex"]   # Text processing (head, tail, sort, etc.)
search = ["regex", "glob"]  # Find, grep, which, where
system-info = ["sysinfo"]   # System information tools
shell-exec = ["tokio"]      # Shell executors (CRITICAL FOR AGENTS)

# Optional advanced features
mmap = ["memmap2"]          # Memory-mapped I/O
compression = ["flat2"]    # Compression support
all-hashes = []             # All hash algorithms

# Preset bundles
minimal = ["file-ops"]
standard = ["file-ops", "text-proc", "search"]
full = ["default", "mmap", "compression", "all-hashes", "system-info"]

[profile.release]
lto = "thin"        # Thin LTO for faster builds
codegen-units = 4   # Parallel codegen
opt-level = 3
strip = true
panic = "abort"

[profile.release-full]
inherits = "release"
lto = "fat"         # Full LTO for maximum optimization
codegen-units = 1
```

## Revised Implementation Timeline

### Phase 1: Foundation (Weeks 1-2)

**Week 1: Core Infrastructure**

- Create complete module structure
- Extract pathlib from winpath
- Implement error types and result types
- Implement configuration system
- Basic tests

**Week 2: File Operations (12 utilities)**

- cat, cp, mv, rm (priority)
- ls, mkdir, rmdir
- ln, touch, dd
- dir, vdir
- Unit tests for all

### Phase 2: Text Processing (Weeks 3-5)

**Week 3: Essential Text Tools (8 utilities)**

- head, tail, wc (high priority)
- cut, paste, join
- sort, uniq
- Unit tests

**Week 4: Text Formatting (9 utilities)**

- fmt, fold, expand, unexpand
- nl, pr, tr
- tac, more
- Unit tests

**Week 5: Advanced Text (8 utilities)**

- split, csplit, comm
- shuf, tsort, ptx, od
- base32, base64, basenc
- Unit tests

### Phase 3: Shell Executors (Weeks 6-7) **CRITICAL**

**Week 6: Core Shell Infrastructure**

- Shell executor framework
- Sandbox integration for shell commands
- Command validation and blocking
- PowerShell wrapper (pwsh)
- Basic tests

**Week 7: Additional Shells**

- Command Prompt wrapper (cmd)
- Bash wrapper with Git Bash/WSL detection
- Path translation (Windows ↔ Git Bash ↔ WSL)
- Interactive execution
- Comprehensive tests
- **THIS IS CRUCIAL FOR AGENT AUTOMATION**

### Phase 4: Analysis & System Tools (Week 8)

**File Analysis (5 utilities)**

- cksum, sum, hashsum
- wc (if not done in Phase 2), du

**System Information (10 utilities)**

- arch, hostname, whoami, nproc
- date, env, printenv, pwd
- df, sync

### Phase 5: Specialized Tools (Week 9)

**Path Operations (4 utilities)**

- basename, dirname
- readlink, realpath

**Output Utilities (5 utilities)**

- echo, printf, yes
- true, false

**Numeric/Math (4 utilities)**

- expr, factor
- numfmt, seq

**Testing (2 utilities)**

- test, sleep

### Phase 6: Security & Search (Week 10)

**File Security (5 utilities)**

- shred, truncate, mktemp
- link, unlink
- dircolors

**Enhanced Search (5 utilities)**

- find (with fd backend)
- grep (with ripgrep backend)
- tree, which, where

### Phase 7: Integration & Polish (Weeks 11-12)

**Week 11: Schemas & Integration**

- Generate JSON schemas for all 80+ tools
- Update agent_mode.rs with complete tool mapping
- Create tool discovery system
- Integration tests

**Week 12: Documentation & Optimization**

- API documentation for all tools
- Usage examples for each utility
- Performance optimization
- Final testing
- Release preparation

## Total Effort Revised

**Original Estimate**: 4-5 weeks (15-20 utilities)\
**Revised Estimate**: **12 weeks** (80+ utilities + shell executors)

**Breakdown**:

- Foundation: 2 weeks
- File operations: 1 week
- Text processing: 3 weeks
- Shell executors: 2 weeks (**CRITICAL**)
- Analysis & system: 1 week
- Specialized tools: 1 week
- Security & search: 1 week
- Integration & polish: 2 weeks

**Total**: 13 weeks (3 months) for complete implementation

## Shell Executor Priority Justification

### Why Shell Execution is Critical for Agents

1. **System Administration**:

   ```rust
   // Start/stop services
   toolkit.pwsh("Start-Service -Name MyService")?;

   // Manage processes
   toolkit.pwsh("Get-Process | Where-Object CPU -gt 50")?;
   ```

1. **Build Automation**:

   ```rust
   // Run cargo build
   toolkit.bash("cargo build --release")?;

   // Execute npm scripts
   toolkit.cmd("npm run build")?;
   ```

1. **DevOps Tasks**:

   ```rust
   // Deploy applications
   toolkit.pwsh("docker-compose up -d")?;

   // Run tests
   toolkit.bash("pytest tests/")?;
   ```

1. **System Configuration**:

   ```rust
   // Set environment
   toolkit.cmd("setx PATH \"%PATH%;C:\\tools\"")?

   // Configure git
   toolkit.bash("git config --global user.name 'Agent'")?;
   ```

1. **Scripting Power**:

   ```rust
   // Execute complex PowerShell script
   let script = r#"
   $files = Get-ChildItem -Recurse -Filter *.log
   $files | ForEach-Object {
       if ($_.Length -gt 10MB) {
           Remove-Item $_.FullName
       }
   }
   "#;
   toolkit.pwsh(script)?;
   ```

## Security Model for Shell Execution

### Three-Tier Security

1. **Command Validation** (Pre-execution):

   ```rust
   impl ShellExecutor {
       fn validate_command(&self, cmd: &str) -> Result<()> {
           // Check against blocklist
           if self.is_dangerous_command(cmd) {
               return Err(SecurityError::DangerousCommand);
           }
           
           // Check working directory is in sandbox
           if !self.sandbox.is_path_allowed(&self.working_dir) {
               return Err(SecurityError::OutsideSandbox);
           }
           
           // Validate environment variables
           // Limit execution time
           // etc.
           
           Ok(())
       }
   }
   ```

1. **Execution Monitoring** (During execution):

   - Timeout enforcement
   - Output size limits
   - Resource usage monitoring

1. **Output Sanitization** (Post-execution):

   - Remove sensitive data
   - Limit output size
   - Log execution for audit

### Default Security Profile

```rust
let default_shell_config = ShellSandboxConfig {
    blocked_commands: vec![
        "rm -rf /".to_string(),
        "rm -rf C:\\".to_string(),
        "format".to_string(),
        "mkfs".to_string(),
        "dd if=/dev/zero".to_string(),
        // ... more dangerous commands
    ],
    restrict_to_path: true,  // Only run commands in PATH
    timeout: Duration::from_secs(300),  // 5 minute max
    allowed_dirs: vec![/* sandbox roots */],
    disable_dangerous: true,  // Block known dangerous patterns
};
```

## LLM Tool Schemas (Expanded)

### Schema Generation

```rust
// Auto-generate schemas from function signatures
#[derive(ToolSchema)]
pub struct CatOptions {
    #[schema(description = "Show line numbers")]
    pub number_lines: bool,
    
    #[schema(description = "Show non-printing characters")]
    pub show_nonprinting: bool,
    
    // ...
}

// Schema output:
{
  "name": "cat",
  "description": "Concatenate and display files",
  "parameters": {
    "type": "object",
    "properties": {
      "files": {
        "type": "array",
        "items": {"type": "string"},
        "description": "Files to concatenate"
      },
      "options": {
        "type": "object",
        "properties": {
          "number_lines": {
            "type": "boolean",
            "description": "Show line numbers",
            "default": false
          }
        }
      }
    },
    "required": ["files"]
  }
}
```

### Tool Categories for LLM

```json
{
  "categories": [
    {
      "name": "file_operations",
      "description": "File and directory manipulation",
      "tools": ["cat", "cp", "mv", "rm", "mkdir", ...]
    },
    {
      "name": "text_processing",
      "description": "Text manipulation and analysis",
      "tools": ["head", "tail", "sort", "grep", ...]
    },
    {
      "name": "shell_execution",
      "description": "Execute system commands and scripts",
      "tools": ["pwsh", "cmd", "bash", "execute"],
      "warning": "Powerful but requires careful use. Commands are sandboxed."
    },
    {
      "name": "search",
      "description": "Find files and search content",
      "tools": ["find", "grep", "which", "where", "tree"]
    },
    {
      "name": "system_info",
      "description": "System information and status",
      "tools": ["arch", "hostname", "df", "nproc", ...]
    }
  ]
}
```

## Integration with agent_mode.rs (Expanded)

```rust
use mistralrs_agent_tools::{AgentToolkit, ToolkitConfig, ShellType};

// Initialize with full toolkit
let config = ToolkitConfig {
    sandbox_config: SandboxConfig::default(),
    shell_config: ShellSandboxConfig::default(),
    enable_shell_execution: true,  // Enable shell executors
    enable_network_tools: false,    // Disable for security
    verbose_logging: true,
};

let toolkit = AgentToolkit::new(config);

// In execute_tool_calls(), map to 80+ operations:
match function_name.as_str() {
    // File operations
    "cat" => toolkit.cat(&files, parse_options(args)?),
    "cp" => toolkit.cp(src, dst, parse_options(args)?),
    "ls" => toolkit.ls(&paths, parse_options(args)?),
    
    // Text processing
    "head" => toolkit.head(&files, n),
    "tail" => toolkit.tail(&files, n),
    "sort" => toolkit.sort(&files, parse_options(args)?),
    "wc" => toolkit.wc(&files, parse_options(args)?),
    
    // Shell execution (POWERFUL)
    "pwsh" | "powershell" => toolkit.pwsh(command, parse_options(args)?),
    "cmd" | "command_prompt" => toolkit.cmd(command, parse_options(args)?),
    "bash" | "shell" => toolkit.bash(command, parse_options(args)?),
    "execute" => toolkit.execute(command),  // Auto-detect shell
    
    // Search
    "find" => toolkit.find(&paths, parse_options(args)?),
    "grep" => toolkit.grep(pattern, &files, parse_options(args)?),
    "tree" => toolkit.tree(root, parse_options(args)?),
    
    // System
    "hostname" => toolkit.hostname(false),
    "whoami" => toolkit.whoami(),
    "arch" => toolkit.arch(),
    "df" => toolkit.df(&paths, parse_options(args)?),
    
    // ... 70+ more tools
    
    _ => Err(format!("Unknown tool: {}", function_name)),
}
```

## Benefits of Full Integration

### For LLM Agents

1. **Complete Filesystem Control**:

   - All file operations available
   - Text processing for analysis
   - Search capabilities

1. **System Command Execution** (**GAME CHANGER**):

   - Run any system command
   - Execute build tools (cargo, npm, make)
   - Manage processes
   - Configure system

1. **Cross-Platform Path Handling**:

   - Automatic path translation (Windows/Git Bash/WSL)
   - No more path confusion

1. **Comprehensive Toolset**:

   - 80+ utilities at fingertips
   - GNU coreutils compatibility
   - Windows-optimized

### For Developers

1. **Single Source of Truth**:

   - One crate for all filesystem operations
   - Consistent API across all tools
   - Type-safe operations

1. **Powerful Automation**:

   - Shell executors enable complex workflows
   - Script execution support
   - Pipeline capabilities

1. **Safe by Default**:

   - Sandbox enforced for all operations
   - Shell command validation
   - Dangerous command blocking

## Summary

This expanded integration plan includes:

✅ **All 90+ utilities** from winutils\
✅ **Shell executors** (pwsh, cmd, bash) - **CRITICAL FOR AGENTS**\
✅ **Complete API** for every tool\
✅ **Security model** for safe shell execution\
✅ **Path translation** (Windows/Git Bash/WSL)\
✅ **JSON schemas** for LLM tool discovery\
✅ **12-week timeline** for full implementation

**Total Scope**:

- 80+ utilities
- 3 shell executors
- 2 core libraries
- Comprehensive sandbox integration
- Full test coverage
- Complete documentation

**Key Focus**: Shell executors are prioritized (Weeks 6-7) as they are the most powerful capability for autonomous agents.

**Estimated Build Time**: ~2 minutes (clean build with all features)

**Ready to proceed with full implementation!** 🚀
