# GitHub 子目录下拉菜单功能

## 概述

GUI 应用现在支持在添加 GitHub mod 时自动获取并显示仓库的目录结构，用户可以通过下拉菜单选择子目录。

## 功能说明

### 1. 自动获取目录结构

当用户选择一个分支后，系统会自动：
- 调用 GitHub API 获取该分支的完整目录树
- 筛选出所有目录（排除文件）
- 按字母顺序排序
- 在列表顶部添加 "(根目录)" 选项

### 2. 下拉菜单选择

- **分支未选择时**：子目录输入框不可用
- **分支选择后，目录加载中**：显示加载进度指示器
- **目录加载完成**：显示下拉菜单，包含所有可用目录
- **目录加载失败**：回退到手动文本输入框

### 3. 特殊选项处理

- **(根目录)**：表示使用仓库根目录，不添加子目录路径
- **其他目录**：完整的目录路径，例如 `mods/my_mod`

## 技术实现

### GitHub API 调用

```
GET https://api.github.com/repos/{owner}/{repo}/git/trees/{branch}?recursive=1
```

参数说明：
- `{owner}/{repo}`: 仓库完整路径
- `{branch}`: 选定的分支名称
- `recursive=1`: 递归获取整个目录树

### 响应处理

API 返回包含 `tree` 数组的 JSON，每个元素包含：
- `path`: 文件或目录路径
- `type`: "blob"（文件）或 "tree"（目录）

代码筛选 `type == "tree"` 的元素，提取其路径。

### UI 状态管理

使用 `Arc<Mutex<T>>` 实现跨线程状态共享：

```rust
pub struct GitHubDialog {
    pub subdirs: Arc<Mutex<Vec<String>>>,           // 目录列表
    pub is_loading_dirs: Arc<Mutex<bool>>,          // 加载状态
    // ... 其他字段
}
```

### 分支变化检测

```rust
let prev_branch = dialog.selected_branch.clone();

// 用户选择分支...

if prev_branch != dialog.selected_branch && dialog.selected_branch.is_some() {
    should_fetch_dirs = true;  // 触发目录获取
}
```

### 路径生成

在 `add_github_mod()` 方法中：

```rust
if let Some(subdir) = &dialog.selected_subdir {
    // 忽略 "(根目录)" 选项
    if !subdir.is_empty() && subdir != "(根目录)" {
        github_path = format!("{}:{}", github_path, subdir);
    }
}
```

## 用户体验流程

1. 用户粘贴 GitHub 仓库 URL
2. 点击 "🔍 获取分支信息" 按钮
3. 系统显示分支下拉菜单
4. 用户从下拉菜单选择分支
5. **系统自动获取并显示该分支的目录结构**
6. 用户从子目录下拉菜单选择目录（或选择 "(根目录)"）
7. 点击 "✅ 添加" 完成

## 错误处理

### 网络错误
- 显示错误消息："网络错误: {详细信息}"
- 回退到手动文本输入

### API 错误
- 显示错误消息："无法获取目录结构: {HTTP状态码}"
- 回退到手动文本输入

### 空目录树
- 如果仓库没有任何子目录，只显示 "(根目录)" 选项

## 兼容性

- 与现有的手动输入功能兼容
- 如果目录获取失败，用户仍可手动输入子目录路径
- 支持所有公开的 GitHub 仓库（无需认证）

## 示例

### 仓库结构
```
my-repo/
├── README.md
├── mods/
│   ├── mod1/
│   └── mod2/
└── tools/
    └── scripts/
```

### 下拉菜单显示
```
(根目录)
mods
mods/mod1
mods/mod2
tools
tools/scripts
```

### 生成的路径

| 选择 | 生成路径 |
|------|----------|
| (根目录) | `github:user/my-repo` |
| mods | `github:user/my-repo:mods` |
| mods/mod1 | `github:user/my-repo:mods/mod1` |

## 限制

1. **公开仓库**：目前不支持私有仓库（需要 token 认证）
2. **API 限流**：GitHub API 对未认证请求有限流（每小时 60 次）
3. **大型仓库**：目录树过大可能导致加载缓慢

## 未来改进

- [ ] 支持 GitHub token 认证以访问私有仓库
- [ ] 缓存目录结构以减少 API 调用
- [ ] 显示子目录的 README 预览
- [ ] 支持多选子目录（批量添加）
