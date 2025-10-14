use crate::lua_api::InfiniteApi;
use crate::mod_manager::LoadedMod;
use crate::runtime::Context;
use anyhow::Result;
use mlua::prelude::*;
use std::sync::Arc;

/// Executor for running mod Lua scripts
pub struct ModExecutor {
    lua: Lua,
}

impl ModExecutor {
    /// Create a new mod executor
    pub fn new() -> Result<Self> {
        let lua = Lua::new();

        // Disable dangerous functions for security
        lua.load(
            r#"
            -- Disable dangerous functions
            os.execute = nil
            os.remove = nil
            os.rename = nil
            io = nil
            loadfile = nil
            dofile = nil
            require = nil
            package = nil
        "#,
        )
        .exec()
        .map_err(|e| anyhow::anyhow!("Failed to disable dangerous functions: {}", e))?;

        Ok(Self { lua })
    }

    /// Execute a mod's Lua script
    pub async fn execute_mod(&self, mod_data: &LoadedMod, context: Arc<Context>) -> Result<()> {
        // Register infinite API
        let api = InfiniteApi::new(context.clone());
        api.register_globals(&self.lua)
            .map_err(|e| anyhow::anyhow!("Failed to register infinite API: {}", e))?;

        // Register console API
        api.register_console(&self.lua)
            .map_err(|e| anyhow::anyhow!("Failed to register console API: {}", e))?;

        // Register config global variable
        let config_value = self
            .lua
            .to_value(&mod_data.user_config)
            .map_err(|e| anyhow::anyhow!("Failed to convert config to Lua value: {}", e))?;
        self.lua
            .globals()
            .set("config", config_value)
            .map_err(|e| anyhow::anyhow!("Failed to set config global: {}", e))?;

        // Read mod.lua
        let lua_path = mod_data.path.join("mod.lua");
        let lua_code = tokio::fs::read_to_string(&lua_path)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read mod.lua: {}", e))?;

        // Execute Lua code
        self.lua
            .load(&lua_code)
            .set_name(format!("{}/mod.lua", mod_data.id))
            .exec_async()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to execute mod.lua: {}", e))?;

        Ok(())
    }
}

impl Default for ModExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create ModExecutor")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system::FileManager;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_execute_simple_mod() {
        let temp_dir = TempDir::new().unwrap();
        let mod_dir = temp_dir.path().join("TestMod");
        std::fs::create_dir(&mod_dir).unwrap();

        // Create mod.json
        let mod_json = r#"
        {
            "name": "Test Mod",
            "version": "1.0"
        }
        "#;
        std::fs::write(mod_dir.join("mod.json"), mod_json).unwrap();

        // Create mod.lua
        let mod_lua = r#"
        console.log("Hello from Lua!")
        local version = infinite.getVersion()
        if version < 1.0 then
            infinite.error("Version too old!")
        end
        "#;
        std::fs::write(mod_dir.join("mod.lua"), mod_lua).unwrap();

        // Load mod
        let loader = crate::mod_manager::ModLoader::new(temp_dir.path());
        let mod_data = loader.load_mod(&mod_dir).unwrap();

        // Create context
        let context = Arc::new(Context {
            mod_id: mod_data.id.clone(),
            mod_path: mod_data.path.clone(),
            config: serde_json::json!({}),
            file_manager: Arc::new(RwLock::new(FileManager::new())),
            game_path: PathBuf::from("."),
            output_path: temp_dir.path().to_path_buf(),
            dry_run: true,
        });

        // Execute mod
        let executor = ModExecutor::new().unwrap();
        let result = executor.execute_mod(&mod_data, context).await;

        assert!(result.is_ok());
    }
}
