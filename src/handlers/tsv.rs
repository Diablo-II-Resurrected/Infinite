use anyhow::{Context, Result};
use std::path::Path;
use std::collections::HashMap;

/// Handler for TSV (Tab-Separated Values) files
pub struct TsvHandler;

impl TsvHandler {
    /// Read a TSV file as a 2D array of strings
    pub async fn read(path: &Path) -> Result<Vec<Vec<String>>> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read TSV file")?;

        let mut reader = csv::ReaderBuilder::new()
            .delimiter(b'\t')
            .has_headers(false)
            .flexible(true) // Allow variable number of fields
            .quoting(true)  // 启用引号处理
            .double_quote(true)  // 支持双引号转义
            .from_reader(content.as_bytes());

        let mut rows = Vec::new();

        for result in reader.records() {
            let record = result.context("Failed to parse TSV record")?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            rows.push(row);
        }

        Ok(rows)
    }

    /// Write a TSV file from a 2D array of strings
    pub async fn write(path: &Path, data: &[Vec<String>]) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directory")?;
        }

        // D2R TSV 文件需要特殊处理:
        // - 包含逗号的字段需要用双引号包围
        // - 这是 D2R 游戏引擎的要求
        let mut content = String::new();
        
        for row in data {
            let formatted_row: Vec<String> = row
                .iter()
                .map(|field| {
                    // 如果字段包含逗号,用双引号包围
                    if field.contains(',') {
                        format!("\"{}\"", field)
                    } else {
                        field.clone()
                    }
                })
                .collect();
            
            content.push_str(&formatted_row.join("\t"));
            content.push('\n');
        }

        tokio::fs::write(path, content)
            .await
            .context("Failed to write TSV file")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tsv_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let tsv_path = temp_dir.path().join("test.tsv");

        let data = vec![
            vec!["Name".to_string(), "Age".to_string(), "City".to_string()],
            vec!["Alice".to_string(), "30".to_string(), "NYC".to_string()],
            vec!["Bob".to_string(), "25".to_string(), "LA".to_string()],
        ];

        TsvHandler::write(&tsv_path, &data).await.unwrap();
        let read_data = TsvHandler::read(&tsv_path).await.unwrap();

        assert_eq!(data, read_data);
    }
}
