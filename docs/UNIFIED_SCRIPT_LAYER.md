# 统一脚本层架构设计

## 🎯 设计目标

创建一个统一的脚本抽象层，隐藏 Lua 和 JavaScript 的实现细节，对外提供一致的接口。

## 📐 架构设计

```
┌─────────────────────────────────────────────────────┐
│            Mod Manager (调用层)                      │
│  - 加载 mod                                         │
│  - 执行 mod                                         │
│  - 处理结果                                         │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│         Script Runtime (统一抽象层)                  │
│  trait ScriptRuntime {                              │
│    fn execute(&mut self) -> Result<()>              │
│    fn setup_api(&mut self) -> Result<()>            │
│    fn cleanup(&mut self) -> Result<()>              │
│  }                                                   │
└──────────────┬────────────────────┬─────────────────┘
               │                    │
       ┌───────▼────────┐   ┌──────▼───────────┐
       │  Lua Runtime   │   │  JavaScript RT   │
       │   (mlua)       │   │ (quickjs_runtime)│
       └───────┬────────┘   └──────┬───────────┘
               │                    │
       ┌───────▼────────┐   ┌──────▼───────────┐
       │ Lua API Layer  │   │  JS API Layer    │
       │ infinite.*     │   │  D2RMM.*         │
       └────────────────┘   └──────────────────┘
                   │                │
                   └────────┬───────┘
                           ▼
                  ┌────────────────┐
                  │  Core Services │
                  │  - 文件读写    │
                  │  - TSV 处理    │
                  │  - JSON 处理   │
                  └────────────────┘
```

## 📝 核心代码设计

### 1. 统一脚本运行时 Trait

**文件**: `src/runtime/script_runtime.rs`

```rust
use anyhow::Result;
use std::path::{Path, PathBuf};
use serde_json::Value as JsonValue;

/// 统一的脚本运行时接口
pub trait ScriptRuntime: Send {
    /// 设置 API（注入全局对象和函数）
    fn setup_api(&mut self, services: &ScriptServices) -> Result<()>;
    
    /// 设置用户配置
    fn setup_config(&mut self, config: &UserConfig) -> Result<()>;
    
    /// 执行脚本
    fn execute(&mut self) -> Result<()>;
    
    /// 清理资源
    fn cleanup(&mut self) -> Result<()>;
    
    /// 获取运行时类型
    fn runtime_type(&self) -> ScriptType;
}

/// 脚本类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptType {
    Lua,
    JavaScript,
}

impl std::fmt::Display for ScriptType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScriptType::Lua => write!(f, "Lua"),
            ScriptType::JavaScript => write!(f, "JavaScript"),
        }
    }
}

/// 脚本服务 - 提供给所有运行时的核心功能
pub struct ScriptServices {
    pub mod_path: PathBuf,
    pub output_path: PathBuf,
    pub game_path: PathBuf,
}

impl ScriptServices {
    pub fn new(mod_path: PathBuf, output_path: PathBuf, game_path: PathBuf) -> Self {
        Self {
            mod_path,
            output_path,
            game_path,
        }
    }
    
    /// 读取 JSON 文件
    pub fn read_json(&self, path: &str) -> Result<JsonValue> {
        let full_path = self.resolve_path(path);
        let content = std::fs::read_to_string(&full_path)?;
        Ok(serde_json::from_str(&content)?)
    }
    
    /// 写入 JSON 文件
    pub fn write_json(&self, path: &str, data: &JsonValue) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(data)?;
        std::fs::write(&full_path, content)?;
        Ok(())
    }
    
    /// 读取 TSV 文件
    pub fn read_tsv(&self, path: &str) -> Result<TsvData> {
        let full_path = self.resolve_path(path);
        TsvData::from_file(&full_path)
    }
    
    /// 写入 TSV 文件
    pub fn write_tsv(&self, path: &str, data: &TsvData) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        data.write_to_file(&full_path)
    }
    
    /// 读取文本文件
    pub fn read_txt(&self, path: &str) -> Result<String> {
        let full_path = self.resolve_path(path);
        Ok(std::fs::read_to_string(&full_path)?)
    }
    
    /// 写入文本文件
    pub fn write_txt(&self, path: &str, content: &str) -> Result<()> {
        let full_path = self.resolve_output_path(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full_path, content)?;
        Ok(())
    }
    
    /// 复制文件
    pub fn copy_file(&self, src: &str, dst: &str, overwrite: bool) -> Result<()> {
        let src_path = self.mod_path.join(src);
        let dst_path = self.output_path.join(dst);
        
        if !overwrite && dst_path.exists() {
            return Ok(());
        }
        
        if let Some(parent) = dst_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
        
        Ok(())
    }
    
    /// 解析路径（从游戏目录或输出目录读取）
    fn resolve_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");
        
        // 先尝试输出目录
        let output_path = self.output_path.join(&normalized);
        if output_path.exists() {
            return output_path;
        }
        
        // 再尝试游戏目录
        self.game_path.join(&normalized)
    }
    
    /// 解析输出路径
    fn resolve_output_path(&self, path: &str) -> PathBuf {
        let normalized = path.replace('\\', "/");
        self.output_path.join(&normalized)
    }
}

/// TSV 数据结构
#[derive(Debug, Clone)]
pub struct TsvData {
    pub headers: Vec<String>,
    pub rows: Vec<TsvRow>,
}

#[derive(Debug, Clone)]
pub struct TsvRow {
    pub data: std::collections::HashMap<String, String>,
}

impl TsvData {
    pub fn from_file(path: &Path) -> Result<Self> {
        // 实现 TSV 读取逻辑
        // （使用现有的 TSV 处理代码）
        todo!()
    }
    
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        // 实现 TSV 写入逻辑
        todo!()
    }
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    
    Ok(())
}

/// 用户配置
#[derive(Debug, Clone)]
pub struct UserConfig {
    pub values: std::collections::HashMap<String, ConfigValue>,
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    Bool(bool),
    Number(f64),
    String(String),
}
```

