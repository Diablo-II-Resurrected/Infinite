use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::casc::CascStorage;
use anyhow::Result;

/// In-memory cache of file contents
#[derive(Debug, Clone)]
pub struct CachedFile {
    /// File content as bytes
    pub content: Vec<u8>,
    /// Whether this is the latest version
    pub dirty: bool,
}

/// Type of file operation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileOperationType {
    /// File was extracted from game data
    Extract,
    /// File was read by a mod
    Read,
    /// File was written/modified by a mod
    Write,
}

/// A single file operation record
#[derive(Debug, Clone)]
pub struct FileOperation {
    /// Type of operation
    pub op_type: FileOperationType,
    /// ID of the mod that performed the operation
    pub mod_id: String,
}

/// Status and history of a file
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// Whether the file currently exists
    pub exists: bool,
    /// Whether the file has been extracted from game data
    pub extracted: bool,
    /// Normalized file path
    pub file_path: String,
    /// Whether this is a game file (true) or mod file (false)
    pub game_file: Option<bool>,
    /// Whether the file has been modified
    pub modified: bool,
    /// History of operations on this file
    pub operations: Vec<FileOperation>,
}

/// File manager that tracks all file operations
pub struct FileManager {
    files: HashMap<String, FileStatus>,
    casc_storage: Option<Arc<CascStorage>>,
    output_path: Option<PathBuf>,
    game_path: Option<PathBuf>,
    /// In-memory cache of file contents for chaining modifications
    file_cache: HashMap<String, CachedFile>,
}

