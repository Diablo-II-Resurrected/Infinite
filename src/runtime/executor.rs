use crate::mod_manager::LoadedMod;
use crate::runtime::{Context, RuntimeFactory, ScriptServices, ScriptRuntime};
use anyhow::Result;
use std::sync::Arc;

/// Executor for running mod scripts (Lua or JavaScript)
pub struct ModExecutor;

impl ModExecutor {
    /// Execute a mod's script using the appropriate runtime
    pub async fn execute_mod(mod_data: &LoadedMod, context: Arc<Context>) -> Result<()> {
        // Create script services from context
        let services = ScriptServices::from_context(context.clone());

        // Create appropriate runtime (Lua or JavaScript) based on mod files
        let mut runtime = RuntimeFactory::create_runtime(&mod_data.path, services)?;

        // Setup API
        runtime.setup_api()?;

        // Setup config
        runtime.setup_config(&mod_data.user_config)?;

        // Execute the script
        runtime.execute()?;

        // Cleanup
        runtime.cleanup()?;

        Ok(())
    }
}
