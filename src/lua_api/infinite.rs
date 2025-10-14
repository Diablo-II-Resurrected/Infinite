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
        // D2R TSV 文件的第一行是标题,需要转换为字典格式
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

                    if rows.is_empty() {
                        return Ok(lua.create_table()?);
                    }

                    // 第一行是标题
                    let headers = &rows[0];
                    
                    // 转换为 Lua 字典数组
                    let table = lua.create_table()?;
                    for (i, row) in rows.iter().enumerate().skip(1) {
                        let row_table = lua.create_table()?;
                        
                        // 使用列名作为键
                        for (j, cell) in row.iter().enumerate() {
                            if j < headers.len() {
                                let header = &headers[j];
                                row_table.set(header.as_str(), cell.as_str())?;
                            }
                            // 同时保留数字索引以兼容旧代码
                            row_table.set(j + 1, cell.as_str())?;
                        }
                        
                        table.set(i, row_table)?;  // i 从 1 开始 (跳过标题行)
                    }
                    
                    // 保存标题到特殊字段 __headers__ 以便写回时使用
                    let headers_table = lua.create_table()?;
                    for (i, header) in headers.iter().enumerate() {
                        headers_table.set(i + 1, header.as_str())?;
                    }
                    table.set("__headers__", headers_table)?;

                    Ok(table)
                }
            })?,
        )?;

        // writeTsv(path, data)
        // D2R TSV 文件需要标题行,从 __headers__ 字段读取
        let ctx = self.context.clone();
        infinite.set(
            "writeTsv",
            lua.create_async_function(move |_lua, (path, data): (String, LuaTable)| {
                let ctx = ctx.clone();
                async move {
                    let mut rows = Vec::new();
                    
                    // 提取标题
                    let headers: Option<LuaTable> = data.get("__headers__").ok();
                    if let Some(headers_table) = headers {
                        let mut header_row = Vec::new();
                        for pair in headers_table.pairs::<usize, String>() {
                            let (_, header) = pair?;
                            header_row.push(header);
                        }
                        if !header_row.is_empty() {
                            rows.push(header_row);
                        }
                    }
                    
                    // 转换数据行 (跳过 __headers__)
                    let mut data_indices: Vec<usize> = Vec::new();
                    for key in data.clone().pairs::<mlua::Value, mlua::Value>() {
                        let (k, _) = key?;
                        if let mlua::Value::Integer(i) = k {
                            if i > 0 {
                                data_indices.push(i as usize);
                            }
                        }
                    }
                    data_indices.sort();
                    
                    for idx in data_indices {
                        let row_table: LuaTable = data.get(idx)?;
                        let mut row = Vec::new();
                        
                        // 使用数字索引读取
                        let mut col_idx = 1;
                        loop {
                            match row_table.get::<usize, String>(col_idx) {
                                Ok(cell) => {
                                    row.push(cell);
                                    col_idx += 1;
                                }
                                Err(_) => break,
                            }
                        }
                        
                        if !row.is_empty() {
                            rows.push(row);
                        }
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
        
        // extractFile(path) - Extract file from CASC if not already extracted
        let ctx = self.context.clone();
        infinite.set(
            "extractFile",
            lua.create_async_function(move |_, path: String| {
                let ctx = ctx.clone();
                async move {
                    ctx.extract_file(&path)
                        .await
                        .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                    Ok(())
                }
            })?,
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
