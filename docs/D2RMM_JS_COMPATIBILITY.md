# D2RMM JavaScript Mod 兼容性方案

## 📋 当前状况

### Infinite 现状
- **脚本语言**: Lua 5.4
- **API**: `infinite.*` 全局对象
- **文件**: `mod.lua`
- **运行时**: mlua (Rust 嵌入的 Lua)

### D2RMM 格式
- **脚本语言**: JavaScript
- **API**: `D2RMM.*` 全局对象
- **文件**: `mod.js`
- **运行时**: QuickJS (沙箱 JS 引擎)

## 🎯 兼容性方案

### 方案 1: JavaScript 运行时集成 ⭐ 推荐

#### 技术选型

**选项 A: QuickJS** (与 D2RMM 相同)
```toml
[dependencies]
rquickjs = "0.6"  # QuickJS Rust 绑定
```

**优势**:
- ✅ 与 D2RMM 100% 兼容 (相同引擎)
- ✅ 轻量级 (~1MB)
- ✅ 启动快
- ✅ 内存占用低
- ✅ 支持 ES2020

**劣势**:
- ❌ 生态较小
- ❌ 不支持所有 Node.js API

**选项 B: Deno Core**
```toml
[dependencies]
deno_core = "0.281"
```

**优势**:
- ✅ 现代 JS/TS 支持
- ✅ 安全沙箱
- ✅ V8 引擎性能好

**劣势**:
- ❌ 体积大 (~20MB)
- ❌ 复杂度高
- ❌ 启动较慢

**推荐: QuickJS (rquickjs)**

### 方案 2: 自动转换 Lua → JS

创建一个转换层，将 D2RMM 的 JavaScript 代码自动转换为 Lua。

**挑战**:
- ❌ JavaScript 语法复杂
- ❌ 动态特性难以转换
- ❌ 维护成本高
- ❌ 不能保证 100% 兼容

**不推荐**：转换方案不可靠

### 方案 3: 双运行时支持 ⭐⭐ 最佳方案

同时支持 Lua 和 JavaScript：
- **Lua 脚本**: 使用 mlua (已有)
- **JS 脚本**: 使用 QuickJS (新增)
- **自动检测**: 根据 `mod.lua` 或 `mod.js` 选择运行时

## 🚀 实现方案 3 (推荐)

### 架构设计

```
Infinite Mod Manager (Rust)
    ├── Lua Runtime (mlua)
    │   └── API: infinite.*
    │   └── 执行: mod.lua
    │
    └── JavaScript Runtime (rquickjs)
        └── API: D2RMM.*
        └── 执行: mod.js
```

### 依赖添加

```toml
# Cargo.toml
[dependencies]
# 现有 Lua 支持
mlua = { version = "0.9", features = ["lua54", "async", "serialize", "vendored"] }

# 新增 JavaScript 支持
rquickjs = { version = "0.6", features = ["array-buffer", "loader"] }
```

### 代码结构

```rust
// src/runtime/mod.rs
pub enum ScriptRuntime {
    Lua(LuaContext),
    JavaScript(JSContext),
}

impl ScriptRuntime {
    pub fn from_mod(mod_path: &Path) -> Result<Self> {
        if mod_path.join("mod.lua").exists() {
            Ok(Self::Lua(LuaContext::new(mod_path)?))
        } else if mod_path.join("mod.js").exists() {
            Ok(Self::JavaScript(JSContext::new(mod_path)?))
        } else {
            bail!("No mod.lua or mod.js found")
        }
    }
    
    pub fn execute(&mut self) -> Result<()> {
        match self {
            Self::Lua(ctx) => ctx.execute(),
            Self::JavaScript(ctx) => ctx.execute(),
        }
    }
}
```

### API 映射

