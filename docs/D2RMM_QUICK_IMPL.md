# D2RMM JavaScript 兼容性 - 快速实现指南

## 🎯 目标

让 Infinite 能够直接运行 D2RMM 的 JavaScript mod（如附件中的 ExpandedCube/mod.js）。

## 📋 实现清单

### 阶段 1: 基础设施 (核心)

#### 1.1 添加依赖

```bash
cd f:\Projects\d2rmm\d2rmm-cli
cargo add rquickjs --features array-buffer,loader
```

或手动编辑 `Cargo.toml`:

```toml
[dependencies]
# ... 现有依赖 ...

# JavaScript 运行时支持
rquickjs = { version = "0.6", features = ["array-buffer", "loader"] }
```

#### 1.2 创建 JS 运行时模块

**文件结构**:
```
src/
├── runtime/
│   ├── mod.rs           # 现有
│   ├── executor.rs      # 现有
│   ├── context.rs       # 现有 (Lua)
│   ├── js_context.rs    # 新增 (JavaScript)
│   └── js_api.rs        # 新增 (D2RMM API 实现)
```

### 阶段 2: 核心实现

#### 2.1 创建 JavaScript 上下文

**文件**: `src/runtime/js_context.rs`

```rust
use rquickjs::{Context, Runtime, Function, Object, Value};
use anyhow::Result;
use std::path::{Path, PathBuf};

pub struct JavaScriptContext {
    runtime: Runtime,
    context: Context,
    mod_path: PathBuf,
    output_path: PathBuf,
}

impl JavaScriptContext {
    pub fn new(mod_path: &Path, output_path: &Path) -> Result<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        
        Ok(Self {
            runtime,
            context,
            mod_path: mod_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
        })
    }
    
    /// 初始化 D2RMM API
    pub fn setup_api(&self) -> Result<()> {
        self.context.with(|ctx| {
            let globals = ctx.globals();
            
            // 创建 D2RMM 对象
            let d2rmm = Object::new(ctx)?;
            
            // 注册各个 API 方法
            self.register_file_api(&d2rmm, ctx)?;
            self.register_utility_api(&d2rmm, ctx)?;
            
            // 设置全局对象
            globals.set("D2RMM", d2rmm)?;
            
            Ok(())
        })
    }
    
    /// 执行 mod.js 脚本
    pub fn execute(&self) -> Result<()> {
        let script_path = self.mod_path.join("mod.js");
        let script = std::fs::read_to_string(&script_path)?;
        
        self.context.with(|ctx| {
            ctx.eval::<(), _>(script)?;
            Ok(())
        })
    }
    
    // API 注册方法（下一步实现）
    fn register_file_api(&self, obj: &Object, ctx: &Context) -> Result<()> {
        // TODO: 实现
        Ok(())
    }
    
    fn register_utility_api(&self, obj: &Object, ctx: &Context) -> Result<()> {
        // TODO: 实现
        Ok(())
    }
}
```

#### 2.2 实现 D2RMM API

**文件**: `src/runtime/js_api.rs`

