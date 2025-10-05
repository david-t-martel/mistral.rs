use anyhow::{Context, Result};
use crossbeam_channel::bounded;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use winpath::normalize_path;

use crate::cli::Args;
use crate::output::{OutputFormatter, TreeNode};
use crate::utils::FileInfo;
use crate::windows::FileAttributes;

pub struct TreeWalker {
    args: Args,
    formatter: Box<dyn OutputFormatter>,
    stats: Arc<TreeStats>,
}

#[derive(Debug, Default)]
pub struct TreeStats {
    pub files: AtomicUsize,
    pub directories: AtomicUsize,
    pub total_size: AtomicUsize,
    pub symlinks: AtomicUsize,
    pub junction_points: AtomicUsize,
    pub hidden_files: AtomicUsize,
    pub errors: AtomicUsize,
}

impl TreeWalker {
    pub fn new(args: Args) -> Result<Self> {
        let formatter = crate::output::create_formatter(&args)?;
        let stats = Arc::new(TreeStats::default());

        Ok(Self {
            args,
            formatter,
            stats,
        })
    }

    pub fn walk(&self) -> Result<()> {
        let start_time = SystemTime::now();

        // Normalize and validate the starting directory to handle Git Bash mangled paths
        let normalized_path = self.args.get_normalized_directory()
            .map_err(|e| anyhow::anyhow!("Invalid directory path '{}': {}", self.args.directory.display(), e))?;

        let start_path = dunce::canonicalize(&normalized_path)
            .with_context(|| format!("Cannot access directory: {}", self.args.directory.display()))?;

        if !start_path.is_dir() {
            anyhow::bail!("Not a directory: {}", start_path.display());
        }

        // Build the tree
        let root_node = if self.args.get_thread_count() > 1 && !self.args.no_parallel {
            self.walk_parallel(&start_path)?
        } else {
            self.walk_sequential(&start_path)?
        };

        // Output the results
        self.formatter.output(&root_node, &self.stats)?;

        // Show timing and summary if requested
        if self.args.show_summary {
            let elapsed = start_time.elapsed().unwrap_or_default();
            self.print_summary(elapsed)?;
        }

        Ok(())
    }

    fn walk_sequential(&self, path: &Path) -> Result<TreeNode> {
        self.walk_directory(path, 0, true)
    }

    fn walk_parallel(&self, path: &Path) -> Result<TreeNode> {
        let thread_count = self.args.get_thread_count();
        let (work_sender, work_receiver) = bounded::<WorkItem>(1000);
        let (result_sender, result_receiver) = bounded::<WorkResult>(1000);

        // Clone args for thread safety
        let args_clone = self.args.clone();

        // Start worker threads
        let workers: Vec<_> = (0..thread_count)
            .map(|_| {
                let args = args_clone.clone();
                let stats = Arc::clone(&self.stats);
                let work_rx = work_receiver.clone();
                let result_tx = result_sender.clone();

                std::thread::spawn(move || {
                    while let Ok(work_item) = work_rx.recv() {
                        let result = Self::process_work_item(&work_item, &args, &stats);
                        if result_tx.send(WorkResult { work_item, result }).is_err() {
                            break;
                        }
                    }
                })
            })
            .collect();

        // Drop extra senders/receivers
        drop(work_receiver);
        drop(result_sender);

        // Submit initial work
        let root_item = WorkItem {
            path: path.to_owned(),
            depth: 0,
            parent_id: None,
            id: 0,
        };
        work_sender.send(root_item)?;

        // Track work and results
        let mut pending_work = HashMap::new();
        let mut completed_nodes = HashMap::new();
        let mut next_id = 1;

        // Process results
        while let Ok(result) = result_receiver.recv() {
            let work_item_id = result.work_item.id;
            let work_item = result.work_item;

            match result.result {
                Ok(node_info) => {
                    // Submit child work items
                    if let Some(ref children) = node_info.children {
                        for child_path in children {
                            if let Some(max_depth) = self.args.effective_max_depth() {
                                if work_item.depth >= max_depth {
                                    continue;
                                }
                            }

                            let child_item = WorkItem {
                                path: child_path.clone(),
                                depth: work_item.depth + 1,
                                parent_id: Some(work_item.id),
                                id: next_id,
                            };
                            next_id += 1;

                            pending_work.insert(child_item.id, child_item.clone());
                            work_sender.send(child_item)?;
                        }
                    }

                    completed_nodes.insert(work_item.id, (work_item, node_info));
                }
                Err(e) => {
                    self.stats.errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Error processing {}: {}", work_item.path.display(), e);
                }
            }

            // Check if we're done
            pending_work.remove(&work_item_id);
            if pending_work.is_empty() {
                break;
            }
        }

        // Clean up workers
        drop(work_sender);
        for worker in workers {
            worker.join().unwrap();
        }

        // Build tree from completed nodes
        self.build_tree_from_results(completed_nodes)
    }

