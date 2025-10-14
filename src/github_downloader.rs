use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::future::Future;
use tokio::fs;

/// Downloads mods from GitHub repositories
pub struct GitHubDownloader {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl GitHubDownloader {
    /// Create a new GitHub downloader
    pub fn new(cache_dir: PathBuf) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("infinite-d2rmm-cli")
            .build()
            .unwrap();

        Self { client, cache_dir }
    }

    /// Download a mod from GitHub
    /// Returns the local path where the mod was downloaded
    pub async fn download(
        &self,
        repo: &str,
        subdir: Option<&str>,
        branch: Option<&str>,
    ) -> Result<PathBuf> {
        let branch = branch.unwrap_or("main");

        // Create cache directory structure: cache_dir/owner/repo/branch/subdir
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid repo format: {}", repo);
        }

        let owner = parts[0];
        let repo_name = parts[1];

        let mut target_dir = self.cache_dir.join(owner).join(repo_name).join(branch);
        if let Some(subdir) = subdir {
            target_dir = target_dir.join(subdir);
        }

        // Check if already downloaded
        if target_dir.exists() {
            tracing::info!("Using cached mod from: {}", target_dir.display());
            return Ok(target_dir);
        }

        tracing::info!("Downloading from GitHub: {}/{} (branch: {})", owner, repo_name, branch);
        if let Some(subdir) = subdir {
            tracing::info!("  Subdirectory: {}", subdir);
        }

        // Download using GitHub API
        let base_path = subdir.unwrap_or("");
        self.download_directory(owner, repo_name, branch, base_path, &target_dir)
            .await
            .context("Failed to download from GitHub")?;

        Ok(target_dir)
    }

    /// Download a directory from GitHub using the API
    fn download_directory<'a>(
        &'a self,
        owner: &'a str,
        repo: &'a str,
        branch: &'a str,
        path: &'a str,
        target_dir: &'a Path,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
        // Use GitHub Contents API
        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}?ref={}",
            owner, repo, path, branch
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch from GitHub API")?;

        if !response.status().is_success() {
            anyhow::bail!(
                "GitHub API request failed with status {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            );
        }

        let items: Vec<GitHubContentItem> = response
            .json()
            .await
            .context("Failed to parse GitHub API response")?;

        // Create target directory
        fs::create_dir_all(target_dir)
            .await
            .context("Failed to create target directory")?;

        // Download each item
        for item in items {
            let item_path = target_dir.join(&item.name);

            match item.item_type.as_str() {
                "file" => {
                    // Download file content
                    if let Some(download_url) = item.download_url {
                        tracing::debug!("Downloading file: {}", item.name);
                        let content = self
                            .client
                            .get(&download_url)
                            .send()
                            .await
                            .context("Failed to download file")?
                            .bytes()
                            .await
                            .context("Failed to read file content")?;

                        fs::write(&item_path, content)
                            .await
                            .context("Failed to write file")?;
                    }
                }
                "dir" => {
                    // Recursively download subdirectory
                    tracing::debug!("Downloading directory: {}", item.name);
                    self.download_directory(owner, repo, branch, &item.path, &item_path)
                        .await?;
                }
                _ => {
                    tracing::debug!("Skipping item type: {}", item.item_type);
                }
            }
        }

        Ok(())
        })
    }

    /// Clear the download cache
    pub async fn clear_cache(&self) -> Result<()> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir)
                .await
                .context("Failed to clear cache")?;
        }
        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
struct GitHubContentItem {
    name: String,
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    download_url: Option<String>,
}
