//! Shared tool registry for interactive and agent modes.
//! Provides helper to build Tool definitions and associated callbacks
//! from the `mistralrs_agent_tools::AgentToolkit`.

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use anyhow::anyhow;
use mistralrs_agent_tools::{
    AgentToolkit, CatOptions, CommandOptions, GrepOptions, HeadOptions, LsOptions, ShellType,
    SortOptions, TailOptions, UniqOptions, WcOptions,
};
use mistralrs_core::{Function, Tool, ToolCallback, ToolCallbackWithTool, ToolType};
use serde_json::{json, Value};

/// Build a list of tool definitions (for inclusion in a request) plus an owned
/// callback map whose closures delegate to the AgentToolkit. The callbacks all
/// return a `String` (stdout / textual result) or an error string.
pub fn build_tool_definitions_and_callbacks(
    toolkit: &AgentToolkit,
) -> (Vec<Tool>, HashMap<String, ToolCallbackWithTool>) {
    // Helper for constructing tool objects.
    fn mk(name: &str, desc: &str, params: serde_json::Value) -> Tool {
        let mut param_map = HashMap::new();
        if let Some(obj) = params.as_object() {
            for (k, v) in obj.iter() {
                param_map.insert(k.clone(), v.clone());
            }
        }
        Tool {
            tp: ToolType::Function,
            function: Function {
                name: name.to_string(),
                description: Some(desc.to_string()),
                parameters: Some(param_map),
            },
        }
    }

    // Build definitions (mirrors those in agent_mode prior to extraction).
    let tool_specs: Vec<(Tool, Arc<ToolCallback>)> = vec![
        (
            mk(
                "cat",
                "Read and concatenate text files",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "number_lines": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    // Parse arguments JSON
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("cat: 'paths' must be an array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("cat: no paths provided"));
                    }
                    let number_lines = v
                        .get("number_lines")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(false);
                    let options = CatOptions {
                        number_lines,
                        ..Default::default()
                    };
                    let owned: Vec<PathBuf> = paths;
                    let refs: Vec<&std::path::Path> = owned.iter().map(|p| p.as_path()).collect();
                    let out = tk
                        .cat(&refs, &options)
                        .map_err(|e| anyhow!(e.to_string()))?;
                    Ok(out)
                }
            }) as Arc<ToolCallback>,
        ),
        (
            mk(
                "ls",
                "List directory contents",
                json!({
                    "path": {"type": "string"},
                    "all": {"type": "boolean"},
                    "long": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let path = v.get("path").and_then(|p| p.as_str()).unwrap_or(".");
                    let all = v.get("all").and_then(|b| b.as_bool()).unwrap_or(false);
                    let long = v.get("long").and_then(|b| b.as_bool()).unwrap_or(false);
                    let opts = LsOptions {
                        all,
                        long,
                        ..Default::default()
                    };
                    let res = tk
                        .ls(std::path::Path::new(path), &opts)
                        .map_err(|e| anyhow!(e.to_string()))?;
                    // Format result
                    let mut lines = Vec::new();
                    for entry in res.entries {
                        if long {
                            let kind = if entry.is_dir { "<DIR>" } else { "FILE" };
                            lines.push(format!("{}\t{}\t{}", entry.name, kind, entry.size));
                        } else {
                            lines.push(entry.name);
                        }
                    }
                    Ok(lines.join("\n"))
                }
            }),
        ),
        (
            mk(
                "head",
                "Show first lines of files",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "lines": {"type": "integer"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("head: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("head: no paths provided"));
                    }
                    let lines = v.get("lines").and_then(|n| n.as_u64()).unwrap_or(10) as usize;
                    let opts = HeadOptions {
                        lines,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let out = tk.head(&refs, &opts).map_err(|e| anyhow!(e.to_string()))?;
                    Ok(out)
                }
            }),
        ),
        (
            mk(
                "tail",
                "Show last lines of files",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "lines": {"type": "integer"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("tail: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("tail: no paths provided"));
                    }
                    let lines = v.get("lines").and_then(|n| n.as_u64()).unwrap_or(10) as usize;
                    let opts = TailOptions {
                        lines,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let out = tk.tail(&refs, &opts).map_err(|e| anyhow!(e.to_string()))?;
                    Ok(out)
                }
            }),
        ),
        (
            mk(
                "wc",
                "Count lines / words / bytes of files",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "lines": {"type": "boolean"},
                    "words": {"type": "boolean"},
                    "bytes": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("wc: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("wc: no paths provided"));
                    }
                    let lines_flag = v.get("lines").and_then(|b| b.as_bool()).unwrap_or(true);
                    let words_flag = v.get("words").and_then(|b| b.as_bool()).unwrap_or(true);
                    let bytes_flag = v.get("bytes").and_then(|b| b.as_bool()).unwrap_or(false);
                    let opts = WcOptions {
                        lines: lines_flag,
                        words: words_flag,
                        bytes: bytes_flag,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let res = tk.wc(&refs, &opts).map_err(|e| anyhow!(e.to_string()))?;
                    let mut out_lines = Vec::new();
                    for (path, r) in res {
                        out_lines.push(format!(
                            "{}\tlines:{} words:{} bytes:{}",
                            path, r.lines, r.words, r.bytes
                        ));
                    }
                    Ok(out_lines.join("\n"))
                }
            }),
        ),
        (
            mk(
                "grep",
                "Search for a regex pattern in files",
                json!({
                    "pattern": {"type": "string"},
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "ignore_case": {"type": "boolean"},
                    "line_number": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let pattern = v
                        .get("pattern")
                        .and_then(|p| p.as_str())
                        .ok_or_else(|| anyhow!("grep: 'pattern' required"))?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("grep: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("grep: no paths provided"));
                    }
                    let ignore_case = v
                        .get("ignore_case")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(false);
                    let line_number = v
                        .get("line_number")
                        .and_then(|b| b.as_bool())
                        .unwrap_or(true);
                    let opts = GrepOptions {
                        ignore_case,
                        line_number,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let matches = tk
                        .grep(pattern, &refs, &opts)
                        .map_err(|e| anyhow!(e.to_string()))?;
                    let mut lines = Vec::new();
                    for m in matches {
                        lines.push(format!("{}:{}:{}", m.path, m.line_number, m.line));
                    }
                    Ok(lines.join("\n"))
                }
            }),
        ),
        (
            mk(
                "sort",
                "Sort lines from files",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "numeric": {"type": "boolean"},
                    "reverse": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("sort: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("sort: no paths provided"));
                    }
                    let numeric = v.get("numeric").and_then(|b| b.as_bool()).unwrap_or(false);
                    let reverse = v.get("reverse").and_then(|b| b.as_bool()).unwrap_or(false);
                    let opts = SortOptions {
                        numeric,
                        reverse,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let out = tk.sort(&refs, &opts).map_err(|e| anyhow!(e.to_string()))?;
                    Ok(out)
                }
            }),
        ),
        (
            mk(
                "uniq",
                "Filter adjacent duplicate lines",
                json!({
                    "paths": {"type": "array", "items": {"type": "string"}},
                    "count": {"type": "boolean"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let paths: Vec<PathBuf> = v
                        .get("paths")
                        .and_then(|p| p.as_array())
                        .ok_or_else(|| anyhow!("uniq: 'paths' must be array"))?
                        .iter()
                        .filter_map(|s| s.as_str())
                        .map(PathBuf::from)
                        .collect();
                    if paths.is_empty() {
                        return Err(anyhow!("uniq: no paths provided"));
                    }
                    let count = v.get("count").and_then(|b| b.as_bool()).unwrap_or(false);
                    let opts = UniqOptions {
                        count,
                        ..Default::default()
                    };
                    let refs: Vec<&std::path::Path> = paths.iter().map(|p| p.as_path()).collect();
                    let out = tk.uniq(&refs, &opts).map_err(|e| anyhow!(e.to_string()))?;
                    Ok(out)
                }
            }),
        ),
        (
            mk(
                "execute",
                "Execute a shell command and capture output",
                json!({
                    "command": {"type": "string"},
                    "shell": {"type": "string", "enum": ["powershell", "cmd", "bash"]},
                    "timeout": {"type": "integer"}
                }),
            ),
            Arc::new({
                let tk = toolkit.clone();
                move |cf: &mistralrs_core::CalledFunction| {
                    let v: Value = serde_json::from_str(&cf.arguments)?;
                    let command = v
                        .get("command")
                        .and_then(|c| c.as_str())
                        .ok_or_else(|| anyhow!("execute: 'command' required"))?;
                    let shell = match v.get("shell").and_then(|s| s.as_str()) {
                        Some("powershell") | None => ShellType::PowerShell,
                        Some("cmd") => ShellType::Cmd,
                        Some("bash") => ShellType::Bash,
                        Some(other) => {
                            return Err(anyhow!(format!("execute: unsupported shell {other}")))
                        }
                    };
                    let timeout = v.get("timeout").and_then(|t| t.as_u64());
                    let opts = CommandOptions {
                        shell,
                        timeout,
                        ..Default::default()
                    };
                    let res = tk
                        .execute(command, &opts)
                        .map_err(|e| anyhow!(e.to_string()))?;
                    let mut out = String::new();
                    out.push_str(&format!("status: {}\n", res.status));
                    if !res.stdout.trim().is_empty() {
                        out.push_str(&format!("stdout:\n{}\n", res.stdout));
                    }
                    if !res.stderr.trim().is_empty() {
                        out.push_str(&format!("stderr:\n{}\n", res.stderr));
                    }
                    Ok(out)
                }
            }),
        ),
    ];

    let mut defs = Vec::with_capacity(tool_specs.len());
    let mut callbacks = HashMap::with_capacity(tool_specs.len());
    for (tool, cb) in tool_specs {
        let name = tool.function.name.clone();
        defs.push(tool.clone());
        callbacks.insert(name.clone(), ToolCallbackWithTool { callback: cb, tool });
    }
    (defs, callbacks)
}
