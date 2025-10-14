# GUI GitHub Mod 添加功能

## 功能说明

GUI 现在支持直接添加 GitHub 上的 mod，无需手动下载！

### 支持的功能

1. ✅ **粘贴 GitHub 链接** - 支持多种 URL 格式
2. ✅ **自动获取分支列表** - 从 GitHub API 获取所有分支
3. ✅ **选择分支** - 通过下拉菜单选择特定分支
4. ✅ **指定子目录** - 可选指定仓库中的子目录

## 使用方法

### 步骤 1: 打开对话框

点击 **🌐 添加GitHub Mod** 按钮

### 步骤 2: 输入仓库地址

支持以下格式：

```
✅ user/repo
✅ github.com/user/repo
✅ https://github.com/user/repo
```

示例：
```
olegbl/d2rmm.mods
```

### 步骤 3: 获取分支信息

点击 **🔍 获取分支信息** 按钮

系统会：
- 连接 GitHub API
- 获取所有分支列表
- 显示在下拉菜单中

### 步骤 4: 选择分支（可选）

从下拉菜单中选择要使用的分支：
- 默认显示第一个分支
- 通常是 `main` 或 `master`
- 可以选择任何其他分支

### 步骤 5: 指定子目录（可选）

如果 mod 位于仓库的子目录中：

```
输入: mods/loot_filter
```

留空则使用仓库根目录。

### 步骤 6: 添加

点击 **✅ 添加** 按钮

Mod 会被添加到列表，格式如下：
```
github:user/repo                          # 默认分支，根目录
github:user/repo:mods/subdir              # 默认分支，子目录
github:user/repo@branch                   # 指定分支，根目录
github:user/repo:mods/subdir@branch       # 指定分支和子目录
```

## 示例场景

### 场景 1: 简单仓库

```
仓库: olegbl/d2rmm.mods
分支: main (默认)
子目录: (空)

结果: github:olegbl/d2rmm.mods
```

### 场景 2: 带子目录

```
仓库: SomeUser/ModCollection
分支: master (默认)
子目录: mods/my_awesome_mod

结果: github:SomeUser/ModCollection:mods/my_awesome_mod
```

### 场景 3: 特定分支

```
仓库: Developer/D2Mods
分支: dev
子目录: (空)

结果: github:Developer/D2Mods@dev
```

### 场景 4: 完整配置

```
仓库: https://github.com/User/Repository
分支: feature-branch
子目录: src/mods/loot

结果: github:User/Repository:src/mods/loot@feature-branch
```

## 技术实现

### 对话框结构

```rust
struct GitHubDialog {
    repo_url: String,                               // 仓库 URL
    branches: Arc<Mutex<Vec<String>>>,              // 分支列表（共享）
    selected_branch: Option<String>,                // 选中的分支
    selected_subdir: Option<String>,                // 子目录
    is_loading: Arc<Mutex<bool>>,                   // 加载状态（共享）
    error_message: Arc<Mutex<Option<String>>>,      // 错误消息（共享）
}
```

### GitHub API 调用

```rust
// API 端点
let url = format!("https://api.github.com/repos/{}/branches", repo);

// 异步获取
std::thread::spawn(move || {
    let response = reqwest::blocking::Client::new()
        .get(&url)
        .header("User-Agent", "infinite-mod-manager")
        .send()?;
    
    // 解析分支列表
    let branches = parse_branches(response)?;
    
    // 更新共享状态
    *branches_shared.lock().unwrap() = branches;
});
```

### URL 解析

```rust
fn parse_github_url(url: &str) -> Option<String> {
    // 支持的格式:
    // https://github.com/user/repo
    // github.com/user/repo
    // user/repo
    
    // 提取并返回: user/repo
}
```

### Mod 路径生成

```rust
let mut path = format!("github:{}", repo);

if let Some(subdir) = subdir {
    path = format!("{}:{}", path, subdir);
}

if let Some(branch) = branch {
    if branch != "main" && branch != "master" {
        path = format!("{}@{}", path, branch);
    }
}

// 结果: github:user/repo:subdir@branch
```

## UI 设计

### 对话框布局

