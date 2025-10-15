use super::script_runtime::*;
use super::api::{InfiniteApiCore, ConsoleApi};
use anyhow::Result;
use mlua::{Lua, Table, Value as LuaValue};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LuaScriptRuntime {
    lua: Lua,
    mod_path: PathBuf,
    api_core: Arc<InfiniteApiCore>,
}

impl LuaScriptRuntime {
    pub fn new(mod_path: &Path, services: ScriptServices) -> Result<Self> {
        let lua = Lua::new();
        let services_arc = Arc::new(services);
        let api_core = Arc::new(InfiniteApiCore::new(services_arc));

        Ok(Self {
            lua,
            mod_path: mod_path.to_path_buf(),
            api_core,
        })
    }
}

impl ScriptRuntime for LuaScriptRuntime {
    fn setup_api(&mut self) -> Result<()> {
        let globals = self.lua.globals();

        // Create D2RMM table with full API
        let d2rmm = self.lua.create_table()?;

        // Set version
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("getVersion", self.lua.create_function(move |_, ()| {
            Ok(api_core.get_version())
        })?)?;

        // Register readJson
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("readJson", self.lua.create_function(move |lua, path: String| {
            let json = api_core.read_json(&path)
                .map_err(|e| mlua::Error::external(e))?;
            json_to_lua_value(lua, &json)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register writeJson
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("writeJson", self.lua.create_function(move |lua, (path, data): (String, LuaValue)| {
            let json = lua_value_to_json(lua, data)
                .map_err(|e| mlua::Error::external(e))?;
            api_core.write_json(&path, &json)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register readTsv
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("readTsv", self.lua.create_function(move |lua, path: String| {
            let tsv = api_core.read_tsv(&path)
                .map_err(|e| mlua::Error::external(e))?;

            // Convert TSV to Lua table
            let table = lua.create_table()?;

            // headers
            let headers_table = lua.create_table()?;
            for (i, header) in tsv.headers.iter().enumerate() {
                headers_table.set(i + 1, header.as_str())?;
            }
            table.set("headers", headers_table)?;

            // rows
            let rows_table = lua.create_table()?;
            for (i, row) in tsv.rows.iter().enumerate() {
                let row_table = lua.create_table()?;
                for (key, value) in &row.data {
                    row_table.set(key.as_str(), value.as_str())?;
                }
                rows_table.set(i + 1, row_table)?;
            }
            table.set("rows", rows_table)?;

            Ok(table)
        })?)?;

        // Register writeTsv
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("writeTsv", self.lua.create_function(move |_lua, (path, data): (String, Table)| {
            let headers: Vec<String> = data.get::<_, Table>("headers")?
                .sequence_values::<String>()
                .collect::<Result<_, _>>()?;

            let rows_table: Table = data.get("rows")?;
            let mut rows = Vec::new();

            for pair in rows_table.pairs::<i64, Table>() {
                let (_, row_table) = pair?;
                let mut row_data = std::collections::HashMap::new();

                for pair in row_table.pairs::<String, String>() {
                    let (key, value) = pair?;
                    row_data.insert(key, value);
                }

                rows.push(TsvRow { data: row_data });
            }

            let tsv = TsvData { headers, rows };
            api_core.write_tsv(&path, &tsv)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register readTxt
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("readTxt", self.lua.create_function(move |_lua, path: String| {
            api_core.read_txt(&path)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register writeTxt
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("writeTxt", self.lua.create_function(move |_lua, (path, content): (String, String)| {
            api_core.write_txt(&path, &content)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register copyFile
        let api_core = Arc::clone(&self.api_core);
        d2rmm.set("copyFile", self.lua.create_function(move |_lua, (src, dst): (String, String)| {
            api_core.copy_file(&src, &dst, false)
                .map_err(|e| mlua::Error::external(e))
        })?)?;

        // Register error function
        d2rmm.set("error", self.lua.create_function(|_lua, msg: String| {
            tracing::error!("[Lua MOD ERROR] {}", msg);
            Err::<(), _>(mlua::Error::RuntimeError(msg))
        })?)?;

        globals.set("D2RMM", d2rmm.clone())?;
        // Also set as "infinite" for compatibility
        globals.set("infinite", d2rmm)?;

        // Create console table
        let console = self.lua.create_table()?;
        console.set("log", self.lua.create_function(|_, msg: String| {
            ConsoleApi::log(&msg);
            Ok(())
        })?)?;
        console.set("debug", self.lua.create_function(|_, msg: String| {
            ConsoleApi::debug(&msg);
            Ok(())
        })?)?;
        console.set("warn", self.lua.create_function(|_, msg: String| {
            ConsoleApi::warn(&msg);
            Ok(())
        })?)?;
        console.set("error", self.lua.create_function(|_, msg: String| {
            ConsoleApi::error(&msg);
            Ok(())
        })?)?;
        globals.set("console", console)?;

        Ok(())
    }

    fn setup_config(&mut self, config: &UserConfig) -> Result<()> {
        let globals = self.lua.globals();
        let config_table = self.lua.create_table()?;

        // Convert HashMap<String, serde_json::Value> to Lua table
        for (key, value) in config {
            let lua_value = json_to_lua_value(&self.lua, value)?;
            config_table.set(key.as_str(), lua_value)?;
        }

        globals.set("config", config_table)?;
        Ok(())
    }

    fn execute(&mut self) -> Result<()> {
        let script_path = self.mod_path.join("mod.lua");
        let script = std::fs::read_to_string(&script_path)?;

        self.lua.load(&script).set_name("mod.lua").exec()?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        // Lua handles cleanup automatically through RAII
        Ok(())
    }

    fn runtime_type(&self) -> ScriptType {
        ScriptType::Lua
    }
}

// Helper function to convert serde_json::Value to mlua::Value
fn json_to_lua_value<'lua>(lua: &'lua Lua, json: &serde_json::Value) -> Result<LuaValue<'lua>> {
    use serde_json::Value as JV;

    Ok(match json {
        JV::Null => LuaValue::Nil,
        JV::Bool(b) => LuaValue::Boolean(*b),
        JV::Number(n) => {
            if let Some(i) = n.as_i64() {
                LuaValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                LuaValue::Number(f)
            } else {
                LuaValue::Nil
            }
        }
        JV::String(s) => LuaValue::String(lua.create_string(s)?),
        JV::Array(arr) => {
            let table = lua.create_table()?;
            for (i, item) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua_value(lua, item)?)?;
            }
            LuaValue::Table(table)
        }
        JV::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj {
                table.set(k.as_str(), json_to_lua_value(lua, v)?)?;
            }
            LuaValue::Table(table)
        }
    })
}

// Helper function to convert mlua::Value to serde_json::Value
fn lua_value_to_json<'lua>(lua: &'lua Lua, val: LuaValue<'lua>) -> Result<serde_json::Value> {
    use serde_json::Value as JV;

    Ok(match val {
        LuaValue::Nil => JV::Null,
        LuaValue::Boolean(b) => JV::Bool(b),
        LuaValue::Integer(i) => JV::Number(i.into()),
        LuaValue::Number(n) => {
            JV::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
        }
        LuaValue::String(s) => JV::String(s.to_str()?.to_string()),
        LuaValue::Table(table) => {
            // Check if it's an array (sequential integer keys starting from 1)
            let mut is_array = true;
            let mut max_idx = 0;
            for pair in table.clone().pairs::<LuaValue, LuaValue>() {
                let (key, _) = pair?;
                if let LuaValue::Integer(i) = key {
                    if i > 0 {
                        max_idx = max_idx.max(i);
                    } else {
                        is_array = false;
                        break;
                    }
                } else {
                    is_array = false;
                    break;
                }
            }

            if is_array && max_idx > 0 {
                // It's an array
                let mut arr = Vec::new();
                for i in 1..=max_idx {
                    match table.get::<_, LuaValue>(i) {
                        Ok(LuaValue::Nil) => break,
                        Ok(val) => arr.push(lua_value_to_json(lua, val)?),
                        Err(_) => break,
                    }
                }
                JV::Array(arr)
            } else {
                // It's an object
                let mut obj = serde_json::Map::new();
                for pair in table.pairs::<String, LuaValue>() {
                    let (key, value) = pair?;
                    obj.insert(key, lua_value_to_json(lua, value)?);
                }
                JV::Object(obj)
            }
        }
        _ => JV::Null,
    })
}
