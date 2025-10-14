# TSV增强功能：Header访问和add()方法

## 功能概述

TSV处理现在支持两个强大的新功能：

1. **通过Header名称读写单元格** - 可以使用列名而不是数字索引来访问数据
2. **add()方法** - 轻松添加空行，自动初始化所有列

## 1. 通过Header名称访问单元格

### 读取单元格

```lua
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- 旧方式：使用数字索引（仍然支持）
local tc_name = tsv[1][1]        -- 第1列
local picks = tsv[1][2]          -- 第2列

-- 新方式：使用header名称（推荐）
local tc_name = tsv[1]['Treasure Class']
local picks = tsv[1]['Picks']
local nodrop = tsv[1]['NoDrop']
local item1 = tsv[1]['Item1']
```

### 修改单元格

```lua
-- 旧方式：需要知道列的索引位置
tsv[1][1] = 'NewTCName'
tsv[1][9] = '50'  -- NoDrop是第9列？

-- 新方式：直接使用列名
tsv[1]['Treasure Class'] = 'NewTCName'
tsv[1]['NoDrop'] = '50'
```

### 写入时自动处理

写入TSV时，系统会自动：
1. 优先使用数字索引的值
2. 如果数字索引没有值，则使用header名称的值
3. 确保列数正确，nil值转为空字符串

```lua
local row = tsv[1]

-- 只通过header设置
row['Treasure Class'] = 'Test'
row['Picks'] = '1'

-- 混合使用（两种方式都支持）
row['NoDrop'] = '10'
row[7] = '100'  -- Prob1

infinite.writeTsv(filename, tsv)  -- 自动处理两种访问方式
```

## 2. add() 方法

### 基本用法

`add()` 方法会：
- 自动创建一个新的空行
- 根据表头初始化所有列（全部为空字符串）
- 将新行添加到表的末尾
- 返回新行对象和索引

```lua
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- 添加一个空行
local new_row, index = tsv:add()

-- 填充数据（使用header名称）
new_row['Treasure Class'] = 'MyNewTC'
new_row['Picks'] = '1'
new_row['NoDrop'] = '10'
new_row['Prob1'] = '100'
new_row['Item1'] = 'amu'

-- 保存
infinite.writeTsv(filename, tsv)
```

### 添加多行

```lua
-- 添加多个条目
for i = 1, 5 do
    local row = tsv:add()
    row['Treasure Class'] = 'TC_' .. i
    row['Picks'] = tostring(i)
    row['Item1'] = 'item' .. i
end
```

## 3. 完整示例

### 示例1：简化的mod代码

```lua
-- Mod B: Add another item to TreasureClassEx
local filename = 'global\\excel\\treasureclassex.txt'
local tsv = infinite.readTsv(filename)

-- 旧方式（复杂）
--[[
local new_row = {}
new_row['Treasure Class'] = 'ModB_TestItem'
new_row['Prob1'] = '100'
new_row['Item1'] = 'sol'
new_row['NoDrop'] = '20'

local next_idx = 1
for k, v in pairs(tsv) do
  if type(k) == 'number' and k >= next_idx then
    next_idx = k + 1
  end
end
tsv[next_idx] = new_row
]]

-- 新方式（简单）
local new_row = tsv:add()
new_row['Treasure Class'] = 'ModB_TestItem'
new_row['Prob1'] = '100'
new_row['Item1'] = 'sol'
new_row['NoDrop'] = '20'

infinite.writeTsv(filename, tsv)
console.log('Mod B: Added ModB_TestItem to treasureclassex.txt')
```

### 示例2：批量修改

```lua
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- 增加所有宝藏类别的掉落率
for i = 1, #tsv do
    local row = tsv[i]
    local nodrop = tonumber(row['NoDrop'])
    
    if nodrop and nodrop > 0 then
        -- 减半NoDrop值
        row['NoDrop'] = tostring(math.floor(nodrop / 2))
        console.log('Modified:', row['Treasure Class'])
    end
end

infinite.writeTsv(filename, tsv)
```

### 示例3：条件添加

```lua
local tsv = infinite.readTsv('global\\excel\\treasureclassex.txt')

-- 为特定物品创建新的宝藏类别
local items = {'amu', 'rin', 'jew'}
for _, item in ipairs(items) do
    local row = tsv:add()
    row['Treasure Class'] = 'Special_' .. item
    row['Picks'] = '1'
    row['Prob1'] = '100'
    row['Item1'] = item
    console.log('Added TC for:', item)
end

infinite.writeTsv(filename, tsv)
```

## 优势对比

### 旧方式的问题

```lua
-- ❌ 需要手动创建表
local new_row = {}

-- ❌ 需要知道确切的列索引
new_row[1] = 'Name'
new_row[9] = '50'  -- 第9列是什么？

-- ❌ 需要手动计算下一个索引
local next_idx = 1
for k, v in pairs(tsv) do
  if type(k) == 'number' and k >= next_idx then
    next_idx = k + 1
  end
end
tsv[next_idx] = new_row
```

### 新方式的优势

```lua
-- ✓ 自动创建并初始化所有列
local new_row = tsv:add()

-- ✓ 使用有意义的列名
new_row['Treasure Class'] = 'Name'
new_row['NoDrop'] = '50'

-- ✓ 自动处理索引
-- (add方法已经将行添加到正确位置)
```

## 向后兼容性

所有旧代码继续工作：
- 数字索引访问仍然完全支持
- 手动创建行和添加的方式仍然可用
- 没有破坏性变更

新功能是增强，不是替代！

## 测试

运行测试模组验证功能：

```bash
cargo run -- install tsv_header_test
```

## 技术实现

### readTsv返回的表结构

```lua
{
  [1] = { [1]="val1", [2]="val2", ["Header1"]="val1", ["Header2"]="val2" },
  [2] = { [1]="val3", [2]="val4", ["Header1"]="val3", ["Header2"]="val4" },
  __headers__ = { [1]="Header1", [2]="Header2" },
  
  -- 元表方法
  add = function() ... end
}
```

### writeTsv的优先级

写入时按以下优先级读取单元格：
1. 数字索引 `row[1]`
2. Header名称 `row['Header1']`
3. 空字符串（如果都不存在）

这确保了灵活性和正确性。