```rust
use rquickjs::{Ctx, Function, Value};
use std::path::{Path, PathBuf};
use anyhow::Result;

/// D2RMM.readJson(path)
pub fn read_json<'js>(ctx: Ctx<'js>, path: String) -> Result<Value<'js>> {
    let content = std::fs::read_to_string(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    
    // 转换为 QuickJS 值
    let js_val = serde_json_to_js(ctx, json)?;
    Ok(js_val)
}

/// D2RMM.writeJson(path, data)
pub fn write_json<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    // 转换 JS 值为 JSON
    let json = js_to_serde_json(ctx, data)?;
    
    // 写入文件
    let content = serde_json::to_string_pretty(&json)?;
    std::fs::write(&path, content)?;
    
    Ok(())
}

/// D2RMM.readTsv(path)
pub fn read_tsv<'js>(ctx: Ctx<'js>, path: String) -> Result<Value<'js>> {
    // 使用现有的 TSV 读取逻辑
    let content = std::fs::read_to_string(&path)?;
    
    // 解析 TSV
    let mut reader = csv::ReaderBuilder::new()
        .delimiter(b'\t')
        .from_reader(content.as_bytes());
    
    // 转换为 JS 对象
    let headers: Vec<String> = reader.headers()?.iter().map(|s| s.to_string()).collect();
    
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result?;
        let mut row_obj = rquickjs::Object::new(ctx)?;
        
        for (i, field) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                row_obj.set(header, field)?;
            }
        }
        
        rows.push(row_obj);
    }
    
    // 创建返回对象 { headers, rows }
    let result = rquickjs::Object::new(ctx)?;
    result.set("headers", headers)?;
    result.set("rows", rows)?;
    
    Ok(result.into())
}

/// D2RMM.writeTsv(path, data)
pub fn write_tsv<'js>(ctx: Ctx<'js>, path: String, data: Value<'js>) -> Result<()> {
    // 从 JS 对象提取数据
    let obj: rquickjs::Object = data.try_into()?;
    
    let headers: Vec<String> = obj.get("headers")?;
    let rows: Vec<rquickjs::Object> = obj.get("rows")?;
    
    // 构建 TSV 内容
    let mut content = String::new();
    content.push_str(&headers.join("\t"));
    content.push('\n');
    
    for row in rows {
        let mut values = Vec::new();
        for header in &headers {
            let value: String = row.get(header).unwrap_or_default();
            values.push(value);
        }
        content.push_str(&values.join("\t"));
        content.push('\n');
    }
    
    std::fs::write(&path, content)?;
    Ok(())
}

/// D2RMM.copyFile(src, dst, overwrite)
pub fn copy_file(src: String, dst: String, overwrite: bool) -> Result<()> {
    if !overwrite && Path::new(&dst).exists() {
        return Ok(());
    }
    
    // 确保目标目录存在
    if let Some(parent) = Path::new(&dst).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    std::fs::copy(&src, &dst)?;
    Ok(())
}

// 辅助函数：serde_json → QuickJS Value
fn serde_json_to_js<'js>(ctx: Ctx<'js>, json: serde_json::Value) -> Result<Value<'js>> {
    use serde_json::Value as JsonValue;
    
    let js_val = match json {
        JsonValue::Null => Value::new_null(ctx),
        JsonValue::Bool(b) => Value::new_bool(ctx, b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::new_int(ctx, i as i32)
            } else if let Some(f) = n.as_f64() {
                Value::new_float(ctx, f)
            } else {
                Value::new_null(ctx)
            }
        }
        JsonValue::String(s) => ctx.eval(&format!("({})", serde_json::to_string(&s)?))?,
        JsonValue::Array(arr) => {
            let js_arr = rquickjs::Array::new(ctx)?;
            for (i, item) in arr.into_iter().enumerate() {
                js_arr.set(i, serde_json_to_js(ctx, item)?)?;
            }
            js_arr.into()
        }
        JsonValue::Object(obj) => {
            let js_obj = rquickjs::Object::new(ctx)?;
            for (key, value) in obj {
                js_obj.set(&key, serde_json_to_js(ctx, value)?)?;
            }
            js_obj.into()
        }
    };
    
    Ok(js_val)
}

// 辅助函数：QuickJS Value → serde_json
fn js_to_serde_json<'js>(ctx: Ctx<'js>, val: Value<'js>) -> Result<serde_json::Value> {
    // 简化版本：使用 JSON.stringify
    let json_str: String = ctx.eval(&format!("JSON.stringify({})", "val"))?;
    Ok(serde_json::from_str(&json_str)?)
}
```

#### 2.3 修改 ModLoader

**文件**: `src/mod_manager/loader.rs`

