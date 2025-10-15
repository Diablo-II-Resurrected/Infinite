use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// Re-export UserConfig from mod_manager
pub use crate::mod_manager::config::UserConfig;

// Re-export TSV types from api
pub use super::api::{TsvData, TsvRow};

/// Unified script runtime interface
pub trait ScriptRuntime {
    /// 设置 API（注入全局对象和函数）
    fn setup_api(&mut self) -> Result<()>;

    /// 设置用户配置
    fn setup_config(&mut self, config: &UserConfig) -> Result<()>;

    /// 执行脚本
    fn execute(&mut self) -> Result<()>;

    /// 清理资源
    fn cleanup(&mut self) -> Result<()>;

    /// 获取运行时类型
    fn runtime_type(&self) -> ScriptType;
}

/// 脚本类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptType {
    Lua,
    JavaScript,
}

impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptType::Lua => write!(f, "Lua"),
            ScriptType::JavaScript => write!(f, "JavaScript"),
        }
    }
}

/// 脚本服务 - 提供给所有运行时的核心功能
#[derive(Clone)]
pub struct ScriptServices {
    pub mod_path: PathBuf,
    pub output_path: PathBuf,
    pub game_path: PathBuf,
    pub file_manager: std::sync::Arc<tokio::sync::RwLock<crate::file_system::FileManager>>,
}

impl ScriptServices {
    pub fn new(
        mod_path: PathBuf,
        output_path: PathBuf,
        game_path: PathBuf,
        file_manager: std::sync::Arc<tokio::sync::RwLock<crate::file_system::FileManager>>,
    ) -> Self {
        Self {
            mod_path,
            output_path,
            game_path,
            file_manager,
        }
    }

    /// Create services from execution context
    pub fn from_context(context: std::sync::Arc<super::Context>) -> Self {
        Self {
            mod_path: context.mod_path.clone(),
            output_path: context.output_path.clone(),
            game_path: context.game_path.clone(),
            file_manager: context.file_manager.clone(),
        }
    }

    /// 读取 JSON 文件
    pub fn read_json(&self, path: &str) -> Result<JsonValue> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();

        // Use block_in_place to run async code in a sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut fm = file_manager.write().await;

                // Try to read from cache first
                if let Ok(content) = fm.read_file_with_cache(&path, "script").await {
                    let value = crate::handlers::JsonHandler::parse_from_bytes(&content)
                        .map_err(|e| anyhow::anyhow!("Failed to parse JSON: {}", e))?;
                    return Ok(value);
                }

                // Extract from CASC if needed
                let full_path = fm.ensure_extracted(&path, "script").await?;

                // Read the file
                let value = crate::handlers::JsonHandler::read(&full_path).await
                    .map_err(|e| anyhow::anyhow!("Failed to read JSON: {}", e))?;

