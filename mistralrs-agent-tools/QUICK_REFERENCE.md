# Quick Reference Guide - Phase 2 & 3 Features

## Text Processing Utilities

### head - Display first part of files

```rust
use mistralrs_agent_tools::{AgentToolkit, HeadOptions};

let toolkit = AgentToolkit::with_defaults();
let options = HeadOptions {
    lines: 20,           // Number of lines (default: 10)
    bytes: None,         // Or use bytes instead
    verbose: false,      // Show filename headers
    quiet: false,        // Never show headers
};
let content = toolkit.head(&[Path::new("file.txt")], &options)?;
```

### tail - Display last part of files

```rust
use mistralrs_agent_tools::{AgentToolkit, TailOptions};

let toolkit = AgentToolkit::with_defaults();
let options = TailOptions {
    lines: 20,           // Number of lines (default: 10)
    bytes: None,         // Or use bytes instead
    verbose: false,      // Show filename headers
    quiet: false,        // Never show headers
};
let content = toolkit.tail(&[Path::new("file.txt")], &options)?;
```

### wc - Count lines, words, bytes, characters

```rust
use mistralrs_agent_tools::{AgentToolkit, WcOptions};

let toolkit = AgentToolkit::with_defaults();
let options = WcOptions {
    lines: true,         // Count lines
    words: true,         // Count words
    bytes: false,        // Count bytes
    chars: false,        // Count characters
};
let results = toolkit.wc(&[Path::new("file.txt")], &options)?;
for (path, result) in results {
    println!("{}: {} lines, {} words", path, result.lines, result.words);
}
```

### grep - Search for patterns

```rust
use mistralrs_agent_tools::{AgentToolkit, GrepOptions};

let toolkit = AgentToolkit::with_defaults();
let options = GrepOptions {
    ignore_case: true,           // Case-insensitive
    invert_match: false,         // Show non-matching lines
    line_number: true,           // Show line numbers
    count: false,                // Show only count
    files_with_matches: false,   // Show only filenames
    files_without_match: false,  // Show files without matches
    before_context: 2,           // Lines before match
    after_context: 2,            // Lines after match
    extended_regexp: false,      // Extended regex
    fixed_strings: false,        // Literal strings (not regex)
    recursive: false,            // Recursive search
};
let matches = toolkit.grep("pattern", &[Path::new("file.txt")], &options)?;
for m in matches {
    println!("{}:{}: {}", m.path, m.line_number, m.line);
}
```

### sort - Sort lines of text

```rust
use mistralrs_agent_tools::{AgentToolkit, SortOptions};

let toolkit = AgentToolkit::with_defaults();
let options = SortOptions {
    reverse: false,        // Reverse order
    numeric: true,         // Numeric sort
    unique: false,         // Unique lines only
    ignore_case: false,    // Case-insensitive
    version_sort: false,   // Version sort (v1.2 < v1.10)
    month_sort: false,     // Month name sort
    human_numeric: false,  // Human numeric (1K, 1M, 1G)
};
let sorted = toolkit.sort(&[Path::new("numbers.txt")], &options)?;
```

### uniq - Filter duplicate lines

```rust
use mistralrs_agent_tools::{AgentToolkit, UniqOptions};

let toolkit = AgentToolkit::with_defaults();
let options = UniqOptions {
    count: true,           // Prefix with count
    repeated: false,       // Show only duplicates
    unique: false,         // Show only unique lines
    ignore_case: false,    // Case-insensitive
    skip_fields: 0,        // Skip N fields
    skip_chars: 0,         // Skip N characters
};
let unique = toolkit.uniq(&[Path::new("data.txt")], &options)?;
```

## Shell Execution

### execute - Run shell commands

```rust
use mistralrs_agent_tools::{AgentToolkit, CommandOptions, ShellType};
use std::path::PathBuf;

let toolkit = AgentToolkit::with_defaults();

// PowerShell (Windows default)
let options = CommandOptions {
    shell: ShellType::PowerShell,
    working_dir: Some(PathBuf::from("C:\\Projects")),
    env: vec![
        ("VAR1".to_string(), "value1".to_string()),
        ("VAR2".to_string(), "value2".to_string()),
    ],
    timeout: Some(30),      // Seconds (None = no timeout)
    capture_stdout: true,   // Capture stdout
    capture_stderr: true,   // Capture stderr
};
let result = toolkit.execute("Get-Process | Select-Object -First 5", &options)?;

println!("Exit code: {}", result.status);
println!("Output:\n{}", result.stdout);
println!("Errors:\n{}", result.stderr);
println!("Duration: {}ms", result.duration_ms);

// Command Prompt
let options = CommandOptions {
    shell: ShellType::Cmd,
    ..Default::default()
};
let result = toolkit.execute("dir /b", &options)?;

// Bash (Unix/WSL/Git Bash)
let options = CommandOptions {
    shell: ShellType::Bash,
    ..Default::default()
};
let result = toolkit.execute("ls -la", &options)?;
```

## Common Patterns

### Pipeline: grep → sort → uniq

