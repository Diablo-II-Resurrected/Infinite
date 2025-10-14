use anyhow::{Context, Result};
use std::path::Path;

/// Handler for plain text files
pub struct TextHandler;

impl TextHandler {
    /// Read a text file
    pub async fn read(path: &Path) -> Result<String> {
        tokio::fs::read_to_string(path)
            .await
            .context("Failed to read text file")
    }

    /// Write a text file
    pub async fn write(path: &Path, content: &str) -> Result<()> {
        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create parent directory")?;
        }

        tokio::fs::write(path, content)
            .await
            .context("Failed to write text file")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_text_read_write() {
        let temp_dir = TempDir::new().unwrap();
        let text_path = temp_dir.path().join("test.txt");

        let content = "Hello, World!\nThis is a test.";

        TextHandler::write(&text_path, content).await.unwrap();
        let read_content = TextHandler::read(&text_path).await.unwrap();

        assert_eq!(content, read_content);
    }
}
