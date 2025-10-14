use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Represents a source for a mod
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModSource {
    /// Local file system path
    #[serde(rename = "local")]
    Local { path: PathBuf },

    /// GitHub repository
    #[serde(rename = "github")]
    GitHub {
        /// Repository in format "owner/repo"
        repo: String,
        /// Optional subdirectory within the repo
        #[serde(skip_serializing_if = "Option::is_none")]
        subdir: Option<String>,
        /// Optional branch name (defaults to "main")
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
    },
}

impl ModSource {
    /// Parse a mod source from a string
    ///
    /// Formats:
    /// - Local path: `path/to/mod` or `C:\path\to\mod`
    /// - GitHub: `github:owner/repo` or `github:owner/repo@branch` or `github:owner/repo:subdir` or `github:owner/repo:subdir@branch`
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.starts_with("github:") {
            Self::parse_github(&s[7..])
        } else {
            Ok(Self::Local {
                path: PathBuf::from(s),
            })
        }
    }

    fn parse_github(s: &str) -> Result<Self> {
        // Format: owner/repo[:subdir][@branch]
        let (repo_part, branch) = if let Some(pos) = s.rfind('@') {
            (&s[..pos], Some(s[pos + 1..].to_string()))
        } else {
            (s, None)
        };

        let (repo, subdir) = if let Some(pos) = repo_part.find(':') {
            (
                repo_part[..pos].to_string(),
                Some(repo_part[pos + 1..].to_string()),
            )
        } else {
            (repo_part.to_string(), None)
        };

        // Validate repo format
        if !repo.contains('/') {
            anyhow::bail!("GitHub repo must be in format 'owner/repo', got: {}", repo);
        }

        Ok(Self::GitHub {
            repo,
            subdir,
            branch,
        })
    }
}

/// Represents a list of mod sources to install
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModList {
    pub sources: Vec<ModSource>,
}

impl ModList {
    /// Load a mod list from a text file
    /// Each line is a mod source (local path or GitHub URL)
    /// Lines starting with # are comments
    /// Empty lines are ignored
    pub async fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .await
            .context("Failed to read mod list file")?;

        let mut sources = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            match ModSource::parse(line) {
                Ok(source) => sources.push(source),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to parse line {}: {} - {}",
                        line_num + 1,
                        line,
                        e
                    );
                }
            }
        }

        Ok(Self { sources })
    }

    /// Create a mod list from a vector of sources
    pub fn from_sources(sources: Vec<ModSource>) -> Self {
        Self { sources }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_local() {
        let source = ModSource::parse("./mods/my_mod").unwrap();
        match source {
            ModSource::Local { path } => {
                assert_eq!(path, PathBuf::from("./mods/my_mod"));
            }
            _ => panic!("Expected Local source"),
        }
    }

    #[test]
    fn test_parse_github_simple() {
        let source = ModSource::parse("github:user/repo").unwrap();
        match source {
            ModSource::GitHub { repo, subdir, branch } => {
                assert_eq!(repo, "user/repo");
                assert_eq!(subdir, None);
                assert_eq!(branch, None);
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn test_parse_github_with_subdir() {
        let source = ModSource::parse("github:user/repo:mods/my_mod").unwrap();
        match source {
            ModSource::GitHub { repo, subdir, branch } => {
                assert_eq!(repo, "user/repo");
                assert_eq!(subdir, Some("mods/my_mod".to_string()));
                assert_eq!(branch, None);
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn test_parse_github_with_branch() {
        let source = ModSource::parse("github:user/repo@dev").unwrap();
        match source {
            ModSource::GitHub { repo, subdir, branch } => {
                assert_eq!(repo, "user/repo");
                assert_eq!(subdir, None);
                assert_eq!(branch, Some("dev".to_string()));
            }
            _ => panic!("Expected GitHub source"),
        }
    }

    #[test]
    fn test_parse_github_full() {
        let source = ModSource::parse("github:user/repo:mods/my_mod@dev").unwrap();
        match source {
            ModSource::GitHub { repo, subdir, branch } => {
                assert_eq!(repo, "user/repo");
                assert_eq!(subdir, Some("mods/my_mod".to_string()));
                assert_eq!(branch, Some("dev".to_string()));
            }
            _ => panic!("Expected GitHub source"),
        }
    }
}
