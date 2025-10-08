//! Search utilities module.
//!
//! Implements enhanced search tools with modern backends:
//! - find: Search for files (fd-powered)
//! - grep: Search file contents (ripgrep-powered)
//! - tree: Display directory structure
//! - which: Locate commands
//! - where: Locate commands (Windows-style)

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use regex::RegexBuilder;

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};

/// Options controlling file system discovery.
#[derive(Debug, Clone)]
pub struct FindOptions {
    pub name_pattern: Option<String>,
    pub case_insensitive: bool,
    pub max_depth: Option<usize>,
    pub include_hidden: bool,
    pub limit: Option<usize>,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            name_pattern: None,
            case_insensitive: false,
            max_depth: None,
            include_hidden: false,
            limit: Some(1_000),
        }
    }
}

/// Result of a find operation.
#[derive(Debug, Clone)]
pub struct FindResult {
    pub entries: Vec<PathBuf>,
    pub truncated: bool,
}

/// Options for text search (grep-like).
#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub case_insensitive: bool,
    pub context: usize,
    pub max_matches: Option<usize>,
}

impl Default for GrepOptions {
    fn default() -> Self {
        Self {
            case_insensitive: false,
            context: 0,
            max_matches: Some(1_000),
        }
    }
}

/// A single grep match (without context lines for now).
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub file: PathBuf,
    pub line_number: u64,
    pub line: String,
    /// (start,end) byte indices of first match on the line (best-effort)
    pub span: Option<(usize, usize)>,
    /// Lines of context appearing before this line (oldest first)
    pub context_before: Vec<String>,
    /// Lines of context appearing after this line (filled post scan)
    pub context_after: Vec<String>,
}

/// Grep result collection.
#[derive(Debug, Clone)]
pub struct GrepResult {
    pub matches: Vec<GrepMatch>,
    pub truncated: bool,
}

