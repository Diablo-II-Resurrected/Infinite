use crate::runtime::Context;
use mlua::prelude::*;
use std::sync::Arc;

/// Infinite API exposed to Lua
pub struct InfiniteApi {
    context: Arc<Context>,
}

impl InfiniteApi {
    /// Create a new infinite API
    pub fn new(context: Arc<Context>) -> Self {
        Self { context }
    }

    /// Register infinite global table in Lua
    pub fn register_globals(&self, lua: &Lua) -> LuaResult<()> {
        let infinite = lua.create_table()?;

        // getVersion()
        let ctx = self.context.clone();
        infinite.set(
            "getVersion",
            lua.create_function(move |_, ()| Ok(ctx.get_version()))?,
        )?;

        // getFullVersion()
        let ctx = self.context.clone();
        infinite.set(
            "getFullVersion",
            lua.create_function(move |lua, ()| {
                let (major, minor, patch) = ctx.get_full_version();
                let table = lua.create_table()?;
                table.set(1, major)?;
                table.set(2, minor)?;
                table.set(3, patch)?;
                Ok(table)
            })?,
        )?;

        // readJson(path)
        let ctx = self.context.clone();
        infinite.set(
            "readJson",
            lua.create_async_function(move |lua, path: String| {
                let ctx = ctx.clone();
                async move {
                    let value = ctx
                        .read_json(&path)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;

                    lua.to_value(&value)
                }
            })?,
        )?;

        // writeJson(path, data)
        let ctx = self.context.clone();
        infinite.set(
            "writeJson",
            lua.create_async_function(move |lua, (path, data): (String, LuaValue)| {
                let ctx = ctx.clone();
                async move {
                    let json_value: serde_json::Value = lua.from_value(data)?;
                    ctx.write_json(&path, json_value)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                    Ok(())
                }
            })?,
        )?;

        // readTsv(path)
        let ctx = self.context.clone();
        infinite.set(
            "readTsv",
            lua.create_async_function(move |lua, path: String| {
                let ctx = ctx.clone();
                async move {
                    let rows = ctx
                        .read_tsv(&path)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;

                    // Convert to Lua table
                    let table = lua.create_table()?;
                    for (i, row) in rows.iter().enumerate() {
                        let row_table = lua.create_table()?;
                        for (j, cell) in row.iter().enumerate() {
                            row_table.set(j + 1, cell.as_str())?;
                        }
                        table.set(i + 1, row_table)?;
                    }

                    Ok(table)
                }
            })?,
        )?;

        // writeTsv(path, data)
        let ctx = self.context.clone();
        infinite.set(
            "writeTsv",
            lua.create_async_function(move |_lua, (path, data): (String, LuaTable)| {
                let ctx = ctx.clone();
                async move {
                    // Convert Lua table to Vec<Vec<String>>
                    let mut rows = Vec::new();
                    for pair in data.pairs::<usize, LuaTable>() {
                        let (_, row_table) = pair?;
                        let mut row = Vec::new();
                        for cell_pair in row_table.pairs::<usize, String>() {
                            let (_, cell) = cell_pair?;
                            row.push(cell);
                        }
                        rows.push(row);
                    }

                    ctx.write_tsv(&path, rows)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                    Ok(())
                }
            })?,
        )?;

        // readTxt(path)
        let ctx = self.context.clone();
        infinite.set(
            "readTxt",
            lua.create_async_function(move |_, path: String| {
                let ctx = ctx.clone();
                async move {
                    ctx.read_txt(&path)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))
                }
            })?,
        )?;

        // writeTxt(path, content)
        let ctx = self.context.clone();
        infinite.set(
            "writeTxt",
            lua.create_async_function(move |_, (path, content): (String, String)| {
                let ctx = ctx.clone();
                async move {
                    ctx.write_txt(&path, &content)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                    Ok(())
                }
            })?,
        )?;

        // copyFile(src, dst, overwrite?)
        let ctx = self.context.clone();
        infinite.set(
            "copyFile",
            lua.create_async_function(
                move |_, (src, dst, overwrite): (String, String, Option<bool>)| {
                    let ctx = ctx.clone();
                    async move {
                        ctx.copy_file(&src, &dst, overwrite.unwrap_or(false))
                            .await
                            .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                        Ok(())
                    }
                },
            )?,
        )?;

        // error(message)
        infinite.set(
            "error",
            lua.create_function(|_, msg: String| Err::<(), _>(LuaError::RuntimeError(msg)))?,
        )?;

        // Set infinite global
        lua.globals().set("infinite", infinite)?;

        Ok(())
    }

    /// Register console global table in Lua
    pub fn register_console(&self, lua: &Lua) -> LuaResult<()> {
        let console = lua.create_table()?;

        console.set(
            "log",
            lua.create_function(|_, args: LuaMultiValue| {
                let msg = format_lua_args(&args);
                tracing::info!("{}", msg);
                println!("[LOG] {}", msg);
                Ok(())
            })?,
        )?;

        console.set(
            "debug",
            lua.create_function(|_, args: LuaMultiValue| {
                let msg = format_lua_args(&args);
                tracing::debug!("{}", msg);
                println!("[DEBUG] {}", msg);
                Ok(())
            })?,
        )?;

        console.set(
            "warn",
            lua.create_function(|_, args: LuaMultiValue| {
                let msg = format_lua_args(&args);
                tracing::warn!("{}", msg);
                println!("[WARN] {}", msg);
                Ok(())
            })?,
        )?;

        console.set(
            "error",
            lua.create_function(|_, args: LuaMultiValue| {
                let msg = format_lua_args(&args);
                tracing::error!("{}", msg);
                eprintln!("[ERROR] {}", msg);
                Ok(())
            })?,
        )?;

        lua.globals().set("console", console)?;

        Ok(())
    }
}

/// Format Lua arguments for console output
fn format_lua_args(args: &LuaMultiValue) -> String {
    args.iter()
        .map(|v| match v {
            LuaValue::String(s) => s.to_str().unwrap_or("").to_string(),
            LuaValue::Integer(i) => i.to_string(),
            LuaValue::Number(n) => n.to_string(),
            LuaValue::Boolean(b) => b.to_string(),
            LuaValue::Nil => "nil".to_string(),
            _ => format!("{:?}", v),
        })
        .collect::<Vec<_>>()
        .join(" ")
}
