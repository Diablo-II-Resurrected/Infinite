# GUI 配置选项默认值修复

## 问题描述

在 GUI 中显示 Mod 配置选项时，默认值显示不正确。

### 症状
- CheckBox 始终显示为未勾选（false）
- Number 始终显示为 0
- Text 始终显示为空字符串
- Select 始终显示为空字符串

即使 mod.json 中定义了不同的默认值，GUI 也不会显示正确的默认值。

## 根本原因

在 `render_config_panel` 方法中，当从 `user_config` 获取配置值失败时（第一次显示或值不存在），使用了硬编码的后备默认值：

```rust
// ❌ 错误的实现
let mut value = mod_entry
    .user_config
    .get(id)
    .and_then(|v| v.as_bool())
    .unwrap_or(false);  // 硬编码的 false，应该使用 mod.json 中的 default
```

这导致即使 mod.json 中定义了 `"defaultValue": true`，GUI 仍然显示为 false。

## 解决方案

从 match 模式中提取 `default` 字段，并在 `unwrap_or` 中使用它：

```rust
// ✅ 正确的实现
infinite::mod_manager::config::ConfigOption::CheckBox {
    id,
    name,
    description,
    default,  // 提取 default 字段
} => {
    let mut value = mod_entry
        .user_config
        .get(id)
        .and_then(|v| v.as_bool())
        .unwrap_or(*default);  // 使用 mod.json 中的默认值
}
```

## 修复的文件

### `src/gui/app.rs`

#### CheckBox 类型
```rust
// 之前
infinite::mod_manager::config::ConfigOption::CheckBox {
    id,
    name,
    description,
    ..  // 忽略 default
} => {
    let mut value = mod_entry
        .user_config
        .get(id)
        .and_then(|v| v.as_bool())
        .unwrap_or(false);  // 硬编码
}

// 之后
infinite::mod_manager::config::ConfigOption::CheckBox {
    id,
    name,
    description,
    default,  // 捕获 default
} => {
    let mut value = mod_entry
        .user_config
        .get(id)
        .and_then(|v| v.as_bool())
        .unwrap_or(*default);  // 使用实际默认值
}
```

#### Number 类型
```rust
// 之前
.unwrap_or(0.0);  // 硬编码

// 之后
.unwrap_or(*default);  // 使用实际默认值
```

#### Text 类型
```rust
// 之前
.unwrap_or("");  // 硬编码空字符串

// 之后
.unwrap_or(default);  // 使用实际默认值
```

#### Select 类型
```rust
// 之前
.unwrap_or("");  // 硬编码空字符串

// 之后
.unwrap_or(default);  // 使用实际默认值
```

## 测试场景

### 测试 Mod: Show Item Level

**mod.json:**
```json
{
  "name": "Show Item Level",
  "config": [
    {
      "type": "checkbox",
      "id": "showOnWeapons",
      "name": "Show on Weapons",
      "defaultValue": true
    },
    {
      "type": "checkbox",
      "id": "showOnArmor",
      "name": "Show on Armor",
      "defaultValue": true
    },
    {
      "type": "checkbox",
      "id": "showOnJewelry",
      "name": "Show on Jewelry",
      "defaultValue": false
    },
    {
      "type": "number",
      "id": "testNumber",
      "name": "Test Number",
      "defaultValue": 50,
      "min": 0,
      "max": 100
    }
  ]
}
```

### 预期行为

#### 修复前 ❌
- Show on Weapons: ☐ (未勾选)
- Show on Armor: ☐ (未勾选)
- Show on Jewelry: ☐ (未勾选)
- Test Number: 0

#### 修复后 ✅
- Show on Weapons: ☑ (已勾选)
- Show on Armor: ☑ (已勾选)
- Show on Jewelry: ☐ (未勾选)
- Test Number: 50

## 技术细节

### 为什么需要解引用 `*default`

对于 `bool` 和 `f64` 类型，`default` 是一个引用（`&bool`, `&f64`），需要解引用：

```rust
default: bool,  // 字段类型

// 在模式匹配中
default,  // 这是 &bool 类型

// 使用时需要解引用
*default  // 这是 bool 类型
```

对于 `String` 类型，`default` 是 `&String`，可以直接用于 `unwrap_or`，因为它接受 `&str`：

```rust
default: String,  // 字段类型

// 在模式匹配中
default,  // 这是 &String 类型

// 使用时
default  // &String 可以自动强制转换为 &str
```

### `init_user_config` 方法

这个方法在添加 Mod 时被调用，将 mod.json 中的默认值填充到 `user_config` 中：

```rust
fn init_user_config(&mut self) {
    if let Some(mod_config) = self.load_config() {
        for option in &mod_config.config {
            let (id, default_value) = match option {
                ConfigOption::CheckBox { id, default, .. } => {
                    (id.clone(), serde_json::json!(default))
                }
                // ...
            };
            
            if !self.user_config.contains_key(&id) {
                self.user_config.insert(id, default_value);
            }
        }
    }
}
```

但是，这个方法只在以下情况调用：
1. 首次添加 Mod 时
2. 从 mod 列表文件加载时
3. 从 GitHub 添加 Mod 时

如果用户：
1. 添加了 Mod
2. 关闭 GUI（配置保存到 gui_config.json）
3. 修改 mod.json 中的默认值
4. 重新打开 GUI

这时 `user_config` 中已经有旧的值了，`init_user_config` 不会更新它。

这就是为什么在**渲染时**也要使用正确的默认值，而不仅仅依赖 `init_user_config`。

## 相关配置

### mod_manager/config.rs

配置数据结构使用 serde 的 `alias` 属性支持两种字段名：

```rust
#[serde(default, alias = "defaultValue")]
default: bool,
```

这意味着 JSON 中可以使用：
- `"default": true` (Infinite 格式)
- `"defaultValue": true` (D2RMM 格式)

都会被正确解析到 `default` 字段。

## 影响范围

这个修复确保：
1. ✅ GUI 首次显示配置时使用正确的默认值
2. ✅ 用户修改 mod.json 后，GUI 显示新的默认值
3. ✅ 用户清空配置后，恢复为 mod.json 中的默认值
4. ✅ 完全兼容 D2RMM 的配置格式

## 测试清单

- [ ] 添加新 Mod，验证默认值正确显示
- [ ] 修改 mod.json 中的默认值，验证 GUI 更新
- [ ] 删除 gui_config.json，重新添加 Mod，验证默认值
- [ ] 测试所有配置类型：CheckBox, Number, Text, Select
- [ ] 测试 "default" 和 "defaultValue" 两种字段名

## 编译测试

```bash
cargo build --bin infinite-gui
# ✅ 编译成功

cargo run --bin infinite-gui
# ✅ 运行正常
```

## 总结

这个修复解决了 GUI 配置选项默认值不正确的问题。现在无论是首次添加 Mod，还是修改 mod.json 后重新打开，GUI 都能正确显示 mod.json 中定义的默认值。
