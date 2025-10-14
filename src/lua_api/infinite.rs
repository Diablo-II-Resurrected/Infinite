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

                    // 添加元表，支持 add() 方法
                    let metatable = lua.create_table()?;

                    // add() 方法：添加一个空行
                    // 注意：使用 create_function 而不是 create_method，因为我们需要手动处理 self
                    metatable.set(
                        "add",
                        lua.create_function(|lua, this: LuaTable| {
                            // 查找下一个索引
                            let mut next_idx = 1;
                            for pair in this.clone().pairs::<mlua::Value, mlua::Value>() {
                                let (k, _) = pair?;
                                if let mlua::Value::Integer(i) = k {
                                    if i > 0 {
                                        next_idx = next_idx.max(i + 1);
                                    }
                                }
                            }

                            // 创建空行，列数基于表头
                            let new_row = lua.create_table()?;
                            if let Ok(headers_table) = this.get::<_, LuaTable>("__headers__") {
                                // 只设置header键为空字符串，不设置数字索引
                                // 这样当用户通过header名称设置值时，writeTsv会正确读取
                                for pair in headers_table.pairs::<usize, String>() {
                                    let (_, header) = pair?;
                                    new_row.set(header.as_str(), "")?;
                                }
                            }

                            // 添加到表中 - 直接添加，不要clone
                            this.set(next_idx, new_row.clone())?;

                            // 返回新行和索引
                            Ok((new_row, next_idx))
                        })?,
                    )?;

                    // 设置 __index 为元表自身，使方法可以通过 tsv:add() 调用
                    metatable.set("__index", metatable.clone())?;

                    table.set_metatable(Some(metatable));

                    Ok(table)
                }
            })?,
        )?;

        // writeTsv(path, data)
        // D2R TSV 文件需要标题行,从 __headers__ 字段读取
        // 支持通过header名称或数字索引访问单元格
        let ctx = self.context.clone();
        infinite.set(
            "writeTsv",
            lua.create_async_function(move |_lua, (path, data): (String, LuaTable)| {
                let ctx = ctx.clone();
                async move {
                    let mut rows = Vec::new();

                    // 提取标题并计算列数
                    let headers: Option<LuaTable> = data.get("__headers__").ok();
                    let (header_row, num_columns) = if let Some(headers_table) = headers {
                        let mut header_row = Vec::new();
                        for pair in headers_table.pairs::<usize, String>() {
                            let (_, header) = pair?;
                            header_row.push(header);
                        }
                        let col_count = header_row.len();
                        (header_row, col_count)
                    } else {
                        (Vec::new(), 0)
                    };

                    // 写入标题行
                    if !header_row.is_empty() {
                        rows.push(header_row.clone());
                    }

                    // 转换数据行 (跳过 __headers__ 和元表方法)
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

                        // 使用数字索引读取，基于表头的列数
                        // 如果没有表头，则查找最大的列索引
                        let max_col = if num_columns > 0 {
                            num_columns
                        } else {
                            // 查找该行的最大列索引
                            let mut max = 0;
                            for pair in row_table.clone().pairs::<mlua::Value, mlua::Value>() {
                                let (k, _) = pair?;
                                if let mlua::Value::Integer(i) = k {
                                    if i > 0 {
                                        max = max.max(i as usize);
                                    }
                                }
                            }
                            max
                        };

                        // 遍历所有列
                        for col_idx in 1..=max_col {
                            // 首先尝试通过数字索引读取
                            let cell = if let Ok(value) = row_table.get::<usize, String>(col_idx) {
                                value
                            } else if col_idx <= header_row.len() {
                                // 如果数字索引没有值，尝试通过header名称读取
                                let header = &header_row[col_idx - 1];
                                row_table.get::<_, String>(header.as_str())
                                    .unwrap_or_else(|_| String::new())
                            } else {
                                String::new()
                            };
                            row.push(cell);
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