### 2. 运行时工厂

**文件**: `src/runtime/factory.rs`

```rust
use super::script_runtime::*;
use super::lua_runtime::LuaScriptRuntime;
use super::js_runtime::JavaScriptRuntime;
use anyhow::{Result, bail};
use std::path::Path;

/// 脚本运行时工厂
pub struct RuntimeFactory;

impl RuntimeFactory {
    /// 根据 mod 目录自动创建对应的运行时
    pub fn create_runtime(mod_path: &Path, services: ScriptServices) -> Result<Box<dyn ScriptRuntime>> {
        let lua_script = mod_path.join("mod.lua");
        let js_script = mod_path.join("mod.js");
        
        if lua_script.exists() {
            tracing::info!("Detected Lua script: {}", lua_script.display());
            Ok(Box::new(LuaScriptRuntime::new(mod_path, services)?))
        } else if js_script.exists() {
            tracing::info!("Detected JavaScript script: {}", js_script.display());
            Ok(Box::new(JavaScriptRuntime::new(mod_path, services)?))
        } else {
            bail!("No mod.lua or mod.js found in {:?}", mod_path);
        }
    }
    
    /// 显式创建 Lua 运行时
    pub fn create_lua_runtime(mod_path: &Path, services: ScriptServices) -> Result<Box<dyn ScriptRuntime>> {
        Ok(Box::new(LuaScriptRuntime::new(mod_path, services)?))
    }
    
    /// 显式创建 JavaScript 运行时
    pub fn create_js_runtime(mod_path: &Path, services: ScriptServices) -> Result<Box<dyn ScriptRuntime>> {
        Ok(Box::new(JavaScriptRuntime::new(mod_path, services)?))
    }
}
```

### 3. Lua 运行时实现

**文件**: `src/runtime/lua_runtime.rs`

```rust
use super::script_runtime::*;
use anyhow::Result;
use mlua::Lua;
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
}

impl ScriptRuntime for LuaScriptRuntime {
    fn setup_api(&mut self, services: &ScriptServices) -> Result<()> {
        let globals = self.lua.globals();
        
        // 创建 infinite 表
        let infinite_table = self.lua.create_table()?;
        
        // 克隆 Arc 以便在闭包中使用
        let services_clone = Arc::clone(&self.services);
        
        // infinite.readJson
        let read_json = self.lua.create_function(move |_, path: String| {
            let json = services_clone.read_json(&path)
                .map_err(|e| mlua::Error::RuntimeError(e.to_string()))?;
            Ok(serde_json::to_string(&json).unwrap())
        })?;
        infinite_table.set("readJson", read_json)?;
        
        // 其他 API...
        
        globals.set("infinite", infinite_table)?;
        
        Ok(())
    }
    
    fn setup_config(&mut self, config: &UserConfig) -> Result<()> {
        let globals = self.lua.globals();
        let config_table = self.lua.create_table()?;
        
        for (key, value) in &config.values {
            match value {
                ConfigValue::Bool(b) => config_table.set(key.as_str(), *b)?,
                ConfigValue::Number(n) => config_table.set(key.as_str(), *n)?,
                ConfigValue::String(s) => config_table.set(key.as_str(), s.as_str())?,
            }
        }
        
        globals.set("config", config_table)?;
        Ok(())
    }
    
    fn execute(&mut self) -> Result<()> {
        let script_path = self.mod_path.join("mod.lua");
        let script = std::fs::read_to_string(&script_path)?;
        
        self.lua.load(&script)
            .set_name("mod.lua")
            .exec()
            .map_err(|e| anyhow::anyhow!("Lua execution error: {}", e))?;
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        // Lua 会自动清理
        Ok(())
    }
    
    fn runtime_type(&self) -> ScriptType {
        ScriptType::Lua
    }
}
```