    fn walk_directory(&self, path: &Path, depth: usize, _is_root: bool) -> Result<TreeNode> {
        let file_info = FileInfo::from_path(path)?;

        // Check depth limit
        if let Some(max_depth) = self.args.effective_max_depth() {
            if depth > max_depth {
                return Ok(TreeNode::new(file_info, Vec::new()));
            }
        }

        let mut children = Vec::new();

        if file_info.is_dir {
            self.stats.directories.fetch_add(1, Ordering::Relaxed);

            match std::fs::read_dir(path) {
                Ok(entries) => {
                    let mut entries: Vec<_> = entries
                        .filter_map(|entry| entry.ok())
                        .filter(|entry| self.should_include_entry(entry))
                        .collect();

                    // Sort entries if requested
                    if self.args.sort || self.args.sort_time {
                        self.sort_entries(&mut entries)?;
                    }

                    for entry in entries {
                        let child_path = entry.path();
                        match self.walk_directory(&child_path, depth + 1, false) {
                            Ok(child_node) => children.push(child_node),
                            Err(e) => {
                                self.stats.errors.fetch_add(1, Ordering::Relaxed);
                                eprintln!("Error reading {}: {}", child_path.display(), e);
                            }
                        }
                    }
                }
                Err(e) => {
                    self.stats.errors.fetch_add(1, Ordering::Relaxed);
                    eprintln!("Error reading directory {}: {}", path.display(), e);
                }
            }
        } else {
            self.stats.files.fetch_add(1, Ordering::Relaxed);
            if let Some(size) = file_info.size {
                self.stats.total_size.fetch_add(size as usize, Ordering::Relaxed);
            }

            // Track special file types
            if file_info.is_symlink {
                self.stats.symlinks.fetch_add(1, Ordering::Relaxed);
            }
            if file_info.attributes.as_ref().map_or(false, |a| a.hidden) {
                self.stats.hidden_files.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(TreeNode::new(file_info, children))
    }

    fn should_include_entry(&self, entry: &std::fs::DirEntry) -> bool {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(meta) => meta,
            Err(_) => return false,
        };

        // Check if we should show hidden files
        if !self.args.show_all {
            if let Ok(attrs) = FileAttributes::from_path(&path) {
                if attrs.hidden || attrs.system {
                    return false;
                }
            }
        }

        // Check if we only want directories
        if self.args.dirs_only && !metadata.is_dir() {
            return false;
        }

        // Check pattern matching
        if let Some(ref pattern) = self.args.pattern {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if !glob_match(pattern, filename) {
                return false;
            }
        }

        // Check ignore pattern
        if let Some(ref ignore_pattern) = self.args.ignore_pattern {
            let filename = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if glob_match(ignore_pattern, filename) {
                return false;
            }
        }

        // Check file extension filter
        if let Some(ref ext) = self.args.filter_extension {
            if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                if !file_ext.eq_ignore_ascii_case(ext) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    fn sort_entries(&self, entries: &mut [std::fs::DirEntry]) -> Result<()> {
        if self.args.sort_time {
            entries.sort_by(|a, b| {
                let a_time = a.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let b_time = b.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                if self.args.reverse {
                    a_time.cmp(&b_time)
                } else {
                    b_time.cmp(&a_time)
                }
            });
        } else if self.args.sort {
            entries.sort_by(|a, b| {
                let a_name = a.file_name();
                let b_name = b.file_name();

                if self.args.reverse {
                    b_name.cmp(&a_name)
                } else {
                    a_name.cmp(&b_name)
                }
            });
        }

        Ok(())
    }

    fn print_summary(&self, elapsed: std::time::Duration) -> Result<()> {
        let files = self.stats.files.load(Ordering::Relaxed);
        let dirs = self.stats.directories.load(Ordering::Relaxed);
        let total_size = self.stats.total_size.load(Ordering::Relaxed);
        let errors = self.stats.errors.load(Ordering::Relaxed);

        println!();
        println!("Summary:");
        println!("  Directories: {}", dirs);
        println!("  Files: {}", files);
        println!("  Total size: {}", humansize::format_size(total_size, humansize::BINARY));

        if self.args.show_links {
            let symlinks = self.stats.symlinks.load(Ordering::Relaxed);
            let junctions = self.stats.junction_points.load(Ordering::Relaxed);
            println!("  Symbolic links: {}", symlinks);
            println!("  Junction points: {}", junctions);
        }

        if errors > 0 {
            println!("  Errors: {}", errors);
        }

        println!("  Time: {:.2}s", elapsed.as_secs_f64());

        Ok(())
    }

    fn process_work_item(
        work_item: &WorkItem,
        args: &Args,
        stats: &TreeStats,
    ) -> Result<NodeInfo> {
        let file_info = FileInfo::from_path(&work_item.path)?;
        let mut children = None;

        if file_info.is_dir {
            stats.directories.fetch_add(1, Ordering::Relaxed);

            if let Some(max_depth) = args.effective_max_depth() {
                if work_item.depth < max_depth {
                    children = Some(Self::read_directory_children(&work_item.path, args)?);
                }
            } else {
                children = Some(Self::read_directory_children(&work_item.path, args)?);
            }
        } else {
            stats.files.fetch_add(1, Ordering::Relaxed);
            if let Some(size) = file_info.size {
                stats.total_size.fetch_add(size as usize, Ordering::Relaxed);
            }
        }

        Ok(NodeInfo {
            file_info,
            children,
        })
    }

    fn read_directory_children(path: &Path, args: &Args) -> Result<Vec<PathBuf>> {
        let entries = std::fs::read_dir(path)?;
        let children: Vec<PathBuf> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                // Apply the same filtering logic as should_include_entry
                // This is a simplified version for parallel processing
                if !args.show_all {
                    if let Ok(attrs) = FileAttributes::from_path(&entry.path()) {
                        if attrs.hidden || attrs.system {
                            return false;
                        }
                    }
                }
                true
            })
            .map(|entry| entry.path())
            .collect();

        Ok(children)
    }

    fn build_tree_from_results(
        &self,
        completed_nodes: HashMap<usize, (WorkItem, NodeInfo)>,
    ) -> Result<TreeNode> {
        // Find the root node (id = 0)
        let (root_work, root_info) = completed_nodes.get(&0)
            .ok_or_else(|| anyhow::anyhow!("Root node not found"))?;

        fn build_node(
            work_item: &WorkItem,
            node_info: &NodeInfo,
            completed_nodes: &HashMap<usize, (WorkItem, NodeInfo)>,
        ) -> TreeNode {
            let mut children = Vec::new();

            // Find all children of this node
            for (_, (child_work, child_info)) in completed_nodes.iter() {
                if child_work.parent_id == Some(work_item.id) {
                    children.push(build_node(child_work, child_info, completed_nodes));
                }
            }

            TreeNode::new(node_info.file_info.clone(), children)
        }

        Ok(build_node(root_work, root_info, &completed_nodes))
    }
}

#[derive(Debug, Clone)]
struct WorkItem {
    path: PathBuf,
    depth: usize,
    parent_id: Option<usize>,
    id: usize,
}

#[derive(Debug)]
struct WorkResult {
    work_item: WorkItem,
    result: Result<NodeInfo>,
}

#[derive(Debug)]
struct NodeInfo {
    file_info: FileInfo,
    children: Option<Vec<PathBuf>>,
}

/// Simple glob matching (supports * and ?)
fn glob_match(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();

    if pattern.is_empty() {
        return text.is_empty();
    }

    if pattern == "*" {
        return true;
    }

    // Simple implementation - can be enhanced with regex if needed
    pattern.chars().zip(text.chars()).all(|(p, t)| p == '*' || p == '?' || p == t)
}
