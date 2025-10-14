# 快速开始 - Infinite GUI

## 🚀 快速启动

### Windows

```powershell
# 开发模式运行
cargo run --bin infinite-gui

# 或构建并运行发布版本
cargo build --release --bin infinite-gui
./target/release/infinite-gui.exe
```

### Linux/Mac

```bash
# 开发模式运行
cargo run --bin infinite-gui

# 或构建并运行发布版本
cargo build --release --bin infinite-gui
./target/release/infinite-gui
```

## 📝 使用步骤

### 1. 启动应用

运行GUI应用后，你会看到一个简洁的窗口界面。

### 2. 选择游戏路径

- 点击 **📁 选择游戏目录** 按钮
- 浏览到你的D2R安装目录
- 例如：`F:\Games\Diablo II Resurrected`

### 3. 添加Mods

**方式A - 打开Mod列表文件：**
- 点击 **📂 打开Mod列表**
- 选择一个`.txt`文件（每行一个mod路径）
- 例如使用项目中的 `test_multi_mod.txt`

**方式B - 逐个添加：**
- 点击 **➕ 添加Mod文件夹**
- 浏览并选择mod目录
- 重复以添加更多mod

### 4. 管理Mods

- ✅ **启用/禁用**：点击复选框
- ⬆⬇ **调整顺序**：使用上下箭头按钮
- 🗑 **删除**：点击删除按钮

### 5. 生成Mods

- 点击 **🚀 生成Mods** 按钮
- 等待处理完成
- Mods将自动生成到：`<游戏路径>/Mods/Infinite/Infinite.mpq/data/`

### 6. 保存配置（可选）

- 点击 **💾 保存Mod列表** 保存当前配置
- 下次可以直接打开这个列表文件

## 🎯 示例工作流

```
1. 启动 infinite-gui
2. 选择游戏路径: F:\Games\Diablo II Resurrected
3. 打开Mod列表: test_multi_mod.txt
4. 查看已加载的mods：
   ☑ Test Mod A - Add Item
   ☑ Test Mod B - Add Another Item
5. 点击 🚀 生成Mods
6. 等待完成: ✅ 成功生成到: F:/Games/.../Mods/Infinite/...
```

## ⚙️ 输出路径说明

Mods**始终**生成到固定路径：
```
<游戏路径>/Mods/Infinite/Infinite.mpq/data/
```

这个路径是硬编码的，确保与游戏的mod加载系统兼容。

### 完整示例

如果游戏路径是：
```
F:\Games\Diablo II Resurrected
```

那么输出路径就是：
```
F:\Games\Diablo II Resurrected\Mods\Infinite\Infinite.mpq\data\
```

生成的文件结构：
```
F:\Games\Diablo II Resurrected\
└── Mods\
    └── Infinite\
        └── Infinite.mpq\
            └── data\
                ├── global\
                │   └── excel\
                │       └── treasureclassex.txt
                └── local\
                    └── lng\
                        └── strings\
                            └── ...
```

## 🔧 故障排查

### 按钮是灰色的/无法点击

- **选择游戏目录**：所有功能都可用
- **打开/保存Mod列表**：需要先选择游戏路径
- **🚀 生成Mods**：需要：
  - 已选择游戏路径 ✓
  - 至少添加了一个mod ✓
  - 至少启用了一个mod ✓
  - 当前没有正在处理的任务 ✓

### 找不到infinite命令

GUI会尝试调用`infinite` CLI工具。确保：
1. 项目已完全构建：`cargo build --release`
2. 或者GUI和CLI在同一目录

### Mods没有在游戏中生效

1. 确认输出路径正确
2. 检查游戏设置中的mod选项
3. 尝试重启游戏
4. 查看生成的文件是否存在

## 📚 更多信息

详细文档请参阅：
- [GUI README](./GUI_README.md) - GUI完整功能说明
- [项目README](../README.md) - 整体项目文档
- [CLI使用说明](../QUICKSTART.md) - 命令行版本使用

## 💡 提示

- 第一次使用时，可以用项目中的测试mods试试：
  ```
  test_mods/mod_a
  test_mods/mod_b
  ```
- Mod加载顺序很重要！后面的mod会覆盖前面的修改
- 经常保存你的mod列表配置，方便复用
