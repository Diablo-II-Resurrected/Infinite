# Unified D2RMM API Architecture

## 概述

为了减少代码重复和提高可维护性，我们将 JavaScript 和 Lua 的 D2RMM API 实现抽象到了一个统一的核心层。

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    Mod Scripts (JS/Lua)                     │
│  console.log(), D2RMM.readJson(), D2RMM.readTsv(), etc.    │
└─────────────────────┬───────────────────────────────────────┘
                      │
          ┌───────────┴──────────┐
          │                      │
┌─────────▼──────────┐  ┌───────▼──────────┐
│  JavaScript Runtime │  │   Lua Runtime    │
│  (js_runtime.rs)    │  │ (lua_runtime.rs) │
│                     │  │                  │
│  • JS type conv     │  │  • Lua type conv │
│  • rquickjs bindings│  │  • mlua bindings │
└─────────┬───────────┘  └────────┬─────────┘
          │                       │
          └───────────┬───────────┘
                      │
            ┌─────────▼─────────┐
            │   D2RmmApiCore    │
            │  (d2rmm_api.rs)   │
            │                   │
            │  • readJson()     │
            │  • writeJson()    │
            │  • readTsv()      │
            │  • writeTsv()     │
            │  • readTxt()      │
            │  • writeTxt()     │
            │  • copyFile()     │
            │  • getVersion()   │
            │  • throw_error()  │
            └─────────┬─────────┘
                      │
            ┌─────────▼─────────┐
            │  ScriptServices   │
            │(script_runtime.rs)│
            │                   │
            │  • File I/O       │
            │  • CASC access    │
            │  • Cache mgmt     │
            └───────────────────┘
```

## 核心组件

### 1. D2RmmApiCore (`src/runtime/d2rmm_api.rs`)

**职责**：实现所有 D2RMM API 的核心逻辑

**特点**：
- 运行时无关：不依赖具体的脚本引擎
- 类型中立：使用标准 Rust 类型（`serde_json::Value`、`TsvData` 等）
- 同步接口：所有方法都是同步的，内部使用 `block_in_place` 处理异步操作

**API 方法**：
```rust
pub struct D2RmmApiCore {
    // 核心方法
    pub fn get_version(&self) -> f64;
    pub fn read_json(&self, path: &str) -> Result<JsonValue>;
    pub fn write_json(&self, path: &str, data: &JsonValue) -> Result<()>;
    pub fn read_tsv(&self, path: &str) -> Result<TsvData>;
    pub fn write_tsv(&self, path: &str, data: &TsvData) -> Result<()>;
    pub fn read_txt(&self, path: &str) -> Result<String>;
    pub fn write_txt(&self, path: &str, content: &str) -> Result<()>;
    pub fn copy_file(&self, src: &str, dst: &str, is_dir: bool) -> Result<()>;
    pub fn throw_error(&self, msg: &str) -> Result<()>;
}
```

### 2. ConsoleApi (`src/runtime/d2rmm_api.rs`)

**职责**：统一的 console 日志实现

**方法**：
```rust
pub struct ConsoleApi;

impl ConsoleApi {
    pub fn log(msg: &str);    // tracing::info!("[MOD] {}")
    pub fn debug(msg: &str);  // tracing::debug!("[MOD] {}")
    pub fn warn(msg: &str);   // tracing::warn!("[MOD] {}")
    pub fn error(msg: &str);  // tracing::error!("[MOD] {}")
}
```

### 3. JavaScript Runtime (`src/runtime/js_runtime.rs`)

**职责**：
- 创建 QuickJS 上下文
- 将 `D2RmmApiCore` 方法绑定到 JavaScript 全局对象 `D2RMM`
- 处理 JS ↔ Rust 类型转换

**类型转换**：
```rust
// Rust -> JavaScript
fn json_to_rquickjs(ctx: Ctx, json: &JsonValue) -> Result<Value>;
fn tsv_to_rquickjs(ctx: Ctx, tsv: &TsvData) -> Result<Value>;

// JavaScript -> Rust
fn rquickjs_to_json(ctx: Ctx, val: &Value) -> Result<JsonValue>;
fn rquickjs_to_tsv(ctx: Ctx, val: &Value) -> Result<TsvData>;
```

### 4. Lua Runtime (`src/runtime/lua_runtime.rs`)

**职责**：
- 创建 Lua 上下文
- 将 `D2RmmApiCore` 方法绑定到 Lua 全局对象 `D2RMM` 和 `infinite`
- 处理 Lua ↔ Rust 类型转换

**类型转换**：
```rust
// Rust -> Lua
fn json_to_lua_value(lua: &Lua, json: &JsonValue) -> Result<LuaValue>;