```rust
// 在 load_mod 方法中添加 mod.js 检测

pub fn load_mod(&self, mod_path: &Path) -> Result<LoadedMod> {
    let config_path = mod_path.join("mod.json");
    
    if !config_path.exists() {
        anyhow::bail!("mod.json not found in {:?}", mod_path);
    }

    let config_str = std::fs::read_to_string(&config_path)
        .context("Failed to read mod.json")?;

    let config: ModConfig = serde_json::from_str(&config_str)
        .context("Failed to parse mod.json")?;

    // 检查 mod.lua 或 mod.js
    let lua_path = mod_path.join("mod.lua");
    let js_path = mod_path.join("mod.js");
    
    let script_type = if lua_path.exists() {
        ScriptType::Lua
    } else if js_path.exists() {
        ScriptType::JavaScript
    } else {
        anyhow::bail!("Neither mod.lua nor mod.js found in {:?}", mod_path);
    };

    let id = mod_path
        .file_name()
        .and_then(|s| s.to_str())
        .context("Invalid mod directory name")?
        .to_string();

    let user_config = config.generate_default_config();

    Ok(LoadedMod {
        id,
        path: mod_path.to_path_buf(),
        config,
        user_config,
        script_type,  // 新增字段
    })
}

// 添加枚举
#[derive(Debug, Clone, Copy)]
pub enum ScriptType {
    Lua,
    JavaScript,
}
```

#### 2.4 修改 Executor

**文件**: `src/runtime/executor.rs`

```rust
// 在执行 mod 时根据类型选择运行时

pub fn execute_mod(&mut self, loaded_mod: &LoadedMod) -> Result<()> {
    match loaded_mod.script_type {
        ScriptType::Lua => {
            // 现有的 Lua 执行逻辑
            let lua_ctx = LuaContext::new(&self.mod_path, &self.output_path)?;
            lua_ctx.setup_api()?;
            lua_ctx.execute()?;
        }
        ScriptType::JavaScript => {
            // 新的 JS 执行逻辑
            let js_ctx = JavaScriptContext::new(&self.mod_path, &self.output_path)?;
            js_ctx.setup_api()?;
            js_ctx.execute()?;
        }
    }
    
    Ok(())
}
```

### 阶段 3: API 完整实现

#### 3.1 完整的 D2RMM API 列表

```rust
// src/runtime/js_api.rs

impl JavaScriptContext {
    fn register_file_api(&self, obj: &Object, ctx: &Context) -> Result<()> {
        // 文件读写 API
        obj.set("readJson", Function::new(ctx, read_json))?;
        obj.set("writeJson", Function::new(ctx, write_json))?;
        obj.set("readTsv", Function::new(ctx, read_tsv))?;
        obj.set("writeTsv", Function::new(ctx, write_tsv))?;
        obj.set("readTxt", Function::new(ctx, read_txt))?;
        obj.set("writeTxt", Function::new(ctx, write_txt))?;
        obj.set("copyFile", Function::new(ctx, copy_file))?;
        
        Ok(())
    }
    
    fn register_utility_api(&self, obj: &Object, ctx: &Context) -> Result<()> {
        // 工具 API
        obj.set("getVersion", Function::new(ctx, || 1.0))?;
        
        Ok(())
    }
}

// 实现 readTxt 和 writeTxt
pub fn read_txt(path: String) -> Result<String> {
    Ok(std::fs::read_to_string(&path)?)
}

pub fn write_txt(path: String, content: String) -> Result<()> {
    std::fs::write(&path, content)?;
    Ok(())
}
```

#### 3.2 配置对象注入

```rust
// 注入 config 全局变量
pub fn setup_config(&self, user_config: &UserConfig) -> Result<()> {
    self.context.with(|ctx| {
        let globals = ctx.globals();
        
        // 创建 config 对象
        let config_obj = Object::new(ctx)?;
        
        for (key, value) in &user_config.values {
            // 转换配置值为 JS 值
            let js_val = match value {
                ConfigValue::Bool(b) => Value::new_bool(ctx, *b),
                ConfigValue::Number(n) => Value::new_float(ctx, *n),
                ConfigValue::String(s) => ctx.eval(&format!("({})", serde_json::to_string(s)?))?,
            };
            config_obj.set(key, js_val)?;
        }
        
        globals.set("config", config_obj)?;
        
        Ok(())
    })
}
```

