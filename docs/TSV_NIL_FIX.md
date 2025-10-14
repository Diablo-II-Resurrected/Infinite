# TSV Nil值列数错位问题修复

## 问题描述

在使用 `infinite.writeTsv()` 写入TSV数据时，如果某个单元格的值为nil或空字符串，会导致该行后续所有列的数据丢失，造成列数错位。

### 原因分析

之前的实现使用了一个循环来逐个读取列数据，当遇到第一个nil值时就会退出循环：

```rust
// 旧代码 - 有问题
let mut col_idx = 1;
loop {
    match row_table.get::<usize, String>(col_idx) {
        Ok(cell) => {
            row.push(cell);
            col_idx += 1;
        }
        Err(_) => break,  // ❌ 遇到nil就退出，后续列都丢失了
    }
}
```

### 修复方案

新的实现基于表头的列数来遍历所有列，即使某些列的值为nil也会保留为空字符串：

```rust
// 新代码 - 已修复
let max_col = if num_columns > 0 {
    num_columns  // 使用表头的列数
} else {
    // 如果没有表头，查找该行的最大列索引
    let mut max = 0;
    for pair in row_table.clone().pairs::<mlua::Value, mlua::Value>() {
        let (k, _) = pair?;
        if let mlua::Value::Integer(i) = k {
            if i > 0 {
                max = max.max(i as usize);
            }
        }
    }
    max
};

// 遍历所有列，nil值转换为空字符串
for col_idx in 1..=max_col {
    let cell = row_table.get::<usize, String>(col_idx)
        .unwrap_or_else(|_| String::new());  // ✓ nil转为空字符串
    row.push(cell);
}
```

## 修复效果

### 修复前

如果有一行数据：`["A", "", "C", "D", "E"]`

写入TSV后会变成：`["A"]`（第2列是空导致后面的列都丢失）

### 修复后

正确保留所有列：`["A", "", "C", "D", "E"]`

## 测试验证

创建了两个测试模组来验证修复：

1. **tsv_nil_simple** - 简单测试，创建包含空值的新TSV文件
2. **tsv_nil_test** - 完整测试，使用真实的游戏数据文件

运行测试：

```bash
cargo run -- install tsv_nil_simple
```

预期输出：

```
✓✓✓ SUCCESS: All rows have correct column count!
The nil value fix is working correctly!
```

## 影响范围

此修复影响所有使用 `infinite.writeTsv()` 的Lua模组。现在可以安全地：

1. 设置某些列为空字符串
2. 删除某些列的值（设为nil或空）
3. 插入包含空值的新行

所有情况下都能正确保持列数对齐。

## 相关文件

- `src/lua_api/infinite.rs` - writeTsv函数的实现
- `test_mods/tsv_nil_simple/` - 简单测试模组
- `test_mods/tsv_nil_test/` - 完整测试模组
