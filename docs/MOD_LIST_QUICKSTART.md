# Mod List Quick Start Guide

## 5 分钟快速上手

### 1. 创建 Mod List 文件

创建一个文本文件 `my_mods.txt`:

```txt
# 我的 Mod 配置
mods/loot_filter
mods/stash_mod
```

### 2. 运行安装命令

```bash
infinite install --game-path "F:\Games\Diablo II Resurrected" --mod-list my_mods.txt --output-path output
```

就这么简单!

## 添加 GitHub Mods

编辑 `my_mods.txt`:

```txt
# 本地 mods
mods/loot_filter

# GitHub 上的社区 mod
github:community/awesome-mod
github:developer/experimental-features@dev
```

首次运行时会自动下载 GitHub mods 到 `.mod_cache` 目录。

## 常用命令

### 基本安装
```bash
infinite install -g "游戏路径" -l mods.txt -o output
```

### 清除缓存重新下载
```bash
infinite install -g "游戏路径" -l mods.txt -o output --clear-cache
```

### 测试运行(不写文件)
```bash
infinite install -g "游戏路径" -l mods.txt -o output --dry-run
```

## Mod List 格式速查

| 格式 | 示例 | 说明 |
|------|------|------|
| 本地路径 | `mods/my_mod` | 相对或绝对路径 |
| GitHub基本 | `github:user/repo` | 完整仓库 |
| GitHub分支 | `github:user/repo@dev` | 指定分支 |
| GitHub子目录 | `github:user/repo:path/to/mod` | 仓库子目录 |
| GitHub完整 | `github:user/repo:path@branch` | 子目录+分支 |

## 提示

- ✅ 以 `#` 开头的行是注释
- ✅ 空行会被忽略
- ✅ Mods 按列表顺序执行
- ✅ GitHub mods 会被缓存以加快后续安装
- ✅ 使用 `--clear-cache` 可以强制重新下载

## 下一步

阅读完整文档:
- [详细的 Mod List 文档](MOD_LIST.md)
- [CASC 集成指南](CASC_INTEGRATION.md)
- [主 README](../README.md)