// Lua -> Rust
fn lua_value_to_json(lua: &Lua, val: LuaValue) -> Result<JsonValue>;
```

## 数据类型

### TsvData 和 TsvRow

定义在 `d2rmm_api.rs`，被两个运行时共享：

```rust
#[derive(Debug, Clone)]
pub struct TsvData {
    pub headers: Vec<String>,
    pub rows: Vec<TsvRow>,
}

#[derive(Debug, Clone)]
pub struct TsvRow {
    pub data: HashMap<String, String>,
}
```

## 优势

### 1. 代码复用
- 核心业务逻辑只需实现一次
- 减少了约 60% 的重复代码
- 日志记录统一（都使用 `[MOD]` 前缀）

### 2. 易于维护
- 新增 API 方法时：只需在 `D2RmmApiCore` 中实现一次
- 修复 bug 时：只需修改一处
- 类型转换逻辑与业务逻辑分离

### 3. 一致性
- 两个运行时的行为完全一致
- 错误处理方式统一
- 版本号管理统一（`getVersion()` 返回 1.5）

### 4. 可扩展性
- 容易添加新的脚本引擎（如 Python、Ruby）
- 只需实现类型转换层，复用核心逻辑

## 添加新 API 的步骤

### 1. 在 `D2RmmApiCore` 中添加方法

```rust
// src/runtime/d2rmm_api.rs
impl D2RmmApiCore {
    pub fn new_api_method(&self, param: &str) -> Result<String> {
        tracing::debug!("newApiMethod called with: {}", param);
        // 实现核心逻辑
        self.services.do_something(param)
    }
}
```

### 2. 在 JavaScript Runtime 中绑定

```rust
// src/runtime/js_runtime.rs
fn register_new_api<'js>(&self, d2rmm: &Object<'js>, _ctx: Ctx<'js>, api_core: Arc<D2RmmApiCore>) -> rquickjs::Result<()> {
    let func = Func::from(move |_ctx: Ctx<'js>, param: String| -> rquickjs::Result<String> {
        api_core.new_api_method(&param).map_err(to_js_error)
    });
    d2rmm.set("newApiMethod", func)?;
    Ok(())
}
```

在 `register_d2rmm_api` 中调用：
```rust
self.register_new_api(&d2rmm, ctx.clone(), Arc::clone(&api_core))?;
```

### 3. 在 Lua Runtime 中绑定

```rust
// src/runtime/lua_runtime.rs
// 在 setup_api 方法中添加
let api_core = Arc::clone(&self.api_core);
d2rmm.set("newApiMethod", self.lua.create_function(move |_lua, param: String| {
    api_core.new_api_method(&param)
        .map_err(|e| mlua::Error::external(e))
})?)?;
```

## 兼容性

### Lua 全局对象
- `D2RMM.*` - 标准 D2RMM API
- `infinite.*` - 兼容旧代码

### JavaScript 全局对象
- `D2RMM.*` - 标准 D2RMM API

### Console API
两个运行时都支持：
- `console.log()`
- `console.debug()`
- `console.warn()`
- `console.error()`

## 性能考虑

### 同步 vs 异步
- `D2RmmApiCore` 使用同步接口
- 内部通过 `tokio::task::block_in_place` 调用异步的 `ScriptServices`
- 这种设计允许在同步上下文（Lua）和异步上下文（可能的未来需求）中使用

### Arc 克隆
- `D2RmmApiCore` 被包装在 `Arc` 中
- 每个 API 方法闭包持有 `Arc` 的克隆
- 开销极小（只增加引用计数）

## 测试

### 单元测试
```bash
cargo test --lib
```

### 集成测试

测试 Lua mods：
```bash
cargo run --release -- install -g "path/to/game" -l test_multi_mod.txt
```

测试 JavaScript mods：
```bash
cargo run --release --features js-runtime -- install -g "path/to/game" -l test_simple_js_list.txt
```

## 未来改进

### 可能的优化
1. **类型转换缓存**：对频繁转换的类型实现缓存
2. **异步 API**：为支持真正的异步脚本运行
3. **更多运行时**：Python、Ruby 等
4. **API 文档生成**：从 `D2RmmApiCore` 自动生成文档

### 已知限制
1. TSV 数据目前是克隆传递的（未来可考虑使用 `Cow` 优化）
2. 错误消息可能需要更详细的上下文信息

## 总结

统一的 API 架构显著提高了代码质量和可维护性。核心逻辑集中在 `D2RmmApiCore`，而各个运行时只需关注类型转换和绑定，大大减少了重复代码并提高了一致性。