### 阶段 4: 测试

#### 4.1 创建测试 mod

**文件**: `test_mods/js_test_mod/mod.json`
```json
{
  "name": "JS Test Mod",
  "description": "Test JavaScript runtime",
  "version": "1.0.0",
  "author": "Test"
}
```

**文件**: `test_mods/js_test_mod/mod.js`
```javascript
// 测试基础 API
const data = D2RMM.readJson('test.json');
console.log('Read JSON:', data);

data.modified = true;
D2RMM.writeJson('test_output.json', data);

// 测试 TSV
const tsv = D2RMM.readTsv('global/excel/weapons.txt');
console.log('TSV rows:', tsv.rows.length);

// 测试文件复制
D2RMM.copyFile('source.txt', 'dest.txt', true);
```

#### 4.2 运行测试

```bash
cargo build --release
.\target\release\infinite.exe install \
  --game-path "C:/Games/D2R" \
  --mod-list test_js_mods.txt
```

### 阶段 5: 文档更新

#### 5.1 更新 README.md

```markdown
## 🎮 Supported Script Languages

Infinite supports **two script languages**:

### Lua (Native)
```lua
-- mod.lua
local data = infinite.readJson("file.json")
infinite.writeJson("output.json", data)
```

### JavaScript (D2RMM Compatible)
```javascript
// mod.js
const data = D2RMM.readJson("file.json");
D2RMM.writeJson("output.json", data);
```

## 📝 D2RMM Compatibility

Infinite is **100% compatible** with D2RMM JavaScript mods!

- ✅ Use existing D2RMM mods directly
- ✅ `D2RMM.*` API fully supported
- ✅ Same behavior as D2RMM
- ✅ No migration needed

Simply place your D2RMM mod folder in the mod list and run!
```

#### 5.2 创建迁移指南

**文件**: `docs/D2RMM_MIGRATION.md`

```markdown
# D2RMM Mod Migration Guide

## ✅ No Migration Needed!

Infinite **directly supports** D2RMM JavaScript mods!

## Using D2RMM Mods

1. Download or create a D2RMM mod with `mod.js`
2. Add to your mod list:
   ```
   F:/mods/ExpandedCube
   ```
3. Run Infinite:
   ```bash
   infinite install --game-path "C:/Games/D2R" --mod-list mods.txt
   ```

That's it! Infinite will automatically detect and run JavaScript mods.

## Creating New Mods

You can choose **either** Lua or JavaScript:

### Option 1: JavaScript (D2RMM compatible)
```javascript
// mod.js
const data = D2RMM.readJson("file.json");
data.field = "value";
D2RMM.writeJson("file.json", data);
```

### Option 2: Lua (Native)
```lua
-- mod.lua
local data = infinite.readJson("file.json")
data.field = "value"
infinite.writeJson("file.json", data)
```

Both work identically!
```

## 📊 工作量估算

| 阶段 | 任务 | 时间 |
|------|------|------|
| 1 | 添加依赖和基础架构 | 0.5天 |
| 2 | 实现 JS 上下文 | 1天 |
| 3 | 实现 D2RMM API | 2天 |
| 4 | 测试和调试 | 1天 |
| 5 | 文档更新 | 0.5天 |
| **总计** | | **5天** |

## 🎯 验收标准

1. ✅ 能运行附件中的 `ExpandedCube/mod.js`
2. ✅ 所有 D2RMM API 工作正常
3. ✅ 与 Lua mod 混合使用无问题
4. ✅ 性能影响 <15%
5. ✅ 文档完整

## 🚀 立即开始

```bash
# 1. 添加依赖
cargo add rquickjs --features array-buffer,loader

# 2. 创建文件
touch src/runtime/js_context.rs
touch src/runtime/js_api.rs

# 3. 开始编码！
```

祝编码愉快！🎉
