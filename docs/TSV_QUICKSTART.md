# TSV增强功能快速指南

## ✨ 新功能

### 1️⃣ 通过Header名称访问单元格

```lua
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- ❌ 旧方式：数字索引
tsv[1][1] = 'NewName'
tsv[1][9] = '50'  -- 第9列是什么？

-- ✅ 新方式：Header名称
tsv[1]['Treasure Class'] = 'NewName'
tsv[1]['NoDrop'] = '50'  -- 清晰明了！
```

### 2️⃣ add() 方法添加空行

```lua
-- ❌ 旧方式：手动创建和添加
local new_row = {}
-- 手动查找下一个索引...
local next_idx = 1
for k, v in pairs(tsv) do
  if type(k) == 'number' and k >= next_idx then
    next_idx = k + 1
  end
end
tsv[next_idx] = new_row

-- ✅ 新方式：一行搞定
local new_row = tsv:add()
```

## 📝 完整示例

```lua
-- 读取TSV
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- 添加新行（自动创建空行）
local new_row = tsv:add()

-- 使用Header名称填充数据
new_row['Treasure Class'] = 'MyNewTC'
new_row['Picks'] = '1'
new_row['NoDrop'] = '10'
new_row['Prob1'] = '100'
new_row['Item1'] = 'amu'

-- 保存
infinite.writeTsv(filename, tsv)
```

## 🔄 修改现有数据

```lua
-- 读取并修改
local tsv = infinite.readTsv(filename)

for i = 1, #tsv do
    local row = tsv[i]
    
    -- 使用Header名称读取和修改
    if row['Treasure Class'] == 'Act 1 Good' then
        row['NoDrop'] = '0'  -- 增加掉落率
        console.log('Modified:', row['Treasure Class'])
    end
end

infinite.writeTsv(filename, tsv)
```

## 💡 优势

| 旧方式 | 新方式 |
|-------|--------|
| `row[9] = '50'` 😵 需要记住列号 | `row['NoDrop'] = '50'` 😊 直观易懂 |
| 手动查找索引 📝 代码冗长 | `tsv:add()` ⚡ 一行搞定 |
| 易出错 ❌ 列号错误 | 不易出错 ✅ 名称清晰 |

## 🧪 测试

```bash
# 运行演示
cargo run -- install tsv_demo

# 运行完整测试
cargo run -- install tsv_header_test
```

## 📚 详细文档

查看 `docs/TSV_HEADER_ACCESS.md` 了解更多信息。

## ⚠️ 向后兼容

- 所有旧代码继续工作
- 数字索引仍然完全支持
- 无破坏性变更
