//! CASC 存档操作封装

use anyhow::{Context, Result};
use casclib::Storage;
use std::path::{Path, PathBuf};
use tracing::{info, debug, warn};

/// CASC 错误类型
#[derive(Debug, thiserror::Error)]
pub enum CascError {
    #[error("Failed to open CASC storage: {0}")]
    OpenFailed(String),
    
    #[error("File not found in CASC: {0}")]
    FileNotFound(String),
    
    #[error("Failed to extract file: {0}")]
    ExtractionFailed(String),
    
    #[error("Invalid game path: {0}")]
    InvalidPath(String),
}

/// CASC 存档管理器
pub struct CascStorage {
    storage: Storage,
    game_path: PathBuf,
}

impl CascStorage {
    /// 打开 CASC 存档
    /// 
    /// # 参数
    /// * `game_path` - 游戏安装目录路径
    /// 
    /// # 示例
    /// ```no_run
    /// use infinite::casc::CascStorage;
    /// 
    /// let storage = CascStorage::open("C:\\Program Files (x86)\\Diablo II Resurrected")?;
    /// ```
    pub fn open<P: AsRef<Path>>(game_path: P) -> Result<Self> {
        let game_path = game_path.as_ref().to_path_buf();
        
        if !game_path.exists() {
            return Err(CascError::InvalidPath(
                format!("Game path does not exist: {}", game_path.display())
            ).into());
        }
        
        info!("Opening CASC storage at: {}", game_path.display());
        
        // D2R 的数据通常在 Data 子目录中
        let data_path = game_path.join("Data");
        let storage_path = if data_path.exists() {
            data_path
        } else {
            game_path.clone()
        };
        
        let storage = casclib::open(storage_path.to_str().ok_or_else(|| {
            CascError::InvalidPath("Path contains invalid UTF-8".to_string())
        })?)
        .map_err(|e| CascError::OpenFailed(format!("{:?}", e)))?;
        
        info!("CASC storage opened successfully");
        
        Ok(Self {
            storage,
            game_path,
        })
    }
    
    /// 检查文件是否存在于 CASC 存档中
    pub fn has_file<P: AsRef<Path>>(&self, path: P) -> bool {
        let path_str = path.as_ref().to_string_lossy();
        
        // 尝试多种路径格式
        // D2R CASC 使用 "data:data\\" 前缀
        let variations = vec![
            format!("data:data\\{}", path_str),  // D2R 标准格式
            format!("data:data/{}", path_str),    // 正斜杠版本
            path_str.to_string(),                 // 原始路径
            path_str.replace("/", "\\"),
            path_str.replace("\\", "/"),
        ];
        
        for variant in variations {
            debug!("Checking CASC file: {}", variant);
            // casclib API: storage.entry(path) returns FileEntry directly
            let entry = self.storage.entry(&variant);
            if entry.open().is_ok() {
                return true;
            }
        }
        
        false
    }
    
