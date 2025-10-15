use super::script_runtime::*;
use anyhow::Result;
use mlua::{Lua, Table, Value as LuaValue};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct LuaScriptRuntime {
    lua: Lua,
    mod_path: PathBuf,
    services: Arc<ScriptServices>,
}

impl LuaScriptRuntime {
    pub fn new(mod_path: &Path, services: ScriptServices) -> Result<Self> {
        let lua = Lua::new();

        Ok(Self {
            lua,
            mod_path: mod_path.to_path_buf(),
            services: Arc::new(services),
        })
    }

    /// 将 JSON 值转换为 Lua 值
    fn json_to_lua(&self, json: &serde_json::Value) -> Result<LuaValue> {
        use serde_json::Value as JsonValue;

        let lua_val = match json {
            JsonValue::Null => LuaValue::Nil,
            JsonValue::Bool(b) => LuaValue::Boolean(*b),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    LuaValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    LuaValue::Number(f)
                } else {
                    LuaValue::Nil
                }
            }
            JsonValue::String(s) => LuaValue::String(self.lua.create_string(s)?),
            JsonValue::Array(arr) => {
                let table = self.lua.create_table()?;
                for (i, item) in arr.iter().enumerate() {
                    table.set(i + 1, self.json_to_lua(item)?)?;
                }
                LuaValue::Table(table)
            }
            JsonValue::Object(obj) => {
                let table = self.lua.create_table()?;
                for (key, value) in obj {
                    table.set(key.as_str(), self.json_to_lua(value)?)?;
                }
                LuaValue::Table(table)
            }
        };

        Ok(lua_val)
    }

    /// 将 Lua 值转换为 JSON 值
    fn lua_to_json(&self, lua_val: LuaValue) -> Result<serde_json::Value> {
        use serde_json::Value as JsonValue;

        let json = match lua_val {
            LuaValue::Nil => JsonValue::Null,
            LuaValue::Boolean(b) => JsonValue::Bool(b),
            LuaValue::Integer(i) => JsonValue::Number(i.into()),
            LuaValue::Number(n) => {
                JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))
            }
            LuaValue::String(s) => JsonValue::String(s.to_str()?.to_string()),
            LuaValue::Table(table) => {
                // 检查是否是数组
                if table.clone().pairs::<i64, LuaValue>().count() > 0 {
                    // 尝试作为数组
                    let mut arr = Vec::new();
                    for i in 1.. {
                        match table.get::<_, LuaValue>(i) {
                            Ok(LuaValue::Nil) => break,
                            Ok(val) => arr.push(self.lua_to_json(val)?),
                            Err(_) => break,
                        }
                    }
                    if !arr.is_empty() {
                        return Ok(JsonValue::Array(arr));
                    }
                }

                // 作为对象
                let mut obj = serde_json::Map::new();
                for pair in table.pairs::<String, LuaValue>() {
                    let (key, value) = pair?;
                    obj.insert(key, self.lua_to_json(value)?);
                }
                JsonValue::Object(obj)
            }
            _ => JsonValue::Null,
        };

        Ok(json)
    }

    /// 将 TSV 数据转换为 Lua 表
    fn tsv_to_lua<'lua>(&'lua self, tsv: &TsvData) -> Result<Table<'lua>> {
        let table = self.lua.create_table()?;

        // headers
        let headers_table = self.lua.create_table()?;
        for (i, header) in tsv.headers.iter().enumerate() {
            headers_table.set(i + 1, header.as_str())?;
        }
        table.set("headers", headers_table)?;

        // rows
        let rows_table = self.lua.create_table()?;
        for (i, row) in tsv.rows.iter().enumerate() {
            let row_table = self.lua.create_table()?;
            for (key, value) in &row.data {
                row_table.set(key.as_str(), value.as_str())?;
            }
            rows_table.set(i + 1, row_table)?;
        }
        table.set("rows", rows_table)?;

        Ok(table)
    }

    /// 将 Lua 表转换为 TSV 数据
    fn lua_to_tsv(&self, table: Table) -> Result<TsvData> {
        let headers: Vec<String> = table.get::<_, Table>("headers")?
            .sequence_values::<String>()
            .collect::<Result<_, _>>()?;

        let rows_table: Table = table.get("rows")?;
        let mut rows = Vec::new();

        for pair in rows_table.pairs::<i64, Table>() {
            let (_, row_table) = pair?;
            let mut data = std::collections::HashMap::new();

            for pair in row_table.pairs::<String, String>() {
                let (key, value) = pair?;
                data.insert(key, value);
            }

            rows.push(TsvRow { data });
        }

        Ok(TsvData { headers, rows })
    }
}

impl ScriptRuntime for LuaScriptRuntime {
    fn setup_api(&mut self) -> Result<()> {
        let globals = self.lua.globals();

        // Create infinite table with basic API
        let infinite = self.lua.create_table()?;
        infinite.set("getVersion", 1.5)?;
        globals.set("infinite", infinite)?;

        // Create console table
        let console = self.lua.create_table()?;
        let log_fn = self.lua.create_function(|_, msg: String| {
            tracing::info!("[Lua] {}", msg);
            Ok(())
        })?;
        console.set("log", log_fn.clone())?;
        console.set("debug", log_fn.clone())?;
        console.set("warn", log_fn.clone())?;
        console.set("error", log_fn)?;
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
