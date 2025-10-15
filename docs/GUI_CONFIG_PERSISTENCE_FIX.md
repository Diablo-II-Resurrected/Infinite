# GUI 配置选项生效修复

## 问题描述

用户在 GUI 中修改了 Mod 配置选项后，点击"生成 Mods"时，实际使用的仍然是 mod.json 中的默认设置，而不是用户修改后的值。

### 症状
1. 在 GUI 中修改配置选项（如勾选/取消勾选复选框）
2. 点击"生成 Mods"
3. 查看生成结果，发现使用的是默认配置
4. 用户的修改没有生效

## 根本原因

### 问题 1: 配置文件保存位置错误

**错误实现:**
```rust
// GUI 将配置保存到临时目录
let temp_config_dir = std::env::temp_dir().join("infinite_gui_configs");
let config_file = temp_config_dir.join(format!("{}.json", mod_name));
std::fs::write(&config_file, config_json);
```

但是 CLI 调用时没有传递配置目录参数，而且 ModLoader 也不知道这个临时目录。

### 问题 2: ModLoader 总是使用默认配置

**错误实现:**
```rust
// ModLoader::load_mod 方法
let user_config = config.generate_default_config();
```

总是调用 `generate_default_config()`，忽略用户的修改。

## 解决方案

### 1. 配置保存到 Mod 目录

将用户配置直接保存到每个 Mod 目录的 `config.json` 文件中：

```rust
// GUI 中的修改
for (mod_path, user_config) in &enabled_mods {
    if !user_config.is_empty() {
        // 直接保存到 mod 目录的 config.json
        let config_file = PathBuf::from(mod_path).join("config.json");
        if let Ok(config_json) = serde_json::to_string_pretty(user_config) {
            std::fs::write(&config_file, config_json)?;
        }
    }
}
```

### 2. ModLoader 加载用户配置

修改 `ModLoader::load_mod` 方法，检查并加载 `config.json`:

```rust
// 尝试加载用户配置文件 config.json
let user_config_path = mod_path.join("config.json");
let user_config = if user_config_path.exists() {
    // 从 config.json 加载用户配置
    match std::fs::read_to_string(&user_config_path) {
        Ok(config_str) => {
            match serde_json::from_str(&config_str) {
                Ok(cfg) => {
                    println!("Loaded user config from {:?}", user_config_path);
                    cfg
                },
                Err(e) => {
                    eprintln!("Warning: Failed to parse config.json: {}", e);
                    config.generate_default_config()
                }
            }
        },
        Err(_) => config.generate_default_config(),
    }
} else {
    // 如果不存在 config.json，使用默认配置
    config.generate_default_config()
};
```

## 配置文件结构

### Mod 目录结构

```
my_mod/
├── mod.json          # Mod 元数据和配置定义
├── config.json       # 用户配置值（由 GUI 创建）
├── mod.lua           # 或 mod.js
└── ...
```

### config.json 示例

```json
{
  "showOnWeapons": true,
  "showOnArmor": false,
  "showOnJewelry": true,
  "testNumber": 75
}
```

## 工作流程

### 完整流程

1. **用户添加 Mod**
   - GUI 读取 `mod.json`
   - 调用 `init_user_config()` 初始化默认配置到 `user_config`
   - 配置保存到 `gui_config.json`

2. **用户修改配置**
   - 在 GUI 配置面板中修改选项
   - 修改保存到 `ModEntry.user_config`
   - 配置保存到 `gui_config.json`

3. **用户点击"生成 Mods"**
   - GUI 遍历启用的 Mods
   - 将每个 Mod 的 `user_config` 写入 `<mod_path>/config.json`
   - 创建临时 mod 列表文件
   - 调用 CLI: `infinite install --game-path ... --mod-list ...`

4. **CLI 加载 Mod**
   - `ModLoader::load_mod` 读取 `mod.json`
   - 检查是否存在 `config.json`
   - 如果存在，加载用户配置
   - 如果不存在，使用默认配置

5. **Mod 执行**
   - 脚本通过 `D2RMM.getConfiguration()` 或 `config` 对象获取配置
   - 使用的是 `config.json` 中的用户配置

## 修改的文件

### 1. `src/gui/app.rs`

#### 移除临时配置目录
```rust
// ❌ 删除
let temp_config_dir = std::env::temp_dir().join("infinite_gui_configs");
std::fs::create_dir_all(&temp_config_dir)?;
let config_file = temp_config_dir.join(format!("{}.json", mod_name));
```

