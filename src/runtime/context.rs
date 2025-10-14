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

        // Try to extract file if needed
        fm.extract_if_needed(file_path, &self.game_path, &self.output_path)
            .await?;

        // Read the file
        let full_path = self.output_path.join(file_path);
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

        let full_path = self.output_path.join(file_path);

        JsonHandler::write(&full_path, &data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write JSON file '{}': {}", file_path, e))?;

        // Record the write operation
        let mut fm = self.file_manager.write().await;
        fm.record_write(file_path, &self.mod_id);

        Ok(())
    }

    /// Read a TSV file
    pub async fn read_tsv(&self, file_path: &str) -> Result<Vec<Vec<String>>> {
        let mut fm = self.file_manager.write().await;

        // Try to extract file if needed
        fm.extract_if_needed(file_path, &self.game_path, &self.output_path)
            .await?;

        // Read the file
        let full_path = self.output_path.join(file_path);
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

        let full_path = self.output_path.join(file_path);

        TsvHandler::write(&full_path, &data)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write TSV file '{}': {}", file_path, e))?;

        // Record the write operation
        let mut fm = self.file_manager.write().await;
        fm.record_write(file_path, &self.mod_id);

        Ok(())
    }

    /// Read a text file
    pub async fn read_txt(&self, file_path: &str) -> Result<String> {
        let mut fm = self.file_manager.write().await;

        // Try to extract file if needed
        fm.extract_if_needed(file_path, &self.game_path, &self.output_path)
            .await?;

        // Read the file
        let full_path = self.output_path.join(file_path);
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

        let full_path = self.output_path.join(file_path);

        TextHandler::write(&full_path, content)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to write text file '{}': {}", file_path, e))?;

        // Record the write operation
        let mut fm = self.file_manager.write().await;
        fm.record_write(file_path, &self.mod_id);

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
}