                fm.record_read(&path, "script");
                Ok(value)
            })
        })
    }

    /// 写入 JSON 文件
    pub fn write_json(&self, path: &str, data: &JsonValue) -> Result<()> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();
        let data = data.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let content = crate::handlers::JsonHandler::to_bytes(&data)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize JSON: {}", e))?;

                let mut fm = file_manager.write().await;
                fm.write_file_to_cache(&path, content, "script");

                Ok(())
            })
        })
    }

    /// 读取 TSV 文件
    pub fn read_tsv(&self, path: &str) -> Result<TsvData> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut fm = file_manager.write().await;

                // Try to read from cache first
                if let Ok(content) = fm.read_file_with_cache(&path, "script").await {
                    let rows = crate::handlers::TsvHandler::parse_from_bytes(&content)?;
                    return Self::tsv_rows_to_data(rows);
                }

                // Extract from CASC if needed
                let full_path = fm.ensure_extracted(&path, "script").await?;

                // Read the file using TsvHandler
                let rows = crate::handlers::TsvHandler::read(&full_path).await?;

                fm.record_read(&path, "script");
                Self::tsv_rows_to_data(rows)
            })
        })
    }

    // Helper to convert TSV rows (Vec<Vec<String>>) to TsvData
    fn tsv_rows_to_data(rows: Vec<Vec<String>>) -> Result<TsvData> {
        if rows.is_empty() {
            return Ok(TsvData {
                headers: vec![],
                rows: vec![],
            });
        }

        // First row is headers
        let headers = rows[0].clone();

        // Remaining rows are data
        let data_rows: Vec<TsvRow> = rows
            .iter()
            .skip(1)
            .map(|row| {
                let mut data = HashMap::new();
                for (i, value) in row.iter().enumerate() {
                    if i < headers.len() {
                        data.insert(headers[i].clone(), value.clone());
                    }
                }
                TsvRow { data }
            })
            .collect();

        Ok(TsvData {
            headers,
            rows: data_rows,
        })
    }

    /// 写入 TSV 文件
    pub fn write_tsv(&self, path: &str, data: &TsvData) -> Result<()> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();
        let data = data.clone();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                // Convert TsvData back to Vec<Vec<String>>
                let mut rows = vec![data.headers.clone()];

                for row in &data.rows {
                    let mut row_vec = Vec::new();
                    for header in &data.headers {
                        row_vec.push(row.data.get(header).cloned().unwrap_or_default());
                    }
                    rows.push(row_vec);
                }

                // Convert to TSV string manually
                let content = rows.iter()
                    .map(|row| {
                        row.iter()
                            .map(|field| {
                                // Quote fields containing commas
                                if field.contains(',') {
                                    format!("\"{}\"", field)
                                } else {
                                    field.clone()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join("\t")
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                let mut fm = file_manager.write().await;
                fm.write_file_to_cache(&path, content.into_bytes(), "script");

                Ok(())
            })
        })
    }

    /// 读取文本文件
    pub fn read_txt(&self, path: &str) -> Result<String> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut fm = file_manager.write().await;

                // Try to read from cache first
                if let Ok(content) = fm.read_file_with_cache(&path, "script").await {
                    return String::from_utf8(content)
                        .map_err(|e| anyhow::anyhow!("Invalid UTF-8: {}", e));
                }

                // Extract from CASC if needed
                let full_path = fm.ensure_extracted(&path, "script").await?;

                // Read the file
                let content = crate::handlers::TextHandler::read(&full_path).await?;

                fm.record_read(&path, "script");
                Ok(content)
            })
        })
    }

    /// 写入文本文件
    pub fn write_txt(&self, path: &str, content: &str) -> Result<()> {
        let file_manager = self.file_manager.clone();
        let path = path.to_string();
        let content = content.to_string();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut fm = file_manager.write().await;
                fm.write_file_to_cache(&path, content.as_bytes().to_vec(), "script");
                Ok(())
            })
        })
    }

    /// 复制文件或目录
    pub fn copy_file(&self, src: &str, dst: &str, _overwrite: bool) -> Result<()> {
        let file_manager = self.file_manager.clone();
        let src = src.to_string();
        let dst = dst.to_string();

        // D2RMM's copyFile can copy directories from the mod folder
        // Source is relative to mod folder, destination is relative to output
        let mod_base = self.mod_path.clone();
        let output_base = self.output_path.clone();

        let src_path = mod_base.join(&src);
        let dst_path = output_base.join(&dst);

        tracing::debug!("copyFile: {} -> {}", src_path.display(), dst_path.display());

        if src_path.is_dir() {
            // Copy entire directory recursively
            tracing::debug!("Copying directory recursively");
            copy_dir_recursive(&src_path, &dst_path)?;
        } else if src_path.is_file() {
            // Copy single file
            tracing::debug!("Copying single file");
            if let Some(parent) = dst_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src_path, &dst_path)?;
        } else {
            // Maybe it's a CASC file path?
            let result: Result<()> = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    let mut fm = file_manager.write().await;

                    // Read source file (may extract from CASC)
                    let content = if let Ok(cached) = fm.read_file_with_cache(&src, "script").await {
                        cached
                    } else {
                        let full_path = fm.ensure_extracted(&src, "script").await?;
                        tokio::fs::read(&full_path).await?
                    };

                    // Write to destination in cache
                    fm.write_file_to_cache(&dst, content, "script");

                    Ok(())
                })
            });
            result?;
        }

        Ok(())
    }

    /// 解析路径（从游戏目录或输出目录读取）
    fn resolve_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");

        tracing::debug!("Resolving path: {} -> {}", path, normalized);
        tracing::debug!("Output path: {:?}", self.output_path);
        tracing::debug!("Game path: {:?}", self.game_path);

        // 先尝试输出目录
        let output_path = self.output_path.join(&normalized);
        tracing::debug!("Checking output_path: {:?}", output_path);
        if output_path.exists() {
            tracing::debug!("Found in output path");
            return output_path;
        }

        // 再尝试游戏目录
        let game_full_path = self.game_path.join(&normalized);
        tracing::debug!("Checking game_path: {:?}", game_full_path);
        game_full_path
    }

    /// 解析输出路径
    fn resolve_output_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");
        self.output_path.join(&normalized)
    }
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}
