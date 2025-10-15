use super::script_runtime::*;
use super::api::{InfiniteApiCore, ConsoleApi};
use anyhow::Result;
use rquickjs::{Context, Runtime, Value, Function, Object, Array, Ctx};
use rquickjs::function::Func;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Helper to convert anyhow errors to rquickjs errors
fn to_js_error(e: anyhow::Error) -> rquickjs::Error {
    rquickjs::Error::new_from_js_message("Error", "RuntimeError", e.to_string())
}

pub struct JavaScriptRuntime {
    _runtime: Runtime,  // Keep alive but mark as intentionally unused
    context: Context,
    mod_path: PathBuf,
    api_core: Arc<InfiniteApiCore>,
}

impl JavaScriptRuntime {
    pub fn new(mod_path: &Path, services: ScriptServices) -> Result<Self> {
        // Create QuickJS runtime
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        let services_arc = Arc::new(services);
        let api_core = Arc::new(InfiniteApiCore::new(services_arc));

        Ok(Self {
            _runtime: runtime,
            context,
            mod_path: mod_path.to_path_buf(),
            api_core,
        })
    }

    /// Register D2RMM API
    fn register_d2rmm_api(&self) -> Result<()> {
        let api_core = Arc::clone(&self.api_core);

        self.context.with(|ctx| {
            let globals = ctx.globals();

            // Create D2RMM object
            let d2rmm = Object::new(ctx.clone())?;

            // Register readJson
            self.register_read_json(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register writeJson
            self.register_write_json(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register readTsv
            self.register_read_tsv(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register writeTsv
            self.register_write_tsv(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register readTxt
            self.register_read_txt(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register writeTxt
            self.register_write_txt(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register copyFile
            self.register_copy_file(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;

            // Register getVersion
            let api_core_ver = Arc::clone(&api_core);
            d2rmm.set("getVersion", Function::new(ctx.clone(), move |_ctx: Ctx| -> rquickjs::Result<f64> {
                Ok(api_core_ver.get_version())
            })?)?;

            // Register error - throws an error that stops execution
            let api_core_err = Arc::clone(&api_core);
            d2rmm.set("error", Function::new(ctx.clone(), move |ctx: Ctx, msg: String| -> rquickjs::Result<()> {
                let _ = api_core_err.throw_error(&msg);
                // Throw a JavaScript Error
                let error_ctor: Function = ctx.globals().get("Error")?;
                let _error: Value = error_ctor.call((msg,))?;
                Err(rquickjs::Error::Exception)
            })?)?;

            globals.set("D2RMM", d2rmm)?;

            // Register console
            self.register_console(ctx.clone())?;

            Ok::<(), rquickjs::Error>(())
        })?;

        Ok(())
    }

    fn register_read_json<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |ctx: Ctx<'js>, path: String| -> rquickjs::Result<Value<'js>> {
            let json = api_core.read_json(&path).map_err(to_js_error)?;
            let result = json_to_rquickjs(ctx, &json)?;
            Ok(result)
        });
        d2rmm.set("readJson", func)?;
        Ok(())
    }

    fn register_write_json<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |ctx: Ctx<'js>, path: String, data: Value<'js>| -> rquickjs::Result<()> {
            let json = rquickjs_to_json(ctx, &data)?;
            api_core.write_json(&path, &json).map_err(to_js_error)?;
            Ok(())
        });
        d2rmm.set("writeJson", func)?;
        Ok(())
    }

    fn register_read_tsv<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |ctx: Ctx<'js>, path: String| -> rquickjs::Result<Value<'js>> {
            let tsv = api_core.read_tsv(&path).map_err(to_js_error)?;
            let result = tsv_to_rquickjs(ctx, &tsv)?;
            Ok(result)
        });
        d2rmm.set("readTsv", func)?;
        Ok(())
    }

    fn register_write_tsv<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |ctx: Ctx<'js>, path: String, data: Value<'js>| -> rquickjs::Result<()> {
            let tsv = rquickjs_to_tsv(ctx, &data)?;
            api_core.write_tsv(&path, &tsv).map_err(to_js_error)?;
            Ok(())
        });
        d2rmm.set("writeTsv", func)?;
        Ok(())
    }

    fn register_read_txt<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |_ctx: Ctx<'js>, path: String| -> rquickjs::Result<String> {
            api_core.read_txt(&path).map_err(to_js_error)
        });
        d2rmm.set("readTxt", func)?;
        Ok(())
    }

    fn register_write_txt<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |_ctx: Ctx<'js>, path: String, content: String| -> rquickjs::Result<()> {
            api_core.write_txt(&path, &content).map_err(to_js_error)
        });
        d2rmm.set("writeTxt", func)?;
        Ok(())
    }

    fn register_copy_file<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<InfiniteApiCore>) -> rquickjs::Result<()> {
        let func = Func::from(move |_ctx: Ctx<'js>, src: String, dst: String| -> rquickjs::Result<()> {
            api_core.copy_file(&src, &dst, false).map_err(to_js_error)
        });
        d2rmm.set("copyFile", func)?;
        Ok(())
    }

    fn register_console<'js>(&self, ctx: Ctx<'js>) -> rquickjs::Result<()> {
        let globals = ctx.globals();
        let console = Object::new(ctx.clone())?;

        // Create separate function instances for each console method
        // Accept variadic arguments and format them
        console.set("log", Func::from(|ctx: Ctx<'js>, args: rquickjs::function::Rest<Value<'js>>| -> rquickjs::Result<()> {
            let msg = format_console_args(ctx, &args.0)?;
            ConsoleApi::log(&msg);
            Ok(())
        }))?;

        console.set("debug", Func::from(|ctx: Ctx<'js>, args: rquickjs::function::Rest<Value<'js>>| -> rquickjs::Result<()> {
            let msg = format_console_args(ctx, &args.0)?;
            ConsoleApi::debug(&msg);
            Ok(())
        }))?;

        console.set("warn", Func::from(|ctx: Ctx<'js>, args: rquickjs::function::Rest<Value<'js>>| -> rquickjs::Result<()> {
            let msg = format_console_args(ctx, &args.0)?;
            ConsoleApi::warn(&msg);
            Ok(())
        }))?;

        console.set("error", Func::from(|ctx: Ctx<'js>, args: rquickjs::function::Rest<Value<'js>>| -> rquickjs::Result<()> {
            let msg = format_console_args(ctx, &args.0)?;
            ConsoleApi::error(&msg);
            Ok(())
        }))?;

        globals.set("console", console)?;
        Ok(())
    }
}

