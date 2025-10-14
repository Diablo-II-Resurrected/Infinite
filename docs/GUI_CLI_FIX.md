# GUI调用CLI修复

## 问题

点击"生成Mods"按钮时，会打开另一个GUI窗口而不是执行mod生成。

### 原因

代码使用了 `std::env::current_exe()` 来调用CLI工具，这会返回当前正在运行的程序路径。
当运行 `infinite-gui.exe` 时，`current_exe()` 返回的是 `infinite-gui.exe` 自己，而不是 `infinite.exe`。

```rust
// ❌ 错误的代码
let result = std::process::Command::new(std::env::current_exe()...)
    .args(&["install", ...])
```

这导致GUI调用自己，打开了新的GUI窗口。

## 解决方案

修改代码查找正确的 `infinite.exe` CLI工具：

```rust
// ✅ 修复后的代码
let cli_exe = if let Ok(current_exe) = std::env::current_exe() {
    // 在GUI程序所在目录查找infinite.exe
    let exe_dir = current_exe.parent().unwrap();
    let infinite_exe = exe_dir.join("infinite.exe");
    if infinite_exe.exists() {
        infinite_exe
    } else {
        // 如果找不到，尝试使用PATH中的infinite命令
        std::path::PathBuf::from("infinite")
    }
} else {
    std::path::PathBuf::from("infinite")
};

let result = std::process::Command::new(&cli_exe)
    .args(&["install", ...])
```

### 查找逻辑

1. **获取GUI程序所在目录**
2. **在同目录查找 `infinite.exe`**
   - 开发环境：`target/debug/infinite.exe`
   - 发布环境：两个exe应该在同一目录
3. **如果找不到，回退到PATH查找**
   - 尝试系统PATH中的 `infinite` 命令

## 部署要求

### 开发环境

两个可执行文件在同一目录：
```
target/debug/
├── infinite.exe       # CLI工具
└── infinite-gui.exe   # GUI程序
```

### 发布环境

打包时需要包含两个文件：
```
Infinite/
├── infinite.exe       # CLI工具
└── infinite-gui.exe   # GUI程序
```

**重要**：两个exe必须在同一目录！

## 构建说明

### 构建两个版本

```bash
# 构建CLI版本
cargo build --release --bin infinite

# 构建GUI版本
cargo build --release --bin infinite-gui
```

### 构建所有二进制文件

```bash
# 一次构建所有
cargo build --release
```

这会生成：
- `target/release/infinite.exe`
- `target/release/infinite-gui.exe`

## 测试

### 1. 确认两个exe都存在

```powershell
# 开发环境
Get-ChildItem target\debug\infinite*.exe

# 发布环境
Get-ChildItem target\release\infinite*.exe
```

应该看到：
```
infinite.exe
infinite-gui.exe
```

### 2. 测试GUI调用CLI

1. 运行GUI：`cargo run --bin infinite-gui`
2. 选择游戏路径
3. 添加一些mod
4. 点击 **🚀 生成Mods**
5. 应该看到处理进度，**不会**打开新窗口

### 3. 验证日志

如果有问题，可以查看终端输出，会显示调用的命令。

## 替代方案

如果不想依赖外部CLI可执行文件，可以考虑：

### 方案1：直接调用库代码（推荐）

不通过外部进程，直接调用共享代码：

```rust
// 将mod处理逻辑提取到库中
use infinite::mod_manager;

// 在GUI中直接调用
tokio::runtime::Runtime::new().unwrap().block_on(async {
    mod_manager::install_mods(game_path, mods, output_path).await
});
```

优点：
- 无需外部exe
- 更快的执行
- 更好的错误处理
- 可以获取实时进度

缺点：
- 需要重构代码结构
- GUI需要包含更多依赖

### 方案2：嵌入CLI到GUI

将CLI作为子命令集成：

```rust
match args {
    GuiArgs => run_gui(),
    CliArgs => run_cli(),
}
```

优点：
- 只需一个exe
- 简化部署

缺点：
- exe文件更大
- 代码耦合

## 当前实现

当前使用**外部进程调用**方案：
- ✅ 简单直接
- ✅ GUI和CLI完全分离
- ✅ 可以独立使用CLI
- ⚠️ 需要确保两个exe在同一目录

## 故障排查

### 问题：点击生成按钮没反应

检查：
1. `infinite.exe` 是否存在于同一目录？
2. 查看状态栏错误消息
3. 检查终端输出

### 问题：找不到infinite.exe

解决：
```bash
# 确保CLI已构建
cargo build --bin infinite

# 检查文件
ls target/debug/infinite.exe
```

### 问题：权限错误

Windows可能需要：
- 以管理员身份运行
- 添加防火墙例外
- 允许程序执行

## 相关文件

- `src/gui/app.rs` - GUI应用逻辑（已修复）
- `src/main.rs` - CLI入口点
- `src/gui/main.rs` - GUI入口点

## 更新日志

- **2025-10-14**: 修复GUI调用自己而不是CLI的问题
- 添加智能查找infinite.exe的逻辑
- 改进错误处理和回退机制
