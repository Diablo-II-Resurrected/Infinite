use super::config::{ModConfig, UserConfig};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// A loaded mod with all its metadata
#[derive(Debug, Clone)]
pub struct LoadedMod {
    /// Unique mod identifier (directory name)
    pub id: String,
    
    /// Path to the mod directory
    pub path: PathBuf,
    
    /// Parsed mod configuration
    pub config: ModConfig,
    
    /// User's configuration values
    pub user_config: UserConfig,
}

/// Mod loader responsible for discovering and loading mods
pub struct ModLoader {
    mods_dir: PathBuf,
}

impl ModLoader {
    /// Create a new mod loader
    pub fn new(mods_dir: impl AsRef<Path>) -> Self {
        Self {
            mods_dir: mods_dir.as_ref().to_path_buf(),
        }
    }

    /// Load all mods from the mods directory
    pub fn load_all(&self) -> Result<Vec<LoadedMod>> {
        if !self.mods_dir.exists() {
            anyhow::bail!("Mods directory does not exist: {:?}", self.mods_dir);
        }

        let mut mods = Vec::new();

        for entry in WalkDir::new(&self.mods_dir)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            match self.load_mod(entry.path()) {
                Ok(mod_data) => {
                    tracing::debug!("Loaded mod: {} v{}", mod_data.config.name, mod_data.config.version);
                    mods.push(mod_data);
                }
                Err(e) => {
                    tracing::warn!("Failed to load mod from {:?}: {}", entry.path(), e);
                }
            }
        }

        tracing::info!("Loaded {} mods", mods.len());
        Ok(mods)
    }

    /// Load a single mod from a directory
    pub fn load_mod(&self, mod_path: &Path) -> Result<LoadedMod> {
        let config_path = mod_path.join("mod.json");
        
        if !config_path.exists() {
            anyhow::bail!("mod.json not found in {:?}", mod_path);
        }

        let config_str = std::fs::read_to_string(&config_path)
            .context("Failed to read mod.json")?;

        let config: ModConfig = serde_json::from_str(&config_str)
            .context("Failed to parse mod.json")?;

        // Check if mod.lua exists
        let lua_path = mod_path.join("mod.lua");
        if !lua_path.exists() {
            anyhow::bail!("mod.lua not found in {:?}", mod_path);
        }

        let id = mod_path
            .file_name()
            .and_then(|s| s.to_str())
            .context("Invalid mod directory name")?
            .to_string();

        // Generate default user configuration
        let user_config = config.generate_default_config();

        Ok(LoadedMod {
            id,
            path: mod_path.to_path_buf(),
            config,
            user_config,
        })
    }

    /// Load a single mod by ID
    pub fn load_mod_by_id(&self, mod_id: &str) -> Result<LoadedMod> {
        let mod_path = self.mods_dir.join(mod_id);
        self.load_mod(&mod_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_mod() {
        let temp_dir = TempDir::new().unwrap();
        let mod_dir = temp_dir.path().join("TestMod");
        fs::create_dir(&mod_dir).unwrap();

        // Create mod.json
        let mod_json = r#"
        {
            "name": "Test Mod",
            "version": "1.0",
            "author": "Test"
        }
        "#;
        fs::write(mod_dir.join("mod.json"), mod_json).unwrap();

        // Create mod.lua
        fs::write(mod_dir.join("mod.lua"), "-- test").unwrap();

        let loader = ModLoader::new(temp_dir.path());
        let mod_data = loader.load_mod(&mod_dir).unwrap();

        assert_eq!(mod_data.id, "TestMod");
        assert_eq!(mod_data.config.name, "Test Mod");
        assert_eq!(mod_data.config.version, "1.0");
    }
}
