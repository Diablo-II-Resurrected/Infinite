use super::script_runtime::*;
use anyhow::Result;
use rquickjs::{Context, Runtime, Value, Function, Object, Array};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct JavaScriptRuntime {
    runtime: Runtime,
    context: Context,
    mod_path: PathBuf,
    services: Arc<ScriptServices>,
}

impl JavaScriptRuntime {
    pub fn new(mod_path: &Path, services: ScriptServices) -> Result<Self> {
        // Create QuickJS runtime
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;

        Ok(Self {
            runtime,
            context,
            mod_path: mod_path.to_path_buf(),
            services: Arc::new(services),
        })
    }

    /// Register D2RMM API
    fn register_d2rmm_api(&self) -> Result<()> {
        let services = Arc::clone(&self.services);
        
        self.context.with(|ctx| -> Result<()> {
            let globals = ctx.globals();
            
            // Create D2RMM object
            let d2rmm = Object::new(ctx.clone())?;
            
            // Register readJson
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |ctx, path: String| -> rquickjs::Result<Value> {
                    let json = services.read_json(&path)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))?;
                    json_to_rquickjs(ctx, &json)
                })?;
                d2rmm.set("readJson", func)?;
            }
            
            // Register writeJson
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |ctx, path: String, data: Value| -> rquickjs::Result<()> {
                    let json = rquickjs_to_json(ctx, &data)?;
                    services.write_json(&path, &json)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))?;
                    Ok(())
                })?;
                d2rmm.set("writeJson", func)?;
            }
            
            // Register readTsv
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |ctx, path: String| -> rquickjs::Result<Value> {
                    let tsv = services.read_tsv(&path)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))?;
                    tsv_to_rquickjs(ctx, &tsv)
                })?;
                d2rmm.set("readTsv", func)?;
            }
            
            // Register writeTsv
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |ctx, path: String, data: Value| -> rquickjs::Result<()> {
                    let tsv = rquickjs_to_tsv(ctx, &data)?;
                    services.write_tsv(&path, &tsv)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))?;
                    Ok(())
                })?;
                d2rmm.set("writeTsv", func)?;
            }
            
            // Register readTxt
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |_ctx, path: String| -> rquickjs::Result<String> {
                    services.read_txt(&path)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))
                })?;
                d2rmm.set("readTxt", func)?;
            }
            
            // Register writeTxt
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |_ctx, path: String, content: String| -> rquickjs::Result<()> {
                    services.write_txt(&path, &content)
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))
                })?;
                d2rmm.set("writeTxt", func)?;
            }
            
            // Register copyFile
            {
                let services = Arc::clone(&services);
                let func = Function::new(ctx.clone(), move |_ctx, src: String, dst: String, overwrite: Option<bool>| -> rquickjs::Result<()> {
                    services.copy_file(&src, &dst, overwrite.unwrap_or(false))
                        .map_err(|e| rquickjs::Error::new_from_js_message("Error", &e.to_string()))
                })?;
                d2rmm.set("copyFile", func)?;
            }
            
            // Register getVersion
            {
                let func = Function::new(ctx.clone(), |_ctx| -> rquickjs::Result<f64> { Ok(1.0) })?;
                d2rmm.set("getVersion", func)?;
            }
            
            globals.set("D2RMM", d2rmm)?;
            
            // Register console
            let console = Object::new(ctx.clone())?;
            let log_func = Function::new(ctx.clone(), |_ctx, msg: String| -> rquickjs::Result<()> {
                tracing::info!("[JS] {}", msg);
                Ok(())
            })?;
            console.set("log", log_func.clone())?;
            console.set("debug", log_func.clone())?;
            console.set("warn", log_func.clone())?;
            console.set("error", log_func)?;
            
            globals.set("console", console)?;
            
            Ok(())
        })?;
        
        Ok(())
    }
}

impl ScriptRuntime for JavaScriptRuntime {
    fn setup_api(&mut self) -> Result<()> {
        self.register_d2rmm_api()
    }

