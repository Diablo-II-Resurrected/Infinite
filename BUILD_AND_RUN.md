# 构建和使用指南

## 📋 前置要求

### Windows
- Rust 工具链 (推荐使用 rustup)
- Visual Studio Build Tools 或完整的 Visual Studio
- PowerShell

### macOS / Linux
- Rust 工具链 (推荐使用 rustup)
- GCC 或 Clang
- 标准构建工具

## 🔧 安装 Rust

如果还没有安装 Rust，请访问 [rustup.rs](https://rustup.rs/) 并按照说明安装。

```bash
# 验证安装
rustc --version
cargo --version
```

## 🏗️ 构建项目

### Debug 构建 (开发用)
```bash
cd infinite
cargo build
```

编译后的二进制文件位于: `target/debug/infinite` (或 Windows 上的 `infinite.exe`)

### Release 构建 (生产用)
```bash
cd infinite
cargo build --release
```

编译后的二进制文件位于: `target/release/infinite` (或 Windows 上的 `infinite.exe`)

**注意**: Release 构建会启用所有优化，速度更快但编译时间更长。

## ▶️ 运行

### 方式 1: 直接运行 (开发中)
```bash
# 列出 mods
cargo run -- list --mods-path ./mods

# 安装 mods
cargo run -- install \
    --game-path "/path/to/D2R" \
    --mods-path ./mods \
    --output-path ./output
```

### 方式 2: 运行编译后的二进制
```bash
# Windows
.\target\release\infinite.exe list --mods-path .\mods

# macOS / Linux
./target/release/infinite list --mods-path ./mods
```

### 方式 3: 安装到系统 (可选)
```bash
cargo install --path .
```

然后可以在任何地方使用:
```bash
infinite list --mods-path ./mods
```

## 📝 命令示例

### 1. 列出所有可用的 mods
```bash
infinite list --mods-path ./mods
```

### 2. 验证 mod 配置
```bash
infinite validate --mod-path ./mods/MyMod
```

### 3. 安装 mods (Dry Run - 不实际写入文件)
```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path ./mods \
    --output-path ./output \
    --dry-run
```

### 4. 实际安装 mods
```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path ./mods \
    --output-path ./output
```

### 5. 启用详细日志
```bash
infinite install --verbose \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path ./mods \
    --output-path ./output
```

## 🧪 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_load_mod

# 显示详细输出
cargo test -- --nocapture
```

## 📦 示例 Mods

项目包含两个示例 mods：

### 1. Simple Example
位置: `examples/simple_example/`

最简单的示例，演示基本 API 使用。

```bash
# 测试
cargo run -- validate --mod-path ./examples/simple_example

cargo run -- install \
    --game-path "." \
    --mods-path ./examples \
    --output-path ./test_output \
    --dry-run
```

### 2. Stack Size Changer
位置: `examples/stack_size_changer/`

实际的游戏 mod 示例，修改物品堆叠大小。

## 🛠️ 开发工具

### 格式化代码
```bash
cargo fmt
```

### 检查代码 (不编译)
```bash
cargo check
```

### Clippy (代码质量检查)
```bash
cargo clippy
```

### 文档生成
```bash
cargo doc --open
```

## 📂 创建你的第一个 Mod

### 步骤 1: 创建目录
```bash
mkdir -p mods/MyMod
cd mods/MyMod
```

### 步骤 2: 创建 mod.json
```json
{
  "name": "My First Mod",
  "description": "This is my first mod",
  "author": "Your Name",
  "version": "1.0.0",
  "config": [
    {
      "type": "checkbox",
      "id": "enabled",
      "name": "Enable Mod",
      "description": "Enable or disable this mod",
      "default": true
    }
  ]
}
```

### 步骤 3: 创建 mod.lua
```lua
-- 检查版本
if infinite.getVersion() < 1.5 then
    infinite.error("需要 infinite 1.5 或更高版本!")
end

-- 记录日志
console.log("正在安装 My First Mod...")

-- 检查配置
if config.enabled then
    console.log("Mod 已启用!")
    
    -- 在这里添加你的 mod 逻辑
    -- 例如：读取和修改游戏文件
    
    console.log("Mod 安装成功!")
else
    console.log("Mod 已禁用")
end
```

### 步骤 4: 验证和测试
```bash
# 验证配置
infinite validate --mod-path ./mods/MyMod

# 干运行测试
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mods-path ./mods \
    --output-path ./output \
    --dry-run
```

## 🐛 故障排除

### 问题: 编译错误 "mlua-sys build failed"
**解决方案**: 确保已安装 Visual Studio Build Tools (Windows) 或 GCC/Clang (macOS/Linux)

### 问题: 找不到 Lua 库
**解决方案**: 项目使用 `vendored` 特性，会自动编译 Lua。如果仍有问题，尝试清理并重新构建:
```bash
cargo clean
cargo build --release
```

### 问题: "CASC extraction not yet implemented"
这是预期的 - CASC 文件提取功能尚未实现。目前需要预先提取游戏文件到输出目录。

### 问题: Mod 执行失败
1. 检查 mod.lua 语法错误
2. 使用 `--verbose` 标志查看详细日志
3. 确保所有必需的文件路径正确

## 🔗 有用的链接

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Lua 5.4 手册](https://www.lua.org/manual/5.4/)
- [mlua 文档](https://docs.rs/mlua/)
- [原版 infinite](https://github.com/olegbl/infinite)
- [D2R Modding Discord](https://discord.gg/diablo2resurrected)

## 💬 获取帮助

遇到问题？

1. 查看 [QUICKSTART.md](./QUICKSTART.md)
2. 查看 [README.md](./README.md) 中的 API 文档
3. 查看示例 mods
4. 在 GitHub 上提 Issue

## 🎓 学习资源

### Rust
- [Rust 程序设计语言](https://doc.rust-lang.org/book/) (官方书籍)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

### Lua
- [Programming in Lua](https://www.lua.org/pil/) (官方书籍)
- [Learn Lua in 15 Minutes](https://learnxinyminutes.com/docs/lua/)

### D2R Modding
- 原版 infinite 文档
- D2R Modding 社区资源

---

祝你 Modding 愉快！ 🎮🦀