### 4. JavaScript 运行时实现

**文件**: `src/runtime/js_runtime.rs`

```rust
use super::script_runtime::*;
use anyhow::Result;
use quickjs_runtime::builder::QuickJsRuntimeBuilder;
use quickjs_runtime::jsutils::Script;
use quickjs_runtime::quickjsrealmadapter::QuickJsRealmAdapter;
use quickjs_runtime::quickjsruntime::QuickJsRuntime;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct JavaScriptRuntime {
    runtime: QuickJsRuntime,
    mod_path: PathBuf,
    services: Arc<ScriptServices>,
}

impl JavaScriptRuntime {
    pub fn new(mod_path: &Path, services: ScriptServices) -> Result<Self> {
        // 创建 QuickJS 运行时
        let runtime = QuickJsRuntimeBuilder::new()
            .build();
        
        Ok(Self {
            runtime,
            mod_path: mod_path.to_path_buf(),
            services: Arc::new(services),
        })
    }
    
    /// 注册 D2RMM API
    fn register_d2rmm_api(&self) -> Result<()> {
        let services = Arc::clone(&self.services);
        
        // 在主 realm 中执行
        self.runtime.exe_rt_task_in_event_loop(move |rt| {
            let realm = rt.get_main_realm();
            
            // 创建 D2RMM 对象
            realm.eval(Script::new("D2RMM.es", "globalThis.D2RMM = {};"))
                .map_err(|e| anyhow::anyhow!("Failed to create D2RMM object: {:?}", e))?;
            
            // 注册 readJson
            Self::register_function(
                realm,
                "D2RMM.readJson",
                Arc::clone(&services),
                |services, args| {
                    let path = args.get(0)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("readJson: path argument required"))?;
                    
                    let json = services.read_json(path)?;
                    Ok(json)
                },
            )?;
            
            // 注册 writeJson
            Self::register_function(
                realm,
                "D2RMM.writeJson",
                Arc::clone(&services),
                |services, args| {
                    let path = args.get(0)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| anyhow::anyhow!("writeJson: path argument required"))?;
                    
                    let data = args.get(1)
                        .ok_or_else(|| anyhow::anyhow!("writeJson: data argument required"))?;
                    
                    services.write_json(path, data)?;
                    Ok(serde_json::Value::Null)
                },
            )?;
            
            // 注册其他 API...
            
            Ok(())
        })?;
        
        Ok(())
    }
    
    /// 辅助函数：注册 JavaScript 函数
    fn register_function<F>(
        realm: &QuickJsRealmAdapter,
        name: &str,
        services: Arc<ScriptServices>,
        handler: F,
    ) -> Result<()>
    where
        F: Fn(&ScriptServices, &[serde_json::Value]) -> Result<serde_json::Value> + Send + 'static,
    {
        // 使用 quickjs_runtime 的 API 注册函数
        // 具体实现取决于库的 API
        todo!("Implement function registration")
    }
}

impl ScriptRuntime for JavaScriptRuntime {
    fn setup_api(&mut self, _services: &ScriptServices) -> Result<()> {
        self.register_d2rmm_api()
    }
    
    fn setup_config(&mut self, config: &UserConfig) -> Result<()> {
        // 将 config 注入为全局变量
        let config_json = serde_json::to_string(&config.values)?;
        
        self.runtime.exe_rt_task_in_event_loop(move |rt| {
            let realm = rt.get_main_realm();
            let script = format!("globalThis.config = {};", config_json);
            realm.eval(Script::new("config.es", &script))
                .map_err(|e| anyhow::anyhow!("Failed to set config: {:?}", e))?;
            Ok(())
        })?;
        
        Ok(())
    }
    
    fn execute(&mut self) -> Result<()> {
        let script_path = self.mod_path.join("mod.js");
        let script_content = std::fs::read_to_string(&script_path)?;
        
        self.runtime.exe_rt_task_in_event_loop(move |rt| {
            let realm = rt.get_main_realm();
            realm.eval(Script::new("mod.js", &script_content))
                .map_err(|e| anyhow::anyhow!("JavaScript execution error: {:?}", e))?;
            Ok(())
        })?;
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        // QuickJS 运行时会自动清理
        Ok(())
    }
    
    fn runtime_type(&self) -> ScriptType {
        ScriptType::JavaScript
    }
}
```

