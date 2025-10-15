/// Unified Infinite API implementation
///
/// This module provides the core API logic that can be used by both
/// JavaScript and Lua runtimes. Each runtime is responsible for:
/// 1. Converting between native types (JS/Lua) and Rust types
/// 2. Wrapping these functions with the appropriate runtime bindings
use super::script_runtime::ScriptServices;
use anyhow::Result;
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Core Infinite API implementation
///
/// All methods are synchronous and use block_in_place for async operations.
/// This allows them to be called from both sync (Lua) and async contexts.
pub struct InfiniteApiCore {
    services: Arc<ScriptServices>,
}

impl InfiniteApiCore {
    pub fn new(services: Arc<ScriptServices>) -> Self {
        Self { services }
    }

    /// Get Infinite version (for compatibility)
    pub fn get_version(&self) -> f64 {
        1.5
    }

    /// Read JSON file
    ///
    /// Returns a serde_json::Value that can be converted to the target type
    pub fn read_json(&self, path: &str) -> Result<JsonValue> {
        tracing::debug!("readJson called with path: {}", path);
        let result = self.services.read_json(path);
        if let Err(ref e) = result {
            tracing::error!("readJson error: {}", e);
        } else {
            tracing::debug!("JSON loaded successfully");
        }
        result
    }

    /// Write JSON file
    ///
    /// Accepts a serde_json::Value converted from the target type
    pub fn write_json(&self, path: &str, data: &JsonValue) -> Result<()> {
        tracing::debug!("writeJson called with path: {}", path);
        self.services.write_json(path, data)
    }

    /// Read TSV file
    ///
    /// Returns TSV data structure with headers and rows
    pub fn read_tsv(&self, path: &str) -> Result<TsvData> {
        tracing::debug!("readTsv called with path: {}", path);
        let tsv = self.services.read_tsv(path);
        if let Ok(ref data) = tsv {
            tracing::debug!("TSV loaded: {} headers, {} rows", data.headers.len(), data.rows.len());
        } else if let Err(ref e) = tsv {
            tracing::error!("readTsv error: {}", e);
        }
        tsv
    }

    /// Write TSV file
    ///
    /// Accepts TSV data structure with headers and rows
    pub fn write_tsv(&self, path: &str, data: &TsvData) -> Result<()> {
        tracing::debug!("writeTsv called with path: {}", path);
        self.services.write_tsv(path, data)
    }

    /// Read text file
    pub fn read_txt(&self, path: &str) -> Result<String> {
        tracing::debug!("readTxt called with path: {}", path);
        self.services.read_txt(path)
    }

    /// Write text file
    pub fn write_txt(&self, path: &str, content: &str) -> Result<()> {
        tracing::debug!("writeTxt called with path: {}", path);
        self.services.write_txt(path, content)
    }

    /// Copy file (with optional directory support)
    pub fn copy_file(&self, src: &str, dst: &str, is_directory: bool) -> Result<()> {
        tracing::debug!("copyFile called: {} -> {} (is_dir: {})", src, dst, is_directory);
        self.services.copy_file(src, dst, is_directory)
    }

    /// Throw an error (for Infinite.error())
    ///
    /// This should be converted to the appropriate error type by each runtime
    pub fn throw_error(&self, msg: &str) -> Result<()> {
        tracing::error!("[MOD ERROR] {}", msg);
        Err(anyhow::anyhow!(msg.to_string()))
    }
}

/// TSV data structure used by both runtimes
#[derive(Debug, Clone)]
pub struct TsvData {
    pub headers: Vec<String>,
    pub rows: Vec<TsvRow>,
}

#[derive(Debug, Clone)]
pub struct TsvRow {
    pub data: std::collections::HashMap<String, String>,
}

impl TsvData {
    pub fn from_file(path: &std::path::Path) -> Result<Self> {
        // Read file content
        let _content = std::fs::read_to_string(path)?;

        // Use async runtime to call the TSV handler
        let runtime = tokio::runtime::Runtime::new()?;
        let rows_data = runtime.block_on(async {
            crate::handlers::tsv::TsvHandler::read(path).await
        })?;

        // First row is headers
        if rows_data.is_empty() {
            return Ok(Self {
                headers: Vec::new(),
                rows: Vec::new(),
            });
        }

        let headers = rows_data[0].clone();

        // Convert remaining rows to TsvRow format
        let rows = rows_data[1..]
            .iter()
            .map(|row| {
                let mut data = std::collections::HashMap::new();
                for (i, value) in row.iter().enumerate() {
                    if let Some(header) = headers.get(i) {
                        data.insert(header.clone(), value.clone());
                    }
                }
                TsvRow { data }
            })
            .collect();

        Ok(Self { headers, rows })
    }

    pub fn write_to_file(&self, path: &std::path::Path) -> Result<()> {
        // Convert back to TSV handler format
        let mut data: Vec<Vec<String>> = Vec::new();

        // First row is headers
        data.push(self.headers.clone());

        // 添加所有行数据
        for row in &self.rows {
            let row_data: Vec<String> = self
                .headers
                .iter()
                .map(|header| row.data.get(header).cloned().unwrap_or_default())
                .collect();
            data.push(row_data);
        }

        // 使用异步运行时执行异步写入
        let runtime = tokio::runtime::Runtime::new()?;
        runtime.block_on(async {
            crate::handlers::tsv::TsvHandler::write(path, &data).await
        })?;

        Ok(())
    }
}


/// Console logging functions
pub struct ConsoleApi;

impl ConsoleApi {
    pub fn log(msg: &str) {
        tracing::info!("[MOD] {}", msg);
    }

    pub fn debug(msg: &str) {
        tracing::debug!("[MOD] {}", msg);
    }

    pub fn warn(msg: &str) {
        tracing::warn!("[MOD] {}", msg);
    }

    pub fn error(msg: &str) {
        tracing::error!("[MOD] {}", msg);
    }
}
