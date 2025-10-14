use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
}

impl ScriptServices {
    pub fn new(mod_path: PathBuf, output_path: PathBuf, game_path: PathBuf) -> Self {
        Self {
            mod_path,
            output_path,
            game_path,
        }
    }

    /// 读取 JSON 文件
    pub fn read_json(&self, path: &str) -> Result<JsonValue> {
        let full_path = self.resolve_path(path);
        let content = std::fs::read_to_string(&full_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// 写入 JSON 文件
    pub fn write_json(&self, path: &str, data: &JsonValue) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(data)?;
        std::fs::write(&full_path, content)?;
        Ok(())
    }

    /// 读取 TSV 文件
    pub fn read_tsv(&self, path: &str) -> Result<TsvData> {
        let full_path = self.resolve_path(path);
        TsvData::from_file(&full_path)
    }

    /// 写入 TSV 文件
    pub fn write_tsv(&self, path: &str, data: &TsvData) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        data.write_to_file(&full_path)
    }

    /// 读取文本文件
    pub fn read_txt(&self, path: &str) -> Result<String> {
        let full_path = self.resolve_path(path);
        Ok(std::fs::read_to_string(&full_path)?)
    }

    /// 写入文本文件
    pub fn write_txt(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
        Ok(())
    }

    /// 复制文件
    pub fn copy_file(&self, src: &str, dst: &str, overwrite: bool) -> Result<()> {
        let src_path = self.mod_path.join(src);
        let dst_path = self.output_path.join(dst);

        if !overwrite && dst_path.exists() {
            return Ok(());
        }

        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }

        Ok(())
    }

    /// 解析路径（从游戏目录或输出目录读取）
    fn resolve_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");

        // 先尝试输出目录
        let output_path = self.output_path.join(&normalized);
        if output_path.exists() {
            return output_path;
        }

        // 再尝试游戏目录
        self.game_path.join(&normalized)
    }

    /// 解析输出路径
    fn resolve_output_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");
        self.output_path.join(&normalized)
    }
}

/// TSV 数据结构
#[derive(Debug, Clone)]
pub struct TsvData {
    pub headers: Vec<String>,
    pub rows: Vec<TsvRow>,
}

#[derive(Debug, Clone)]
pub struct TsvRow {
    pub data: HashMap<String, String>,
}

impl TsvData {
    pub fn from_file(path: &Path) -> Result<Self> {
        // Read file content
        let content = std::fs::read_to_string(path)?;
        
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
                let mut data = HashMap::new();
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

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
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

/// 用户配置
#[derive(Debug, Clone)]
pub struct UserConfig {
    pub values: HashMap<String, ConfigValue>,
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    Bool(bool),
    Number(f64),
    String(String),
}
