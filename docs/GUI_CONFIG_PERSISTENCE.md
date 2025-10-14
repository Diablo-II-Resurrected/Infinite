# GUI 配置持久化功能

## 功能说明

GUI 现在会自动记住上次的选择，包括：
- 游戏路径
- Mod 列表（包括每个 mod 的启用/禁用状态）

下次打开 GUI 时，会自动恢复上次的设置。

## 配置文件位置

配置文件自动保存在系统的标准配置目录：

### Windows
```
%APPDATA%\infinite\gui_config.json
```
通常是：
```
C:\Users\<用户名>\AppData\Roaming\infinite\gui_config.json
```

### Linux
```
~/.config/infinite/gui_config.json
```

### macOS
```
~/Library/Application Support/infinite/gui_config.json
```

## 配置文件格式

```json
{
  "game_path": "F:/Games/Diablo II Resurrected",
  "mods": [
    {
      "path": "F:/Projects/d2rmm/d2rmm-cli/test_mods/mod_a",
      "enabled": true,
      "name": "Test Mod A - Add Item"
    },
    {
      "path": "F:/Projects/d2rmm/d2rmm-cli/test_mods/mod_b",
      "enabled": false,
      "name": "Test Mod B - Add Another Item"
    }
  ]
}
```

## 自动保存时机

配置会在以下情况下自动保存：

1. ✅ **选择游戏路径** - 选择游戏目录后立即保存
2. ✅ **打开Mod列表** - 成功加载 mod 列表后保存
3. ✅ **添加Mod文件夹** - 添加新 mod 后保存
4. ✅ **删除Mod** - 删除 mod 后保存
5. ✅ **调整Mod顺序** - 上移或下移 mod 后保存
6. ✅ **启用/禁用Mod** - 切换复选框状态后保存

## 技术实现

### 配置结构

```rust
#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    game_path: String,
    mods: Vec<ModEntry>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ModEntry {
    path: String,
    enabled: bool,
    name: String,
}
```

### 核心方法

```rust
impl AppConfig {
    /// 获取配置文件路径
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("infinite");
        path.push("gui_config.json");
        path
    }

    /// 从文件加载配置
    fn load() -> Self {
        // 尝试读取配置文件
        // 失败则返回默认值
    }

    /// 保存配置到文件
    fn save(&self) -> std::io::Result<()> {
        // 创建目录
        // 序列化为 JSON
        // 写入文件
    }
}
```

### 应用启动

```rust
impl InfiniteApp {
    pub fn new() -> Self {
        // 自动加载保存的配置
        let config = AppConfig::load();
        
        Self {
            game_path: config.game_path,
            mods: config.mods,
            // ...
        }
    }
}
```

## 使用体验

### 首次使用

1. 打开 GUI
2. 选择游戏路径
3. 添加或加载 mod 列表
4. 配置保存

### 再次打开

1. 打开 GUI
2. **自动恢复**上次的游戏路径
3. **自动恢复**上次的 mod 列表
4. **自动恢复**每个 mod 的启用状态
5. 直接点击"生成Mods"即可

## 错误处理

### 配置文件不存在
- **行为**：使用默认空配置
- **不影响**：正常使用

### 配置文件损坏
- **行为**：忽略损坏的配置，使用默认值
- **不影响**：正常使用
- **提示**：终端可能显示错误信息

### 无法保存配置
- **行为**：在终端显示错误
- **不影响**：GUI 功能正常，只是下次不会记住
- **原因**：可能是权限问题或磁盘空间不足

## 测试验证

### 测试 1: 基本保存与加载

```bash
# 1. 运行 GUI
cargo run --bin infinite-gui

# 2. 选择游戏路径：F:/Games/Diablo II Resurrected
# 3. 添加一些 mod
# 4. 关闭 GUI

# 5. 检查配置文件是否存在
cat $env:APPDATA/infinite/gui_config.json  # Windows PowerShell

# 6. 再次运行 GUI
cargo run --bin infinite-gui

# 7. 验证：游戏路径和 mod 列表都已恢复
```

### 测试 2: 状态保存

```bash
# 1. 运行 GUI，加载 mod
# 2. 禁用某些 mod（取消复选框）
# 3. 关闭 GUI
# 4. 再次打开 GUI
# 5. 验证：禁用的 mod 仍然是禁用状态
```

### 测试 3: 顺序保存

```bash
# 1. 运行 GUI，加载多个 mod
# 2. 调整 mod 顺序（上移/下移）
# 3. 关闭 GUI
# 4. 再次打开 GUI
# 5. 验证：mod 顺序保持一致
```

## 优势

1. **零配置**：完全自动，无需手动保存
2. **实时保存**：每次改变立即保存，不怕程序崩溃
3. **跨平台**：使用标准目录，支持 Windows/Linux/macOS
4. **人类可读**：JSON 格式，可以手动编辑
5. **向后兼容**：配置文件损坏不影响使用

## 手动管理配置

### 查看配置

```bash
# Windows PowerShell
cat $env:APPDATA/infinite/gui_config.json

# Linux/macOS
cat ~/.config/infinite/gui_config.json
```

### 编辑配置

可以手动编辑配置文件，但需要确保 JSON 格式正确。

### 重置配置

```bash
# Windows PowerShell
Remove-Item $env:APPDATA/infinite/gui_config.json

# Linux/macOS
rm ~/.config/infinite/gui_config.json
```

下次打开 GUI 时会使用空配置。

## 依赖

新增依赖：
```toml
dirs = "5.0"  # 获取系统目录路径
```

## 已知限制

1. **单一配置**：只保存一个配置，不支持多个配置文件
2. **完整保存**：保存整个 mod 列表，不支持增量更新
3. **无历史记录**：不保留历史配置，只保留最新的
4. **无云同步**：配置存储在本地，不同机器不同步

## 未来改进

- [ ] 支持多个配置文件（profile）
- [ ] 支持导入/导出配置
- [ ] 支持配置历史记录
- [ ] 在 GUI 中显示配置文件位置
- [ ] 添加"重置配置"按钮

## 相关文件

- `src/gui/app.rs` - 配置持久化实现
- `Cargo.toml` - 添加 `dirs` 依赖

## 总结

- ✅ 自动保存游戏路径
- ✅ 自动保存 mod 列表
- ✅ 自动保存 mod 启用状态
- ✅ 自动保存 mod 顺序
- ✅ 启动时自动加载
- ✅ 实时保存，防止丢失
- ✅ 跨平台支持
- ✅ 错误友好

**现在 GUI 会记住你的所有选择，使用更加便捷！** 🎉
