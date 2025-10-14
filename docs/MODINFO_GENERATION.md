# modinfo.json 自动生成功能

## 功能说明

在执行 `install` 命令时，系统会自动在输出目录的上一级生成 `modinfo.json` 文件。

### 文件位置

如果输出路径为：
```
<game_path>/Mods/Infinite/Infinite.mpq/data
```

则 `modinfo.json` 会生成在：
```
<game_path>/Mods/Infinite/Infinite.mpq/modinfo.json
```

### 文件内容

```json
{
  "name": "Infinite",
  "savepath": "Infinite/"
}
```

这是 Diablo II: Resurrected 识别 mod 所需的元数据文件。

## 目录结构示例

```
<game_path>/
└── Mods/
    └── Infinite/
        └── Infinite.mpq/
            ├── modinfo.json          ← 自动生成
            └── data/                 ← 输出目录
                ├── global/
                │   └── excel/
                │       └── treasureclassex.txt
                └── local/
                    └── lng/
                        └── strings/
```

## 生成条件

### ✅ 会生成 modinfo.json

```bash
# 正常安装模式
infinite install --game-path "..." --mod-list "..."
```

### ❌ 不会生成 modinfo.json

```bash
# Dry-run 模式（不写入任何文件）
infinite install --game-path "..." --mod-list "..." --dry-run
```

## 技术实现

### 代码位置
`src/main.rs` 的 `install_mods` 函数

### 实现逻辑
```rust
// Generate modinfo.json in parent directory of output_path
if !dry_run {
    if let Some(parent_dir) = std::path::Path::new(output_path).parent() {
        let modinfo_path = parent_dir.join("modinfo.json");
        let modinfo_content = serde_json::json!({
            "name": "Infinite",
            "savepath": "Infinite/"
        });

        match std::fs::create_dir_all(parent_dir) {
            Ok(_) => {
                match std::fs::write(&modinfo_path, serde_json::to_string_pretty(&modinfo_content)?) {
                    Ok(_) => {
                        println!("✅ Generated modinfo.json at: {}", modinfo_path.display());
                    }
                    Err(e) => {
                        eprintln!("⚠️ Failed to write modinfo.json: {}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️ Failed to create directory for modinfo.json: {}", e);
            }
        }
    }
}
```

## 使用示例

### 示例 1: 使用默认输出路径

```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mod-list "./mods.txt"

# 输出：
# C:/Program Files (x86)/Diablo II Resurrected/Mods/Infinite/Infinite.mpq/data/ ← mod文件
# C:/Program Files (x86)/Diablo II Resurrected/Mods/Infinite/Infinite.mpq/modinfo.json ← 自动生成
```

### 示例 2: 使用自定义输出路径

```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mod-list "./mods.txt" \
    --output-path "./output/Infinite.mpq/data"

# 输出：
# ./output/Infinite.mpq/data/ ← mod文件
# ./output/Infinite.mpq/modinfo.json ← 自动生成
```

### 示例 3: Dry-run 模式

```bash
infinite install \
    --game-path "C:/Program Files (x86)/Diablo II Resurrected" \
    --mod-list "./mods.txt" \
    --dry-run

# 不会生成任何文件（包括 modinfo.json）
```

## 测试验证

### 测试命令
```bash
cargo run --bin infinite -- install \
    --game-path "F:/Games/Diablo II Resurrected" \
    --mod-list test_multi_mod.txt \
    --output-path "./output_test/Infinite.mpq/data"
```

### 预期输出
```
💾 Flushing cached modifications...
✅ All modifications written to disk
✅ Generated modinfo.json at: ./output_test/Infinite.mpq\modinfo.json
```

### 验证文件内容
```bash
# 查看生成的文件
cat output_test/Infinite.mpq/modinfo.json
```

应该看到：
```json
{
  "name": "Infinite",
  "savepath": "Infinite/"
}
```

## GUI 集成

GUI 调用 CLI 时会自动生成 `modinfo.json`，无需额外配置。

当使用默认输出路径时：
```
<game_path>/Mods/Infinite/Infinite.mpq/data
```

`modinfo.json` 会自动生成在：
```
<game_path>/Mods/Infinite/Infinite.mpq/modinfo.json
```

## 错误处理

### 目录创建失败
如果无法创建父目录，会显示警告但不会中断整个安装过程：
```
⚠️ Failed to create directory for modinfo.json: <错误信息>
```

### 文件写入失败
如果无法写入文件，会显示警告但不会中断整个安装过程：
```
⚠️ Failed to write modinfo.json: <错误信息>
```

### 关键特性
- ✅ 自动创建所需的父目录
- ✅ 错误不会中断 mod 安装流程
- ✅ dry-run 模式下不生成文件
- ✅ 生成格式化的 JSON（易读）

## 注意事项

1. **覆盖行为**：如果 `modinfo.json` 已存在，会被新内容覆盖
2. **路径要求**：输出路径必须至少有一级父目录
3. **权限要求**：需要对目标目录有写入权限
4. **文件格式**：始终生成固定内容，不可自定义

## 相关文件

- `src/main.rs` - 实现代码
- `output_test/Infinite.mpq/modinfo.json` - 测试生成的示例

## D2R Mod 加载说明

Diablo II: Resurrected 通过 `modinfo.json` 识别 mod：

1. **name**: Mod 的显示名称
2. **savepath**: 存档路径（区分不同 mod 的存档）

正确的目录结构：
```
Mods/
└── <ModName>/
    └── <ModName>.mpq/
        ├── modinfo.json      ← 必需
        └── data/             ← mod 文件
```

本工具自动生成符合此标准的目录结构。

## 总结

- ✅ 自动生成 `modinfo.json`
- ✅ 位置正确（输出目录的上一级）
- ✅ 内容符合 D2R 规范
- ✅ 错误处理完善
- ✅ 支持 dry-run 模式
- ✅ GUI 透明集成

无需手动创建或维护 `modinfo.json`！🎉