impl ScriptRuntime for JavaScriptRuntime {
    fn setup_api(&mut self) -> Result<()> {
        self.register_d2rmm_api()
    }

    fn setup_config(&mut self, config: &UserConfig) -> Result<()> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            let config_obj = Object::new(ctx.clone())?;

            for (key, value) in config {
                // Convert serde_json::Value to rquickjs Value
                let js_value = json_to_rquickjs(ctx.clone(), value)?;
                config_obj.set(key.as_str(), js_value)?;
            }

            globals.set("config", config_obj)?;
            Ok::<(), rquickjs::Error>(())
        })?;
        Ok(())
    }

    fn execute(&mut self) -> Result<()> {
        let script_path = self.mod_path.join("mod.js");
        let script_content = std::fs::read_to_string(&script_path)
            .map_err(|e| anyhow::anyhow!("Failed to read mod.js: {}", e))?;

        tracing::debug!("Executing JavaScript from: {:?}", script_path);
        tracing::debug!("Script length: {} bytes", script_content.len());

        self.context.with(|ctx| {
            ctx.eval::<(), _>(script_content.as_bytes())
                .map_err(|e| {
                    // Try to get more detailed error information
                    let error_msg = format!("JavaScript execution error: {:?}", e);
                    tracing::error!("{}", error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
            Ok::<(), anyhow::Error>(())
        })?;

        tracing::debug!("JavaScript execution completed successfully");
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn runtime_type(&self) -> ScriptType {
        ScriptType::JavaScript
    }
}

// Helper functions for type conversion

/// Format console arguments by calling toString() on each value
fn format_console_args<'js>(ctx: Ctx<'js>, args: &[Value<'js>]) -> rquickjs::Result<String> {
    let mut parts = Vec::new();
    for arg in args {
        let s = value_to_string(ctx.clone(), arg)?;
        parts.push(s);
    }
    Ok(parts.join(" "))
}

/// Convert a JavaScript value to string
fn value_to_string<'js>(ctx: Ctx<'js>, value: &Value<'js>) -> rquickjs::Result<String> {
    if value.is_string() {
        Ok(value.as_string().unwrap().to_string()?)
    } else if value.is_int() {
        Ok(value.as_int().unwrap().to_string())
    } else if value.is_float() {
        Ok(value.as_float().unwrap().to_string())
    } else if value.is_bool() {
        Ok(value.as_bool().unwrap().to_string())
    } else if value.is_null() {
        Ok("null".to_string())
    } else if value.is_undefined() {
        Ok("undefined".to_string())
    } else {
        // For objects/arrays, use JSON.stringify
        let json_obj: Object = ctx.globals().get("JSON")?;
        let stringify: Function = json_obj.get("stringify")?;
        let result: String = stringify.call((value.clone(),))?;
        Ok(result)
    }
}

fn json_to_rquickjs<'js>(ctx: Ctx<'js>, json: &serde_json::Value) -> rquickjs::Result<Value<'js>> {
    use serde_json::Value as JsonValue;

    match json {
        JsonValue::Null => Ok(Value::new_undefined(ctx.clone())),
        JsonValue::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_undefined(ctx.clone()))
            }
        }
        JsonValue::String(s) => {
            let obj: Value = ctx.eval(format!("({})", serde_json::to_string(s).unwrap()).as_bytes())?;
            Ok(obj)
        }
        JsonValue::Array(arr) => {
            let js_arr = Array::new(ctx.clone())?;
            for (i, item) in arr.iter().enumerate() {
                js_arr.set(i, json_to_rquickjs(ctx.clone(), item)?)?;
            }
            Ok(js_arr.into_value())
        }
        JsonValue::Object(obj) => {
            let js_obj = Object::new(ctx.clone())?;
            for (key, value) in obj {
                js_obj.set(key.as_str(), json_to_rquickjs(ctx.clone(), value)?)?;
            }
            Ok(js_obj.into_value())
        }
    }
}

