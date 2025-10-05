use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::cli::Args;
use crate::tree::TreeStats;
use crate::utils::FileInfo;

pub trait OutputFormatter {
    fn output(&self, root: &TreeNode, stats: &TreeStats) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub info: FileInfo,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(info: FileInfo, children: Vec<TreeNode>) -> Self {
        Self { info, children }
    }
}

pub fn create_formatter(args: &Args) -> Result<Box<dyn OutputFormatter>> {
    if args.json_output {
        Ok(Box::new(JsonFormatter::new(args)))
    } else {
        Ok(Box::new(TextFormatter::new(args)))
    }
}

/// Text-based tree formatter with Unicode/ASCII box drawing
pub struct TextFormatter {
    use_colors: bool,
    use_unicode: bool,
    show_size: bool,
    show_time: bool,
    show_attributes: bool,
    show_links: bool,
    show_streams: bool,
    full_path: bool,
    show_extensions: bool,
}

impl TextFormatter {
    pub fn new(args: &Args) -> Self {
        Self {
            use_colors: args.use_colors(),
            use_unicode: !args.ascii_only,
            show_size: args.show_size,
            show_time: args.show_time,
            show_attributes: args.show_attributes,
            show_links: args.show_links,
            show_streams: args.show_streams,
            full_path: args.full_path,
            show_extensions: args.show_extensions,
        }
    }

    fn get_tree_chars(&self) -> TreeChars {
        if self.use_unicode {
            TreeChars::unicode()
        } else {
            TreeChars::ascii()
        }
    }

    fn format_file_info(&self, info: &FileInfo, prefix: &str) -> String {
        let mut output = String::new();

        // Add prefix (tree structure)
        output.push_str(prefix);

        // Add file/directory name
        let name = if self.full_path {
            info.path.to_string_lossy().to_string()
        } else {
            info.path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };

        output.push_str(&name);

        // Add directory indicator
        if info.is_dir {
            output.push('/');
        }

        // Add size if requested
        if self.show_size && !info.is_dir {
            if let Some(size) = info.size {
                output.push_str(&format!(" [{}]", humansize::format_size(size, humansize::BINARY)));
            }
        }

        // Add modification time if requested
        if self.show_time {
            if let Some(ref time_str) = info.modified_time {
                output.push_str(&format!(" ({})", time_str));
            }
        }

        // Add Windows attributes if requested
        if self.show_attributes {
            if let Some(ref attrs) = info.attributes {
                let attr_str = attrs.to_compact_string();
                if !attr_str.is_empty() {
                    output.push_str(&format!(" [{}]", attr_str));
                }
            }
        }

        // Add link information if requested
        if self.show_links {
            if let Some(ref reparse_info) = info.reparse_info {
                if reparse_info.is_symlink {
                    output.push_str(" -> ");
                    if let Some(ref target) = reparse_info.target {
                        output.push_str(&target.to_string_lossy());
                    } else {
                        output.push_str("?");
                    }
                } else if reparse_info.is_junction {
                    output.push_str(" <JUNCTION>");
                }
            }
        }

        // Add alternate data streams if requested
        if self.show_streams {
            if let Some(ref streams) = info.alternate_streams {
                if !streams.streams.is_empty() {
                    output.push_str(&format!(" +ADS({})", streams.streams.len()));
                }
            }
        }

        output
    }

    fn write_node(&self, writer: &mut StandardStream, node: &TreeNode, prefix: &str, is_last: bool, depth: usize) -> Result<()> {
        let chars = self.get_tree_chars();

        // Create the current line prefix
        let current_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{} ", prefix, if is_last { chars.last } else { chars.branch })
        };

        // Format and write the current node
        let formatted = self.format_file_info(&node.info, &current_prefix);

        if self.use_colors {
            self.write_colored_line(writer, &node.info, &formatted)?;
        } else {
            writeln!(writer, "{}", formatted)?;
        }

        // Prepare prefix for children
        let child_prefix = if depth == 0 {
            String::new()
        } else {
            format!("{}{}", prefix, if is_last { "    " } else { &format!("{}   ", chars.vertical) })
        };