### 5. 使用示例

**文件**: `src/mod_manager/executor.rs`

```rust
use super::loader::LoadedMod;
use crate::runtime::{RuntimeFactory, ScriptServices, ScriptRuntime};
use anyhow::Result;
use std::path::Path;

pub struct ModExecutor {
    game_path: PathBuf,
    output_path: PathBuf,
}

impl ModExecutor {
    pub fn new(game_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Self {
        Self {
            game_path: game_path.as_ref().to_path_buf(),
            output_path: output_path.as_ref().to_path_buf(),
        }
    }
    
    /// 执行单个 mod
    pub fn execute_mod(&self, loaded_mod: &LoadedMod) -> Result<()> {
        tracing::info!(
            "Executing mod: {} ({})", 
            loaded_mod.config.name,
            loaded_mod.config.version
        );
        
        // 创建服务实例
        let services = ScriptServices::new(
            loaded_mod.path.clone(),
            self.output_path.clone(),
            self.game_path.clone(),
        );
        
        // 自动创建对应的运行时
        let mut runtime = RuntimeFactory::create_runtime(&loaded_mod.path, services)?;
        
        tracing::info!("Using {} runtime", runtime.runtime_type());
        
        // 设置 API
        runtime.setup_api(&services)?;
        
        // 设置用户配置
        runtime.setup_config(&loaded_mod.user_config)?;
        
        // 执行脚本
        runtime.execute()?;
        
        // 清理
        runtime.cleanup()?;
        
        tracing::info!("Mod executed successfully");
        
        Ok(())
    }
    
    /// 执行多个 mod
    pub fn execute_mods(&self, mods: &[LoadedMod]) -> Result<()> {
        for loaded_mod in mods {
            self.execute_mod(loaded_mod)?;
        }
        Ok(())
    }
}
```

## 📊 优势总结

### 1. 统一接口
```rust
// 调用者不需要关心是 Lua 还是 JavaScript
let mut runtime = RuntimeFactory::create_runtime(&mod_path, services)?;
runtime.setup_api(&services)?;
runtime.execute()?;
```

### 2. 易于扩展
```rust
// 未来添加 Python 支持？
pub struct PythonRuntime { ... }

impl ScriptRuntime for PythonRuntime {
    // 实现 trait 即可
}
```

### 3. 代码复用
```rust
// 所有运行时共享相同的核心服务
pub struct ScriptServices {
    // 文件操作
    // TSV 处理
    // JSON 处理
}
```

### 4. 类型安全
```rust
// 编译时保证所有运行时都实现了必要的方法
trait ScriptRuntime {
    fn execute(&mut self) -> Result<()>;
    // ...
}
```

## 🎯 使用效果

### 透明的运行时选择
```
执行 mod: ExpandedCube (1.0.0)
检测到 JavaScript 脚本: mod.js
使用 JavaScript 运行时
Mod 执行成功

执行 mod: MyLuaMod (1.0.0)
检测到 Lua 脚本: mod.lua
使用 Lua 运行时
Mod 执行成功
```

### 混合使用
```bash
# mod_list.txt
F:/mods/ExpandedCube    # JavaScript mod
F:/mods/StackChanger    # Lua mod
F:/mods/LootFilter      # JavaScript mod

# 一个命令执行所有 mod，自动选择运行时
infinite install --game-path "C:/Games/D2R" --mod-list mod_list.txt
```

## 📦 依赖配置

```toml
[dependencies]
# Lua 运行时
mlua = { version = "0.9", features = ["lua54", "async", "serialize", "vendored"] }

# JavaScript 运行时
quickjs_runtime = { version = "0.12", features = ["typescript"] }

# 其他依赖...
```

## 🚀 实现步骤

1. **创建抽象层** (1天)
   - `script_runtime.rs` - Trait 定义
   - `factory.rs` - 工厂模式

2. **实现 Lua 运行时** (0.5天)
   - 迁移现有代码到新接口

3. **实现 JavaScript 运行时** (2天)
   - 使用 `quickjs_runtime`
   - 实现 D2RMM API

4. **集成和测试** (1天)
   - 修改 ModExecutor
   - 测试两种运行时

5. **文档和示例** (0.5天)

**总计**: 5 天

## ✨ 最终效果

完美的抽象！调用者完全不需要关心底层是 Lua 还是 JavaScript：

```rust
// 简单、清晰、统一
let services = ScriptServices::new(mod_path, output_path, game_path);
let mut runtime = RuntimeFactory::create_runtime(&mod_path, services)?;
runtime.setup_api(&services)?;
runtime.setup_config(&user_config)?;
runtime.execute()?;
```

这就是优雅的设计！🎉
