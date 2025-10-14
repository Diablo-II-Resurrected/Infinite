# 多 Mod 修改同一文件支持

## 概述

从此版本开始,infinite CLI 支持多个 mod 修改同一文件,所有修改都会正确地链式应用。

## 工作原理

### 以前的行为
当多个 mod 修改同一文件时,后面的 mod 会覆盖前面 mod 的修改:

```
Mod A: 读取文件 → 修改 → 写入磁盘
Mod B: 读取文件 → 修改 → 写入磁盘  ❌ 覆盖了 Mod A 的修改
```

### 现在的行为
使用内存缓存机制,所有修改都会链式应用:

```
Mod A: 读取文件 → 修改 → 写入缓存
Mod B: 读取缓存 → 修改 → 更新缓存  ✅ 包含 Mod A 的修改
最终: 将缓存刷新到磁盘
```

## 技术实现

### 缓存机制

1. **FileManager 内存缓存**
   - 使用 `HashMap<String, CachedFile>` 存储修改后的文件内容
   - 每个 `CachedFile` 包含文件内容和 dirty 标志

2. **读取顺序**
   - 首先检查缓存
   - 如果缓存中有,直接使用缓存内容
   - 如果没有,从 CASC 或磁盘提取

3. **写入顺序**
   - 所有写操作都先写入缓存
   - 标记文件为 dirty
   - 最后统一刷新到磁盘

### 代码结构

#### CachedFile 结构体
```rust
pub struct CachedFile {
    pub content: Vec<u8>,
    pub dirty: bool,
}
```

#### FileManager 方法
```rust
// 读取文件(优先从缓存)
pub async fn read_file_with_cache(&mut self, file_path: &str, mod_id: &str) -> Result<Vec<u8>>

// 写入到缓存
pub fn write_file_to_cache(&mut self, file_path: &str, content: Vec<u8>, mod_id: &str)

// 刷新缓存到磁盘
pub async fn flush_cache(&mut self) -> Result<()>
```

#### Context 方法更新
所有文件操作方法都已更新以支持缓存:
- `read_tsv()` / `write_tsv()`
- `read_json()` / `write_json()`
- `read_txt()` / `write_txt()`

## 示例

### 测试场景

两个 mod 都修改 `treasureclassex.txt`:

**Mod A:**
```lua
local filename = 'global\\excel\\treasureclassex.txt'
local tsv = infinite.readTsv(filename)

local new_row = {}
new_row[1] = 'ModA_TestItem'
new_row[2] = '100'
new_row[3] = 'rin'
new_row[4] = '0'

tsv[#tsv + 1] = new_row
infinite.writeTsv(filename, tsv)
```

**Mod B:**
```lua
local filename = 'global\\excel\\treasureclassex.txt'
local tsv = infinite.readTsv(filename)

local new_row = {}
new_row[1] = 'ModB_TestItem'
new_row[2] = '100'
new_row[3] = 'sol'
new_row[4] = '0'

tsv[#tsv + 1] = new_row
infinite.writeTsv(filename, tsv)
```

### 测试结果

**原始文件:**
```
Treasure Class  Prob1   Item1   NoDrop
Act 1 Good      100     gld     0
Act 2 Good      100     gld     0
```

**最终输出:**
```
Treasure Class  Prob1   Item1   NoDrop
Act 1 Good      100     gld     0
Act 2 Good      100     gld     0
ModA_TestItem   100     rin     0       ← Mod A 添加
ModB_TestItem   100     sol     0       ← Mod B 添加
```

✅ 两个 mod 的修改都成功应用!

## 性能优势

1. **减少磁盘 I/O**: 每个文件只写入一次
2. **支持复杂链式修改**: 无限数量的 mod 可以修改同一文件
3. **原子性**: 所有修改要么全部成功,要么全部失败

## 兼容性

- ✅ 完全向后兼容现有 mod
- ✅ 自动处理所有文件类型 (TSV, JSON, TXT)
- ✅ 与 CASC 提取无缝集成
- ✅ 与 mod 列表功能配合使用

## 日志输出

运行时可以看到缓存刷新的日志:

```
💾 Flushing cached modifications...
2025-10-14T07:37:33.883197Z  INFO Flushed to disk: global/excel/treasureclassex.txt
✅ All modifications written to disk
```

## 最佳实践

1. **按顺序组织 mod**: 在 mod 列表中按依赖顺序排列 mod
2. **避免冲突**: 虽然支持链式修改,但还是建议避免多个 mod 修改相同的字段
3. **测试**: 使用 `--dry-run` 标志测试 mod 组合

## 实现文件

- `src/file_system/manager.rs`: 缓存机制实现
- `src/runtime/context.rs`: Context 方法更新
- `src/handlers/tsv.rs`: TSV 字节解析
- `src/handlers/json.rs`: JSON 字节解析
- `src/main.rs`: flush_cache() 调用