    fn setup_config(&mut self, config: &UserConfig) -> Result<()> {
        self.context.with(|ctx| -> Result<()> {
            let globals = ctx.globals();
            let config_obj = Object::new(ctx.clone())?;

            for (key, value) in &config.values {
                match value {
                    ConfigValue::Bool(b) => config_obj.set(key.as_str(), *b)?,
                    ConfigValue::Number(n) => config_obj.set(key.as_str(), *n)?,
                    ConfigValue::String(s) => config_obj.set(key.as_str(), s.as_str())?,
                }
            }

            globals.set("config", config_obj)?;
            Ok(())
        })?;
        Ok(())
    }

    fn execute(&mut self) -> Result<()> {
        let script_path = self.mod_path.join("mod.js");
        let script_content = std::fs::read_to_string(&script_path)?;

        self.context.with(|ctx| -> Result<()> {
            ctx.eval::<(), _>(script_content.as_str())
                .map_err(|e| anyhow::anyhow!("JavaScript execution error: {}", e))?;
            Ok(())
        })?;
        Ok(())
    }

    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }

    fn runtime_type(&self) -> ScriptType {
        ScriptType::JavaScript
    }
}

// Helper functions

fn json_to_rquickjs<'js>(ctx: rquickjs::Ctx<'js>, json: &serde_json::Value) -> rquickjs::Result<Value<'js>> {
    use serde_json::Value as JsonValue;

    match json {
        JsonValue::Null => Ok(Value::new_null(ctx.clone())),
        JsonValue::Bool(b) => Ok(Value::new_bool(ctx.clone(), *b)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::new_int(ctx.clone(), i as i32))
            } else if let Some(f) = n.as_f64() {
                Ok(Value::new_float(ctx.clone(), f))
            } else {
                Ok(Value::new_null(ctx.clone()))
            }
        }
        JsonValue::String(s) => {
            ctx.eval(format!("({})", serde_json::to_string(s).unwrap()))
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

fn rquickjs_to_json<'js>(ctx: rquickjs::Ctx<'js>, val: &Value<'js>) -> rquickjs::Result<serde_json::Value> {
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
        let arr = val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "Expected array"))?;
        let mut json_arr = Vec::new();
        for i in 0..arr.len() {
            let item = arr.get::<Value>(i)?;
            json_arr.push(rquickjs_to_json(ctx.clone(), &item)?);
        }
        return Ok(JsonValue::Array(json_arr));
    }

    if val.is_object() {
        let obj = val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "Expected object"))?;
        let mut json_obj = serde_json::Map::new();
        
        for prop in obj.props::<String, Value>() {
            let (key, value) = prop?;
            json_obj.insert(key, rquickjs_to_json(ctx.clone(), &value)?);
        }
        
        return Ok(JsonValue::Object(json_obj));
    }

    Ok(JsonValue::Null)
}

fn tsv_to_rquickjs<'js>(ctx: rquickjs::Ctx<'js>, tsv: &TsvData) -> rquickjs::Result<Value<'js>> {
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

fn rquickjs_to_tsv<'js>(ctx: rquickjs::Ctx<'js>, val: &Value<'js>) -> rquickjs::Result<TsvData> {
    let obj = val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "TSV data must be an object"))?;

    // Extract headers
    let headers_val: Value = obj.get("headers")?;
    let headers_arr = headers_val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "Headers must be an array"))?;
    
    let mut headers = Vec::new();
    for i in 0..headers_arr.len() {
        let header = headers_arr.get::<String>(i)?;
        headers.push(header);
    }

    // Extract rows
    let rows_val: Value = obj.get("rows")?;
    let rows_arr = rows_val.as_array().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "Rows must be an array"))?;
    
    let mut rows = Vec::new();
    for i in 0..rows_arr.len() {
        let row_val = rows_arr.get::<Value>(i)?;
        let row_obj = row_val.as_object().ok_or_else(|| rquickjs::Error::new_from_js_message("Error", "Row must be an object"))?;
        
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