```rust
// src/runtime/js_context.rs
use rquickjs::{Context, Runtime};

pub struct JSContext {
    runtime: Runtime,
    context: Context,
}

impl JSContext {
    pub fn new(mod_path: &Path) -> Result<Self> {
        let runtime = Runtime::new()?;
        let context = Context::full(&runtime)?;
        
        // 注入 D2RMM API
        context.with(|ctx| {
            let global = ctx.globals();
            
            // D2RMM.readJson(path)
            global.set("D2RMM", {
                let obj = rquickjs::Object::new(ctx)?;
                obj.set("readJson", rquickjs::Function::new(ctx, |path: String| {
                    // 调用 Rust 的文件读取逻辑
                    read_json_file(&path)
                }))?;
                
                obj.set("writeJson", rquickjs::Function::new(ctx, |path: String, data: Value| {
                    write_json_file(&path, &data)
                }))?;
                
                // 其他 API...
                obj
            })?;
            
            Ok(())
        })?;
        
        Ok(Self { runtime, context })
    }
    
    pub fn execute(&mut self) -> Result<()> {
        let script = std::fs::read_to_string("mod.js")?;
        self.context.with(|ctx| {
            ctx.eval::<(), _>(script)?;
            Ok(())
        })
    }
}
```

### D2RMM API 完整映射

| D2RMM API | 对应功能 | 实现状态 |
|-----------|---------|---------|
| `D2RMM.readJson(path)` | 读取 JSON | 需实现 |
| `D2RMM.writeJson(path, data)` | 写入 JSON | 需实现 |
| `D2RMM.readTsv(path)` | 读取 TSV | 需实现 |
| `D2RMM.writeTsv(path, data)` | 写入 TSV | 需实现 |
| `D2RMM.readTxt(path)` | 读取文本 | 需实现 |
| `D2RMM.writeTxt(path, data)` | 写入文本 | 需实现 |
| `D2RMM.copyFile(src, dst, overwrite)` | 复制文件 | 需实现 |
| `D2RMM.getVersion()` | 获取版本 | 需实现 |
| `config.*` | 用户配置 | 需实现 |

## 📝 实现步骤

### 第一阶段: 基础架构 (1-2天)

1. **添加 rquickjs 依赖**
   ```bash
   cargo add rquickjs --features array-buffer,loader
   ```

2. **创建 JS 运行时模块**
   - `src/runtime/js_context.rs` - JavaScript 上下文
   - `src/runtime/js_api.rs` - D2RMM API 实现

3. **修改 ModLoader**
   - 检测 `mod.js` 或 `mod.lua`
   - 选择对应的运行时

### 第二阶段: API 实现 (2-3天)

4. **实现 D2RMM API**
   - 文件操作 (readJson, writeJson, readTsv, writeTsv)
   - 配置访问 (config.*)
   - 工具函数 (copyFile, getVersion)

5. **测试兼容性**
   - 使用真实的 D2RMM mod 测试
   - 验证 API 行为一致性

### 第三阶段: 优化和文档 (1-2天)

6. **性能优化**
   - 缓存 JS 运行时
   - 优化 API 调用

7. **文档更新**
   - 更新 README 说明支持 JS
   - 添加 JS mod 示例
   - 编写迁移指南

## 🎯 预期成果

### 功能特性

✅ **完全兼容 D2RMM mod**
- 支持 `mod.js` 脚本
- 支持 `D2RMM.*` API
- 支持 JavaScript ES2020 语法

✅ **双运行时支持**
- Lua mod: 使用 `mod.lua`
- JS mod: 使用 `mod.js`
- 自动检测和选择

✅ **性能保持**
- QuickJS 轻量级 (~1MB)
- 启动时间增加 <100ms
- 内存占用增加 ~5MB

### 使用示例

#### 方式 1: 直接使用 D2RMM mod

```bash
# 下载 D2RMM mod (包含 mod.js)
infinite install --game-path "C:/Games/D2R" --mod-list mods.txt

# mods.txt 内容:
# F:/mods/ExpandedCube  <- 包含 mod.js (D2RMM 格式)
# F:/mods/MyLuaMod      <- 包含 mod.lua (Infinite 格式)
```

