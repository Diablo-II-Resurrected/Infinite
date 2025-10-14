use crate::file_system::FileManager;
use crate::handlers::{JsonHandler, TextHandler, TsvHandler};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Execution context for a mod
pub struct Context {
    /// Mod identifier
    pub mod_id: String,

    /// Path to the mod directory
    pub mod_path: PathBuf,

    /// User configuration for the mod
    pub config: serde_json::Value,

    /// Shared file manager
    pub file_manager: Arc<RwLock<FileManager>>,

    /// Path to the game directory
    pub game_path: PathBuf,

    /// Path to the output directory
    pub output_path: PathBuf,

    /// Whether this is a dry run (don't write files)
    pub dry_run: bool,
}

impl Context {
    /// Get infinite version
    pub fn get_version(&self) -> f64 {
        1.5
    }

    /// Get full infinite version
    pub fn get_full_version(&self) -> (u32, u32, u32) {
        (1, 5, 0)
    }

    /// Read a JSON file
    pub async fn read_json(&self, file_path: &str) -> Result<serde_json::Value> {
        let mut fm = self.file_manager.write().await;

        // Try to read from cache first (for chained modifications)
        if let Ok(content) = fm.read_file_with_cache(file_path, &self.mod_id).await {
            // Parse from cached content
            let value = JsonHandler::parse_from_bytes(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse cached JSON '{}': {}", file_path, e))?;
            return Ok(value);
        }

        // Extract file from CASC if needed
        let full_path = fm.ensure_extracted(file_path, &self.mod_id).await?;

        // Read the file
        let value = JsonHandler::read(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read JSON file '{}': {}", file_path, e))?;

        // Record the read operation
        fm.record_read(file_path, &self.mod_id);

        Ok(value)
    }

    /// Write a JSON file
    pub async fn write_json(&self, file_path: &str, data: serde_json::Value) -> Result<()> {
        if self.dry_run {
            tracing::info!("[DRY RUN] Would write JSON: {}", file_path);
            return Ok(());
        }

        // Write to cache instead of directly to disk
        let content = JsonHandler::to_bytes(&data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON '{}': {}", file_path, e))?;

        let mut fm = self.file_manager.write().await;
        fm.write_file_to_cache(file_path, content, &self.mod_id);

        Ok(())
    }

    /// Read a TSV file
    pub async fn read_tsv(&self, file_path: &str) -> Result<Vec<Vec<String>>> {
        let mut fm = self.file_manager.write().await;

        // Try to read from cache first (for chained modifications)
        if let Ok(content) = fm.read_file_with_cache(file_path, &self.mod_id).await {
            // Parse from cached content
            let rows = TsvHandler::parse_from_bytes(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse cached TSV '{}': {}", file_path, e))?;
            return Ok(rows);
        }

        // Extract file from CASC if needed
        let full_path = fm.ensure_extracted(file_path, &self.mod_id).await?;

        // Read the file
        let rows = TsvHandler::read(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read TSV file '{}': {}", file_path, e))?;

        // Record the read operation
        fm.record_read(file_path, &self.mod_id);

        Ok(rows)
    }

    /// Write a TSV file
    pub async fn write_tsv(&self, file_path: &str, data: Vec<Vec<String>>) -> Result<()> {
        if self.dry_run {
            tracing::info!("[DRY RUN] Would write TSV: {}", file_path);
            return Ok(());
        }

        // Write to cache instead of directly to disk
        let content = TsvHandler::to_bytes(&data)
            .map_err(|e| anyhow::anyhow!("Failed to serialize TSV '{}': {}", file_path, e))?;

        let mut fm = self.file_manager.write().await;
        fm.write_file_to_cache(file_path, content, &self.mod_id);

        Ok(())
    }

    /// Read a text file
    pub async fn read_txt(&self, file_path: &str) -> Result<String> {
        let mut fm = self.file_manager.write().await;

        // Try to read from cache first (for chained modifications)
        if let Ok(content) = fm.read_file_with_cache(file_path, &self.mod_id).await {
            // Parse from cached content
            let text = String::from_utf8(content)
                .map_err(|e| anyhow::anyhow!("Failed to parse cached text '{}' as UTF-8: {}", file_path, e))?;
            return Ok(text);
        }

        // Extract file from CASC if needed
        let full_path = fm.ensure_extracted(file_path, &self.mod_id).await?;

        // Read the file
        let content = TextHandler::read(&full_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read text file '{}': {}", file_path, e))?;

        // Record the read operation
        fm.record_read(file_path, &self.mod_id);

        Ok(content)
    }

    /// Write a text file
    pub async fn write_txt(&self, file_path: &str, content: &str) -> Result<()> {
        if self.dry_run {
            tracing::info!("[DRY RUN] Would write text: {}", file_path);
            return Ok(());
        }

        // Write to cache instead of directly to disk
        let bytes = content.as_bytes().to_vec();

        let mut fm = self.file_manager.write().await;
        fm.write_file_to_cache(file_path, bytes, &self.mod_id);

        Ok(())
    }

    /// Copy a file from mod directory to output
    pub async fn copy_file(&self, src: &str, dst: &str, overwrite: bool) -> Result<()> {
        if self.dry_run {
            tracing::info!("[DRY RUN] Would copy: {} -> {}", src, dst);
            return Ok(());
        }

        let src_path = self.mod_path.join(src);
        let dst_path = self.output_path.join(dst);

        // Check if destination exists
        if !overwrite && tokio::fs::try_exists(&dst_path).await? {
            tracing::debug!("Skipping copy (file exists): {}", dst);
            return Ok(());
        }

        // Create parent directory
        if let Some(parent) = dst_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Copy the file
        tokio::fs::copy(&src_path, &dst_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to copy file '{}' -> '{}': {}", src, dst, e))?;

        // Record the write operation
        let mut fm = self.file_manager.write().await;
        fm.record_write(dst, &self.mod_id);

        Ok(())
    }

    /// Extract a file from CASC storage
    /// This ensures the file is available for reading
    pub async fn extract_file(&self, file_path: &str) -> Result<()> {
        if self.dry_run {
            tracing::info!("[DRY RUN] Would extract: {}", file_path);
            return Ok(());
        }

        let mut fm = self.file_manager.write().await;

        // Use the ensure_extracted method from FileManager
        let extracted_path = fm.ensure_extracted(file_path, &self.mod_id).await?;

        tracing::info!("Extracted: {} -> {}", file_path, extracted_path.display());

        Ok(())
    }
}