#### 直接保存到 Mod 目录
```rust
// ✅ 新增
let config_file = PathBuf::from(mod_path).join("config.json");
std::fs::write(&config_file, config_json)?;
```

#### 移除临时目录清理
```rust
// ❌ 删除
let _ = std::fs::remove_dir_all(&temp_config_dir);
```

### 2. `src/mod_manager/loader.rs`

#### 加载用户配置
```rust
// ✅ 新增
let user_config_path = mod_path.join("config.json");
let user_config = if user_config_path.exists() {
    match std::fs::read_to_string(&user_config_path) {
        Ok(config_str) => {
            match serde_json::from_str(&config_str) {
                Ok(cfg) => {
                    println!("Loaded user config from {:?}", user_config_path);
                    cfg
                },
                Err(e) => {
                    eprintln!("Warning: Failed to parse config.json: {}", e);
                    config.generate_default_config()
                }
            }
        },
        Err(_) => config.generate_default_config(),
    }
} else {
    config.generate_default_config()
};
```

## 测试场景

### 测试步骤

1. **准备测试 Mod**
   ```bash
   # 使用 test_mods/show_item_level
   ```

2. **GUI 操作**
   - 打开 GUI
   - 添加 Show Item Level Mod
   - 点击配置按钮 ⚙
   - 修改配置:
     - Show on Weapons: ☑ (勾选)
     - Show on Armor: ☐ (取消勾选)
     - Test Number: 75

3. **生成 Mods**
   - 点击"🚀 生成 Mods"
   - 等待完成

4. **验证配置文件**
   ```bash
   # 检查 config.json 是否创建
   cat test_mods/show_item_level/config.json
   ```

   **预期内容:**
   ```json
   {
     "showOnWeapons": true,
     "showOnArmor": false,
     "showOnJewelry": false,
     "testNumber": 75
   }
   ```

5. **验证日志输出**
   ```
   Loaded user config from "test_mods/show_item_level/config.json"
   Show Item Level Config: { showOnWeapons: true, showOnArmor: false, ... }
   Processing weapons...
   Test number: 75
   ```

### 预期结果

#### 修复前 ❌
- config.json 保存到临时目录 `%TEMP%\infinite_gui_configs\`
- ModLoader 使用默认配置
- 脚本输出显示默认值

#### 修复后 ✅
- config.json 保存到 `test_mods/show_item_level/config.json`
- ModLoader 加载用户配置
- 脚本输出显示用户修改的值

## 优点

### 1. 简单直观
- 配置文件与 Mod 在同一目录
- 易于查看和手动编辑
- 不需要额外的 CLI 参数

### 2. 持久化
- 配置保存在 Mod 目录中
- 不会因为临时文件清理而丢失
- 可以跨 GUI 会话共享配置

### 3. 兼容性
- CLI 命令行用户也可以手动创建 config.json
- GUI 和 CLI 使用相同的配置机制
- 遵循约定优于配置原则

## 注意事项

### 1. GitHub Mods

对于从 GitHub 下载的 Mods，配置会保存到下载缓存目录：
```
~/.cache/infinite/github/<owner>/<repo>/config.json
```

### 2. 配置冲突

如果手动修改了 config.json，GUI 会在下次打开时读取并显示这些值。

### 3. 权限问题

如果 Mod 目录没有写入权限，GUI 会显示警告但不会阻止安装:
```rust
if let Err(e) = std::fs::write(&config_file, config_json) {
    eprintln!("Warning: Failed to write config for {}: {}", mod_path, e);
}
```

## 未来改进

### 1. 配置版本控制
- 记录 config.json 的版本
- 当 mod.json 的配置定义改变时，提示用户更新配置

### 2. 配置验证
- 在加载时验证配置值的有效性
- 对于超出范围的数字，自动修正

### 3. 配置导入/导出
- 支持导出配置预设
- 支持导入他人分享的配置

## 编译测试

```bash
cargo build --bin infinite-gui
# ✅ 编译成功

cargo run --bin infinite-gui
# ✅ 运行正常
```

## 总结

这个修复解决了用户配置不生效的问题。通过将配置保存到 Mod 目录的 `config.json` 文件，并在 ModLoader 中加载它，确保了用户在 GUI 中的修改能够正确传递到 Mod 脚本中。

现在用户可以：
1. ✅ 在 GUI 中修改配置
2. ✅ 配置正确保存到 config.json
3. ✅ CLI 加载并使用用户配置
4. ✅ Mod 脚本使用修改后的值