        // Write children
        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            self.write_node(writer, child, &child_prefix, is_last_child, depth + 1)?;
        }

        Ok(())
    }

    fn write_colored_line(&self, writer: &mut StandardStream, info: &FileInfo, text: &str) -> Result<()> {
        let mut color_spec = ColorSpec::new();

        // Set colors based on file type
        if info.is_dir {
            color_spec.set_fg(Some(Color::Blue)).set_bold(true);
        } else if info.is_symlink {
            color_spec.set_fg(Some(Color::Cyan));
        } else if info.is_executable {
            color_spec.set_fg(Some(Color::Green)).set_bold(true);
        } else {
            // Check file extension for colors
            if let Some(ext) = info.path.extension().and_then(|e| e.to_str()) {
                match ext.to_lowercase().as_str() {
                    "rs" | "py" | "js" | "ts" | "go" | "c" | "cpp" | "h" | "hpp" => {
                        color_spec.set_fg(Some(Color::Yellow));
                    }
                    "txt" | "md" | "doc" | "docx" | "pdf" => {
                        color_spec.set_fg(Some(Color::White));
                    }
                    "zip" | "rar" | "7z" | "tar" | "gz" => {
                        color_spec.set_fg(Some(Color::Magenta));
                    }
                    "exe" | "dll" | "msi" => {
                        color_spec.set_fg(Some(Color::Red)).set_bold(true);
                    }
                    _ => {
                        color_spec.set_fg(Some(Color::White));
                    }
                }
            }
        }

        // Apply special formatting for hidden files
        if info.attributes.as_ref().map_or(false, |a| a.hidden) {
            color_spec.set_dimmed(true);
        }

        writer.set_color(&color_spec)?;
        writeln!(writer, "{}", text)?;
        writer.reset()?;

        Ok(())
    }
}

impl OutputFormatter for TextFormatter {
    fn output(&self, root: &TreeNode, _stats: &TreeStats) -> Result<()> {
        let color_choice = if self.use_colors {
            ColorChoice::Auto
        } else {
            ColorChoice::Never
        };

        let mut writer = StandardStream::stdout(color_choice);
        self.write_node(&mut writer, root, "", true, 0)?;

        Ok(())
    }
}

/// JSON formatter for machine-readable output
pub struct JsonFormatter {
    pretty: bool,
    include_metadata: bool,
}

impl JsonFormatter {
    pub fn new(args: &Args) -> Self {
        Self {
            pretty: true, // Always pretty print for readability
            include_metadata: args.show_size || args.show_time || args.show_attributes,
        }
    }

    fn node_to_json(&self, node: &TreeNode) -> JsonNode {
        JsonNode {
            name: node.info.path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: node.info.path.to_string_lossy().to_string(),
            is_directory: node.info.is_dir,
            is_symlink: node.info.is_symlink,
            size: node.info.size,
            modified: node.info.modified_time.clone(),
            attributes: if self.include_metadata {
                node.info.attributes.as_ref().map(|a| a.to_compact_string())
            } else {
                None
            },
            children: if node.children.is_empty() {
                None
            } else {
                Some(node.children.iter().map(|child| self.node_to_json(child)).collect())
            },
        }
    }
}

impl OutputFormatter for JsonFormatter {
    fn output(&self, root: &TreeNode, stats: &TreeStats) -> Result<()> {
        let json_root = self.node_to_json(root);
        let output = JsonOutput {
            tree: json_root,
            stats: JsonStats {
                files: stats.files.load(std::sync::atomic::Ordering::Relaxed),
                directories: stats.directories.load(std::sync::atomic::Ordering::Relaxed),
                total_size: stats.total_size.load(std::sync::atomic::Ordering::Relaxed),
                symlinks: stats.symlinks.load(std::sync::atomic::Ordering::Relaxed),
                junction_points: stats.junction_points.load(std::sync::atomic::Ordering::Relaxed),
                hidden_files: stats.hidden_files.load(std::sync::atomic::Ordering::Relaxed),
                errors: stats.errors.load(std::sync::atomic::Ordering::Relaxed),
            },
        };

        if self.pretty {
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", serde_json::to_string(&output)?);
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct JsonOutput {
    tree: JsonNode,
    stats: JsonStats,
}

#[derive(Serialize, Deserialize)]
struct JsonNode {
    name: String,
    path: String,
    is_directory: bool,
    is_symlink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attributes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<JsonNode>>,
}

#[derive(Serialize, Deserialize)]
struct JsonStats {
    files: usize,
    directories: usize,
    total_size: usize,
    symlinks: usize,
    junction_points: usize,
    hidden_files: usize,
    errors: usize,
}

/// Characters used for drawing the tree structure
struct TreeChars {
    branch: &'static str,
    last: &'static str,
    vertical: &'static str,
    horizontal: &'static str,
}

impl TreeChars {
    fn unicode() -> Self {
        Self {
            branch: "├──",
            last: "└──",
            vertical: "│",
            horizontal: "───",
        }
    }

    fn ascii() -> Self {
        Self {
            branch: "|--",
            last: "`--",
            vertical: "|",
            horizontal: "---",
        }
    }
}
