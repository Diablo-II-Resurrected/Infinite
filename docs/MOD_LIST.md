# Mod List 功能说明

## 概述

Mod List 功能允许你通过文本文件管理要安装的 mod 列表,支持本地路径和 GitHub 仓库。

## 使用方法

### 基本命令

```bash
infinite install --game-path <游戏路径> --mod-list <列表文件> --output-path <输出路径>
```

### 示例

```bash
infinite install --game-path "F:\Games\Diablo II Resurrected" --mod-list mods.txt --output-path output
```

## Mod List 文件格式

Mod list 是一个简单的文本文件,每行一个 mod 源:

```txt
# 注释行以 # 开头
# 空行会被忽略

# 本地路径
mods/my_mod
C:\Users\username\Documents\D2Mods\awesome_mod

# GitHub 仓库 (基本格式)
github:username/repository

# GitHub 仓库 (指定分支)
github:username/repository@dev

# GitHub 仓库 (指定子目录)
github:username/repository:mods/specific_mod

# GitHub 仓库 (完整格式:子目录和分支)
github:username/repository:mods/specific_mod@dev
```

## 本地路径格式

- **相对路径**: `mods/my_mod`
- **绝对路径**: `C:\Users\username\mods\my_mod`
- **单个 mod 目录**: 包含 `mod.json` 的目录
- **Mods 容器目录**: 包含多个 mod 子目录的目录

## GitHub 格式详解

### 基本格式
```
github:owner/repo
```
- 下载整个仓库的 `main` 分支
- 示例: `github:olegbl/d2rmm`

### 指定分支
```
github:owner/repo@branch
```
- 下载指定分支
- 示例: `github:olegbl/d2rmm@dev`

### 指定子目录
```
github:owner/repo:path/to/mod
```
- 只下载仓库中的特定子目录
- 示例: `github:olegbl/d2rmm:mods/loot-filter`

### 完整格式
```
github:owner/repo:path/to/mod@branch
```
- 指定分支和子目录
- 示例: `github:olegbl/d2rmm:mods/loot-filter@experimental`

## GitHub 下载缓存

### 缓存位置
下载的 GitHub mods 会缓存在 `.mod_cache` 目录:
```
.mod_cache/
  owner/
    repo/
      branch/
        ...mod files...
```

### 清除缓存
使用 `--clear-cache` 选项重新下载所有 mods:

```bash
infinite install --game-path <游戏路径> --mod-list mods.txt --output-path output --clear-cache
```

## 示例 Mod List

### 示例 1: 本地 Mods
```txt
# My personal mods
mods/loot_filter
mods/increased_stash
mods/quality_of_life
```

### 示例 2: GitHub Mods
```txt
# Popular community mods
github:user1/d2r-loot-filter
github:user2/d2r-mods:stash_mod
github:user3/d2r-qol@latest
```

### 示例 3: 混合
```txt
# Mix of local and GitHub mods
mods/my_custom_mod
github:community/popular-mod
github:developer/experimental:beta_features@dev
C:\Downloads\special_mod
```

## 命令行选项

### --mod-list (-l)
指定 mod list 文件路径
```bash
--mod-list mods.txt
-l my_mods.txt
```

### --mods-path (-m)
传统方式: 指定 mods 目录(与 --mod-list 互斥)
```bash
--mods-path mods/
-m mods/
```

### --clear-cache
清除 GitHub 下载缓存并重新下载
```bash
--clear-cache
```

### --dry-run
测试运行,不实际写入文件
```bash
--dry-run
```

## 工作流程

1. **解析 Mod List**: 读取文本文件,解析每一行
2. **处理本地源**: 直接使用本地路径
3. **下载 GitHub 源**: 
   - 检查缓存
   - 如果未缓存,从 GitHub API 下载
   - 存储到 `.mod_cache`
4. **加载 Mods**: 从所有源加载 mod 配置
5. **安装 Mods**: 按顺序执行所有 mods

## 注意事项

### GitHub API 限制
- 未认证请求: 60 次/小时
- 如果遇到限制,请稍后重试
- 使用缓存可以减少 API 调用

### Mod 冲突
- Mods 按 list 中的顺序执行
- 后面的 mod 可能覆盖前面的修改
- 合理安排 mod 顺序很重要

### 路径格式
- Windows: 使用 `\` 或 `/` 都可以
- Linux/Mac: 使用 `/`
- 建议使用相对路径以提高可移植性

## 故障排除

### Mod 未找到
- 检查路径是否正确
- 确保目录包含 `mod.json` 文件
- 使用绝对路径测试

### GitHub 下载失败
- 检查仓库名称格式: `owner/repo`
- 检查分支/子目录是否存在
- 确保网络连接正常
- 使用 `--clear-cache` 重试

### 缓存问题
- 使用 `--clear-cache` 清除并重新下载
- 手动删除 `.mod_cache` 目录

## 高级用法

### 创建共享 Mod List
团队可以共享同一个 mod list 文件:
```txt
# Team Mod Configuration - v1.2
# Last updated: 2025-01-01

github:team/core-mods:config@stable
github:team/loot-system@v2.0
github:contributor/fixes:hotfix
```

### 版本控制
将 mod list 文件提交到 Git:
```bash
git add mods.txt
git commit -m "Update mod list with new QoL features"
```

### 环境特定配置
创建不同的列表文件:
- `mods_dev.txt` - 开发环境
- `mods_prod.txt` - 生产环境
- `mods_test.txt` - 测试环境
