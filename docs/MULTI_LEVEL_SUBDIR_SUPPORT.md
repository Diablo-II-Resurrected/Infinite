# 多级子目录支持说明

## 概述

GitHub mod 添加功能**已完整支持多级子目录选择**，无需任何额外配置。

## 支持情况

✅ **完全支持** - 可以选择任意深度的子目录

### 技术实现

#### 1. GitHub API 递归获取
```rust
let url = format!("https://api.github.com/repos/{}/git/trees/{}?recursive=1", repo, branch);
```

使用 `recursive=1` 参数时，GitHub API 会返回**完整的目录树**，包括所有层级的目录。

#### 2. 路径提取
```rust
let mut dirs: Vec<String> = tree_array
    .iter()
    .filter_map(|item| {
        if item.get("type")?.as_str()? == "tree" {
            Some(item.get("path")?.as_str()?.to_string())  // 完整路径
        } else {
            None
        }
    })
    .collect();
```

GitHub API 返回的路径已经是完整的相对路径，例如：
- `mods/mod1/config` (三级目录)
- `tools/scripts/converter` (三级目录)
- `data/assets/images` (三级目录)

## 实际示例

### 仓库结构示例 1
```
D2RMM-Community-Mods/
├── mods/
│   ├── StackSizeChanger/
│   │   ├── mod.json
│   │   └── mod.js
│   ├── RunewordMod/
│   │   ├── mod.json
│   │   └── scripts/
│   │       ├── main.lua
│   │       └── config/
│   │           └── settings.json
│   └── LootFilter/
│       └── mod.json
└── tools/
```

**下拉菜单显示：**
```
(根目录)
mods
mods/LootFilter
mods/RunewordMod
mods/RunewordMod/scripts
mods/RunewordMod/scripts/config
mods/StackSizeChanger
tools
```

### 仓库结构示例 2：深层嵌套
```
mod-collection/
├── src/
│   ├── gameplay/
│   │   ├── balance/
│   │   │   ├── items/
│   │   │   │   └── mod.json
│   │   │   └── skills/
│   │   └── difficulty/
│   └── visual/
│       └── ui/
│           └── fonts/
└── docs/
```

**下拉菜单显示：**
```
(根目录)
docs
src
src/gameplay
src/gameplay/balance
src/gameplay/balance/items
src/gameplay/balance/skills
src/gameplay/difficulty
src/visual
src/visual/ui
src/visual/ui/fonts
```

## 使用方法

### 标准流程
1. 在 GitHub 对话框输入仓库 URL
2. 点击"获取分支信息"
3. 选择目标分支
4. **系统自动获取所有层级的目录**
5. 从下拉菜单选择任意层级的目录
6. 点击"添加"

### 选择深层目录
例如要添加 `mods/RunewordMod/scripts/config` 目录：
1. 在子目录下拉菜单中找到并选择 `mods/RunewordMod/scripts/config`
2. 点击"添加"
3. 生成的路径：`github:user/repo:mods/RunewordMod/scripts/config@branch`

## 生成路径格式

### 格式说明
```
github:{owner}/{repo}:{subdir}@{branch}
```

### 多级目录示例

| 选择的子目录 | 生成的完整路径 |
|------------|---------------|
| `(根目录)` | `github:user/repo@main` |
| `mods` | `github:user/repo:mods@main` |
| `mods/mod1` | `github:user/repo:mods/mod1@main` |
| `mods/mod1/config` | `github:user/repo:mods/mod1/config@main` |
| `data/assets/textures` | `github:user/repo:data/assets/textures@main` |

### 路径分隔符
- Windows 和 Unix 系统通用：使用 `/` 分隔符
- GitHub API 返回的路径统一使用 `/`
- mod 管理器会正确处理路径

## 限制和注意事项

### 目录深度
- ✅ **无深度限制**：支持任意深度的目录结构
- ✅ GitHub API 的 `recursive=1` 参数会获取所有层级

