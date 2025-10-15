# GUI 配置面板改进

## 更新日期
2025年10月15日

## 改进内容

### 1. 智能隐藏配置按钮

**问题:** 所有 Mod 都显示配置按钮 ⚙,即使没有配置选项

**解决方案:** 
- 检查 Mod 是否有配置选项
- 只在有配置选项时显示 ⚙ 按钮和可选中的名称
- 无配置选项的 Mod 使用普通标签显示名称

**实现代码:**
```rust
// 检查是否有配置选项
let has_config = mod_entry.load_config()
    .map(|cfg| !cfg.config.is_empty())
    .unwrap_or(false);

// Mod名称 - 如果有配置,点击可选中/取消选中
if has_config {
    let name_response = ui.selectable_label(is_selected, &mod_entry.name);
    if name_response.clicked() {
        self.selected_mod_index = if is_selected {
            None
        } else {
            Some(index)
        };
    }
} else {
    ui.label(&mod_entry.name);
}

// 配置按钮 - 只在有配置选项时显示
if has_config {
    if ui.button("⚙").clicked() {
        self.selected_mod_index = if is_selected {
            None
        } else {
            Some(index)
        };
    }
}
```

**效果:**
- ✅ 有配置的 Mod: 显示 ⚙ 按钮,名称可点击选中
- ✅ 无配置的 Mod: 不显示 ⚙ 按钮,名称为普通文本
- ✅ UI 更简洁,避免混淆

### 2. 配置面板全宽显示

**问题:** 配置面板宽度受限,不能充分利用窗口空间

**解决方案:**
- 使用 `ui.available_width()` 获取可用宽度
- 使用 `ui.allocate_ui_with_layout()` 强制面板使用全宽
- 设置内部 UI 宽度匹配可用宽度

**实现代码:**
```rust
// 使用 available_width 使配置面板与窗口同宽
ui.allocate_ui_with_layout(
    egui::vec2(ui.available_width(), 0.0),
    egui::Layout::top_down(egui::Align::Min),
    |ui| {
        ui.group(|ui| {
            // 确保内部也使用全宽
            ui.set_width(ui.available_width());
            
            ui.heading(format!("⚙ {} - 配置", mod_name));
            // ... 配置选项 ...
        });
    },
);
```

**效果:**
- ✅ 配置面板宽度 = 窗口宽度
- ✅ 更好的视觉效果
- ✅ 更多空间显示配置项

## 使用体验

### 场景 1: 有配置选项的 Mod

```
┌─────────────────────────────────────────────────┐
│ ☑ [Show Item Level]    ⬆ ⬇ ⚙ 🗑  path/to/mod   │
└─────────────────────────────────────────────────┘
```
- 点击名称或 ⚙ 按钮 → 打开配置面板
- 配置面板占满窗口宽度

### 场景 2: 无配置选项的 Mod

```
┌─────────────────────────────────────────────────┐
│ ☑ Simple Mod           ⬆ ⬇ 🗑     path/to/mod   │
└─────────────────────────────────────────────────┘
```
- 没有 ⚙ 按钮
- 名称不可点击选中
- UI 更简洁

## 技术细节

### 配置检测逻辑

```rust
let has_config = mod_entry.load_config()
    .map(|cfg| !cfg.config.is_empty())
    .unwrap_or(false);
```

1. 调用 `load_config()` 尝试读取 mod.json
2. 如果成功,检查 `config` 数组是否为空
3. 如果失败或为空,返回 `false`

### 全宽布局

```rust
ui.allocate_ui_with_layout(
    egui::vec2(ui.available_width(), 0.0),  // 宽度 = 可用宽度, 高度 = 自动
    egui::Layout::top_down(egui::Align::Min),  // 从上到下,左对齐
    |ui| {
        ui.group(|ui| {
            ui.set_width(ui.available_width());  // 强制内部也全宽
            // ...
        });
    },
);
```

## 测试清单

- [x] 添加有配置的 Mod (ShowItemLevel)
  - [x] 显示 ⚙ 按钮
  - [x] 点击名称显示配置面板
  - [x] 配置面板全宽显示
  
- [x] 添加无配置的 Mod
  - [x] 不显示 ⚙ 按钮
  - [x] 名称不可选中
  - [x] 点击名称无反应

- [x] 窗口调整大小
  - [x] 配置面板自动适应宽度

## 相关文件

- `src/gui/app.rs` - GUI 主逻辑
  - `render_config_panel()` - 配置面板渲染
  - Mod 列表渲染逻辑

## 未来改进

1. **配置面板高度**
   - 当前固定 max_height = 200px
   - 可以改为自适应窗口高度

2. **配置分组**
   - 支持将配置项分组显示
   - 可折叠的配置组

3. **配置搜索**
   - 在配置项较多时提供搜索功能
   - 高亮匹配项

## 编译测试

```bash
cargo build --bin infinite-gui
# ✅ 编译成功

cargo run --bin infinite-gui
# ✅ 运行正常
```

## 总结

这次改进让 GUI 更加智能和美观:
- ✅ 自动隐藏不需要的按钮
- ✅ 充分利用窗口空间
- ✅ 更好的用户体验