#### 方式 2: 混合使用

一个项目同时包含 Lua 和 JS mod：

```
my_mod_pack/
├── mod1/
│   ├── mod.json
│   └── mod.lua      <- Infinite Lua mod
├── mod2/
│   ├── mod.json
│   └── mod.js       <- D2RMM JavaScript mod
└── mod3/
    ├── mod.json
    └── mod.lua
```

## 📊 工作量评估

| 任务 | 预计时间 | 难度 |
|------|---------|------|
| 添加 rquickjs 依赖 | 0.5天 | 低 |
| JS 运行时基础架构 | 1天 | 中 |
| D2RMM API 实现 | 2天 | 中 |
| 测试和调试 | 1天 | 中 |
| 文档编写 | 0.5天 | 低 |
| **总计** | **5天** | **中等** |

## 🔍 风险评估

### 技术风险

🟡 **中等风险: API 语义差异**
- D2RMM 的某些 API 行为可能有细微差异
- 缓解: 详细测试真实 mod

🟢 **低风险: 依赖冲突**
- rquickjs 与 mlua 可以共存
- 缓解: 分离模块

🟢 **低风险: 性能影响**
- QuickJS 轻量级
- 缓解: 性能测试

### 维护风险

🟡 **中等风险: 双 API 维护**
- 需要维护两套 API (Lua + JS)
- 缓解: 共享底层实现

🟢 **低风险: 文档复杂度**
- 需要两套示例
- 缓解: 清晰的文档结构

## 🎨 备选方案

### 备选 1: Lua 桥接层

为 D2RMM mod 创建一个 Lua 包装器：

```lua
-- d2rmm_compat.lua
D2RMM = {
    readJson = function(path)
        return infinite.readJson(path)
    end,
    -- ...
}

-- 加载 mod.js 的 Lua 版本
dofile("mod.lua")
```

**问题**: 需要手动转换 JS 代码

### 备选 2: 仅文档说明

提供详细的迁移文档，让用户手动转换 mod：

```markdown
# JS → Lua 迁移指南

## 1. 文件重命名
mod.js → mod.lua

## 2. 语法转换
const x = 10;  →  local x = 10

## 3. API 重命名
D2RMM.readJson  →  infinite.readJson
```

**问题**: 用户体验差，迁移成本高

## 💡 推荐决策

### ⭐ 推荐实现方案 3

**理由**:
1. **用户友好**: 直接支持 D2RMM mod，无需转换
2. **生态兼容**: 可以利用现有的 D2RMM mod 社区
3. **技术可行**: QuickJS 轻量级，集成简单
4. **性能可接受**: 体积和性能影响小
5. **未来扩展**: 为其他脚本语言留下接口

### 实现优先级

1. **第一步**: 实现基础 JS 运行时 (1-2天)
2. **第二步**: 实现核心 API (readJson, writeJson, readTsv, writeTsv) (1-2天)
3. **第三步**: 测试真实 D2RMM mod (ExpandedCube 等) (1天)
4. **第四步**: 完善文档和示例 (0.5天)

### 成功标准

✅ 能成功运行附件中的 ExpandedCube mod.js
✅ 所有 D2RMM API 正常工作
✅ 性能影响 <10%
✅ 文档完整清晰

## 📚 参考资源

- [rquickjs 文档](https://docs.rs/rquickjs/)
- [QuickJS 官网](https://bellard.org/quickjs/)
- [D2RMM 源码](https://github.com/olegbl/d2rmm)
- [mlua 文档](https://docs.rs/mlua/)

## 🚦 下一步行动

如果决定实施，建议按以下顺序进行：

1. ✅ 创建功能分支: `feature/js-runtime`
2. ✅ 添加 rquickjs 依赖
3. ✅ 实现基础 JS 上下文
4. ✅ 逐个实现 D2RMM API
5. ✅ 使用真实 mod 测试
6. ✅ 合并到主分支
7. ✅ 发布新版本

预计总工作量: **5-7天**