### 性能考虑
- 对于**非常大**的仓库（数千个目录），加载可能需要几秒钟
- GitHub API 有响应大小限制（通常足够大，不会是问题）
- 建议仓库目录数量控制在合理范围内（< 500 个）

### 显示排序
目录按**字母顺序**排序，这意味着：
```
mods
mods/a-mod
mods/b-mod
mods/b-mod/config
```

如果需要按层级分组显示，这是未来的增强方向。

## 常见使用场景

### 场景 1：Monorepo 风格的 mod 仓库
```
mod-pack/
├── mods/
│   ├── balance/
│   ├── loot/
│   └── visual/
```
**选择**: `mods/balance` 或 `mods/loot` 等

### 场景 2：复杂项目结构
```
d2r-enhancement/
├── src/
│   └── features/
│       └── stacking/
└── build/
    └── output/
```
**选择**: `src/features/stacking` - 精确定位 mod 位置

### 场景 3：嵌套配置
```
my-mod/
└── mod/
    └── config/
        └── localized/
            └── zh-CN/
```
**选择**: `mod/config/localized/zh-CN` - 选择本地化版本

## 测试验证

### 测试步骤
1. 启动 GUI：`.\target\release\infinite-gui.exe`
2. 点击"添加 GitHub Mod"
3. 输入一个有多级目录的仓库（如 `olegbl/d2rmm-mods`）
4. 获取分支信息并选择分支
5. 查看子目录下拉菜单

### 预期结果
- ✅ 下拉菜单显示所有层级的目录
- ✅ 目录路径完整（如 `mods/StackSizeChanger`）
- ✅ 可以选择任意层级的目录
- ✅ 选择后路径正确生成

### 真实仓库测试
可以使用这些公开仓库进行测试：
- `olegbl/d2rmm` - 有 `mods/` 子目录
- `Diablo-II-Resurrected/Infinite` - 有多个示例目录
- 任何标准的 D2RMM mod 仓库

## 技术细节

### GitHub API 响应格式
```json
{
  "tree": [
    {
      "path": "mods",
      "type": "tree",
      "sha": "..."
    },
    {
      "path": "mods/mod1",
      "type": "tree",
      "sha": "..."
    },
    {
      "path": "mods/mod1/config",
      "type": "tree",
      "sha": "..."
    },
    {
      "path": "mods/mod1/config/settings.json",
      "type": "blob",
      "sha": "..."
    }
  ]
}
```

代码只提取 `"type": "tree"` 的条目，自动忽略文件（`"type": "blob"`）。

### 路径处理
- **不需要额外处理**：GitHub API 返回的路径已经是正确格式
- **自动排序**：`dirs.sort()` 按字母顺序排序
- **保持原样**：不修改路径分隔符或结构

## 常见问题

### Q: 可以选择多深的目录？
A: **无限制**，只要 GitHub API 能返回，就可以选择。

### Q: 路径中的 `/` 会有问题吗？
A: 不会，GitHub 和 mod 管理器都正确处理 `/` 分隔符。

### Q: 空目录会显示吗？
A: 会，只要目录存在于 Git 仓库中（即使为空）。

### Q: 如果目录很多，怎么快速找到？
A: 当前版本按字母排序。未来可以考虑添加搜索/过滤功能。

### Q: 能否显示目录的层级结构（缩进）？
A: 当前版本是平面列表。未来可以考虑树状显示或分组。

## 总结

✅ **多级子目录已完全支持**
- 无需任何修改或配置
- 开箱即用
- 支持任意深度的目录结构
- 路径格式标准化

🎯 **现有实现完全满足多级子目录需求**
- GitHub API 的 `recursive=1` 参数确保获取所有层级
- 代码正确提取和显示完整路径
- 用户可以自由选择任意深度的目录

📝 **推荐最佳实践**
- 为 mod 创建清晰的目录结构
- 使用有意义的路径名称
- 保持目录深度合理（2-4 层）
- 在 README 中说明推荐的目录选择

💡 **未来增强方向**
- 树状显示（带缩进）
- 搜索/过滤功能
- 显示每个目录的文件数量
- 智能推荐（检测 mod.json 位置）
