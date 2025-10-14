use super::script_runtime::*;
use anyhow::{bail, Result};
use std::path::Path;

/// Script runtime factory
pub struct RuntimeFactory;

impl RuntimeFactory {
    /// Automatically create corresponding runtime based on mod directory
    pub fn create_runtime(
        mod_path: &Path,
        services: ScriptServices,
    ) -> Result<Box<dyn ScriptRuntime>> {
        let lua_script = mod_path.join("mod.lua");
        let js_script = mod_path.join("mod.js");

        if lua_script.exists() {
            tracing::info!("Detected Lua script: {}", lua_script.display());
            Ok(Box::new(super::lua_runtime::LuaScriptRuntime::new(
                mod_path, services,
            )?))
        } else if js_script.exists() {
            #[cfg(feature = "js-runtime")]
            {
                tracing::info!("Detected JavaScript script: {}", js_script.display());
                Ok(Box::new(super::js_runtime::JavaScriptRuntime::new(
                    mod_path, services,
                )?))
            }
            #[cfg(not(feature = "js-runtime"))]
            {
                bail!(
                    "JavaScript runtime not enabled. Recompile with --features js-runtime to use mod.js files.\nFound: {}",
                    js_script.display()
                );
            }
        } else {
            bail!("No mod.lua or mod.js found in {:?}", mod_path);
        }
    }

    /// Explicitly create Lua runtime
    #[allow(dead_code)]
    pub fn create_lua_runtime(
        mod_path: &Path,
        services: ScriptServices,
    ) -> Result<Box<dyn ScriptRuntime>> {
        Ok(Box::new(super::lua_runtime::LuaScriptRuntime::new(
            mod_path, services,
        )?))
    }

    /// Explicitly create JavaScript runtime
    #[cfg(feature = "js-runtime")]
    #[allow(dead_code)]
    pub fn create_js_runtime(
        mod_path: &Path,
        services: ScriptServices,
    ) -> Result<Box<dyn ScriptRuntime>> {
        Ok(Box::new(super::js_runtime::JavaScriptRuntime::new(
            mod_path, services,
        )?))
    }
}
