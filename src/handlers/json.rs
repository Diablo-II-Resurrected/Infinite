use anyhow::{Context, Result};
use std::path::Path;

/// Handler for JSON files
pub struct JsonHandler;

impl JsonHandler {
    /// Read a JSON file
    pub async fn read(path: &Path) -> Result<serde_json::Value> {
        let content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read JSON file")?;

        // D2R's JSON files may have UTF-8 BOM
        // Remove BOM if present
        let content = content.trim_start_matches('\u{FEFF}');

        // D2R's JSON files may have comments and other non-standard features
        // For now, we use standard JSON parsing
        // TODO: Implement lenient JSON parser for D2R compatibility
        let value: serde_json::Value = serde_json::from_str(content)
            .context("Failed to parse JSON")?;

        Ok(value)
    }

    /// Write a JSON file with pretty formatting
    pub async fn write(path: &Path, data: &serde_json::Value) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directory")?;
        }

        let content = serde_json::to_string_pretty(data)
            .context("Failed to serialize JSON")?;

        tokio::fs::write(path, content)
            .await
            .context("Failed to write JSON file")?;

        Ok(())
    }

    /// Parse JSON from bytes
    pub fn parse_from_bytes(content: &[u8]) -> Result<serde_json::Value> {
        let text = std::str::from_utf8(content)
            .context("Failed to decode UTF-8")?;

        // Remove BOM if present
        let text = text.trim_start_matches('\u{FEFF}');

        let value: serde_json::Value = serde_json::from_str(text)
            .context("Failed to parse JSON")?;

        Ok(value)
    }

    /// Convert JSON data to bytes
    pub fn to_bytes(data: &serde_json::Value) -> Result<Vec<u8>> {
        let content = serde_json::to_string_pretty(data)
            .context("Failed to serialize JSON")?;

        Ok(content.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_json_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("test.json");

        let data = serde_json::json!({
            "name": "test",
            "value": 42,
            "items": ["a", "b", "c"]
        });

        JsonHandler::write(&json_path, &data).await.unwrap();
        let read_data = JsonHandler::read(&json_path).await.unwrap();

        assert_eq!(data, read_data);
    }
}