    /// 从 CASC 存档中提取文件
    /// 
    /// # 参数
    /// * `casc_path` - CASC 存档中的文件路径 (例如: "data\\global\\excel\\treasureclass.json")
    /// * `output_path` - 输出文件路径
    /// 
    /// # 返回
    /// 成功时返回提取的字节数
    pub fn extract_file<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        casc_path: P,
        output_path: Q,
    ) -> Result<usize> {
        let casc_path = casc_path.as_ref();
        let output_path = output_path.as_ref();
        let casc_path_str = casc_path.to_string_lossy();
        
        debug!("Extracting file: {} -> {}", casc_path_str, output_path.display());
        
        // 尝试多种路径格式
        // D2R CASC 使用 "data:data\\" 前缀
        let variations = vec![
            format!("data:data\\{}", casc_path_str),  // D2R 标准格式
            format!("data:data/{}", casc_path_str),    // 正斜杠版本
            casc_path_str.to_string(),                 // 原始路径
            casc_path_str.replace("/", "\\"),
            casc_path_str.replace("\\", "/"),
        ];
        
        let mut last_error = None;
        
        for variant in variations {
            debug!("Trying CASC path variant: {}", variant);
            
            let entry = self.storage.entry(&variant);
            match entry.open() {
                Ok(file) => {
                    info!("✓ Found file in CASC: {}", variant);
                    
                    // 创建输出目录
                    if let Some(parent) = output_path.parent() {
                        std::fs::create_dir_all(parent)
                            .context("Failed to create output directory")?;
                    }
                    
                    // 提取文件
                    let mut writer = std::fs::File::create(output_path)
                        .context("Failed to create output file")?;
                    
                    file.extract(&mut writer)
                        .map_err(|e| CascError::ExtractionFailed(format!("{:?}", e)))?;
                    
                    let file_size = output_path.metadata()?.len() as usize;
                    
                    info!(
                        "Extracted: {} ({} bytes) -> {}",
                        casc_path_str,
                        file_size,
                        output_path.display()
                    );
                    
                    return Ok(file_size);
                }
                Err(e) => {
                    last_error = Some(format!("{:?}", e));
                }
            }
        }
        
        Err(CascError::FileNotFound(format!(
            "{} (last error: {})",
            casc_path_str,
            last_error.unwrap_or_else(|| "unknown".to_string())
        )).into())
    }
    
    /// 提取文件到内存
    /// 
    /// # 参数
    /// * `casc_path` - CASC 存档中的文件路径
    /// 
    /// # 返回
    /// 文件内容的字节数组
    pub fn extract_to_memory<P: AsRef<Path>>(&self, casc_path: P) -> Result<Vec<u8>> {
        let casc_path = casc_path.as_ref();
        let casc_path_str = casc_path.to_string_lossy();
        
        debug!("Extracting to memory: {}", casc_path_str);
        
        // 尝试多种路径格式
        // D2R CASC 使用 "data:data\\" 前缀
        let variations = vec![
            format!("data:data\\{}", casc_path_str),  // D2R 标准格式
            format!("data:data/{}", casc_path_str),    // 正斜杠版本
            casc_path_str.to_string(),                 // 原始路径
            casc_path_str.replace("/", "\\"),
            casc_path_str.replace("\\", "/"),
        ];
        
        let mut last_error = None;
        
        for variant in variations {
            let entry = self.storage.entry(&variant);
            match entry.open() {
                Ok(file) => {
                    let mut buffer = Vec::new();
                    
                    file.extract(&mut buffer)
                        .map_err(|e| CascError::ExtractionFailed(format!("{:?}", e)))?;
                    
                    info!(
                        "Extracted to memory: {} ({} bytes)",
                        casc_path_str,
                        buffer.len()
                    );
                    
                    return Ok(buffer);
                }
                Err(e) => {
                    last_error = Some(format!("{:?}", e));
                }
            }
        }
        
        Err(CascError::FileNotFound(format!(
            "{} (last error: {})",
            casc_path_str,
            last_error.unwrap_or_else(|| "unknown".to_string())
        )).into())
    }
    
    /// 列出所有文件
    /// 
    /// # 返回
    /// 所有文件路径的迭代器
    pub fn list_files(&self) -> Result<Vec<String>> {
        // TODO: 实现文件列表功能
        // casclib API 的 files() 方法需要进一步研究
        warn!("list_files() not yet fully implemented");
        Ok(Vec::new())
    }
    
    /// 获取游戏路径
    pub fn game_path(&self) -> &Path {
        &self.game_path
    }
}

#[cfg(test)]
mod tests {
    // Tests will be added as we understand the casclib API better
    
    #[test]
    fn test_path_normalization() {
        // 测试路径格式转换
        let paths = vec![
            "data/global/excel/treasureclass.json",
            "data\\global\\excel\\treasureclass.json",
        ];
        
        for path in paths {
            let forward = path.replace("\\", "/");
            let backward = path.replace("/", "\\");
            assert!(forward.contains("/") || backward.contains("\\"));
        }
    }
}