```
┌─────────────────────────────────────────┐
│ 🌐 添加 GitHub Mod                       │
├─────────────────────────────────────────┤
│                                         │
│ 仓库地址:                                │
│ ┌─────────────────────────────────────┐ │
│ │ user/repo 或 https://github.com/... │ │
│ └─────────────────────────────────────┘ │
│ 支持格式: user/repo 或 github.com/...   │
│                                         │
│ [ 🔍 获取分支信息 ]                      │
│                                         │
│ ⏳ 正在获取仓库信息...                   │
│                                         │
│ ────────────────────────────────────── │
│ 分支: [main        ▼]                   │
│                                         │
│ 子目录:                                  │
│ ┌─────────────────────────────────────┐ │
│ │ 可选，例如: mods/my_mod             │ │
│ └─────────────────────────────────────┘ │
│ 留空表示使用仓库根目录                   │
│                                         │
│ ────────────────────────────────────── │
│ [ ✅ 添加 ] [ ❌ 取消 ]                 │
└─────────────────────────────────────────┘
```

### 状态反馈

| 状态 | 显示 |
|------|------|
| 初始 | 显示输入框和获取按钮 |
| 加载中 | 显示 spinner + "正在获取..." |
| 成功 | 显示分支选择和子目录输入 |
| 错误 | 显示红色错误消息 |

## 错误处理

### 无效 URL

```
输入: invalid-url
错误: "无效的 GitHub URL 格式"
```

### 网络错误

```
错误: "网络错误: connection timeout"
```

### 仓库不存在

```
错误: "无法获取仓库信息: 404 Not Found"
```

### API 限制

```
错误: "无法获取仓库信息: 403 Forbidden"
提示: GitHub API 有速率限制（未认证: 60次/小时）
```

## 依赖更新

### Cargo.toml

```toml
# HTTP client
reqwest = { version = "0.11", features = ["json", "blocking"] }
```

添加了 `blocking` 特性以支持同步 HTTP 请求。

## 使用限制

### GitHub API 速率限制

| 认证状态 | 限制 |
|----------|------|
| 未认证 | 60 次/小时 |
| 已认证 | 5000 次/小时 |

当前实现使用未认证请求，通常足够个人使用。

### 网络要求

- 需要互联网连接
- 需要访问 `api.github.com`
- 可能被防火墙阻止

## 与 CLI 集成

添加的 GitHub mod 会被保存为特殊格式：

```txt
github:user/repo
github:user/repo:subdir
github:user/repo@branch
github:user/repo:subdir@branch
```

当点击"生成Mods"时，CLI 会：
1. 识别 `github:` 前缀
2. 下载仓库到缓存目录
3. 使用指定分支和子目录
4. 像本地 mod 一样处理

## 最佳实践

### 对于用户

1. **复制完整 URL**：直接从浏览器复制
2. **检查分支**：确认选择了正确的分支
3. **验证子目录**：如果 mod 在子目录中，确保路径正确
4. **测试连接**：首次使用时测试网络连接

### 对于 Mod 作者

1. **提供清晰路径**：在 README 中说明子目录位置
2. **使用稳定分支**：推荐使用 `main` 或 `master`
3. **标记版本**：使用 Git tags 标记稳定版本
4. **文档说明**：说明如何通过 GitHub 安装

## 未来改进

- [ ] 支持 GitHub Token 认证（提高速率限制）
- [ ] 显示仓库描述和星标数
- [ ] 缓存分支列表（减少 API 调用）
- [ ] 支持 Git tags/releases 选择
- [ ] 自动检测 mod 目录（扫描 `mod.json`）
- [ ] 显示最后更新时间
- [ ] 支持其他 Git 托管平台（GitLab, Gitea 等）

## 相关文档

- [MOD_LIST.md](./MOD_LIST.md) - Mod 列表格式文档
- [GUI_README.md](./GUI_README.md) - GUI 使用指南
- [GITHUB_DOWNLOADER.md](./GITHUB_DOWNLOADER.md) - GitHub 下载器技术文档

## 总结

- ✅ 支持直接添加 GitHub mod
- ✅ 自动获取分支列表
- ✅ 可选分支和子目录
- ✅ 友好的用户界面
- ✅ 完善的错误处理
- ✅ 与 CLI 无缝集成

**现在可以直接从 GitHub 添加 mod，无需手动下载！** 🎉