fn rquickjs_to_json<'js>(ctx: Ctx<'js>, val: &Value<'js>) -> rquickjs::Result<serde_json::Value> {
    use serde_json::Value as JsonValue;

    if val.is_null() || val.is_undefined() {
        return Ok(JsonValue::Null);
    }

    if let Some(b) = val.as_bool() {
        return Ok(JsonValue::Bool(b));
    }

    if let Some(i) = val.as_int() {
        return Ok(JsonValue::Number(i.into()));
    }

    if let Some(f) = val.as_float() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Ok(JsonValue::Number(n));
        }
    }

    if let Some(s) = val.as_string() {
        return Ok(JsonValue::String(s.to_string()?));
    }

    if val.is_array() {
        let arr = val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "Expected array"))?;
        let mut json_arr = Vec::new();
        for i in 0..arr.len() {
            let item: Value = arr.get(i)?;
            json_arr.push(rquickjs_to_json(ctx.clone(), &item)?);
        }
        return Ok(JsonValue::Array(json_arr));
    }

    if val.is_object() {
        let obj = val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "Expected object"))?;
        let mut json_obj = serde_json::Map::new();

        for prop in obj.props::<String, Value>() {
            let (key, value) = prop?;
            json_obj.insert(key, rquickjs_to_json(ctx.clone(), &value)?);
        }

        return Ok(JsonValue::Object(json_obj));
    }

    Ok(JsonValue::Null)
}

fn tsv_to_rquickjs<'js>(ctx: Ctx<'js>, tsv: &TsvData) -> rquickjs::Result<Value<'js>> {
    let result = Object::new(ctx.clone())?;

    // headers
    let headers = Array::new(ctx.clone())?;
    for (i, header) in tsv.headers.iter().enumerate() {
        headers.set(i, header.as_str())?;
    }
    result.set("headers", headers)?;

    // rows
    let rows = Array::new(ctx.clone())?;
    for (i, row) in tsv.rows.iter().enumerate() {
        let row_obj = Object::new(ctx.clone())?;
        for (key, value) in &row.data {
            row_obj.set(key.as_str(), value.as_str())?;
        }
        rows.set(i, row_obj)?;
    }
    result.set("rows", rows)?;

    Ok(result.into_value())
}

fn rquickjs_to_tsv<'js>(_ctx: Ctx<'js>, val: &Value<'js>) -> rquickjs::Result<TsvData> {
    let obj = val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "TSV data must be an object"))?;

    // Extract headers
    let headers_val: Value = obj.get("headers")?;
    let headers_arr = headers_val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "Headers must be an array"))?;

    let mut headers = Vec::new();
    for i in 0..headers_arr.len() {
        let header: String = headers_arr.get(i)?;
        headers.push(header);
    }

    // Extract rows
    let rows_val: Value = obj.get("rows")?;
    let rows_arr = rows_val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "Rows must be an array"))?;

    let mut rows = Vec::new();
    for i in 0..rows_arr.len() {
        let row_val: Value = rows_arr.get(i)?;
        let row_obj = row_val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TypeError", "Row must be an object"))?;

        let mut data = std::collections::HashMap::new();
        for prop in row_obj.props::<String, Value>() {
            let (key, value) = prop?;
            let str_val = if let Some(s) = value.as_string() {
                s.to_string()?
            } else if let Some(i) = value.as_int() {
                i.to_string()
            } else if let Some(f) = value.as_float() {
                f.to_string()
            } else {
                String::new()
            };
            data.insert(key, str_val);
        }

        rows.push(TsvRow { data });
    }

    Ok(TsvData { headers, rows })
}
