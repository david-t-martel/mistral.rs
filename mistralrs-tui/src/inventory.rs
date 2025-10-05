//! Model discovery helpers for the terminal UI.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::Deserialize;
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct ModelEntry {
    pub identifier: String,
    pub path: PathBuf,
    pub format: Option<String>,
    pub size_bytes: Option<u64>,
}

impl ModelEntry {
    pub fn display_name(&self) -> &str {
        &self.identifier
    }
}

#[derive(Debug)]
pub struct ModelInventory {
    entries: RwLock<Vec<ModelEntry>>,
    search_paths: Vec<PathBuf>,
    inventory_manifest: Option<PathBuf>,
}

impl ModelInventory {
    pub fn new(search_paths: Vec<PathBuf>, inventory_manifest: Option<PathBuf>) -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            search_paths,
            inventory_manifest,
        }
    }

    pub fn refresh(&self) -> Result<()> {
        let mut entries = Vec::new();
        if let Some(manifest_path) = &self.inventory_manifest {
            if manifest_path.exists() {
                entries.extend(self.load_manifest(manifest_path)?);
            }
        }

        for path in &self.search_paths {
            if path.exists() {
                self.scan_directory(path, &mut entries)?;
            }
        }

        entries.sort_by(|a, b| a.identifier.cmp(&b.identifier));
        entries.dedup_by(|a, b| a.path == b.path);
        *self.entries.write() = entries;
        Ok(())
    }

    pub fn entries(&self) -> Vec<ModelEntry> {
        self.entries.read().clone()
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn default_model_id(&self) -> Option<String> {
        self.entries
            .read()
            .first()
            .map(|entry| entry.identifier.clone())
    }

    fn load_manifest(&self, path: &Path) -> Result<Vec<ModelEntry>> {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("Failed to read manifest at {}", path.display()))?;
        let manifest: Vec<ManifestRecord> = serde_json::from_str(&raw)
            .with_context(|| format!("Failed to parse manifest JSON at {}", path.display()))?;

        Ok(manifest
            .into_iter()
            .filter_map(|record| {
                let path = PathBuf::from(record.path);
                if !path.exists() {
                    return None;
                }
                let identifier = record.name.or(record.id).unwrap_or_else(|| {
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                });
                Some(ModelEntry {
                    identifier,
                    path,
                    format: record.format,
                    size_bytes: record.size_bytes,
                })
            })
            .collect())
    }

    fn scan_directory(&self, path: &Path, entries: &mut Vec<ModelEntry>) -> Result<()> {
        for entry in WalkDir::new(path)
            .follow_links(false)
            .max_depth(4)
            .into_iter()
            .filter_map(|res| res.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let file_path = entry.path();
            if !is_model_file(file_path) {
                continue;
            }
            let identifier = file_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
                .unwrap_or_else(|| file_path.display().to_string());
            let size_bytes = entry.metadata().ok().map(|meta| meta.len());
            let format = file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_ascii_lowercase());
            entries.push(ModelEntry {
                identifier,
                path: file_path.to_path_buf(),
                format,
                size_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct ManifestRecord {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    path: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
}

fn is_model_file(path: &Path) -> bool {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => matches!(
            ext.to_ascii_lowercase().as_str(),
            "gguf" | "ggml" | "safetensors"
        ),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn refresh_discovers_files() {
        let tmp = tempdir().unwrap();
        let model_path = tmp.path().join("example.gguf");
        fs::write(&model_path, b"stub").unwrap();

        let inventory = ModelInventory::new(vec![tmp.path().to_path_buf()], None);
        inventory.refresh().unwrap();
        let entries = inventory.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, model_path);
    }

    #[test]
    fn manifest_records_are_loaded() {
        let tmp = tempdir().unwrap();
        let model = tmp.path().join("model.safetensors");
        fs::write(&model, b"stub").unwrap();
        let manifest = tmp.path().join("manifest.json");
        let json = json!([{
            "name": "custom",
            "path": model,
            "format": "safetensors",
            "size_bytes": 4
        }]);
        fs::write(&manifest, json.to_string()).unwrap();

        let inventory = ModelInventory::new(vec![], Some(manifest));
        inventory.refresh().unwrap();
        let entries = inventory.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].identifier, "custom");
        assert_eq!(entries[0].format.as_deref(), Some("safetensors"));
        assert_eq!(entries[0].size_bytes, Some(4));
    }
}
