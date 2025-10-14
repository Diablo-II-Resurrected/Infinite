# output-path 可选参数更新

## 变更说明

`--output-path` 参数现在是**可选的**。如果不指定，将自动使用默认路径。

### 默认输出路径

```
<game_path>/Mods/Infinite/Infinite.mpq/data
```

### 使用示例

#### 1. 使用默认输出路径

```bash
# 不指定 --output-path，自动使用默认路径
infinite install --game-path "F:/Games/Diablo II Resurrected" --mod-list mods.txt

# 输出将自动保存到：
# F:/Games/Diablo II Resurrected/Mods/Infinite/Infinite.mpq/data
```

#### 2. 自定义输出路径

```bash
# 如果需要，仍然可以指定自定义路径
infinite install --game-path "F:/Games/Diablo II Resurrected" --mod-list mods.txt --output-path "D:/CustomOutput"
```

## GUI 变更

GUI 已自动适配此变更：
- GUI 不再传递 `--output-path` 参数
- 所有 mods 将自动生成到游戏目录下的标准位置
- 用户无需关心输出路径配置

## 技术实现

### 1. CLI 参数定义 (commands.rs)

```rust
/// Path to the output directory (defaults to <game_path>/Mods/Infinite/Infinite.mpq/data)
#[arg(short, long)]
output_path: Option<String>,
```

### 2. 默认路径处理 (main.rs)

```rust
// Use default output path if not specified
let output = output_path.unwrap_or_else(|| {
    format!("{}/Mods/Infinite/Infinite.mpq/data", game_path)
});
```

### 3. GUI 调用 (app.rs)

```rust
// 不再传递 --output-path，使用默认路径
let result = std::process::Command::new(&cli_exe)
    .args(&[
        "install",
        "--game-path",
        &game_path,
        "--mod-list",
        temp_list.to_str().unwrap(),
    ])
    .output();
```

## 优势

1. **简化使用**：大多数用户不需要关心输出路径
2. **标准化**：所有用户的 mods 都在统一位置
3. **兼容性**：仍然支持自定义路径（高级用户）
4. **GUI 友好**：GUI 自动使用正确路径，无需配置

## 迁移指南

### 现有脚本/命令

如果你的脚本中使用了 `--output-path` 参数：

**之前**：
```bash
infinite install --game-path "..." --mod-list "..." --output-path "..."
```

**现在（两种方式都可以）**：
```bash
# 方式1：不指定，使用默认路径（推荐）
infinite install --game-path "..." --mod-list "..."

# 方式2：继续指定自定义路径（兼容旧脚本）
infinite install --game-path "..." --mod-list "..." --output-path "..."
```

## 测试验证

```bash
# 测试默认路径
cargo run --bin infinite -- install --game-path "F:/Games/Diablo II Resurrected" --mod-list test_multi_mod.txt --dry-run

# 输出显示：
# Output: F:/Games/Diablo II Resurrected/Mods/Infinite/Infinite.mpq/data
```

## 相关文件

- `src/cli/commands.rs` - 参数定义
- `src/main.rs` - 默认值处理
- `src/gui/app.rs` - GUI 调用逻辑

## 注意事项

1. **路径格式**：默认路径使用正斜杠 `/`，在 Windows 上会自动转换
2. **目录创建**：输出目录会自动创建，无需手动建立
3. **向后兼容**：旧命令仍然有效，`--output-path` 仍然可用
4. **GUI 行为**：GUI 始终使用默认路径，不提供自定义选项

## 相关文档

- [GUI_README.md](./GUI_README.md) - GUI 使用说明
- [BUILD_AND_RUN.md](../BUILD_AND_RUN.md) - 构建和运行指南
- [QUICKSTART.md](../QUICKSTART.md) - 快速开始