/// Find files within the sandbox based on pattern and depth filters.
pub fn find(sandbox: &Sandbox, root: &Path, options: &FindOptions) -> AgentResult<FindResult> {
    let root = sandbox.validate_read(root)?;

    let mut builder = WalkBuilder::new(&root);
    if let Some(depth) = options.max_depth {
        builder.max_depth(Some(depth));
    }
    if !options.include_hidden {
        builder.hidden(false);
    }

    let name_regex = if let Some(pat) = &options.name_pattern {
        let mut rb = RegexBuilder::new(pat);
        rb.case_insensitive(options.case_insensitive);
        Some(
            rb.build()
                .map_err(|e| AgentError::InvalidInput(format!("Invalid pattern: {e}")))?,
        )
    } else {
        None
    };

    let mut entries = Vec::new();
    let mut truncated = false;
    let limit = options.limit.unwrap_or(usize::MAX);

    for dent in builder.build() {
        let dent = match dent {
            Ok(d) => d,
            Err(err) => {
                // Skip unreadable entries, but surface first error if nothing collected
                if entries.is_empty() {
                    return Err(AgentError::IoError(err.to_string()));
                }
                continue;
            }
        };
        let path = dent.path();
        if dent
            .file_type()
            .map(|ft| ft.is_file() || ft.is_dir())
            .unwrap_or(false)
        {
            if let Some(re) = &name_regex {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if !re.is_match(name) {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            entries.push(path.to_path_buf());
            if entries.len() >= limit {
                truncated = true;
                break;
            }
        }
    }

    Ok(FindResult { entries, truncated })
}

/// Grep for a pattern recursively under root respecting sandbox.
pub fn grep(
    sandbox: &Sandbox,
    root: &Path,
    pattern: &str,
    options: &GrepOptions,
) -> AgentResult<GrepResult> {
    let root = sandbox.validate_read(root)?;
    let mut rb = RegexBuilder::new(pattern);
    rb.case_insensitive(options.case_insensitive)
        .dot_matches_new_line(false); // simple default
    let regex = rb
        .build()
        .map_err(|e| AgentError::InvalidInput(format!("Invalid regex: {e}")))?;

    let mut matches = Vec::new();
    let mut truncated = false;
    let max_matches = options.max_matches.unwrap_or(usize::MAX);

    for dent in WalkBuilder::new(&root).build() {
        let dent = match dent {
            Ok(d) => d,
            Err(_) => continue,
        };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let validated = if let Ok(p) = sandbox.validate_read(dent.path()) {
            p
        } else {
            continue;
        };
        let Ok(file) = File::open(&validated) else {
            continue;
        };
        let reader = BufReader::new(file);
        let mut window: Vec<String> = Vec::new();
        let mut pending_after: Vec<(usize, usize)> = Vec::new(); // (match_index, lines_remaining)
        let mut buffered_lines: Vec<String> = Vec::new();
        for (idx, line_res) in reader.lines().enumerate() {
            let Ok(line) = line_res else { continue };
            let current_line_number = idx + 1;
            // Update context after counters first
            for (_, remaining) in pending_after.iter_mut() {
                if *remaining > 0 {
                    *remaining -= 1;
                }
            }
            // We store the line early for possible after-context association later.
            buffered_lines.push(line.clone());
            // If line matches pattern
            if regex.is_match(&line) {
                let span = regex.find(&line).map(|m| (m.start(), m.end()));
                let before_ctx_start = if window.len() > options.context {
                    window.len() - options.context
                } else {
                    0
                };
                let context_before = window[before_ctx_start..].to_vec();
                let gm_index = matches.len();
                matches.push(GrepMatch {
                    file: validated.clone(),
                    line_number: current_line_number as u64,
                    line: line.clone(),
                    span,
                    context_before,
                    context_after: Vec::new(),
                });
                if options.context > 0 {
                    pending_after.push((gm_index, options.context));
                }
                if matches.len() >= max_matches {
                    truncated = true;
                    break;
                }
            }
            window.push(line.clone());
            if window.len() > options.context {
                window.remove(0);
            }
            // Feed after-context lines into matches
            for (mi, remaining) in pending_after.clone() {
                // iterate snapshot
                if remaining > 0 {
                    // still expecting after lines
                    if let Some(m) = matches.get_mut(mi) {
                        if m.line_number as usize != current_line_number {
                            // avoid duplicating the match line itself
                            m.context_after.push(line.clone());
                        }
                    }
                }
            }
            pending_after.retain(|(_, r)| *r > 0);
        }
        if truncated {
            break;
        }
    }

    Ok(GrepResult { matches, truncated })
}

/// Simple tree listing (depth-first) returning relative paths from root.
pub fn tree(
    sandbox: &Sandbox,
    root: &Path,
    max_depth: Option<usize>,
    limit: Option<usize>,
) -> AgentResult<Vec<PathBuf>> {
    let root = sandbox.validate_read(root)?;
    let mut builder = WalkBuilder::new(&root);
    if let Some(d) = max_depth {
        builder.max_depth(Some(d));
    }
    let limit = limit.unwrap_or(10_000);
    let mut out = Vec::new();
    for dent in builder.build() {
        let dent = match dent {
            Ok(d) => d,
            Err(_) => continue,
        };
        let p = dent.path();
        out.push(p.to_path_buf());
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::sandbox::Sandbox;
    use crate::types::SandboxConfig;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_find_basic() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.txt"), "a").unwrap();
        std::fs::create_dir_all(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/beta.log"), "b").unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(dir.path().to_path_buf()));
        let res = find(
            &sandbox,
            dir.path(),
            &FindOptions {
                name_pattern: Some(".*\\.txt".into()),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(res.entries.len(), 1);
    }

    #[test]
    fn test_grep_basic() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        let mut f = std::fs::File::create(&file_path).unwrap();
        writeln!(f, "Hello world").unwrap();
        writeln!(f, "Find the TODO here").unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(dir.path().to_path_buf()));
        let opts = GrepOptions {
            context: 1,
            ..Default::default()
        };
        let res = grep(&sandbox, dir.path(), "TODO", &opts).unwrap();
        assert_eq!(res.matches.len(), 1);
        let m = &res.matches[0];
        assert!(m.context_before.len() <= 1);
        assert!(m.context_after.len() <= 1);
        assert!(m.span.is_some());
    }
}