impl FileManager {
    /// Create a new file manager
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            casc_storage: None,
            output_path: None,
            game_path: None,
            file_cache: HashMap::new(),
        }
    }

    /// Set the CASC storage for extracting game files
    pub fn set_casc_storage(&mut self, storage: Arc<CascStorage>) {
        self.casc_storage = Some(storage);
    }

    /// Set the game path
    pub fn set_game_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.game_path = Some(path.into());
    }

    /// Set the output path for extracted files
    pub fn set_output_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.output_path = Some(path.into());
    }

    /// Extract a file from CASC storage if needed
    /// Returns the path to the extracted file
    pub async fn ensure_extracted(&mut self, file_path: &str, mod_id: &str) -> Result<PathBuf> {
        let normalized = Self::normalize_path(file_path);

        // Check if already extracted
        if self.is_extracted(&normalized) {
            if let Some(output_path) = &self.output_path {
                let file_path = output_path.join(&normalized);
                if file_path.exists() {
                    return Ok(file_path);
                }
            }
        }

        // Extract from CASC
        if let Some(storage) = &self.casc_storage {
            if let Some(output_path) = &self.output_path {
                let dest_path = output_path.join(&normalized);

                // Create parent directory
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Extract file - use original path for CASC (not normalized)
                // CASC needs backslashes, not forward slashes
                storage.extract_file(file_path, &dest_path)?;

                // Record extraction
                self.record_extract(&normalized, mod_id);

                return Ok(dest_path);
            }
        }

        // If CASC is not available, try to read from game_path directly
        if let Some(game_path) = &self.game_path {
            if let Some(output_path) = &self.output_path {
                let source_path = game_path.join(&normalized);

                // Check if file exists in game directory
                if source_path.exists() {
                    let dest_path = output_path.join(&normalized);

                    // Create parent directory
                    if let Some(parent) = dest_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }

                    // Copy file from game directory to output
                    tokio::fs::copy(&source_path, &dest_path).await?;

                    // Record extraction
                    self.record_extract(&normalized, mod_id);

                    return Ok(dest_path);
                }
            }
        }

        Err(anyhow::anyhow!("CASC storage not configured and file not found in game directory: {}", file_path))
    }

    /// Get or create file status for a given path
    fn get_or_create(&mut self, file_path: &str) -> &mut FileStatus {
        let normalized_path = Self::normalize_path(file_path);

        self.files.entry(normalized_path.clone()).or_insert_with(|| FileStatus {
            exists: false,
            extracted: false,
            file_path: normalized_path,
            game_file: None,
            modified: false,
            operations: Vec::new(),
        })
    }

    /// Normalize a file path (lowercase, forward slashes)
    fn normalize_path(path: &str) -> String {
        path.replace('\\', "/").to_lowercase()
    }

    /// Check if a file has been extracted
    pub fn is_extracted(&self, file_path: &str) -> bool {
        let normalized = Self::normalize_path(file_path);
        self.files
            .get(&normalized)
            .map(|s| s.extracted)
            .unwrap_or(false)
    }

    /// Check if a file exists
    pub fn exists(&self, file_path: &str) -> bool {
        let normalized = Self::normalize_path(file_path);
        self.files
            .get(&normalized)
            .map(|s| s.exists)
            .unwrap_or(false)
    }

    /// Check if a file has been modified
    pub fn is_modified(&self, file_path: &str) -> bool {
        let normalized = Self::normalize_path(file_path);
        self.files
            .get(&normalized)
            .map(|s| s.modified)
            .unwrap_or(false)
    }

    /// Record that a file was extracted
    pub fn record_extract(&mut self, file_path: &str, mod_id: &str) {
        let status = self.get_or_create(file_path);
        status.extracted = true;
        status.exists = true;
        status.game_file = Some(true);
        status.operations.push(FileOperation {
            op_type: FileOperationType::Extract,
            mod_id: mod_id.to_string(),
        });

        tracing::debug!("Extracted: {} (by {})", file_path, mod_id);
    }

    /// Record that a file was read
    pub fn record_read(&mut self, file_path: &str, mod_id: &str) {
        let status = self.get_or_create(file_path);
        status.exists = true;
        status.operations.push(FileOperation {
            op_type: FileOperationType::Read,
            mod_id: mod_id.to_string(),
        });

        tracing::debug!("Read: {} (by {})", file_path, mod_id);
    }

    /// Record that a file was written
    pub fn record_write(&mut self, file_path: &str, mod_id: &str) {
        let status = self.get_or_create(file_path);
        status.exists = true;
        status.modified = true;
        status.operations.push(FileOperation {
            op_type: FileOperationType::Write,
            mod_id: mod_id.to_string(),
        });

        tracing::debug!("Wrote: {} (by {})", file_path, mod_id);
    }

    /// Get file status for a given path
    pub fn get_status(&self, file_path: &str) -> Option<&FileStatus> {
        let normalized = Self::normalize_path(file_path);
        self.files.get(&normalized)
    }

    /// Get all file statuses
    pub fn get_all_statuses(&self) -> impl Iterator<Item = &FileStatus> {
        self.files.values()
    }

    /// Get files modified by a specific mod
    pub fn get_files_modified_by(&self, mod_id: &str) -> Vec<&FileStatus> {
        self.files
            .values()
            .filter(|status| {
                status.operations.iter().any(|op| {
                    op.op_type == FileOperationType::Write && op.mod_id == mod_id
                })
            })
            .collect()
    }

    /// Check if file needs extraction
    pub async fn extract_if_needed(
        &mut self,
        file_path: &str,
        _game_path: &Path,
        output_path: &Path,
    ) -> anyhow::Result<()> {
        // If file already exists in output, don't extract
        if self.exists(file_path) {
            return Ok(());
        }

        let full_path = output_path.join(file_path);

        // Check if file physically exists
        if tokio::fs::try_exists(&full_path).await? {
            self.record_extract(file_path, "system");
            return Ok(());
        }

        // TODO: Implement actual CASC extraction here
        // For now, we assume files are pre-extracted
        tracing::warn!("File not found and CASC extraction not yet implemented: {}", file_path);

        Ok(())
    }

    /// Print a summary of file operations
    pub fn print_summary(&self) {
        let total_files = self.files.len();
        let modified_files = self.files.values().filter(|s| s.modified).count();
        let extracted_files = self.files.values().filter(|s| s.extracted).count();

        println!("\n📊 File Operations Summary:");
        println!("   Total files tracked: {}", total_files);
        println!("   Files extracted: {}", extracted_files);
        println!("   Files modified: {}", modified_files);
    }

    /// Read file content, preferring cached version if available
    /// This allows multiple mods to chain their modifications
    pub async fn read_file_with_cache(&mut self, file_path: &str, mod_id: &str) -> Result<Vec<u8>> {
        let normalized = Self::normalize_path(file_path);

        // Check if we have a cached (modified) version
        if let Some(cached) = self.file_cache.get(&normalized).cloned() {
            tracing::debug!("Reading cached version of: {} (for {})", file_path, mod_id);
            self.record_read(&normalized, mod_id);
            return Ok(cached.content);
        }

        // Otherwise, read from disk
        let output_path = self.output_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Output path not set"))?
            .clone();
        let full_path = output_path.join(&normalized);

        if !full_path.exists() {
            anyhow::bail!("File not found: {}", full_path.display());
        }

        let content = tokio::fs::read(&full_path).await?;
        self.record_read(&normalized, mod_id);

        Ok(content)
    }

    /// Write file content to cache (not to disk yet)
    /// This allows multiple mods to modify the same file
    pub fn write_file_to_cache(&mut self, file_path: &str, content: Vec<u8>, mod_id: &str) {
        let normalized = Self::normalize_path(file_path);

        self.file_cache.insert(normalized.clone(), CachedFile {
            content,
            dirty: true,
        });

        self.record_write(&normalized, mod_id);
        tracing::debug!("Cached write: {} (by {})", file_path, mod_id);
    }

    /// Flush all cached files to disk
    pub async fn flush_cache(&mut self) -> Result<()> {
        let output_path = self.output_path.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Output path not set"))?;

        for (file_path, cached) in self.file_cache.drain() {
            if cached.dirty {
                let full_path = output_path.join(&file_path);

                // Create parent directory
                if let Some(parent) = full_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                tokio::fs::write(&full_path, &cached.content).await?;
                tracing::info!("Flushed to disk: {}", file_path);
            }
        }

        Ok(())
    }

    /// Check if a file is in cache
    pub fn is_cached(&self, file_path: &str) -> bool {
        let normalized = Self::normalize_path(file_path);
        self.file_cache.contains_key(&normalized)
    }
}

impl Default for FileManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tracking() {
        let mut fm = FileManager::new();

        assert!(!fm.exists("test.json"));

        fm.record_extract("test.json", "mod1");
        assert!(fm.exists("test.json"));
        assert!(fm.is_extracted("test.json"));
        assert!(!fm.is_modified("test.json"));

        fm.record_write("test.json", "mod2");
        assert!(fm.is_modified("test.json"));

        let status = fm.get_status("test.json").unwrap();
        assert_eq!(status.operations.len(), 2);
    }

    #[test]
    fn test_path_normalization() {
        let mut fm = FileManager::new();

        fm.record_extract("Path\\To\\File.json", "mod1");

        assert!(fm.exists("path/to/file.json"));
        assert!(fm.exists("PATH\\TO\\FILE.JSON"));
    }
}