```rust
use mistralrs_agent_tools::{AgentToolkit, GrepOptions, SortOptions, UniqOptions};
use std::path::Path;
use std::fs;
use std::io::Write;

let toolkit = AgentToolkit::with_defaults();

// 1. Grep for errors
let grep_opts = GrepOptions {
    ignore_case: true,
    ..Default::default()
};
let matches = toolkit.grep("error", &[Path::new("log.txt")], &grep_opts)?;

// 2. Extract just the lines
let lines: Vec<String> = matches.iter().map(|m| m.line.clone()).collect();

// 3. Write to temp file
let temp_file = Path::new("temp.txt");
let mut file = fs::File::create(temp_file)?;
for line in &lines {
    writeln!(file, "{}", line)?;
}

// 4. Sort
let sort_opts = SortOptions::default();
let sorted = toolkit.sort(&[temp_file], &sort_opts)?;

// 5. Write sorted to another temp
let sorted_file = Path::new("sorted.txt");
fs::write(sorted_file, sorted)?;

// 6. Uniq
let uniq_opts = UniqOptions {
    count: true,
    ..Default::default()
};
let unique = toolkit.uniq(&[sorted_file], &uniq_opts)?;

println!("{}", unique);

// Cleanup
fs::remove_file(temp_file)?;
fs::remove_file(sorted_file)?;
```

### Shell script with error handling

```rust
use mistralrs_agent_tools::{AgentToolkit, CommandOptions, ShellType};

let toolkit = AgentToolkit::with_defaults();

let commands = vec![
    "Write-Host 'Starting build...'",
    "cargo build --release",
    "Write-Host 'Build complete!'",
];

for cmd in commands {
    let options = CommandOptions {
        shell: ShellType::PowerShell,
        timeout: Some(300),  // 5 minute timeout
        ..Default::default()
    };
    
    let result = toolkit.execute(cmd, &options)?;
    
    if result.status != 0 {
        eprintln!("Command failed: {}", cmd);
        eprintln!("Error: {}", result.stderr);
        return Err(AgentError::IoError(format!("Command failed with status {}", result.status)));
    }
    
    println!("{}", result.stdout);
}
```

### Log analysis

```rust
use mistralrs_agent_tools::{AgentToolkit, GrepOptions, SortOptions, UniqOptions};
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();
let log_file = Path::new("application.log");

// Find all ERROR lines
let grep_opts = GrepOptions {
    ignore_case: true,
    line_number: true,
    before_context: 2,
    after_context: 2,
    ..Default::default()
};
let errors = toolkit.grep("ERROR", &[log_file], &grep_opts)?;

println!("Found {} errors:", errors.len());
for error in errors {
    println!("\nLine {}:", error.line_number);
    
    // Show context
    for before in &error.before {
        println!("  {}", before);
    }
    
    println!(">>> {}", error.line);
    
    for after in &error.after {
        println!("  {}", after);
    }
}
```

### Word frequency analysis

```rust
use mistralrs_agent_tools::{AgentToolkit, WcOptions};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

let toolkit = AgentToolkit::with_defaults();
let file_path = Path::new("document.txt");

// Read file
let content = fs::read_to_string(file_path)?;

// Count words manually for frequency
let mut word_counts: HashMap<String, usize> = HashMap::new();
for word in content.split_whitespace() {
    let word_lower = word.to_lowercase();
    *word_counts.entry(word_lower).or_insert(0) += 1;
}

// Get total counts with wc
let wc_opts = WcOptions {
    lines: true,
    words: true,
    chars: true,
    bytes: true,
};
let results = toolkit.wc(&[file_path], &wc_opts)?;

for (path, result) in results {
    println!("File: {}", path);
    println!("Lines: {}", result.lines);
    println!("Words: {}", result.words);
    println!("Characters: {}", result.chars);
    println!("Bytes: {}", result.bytes);
    println!("Unique words: {}", word_counts.len());
}

// Show top 10 most common words
let mut word_vec: Vec<_> = word_counts.iter().collect();
word_vec.sort_by(|a, b| b.1.cmp(a.1));

println!("\nTop 10 words:");
for (word, count) in word_vec.iter().take(10) {
    println!("{}: {}", word, count);
}
```

## Error Handling

All methods return `AgentResult<T>` which is `Result<T, AgentError>`:

```rust
match toolkit.grep("pattern", &[path], &options) {
    Ok(matches) => {
        // Process matches
    }
    Err(AgentError::SandboxViolation(msg)) => {
        eprintln!("Security error: {}", msg);
    }
    Err(AgentError::InvalidInput(msg)) => {
        eprintln!("Invalid input: {}", msg);
    }
    Err(AgentError::IoError(msg)) => {
        eprintln!("I/O error: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Sandbox Configuration

```rust
use mistralrs_agent_tools::{SandboxConfig, Sandbox, AgentToolkit};
use std::path::PathBuf;

// Custom sandbox
let config = SandboxConfig::new(PathBuf::from("C:\\Projects"))
    .allow_read_outside(false)
    .max_read_size(100 * 1024 * 1024)  // 100MB
    .max_batch_size(1000);

let sandbox = Sandbox::new(config)?;
let toolkit = AgentToolkit::new(sandbox);

// Or use defaults (current directory as sandbox root)
let toolkit = AgentToolkit::with_defaults();
```
