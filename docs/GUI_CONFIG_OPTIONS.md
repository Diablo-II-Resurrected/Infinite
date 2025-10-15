# GUI集成 - Mod配置选项

## 功能概述

现在GUI支持在界面上直接配置Mod选项,无需手动编辑JSON文件。

## 实现细节

### 1. 数据结构扩展

#### ModEntry 结构
```rust
#[derive(Clone, Serialize, Deserialize)]
struct ModEntry {
    path: String,
    enabled: bool,
    name: String,
    user_config: HashMap<String, serde_json::Value>,  // 新增:用户配置
}
```

#### InfiniteApp 结构
```rust
pub struct InfiniteApp {
    // ...
    selected_mod_index: Option<usize>,  // 新增:当前选中的mod索引
    // ...
}
```

### 2. 配置加载

当添加Mod时,自动加载 `mod.json` 并初始化用户配置:

```rust
impl ModEntry {
    fn load_config(&self) -> Option<ModConfig> {
        // 读取 mod.json
    }

    fn init_user_config(&mut self) {
        // 从 mod.json 的 config 字段初始化默认值
        // 支持4种类型: CheckBox, Number, Text, Select
    }
}
```

### 3. UI实现

#### Mod列表增强
- ✅ 复选框:启用/禁用Mod
- ✅ Mod名称:点击选中/取消选中
- ✅ ⚙ 按钮:打开配置面板
- ✅ ⬆ ⬇ 按钮:调整Mod顺序
- ✅ 🗑 按钮:删除Mod

#### 配置面板
当选中Mod时,显示配置面板:

**支持的配置类型:**

1. **CheckBox (复选框)**
   ```json
   {
     "type": "checkbox",
     "id": "enableFeature",
     "name": "启用功能",
     "description": "描述文字",
     "defaultValue": true
   }
   ```
   - UI: egui::Checkbox

2. **Number (数字)**
   ```json
   {
     "type": "number",
     "id": "multiplier",
     "name": "倍数",
     "description": "描述文字",
     "defaultValue": 2.0,
     "min": 1.0,
     "max": 10.0
   }
   ```
   - UI: egui::Slider (有范围时) 或 egui::DragValue (无范围时)

3. **Text (文本)**
   ```json
   {
     "type": "text",
     "id": "customText",
     "name": "自定义文本",
     "description": "描述文字",
     "defaultValue": "默认值"
   }
   ```
   - UI: egui::TextEdit

4. **Select (下拉选择)**
   ```json
   {
     "type": "select",
     "id": "difficulty",
     "name": "难度",
     "description": "描述文字",
     "defaultValue": "normal",
     "options": [
       {"label": "简单", "value": "easy"},
       {"label": "普通", "value": "normal"},
       {"label": "困难", "value": "hard"}
     ]
   }
   ```
   - UI: egui::ComboBox

### 4. 配置持久化

#### GUI配置
用户的Mod列表和配置保存到:
```
~/.config/infinite/gui_config.json  (Linux/macOS)
%APPDATA%\infinite\gui_config.json  (Windows)
```

包含:
- 游戏路径
- Mod列表(路径、启用状态、名称)
- 每个Mod的用户配置值

#### 运行时配置传递
当点击"生成Mods"时:
1. 创建临时Mod列表文件
2. 为每个启用的Mod创建用户配置JSON文件
3. 调用 `infinite.exe install` 命令
4. CLI读取并应用用户配置

### 5. 配置面板UI布局

```
┌─────────────────────────────────────────┐
│ ⚙ [Mod名称] - 配置                      │
│ [描述文字]                               │
│ ────────────────────────────────────    │
│                                          │
│ ┌──────────────────────────────────┐   │
│ │ [滚动区域]                        │   │
│ │                                   │   │
│ │ ☑ 选项1名称                       │   │
│ │    选项1描述                       │   │
│ │                                   │   │
│ │ 选项2名称: [━━━━●━━━] 50         │   │
│ │    选项2描述                       │   │
│ │                                   │   │
│ │ 选项3名称: [文本输入框___________] │   │
│ │    选项3描述                       │   │
│ │                                   │   │
│ │ 选项4名称: [下拉选择 ▼]           │   │
│ │    选项4描述                       │   │
│ │                                   │   │
│ └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## 使用示例

### 示例Mod: ShowItemLevel

**mod.json:**
```json
{
  "name": "Show Item Level",
  "description": "Display item level on items",
  "author": "Example",
  "version": "1.0.0",
  "config": [
    {
      "type": "checkbox",
      "id": "showOnWeapons",
      "name": "Show on Weapons",
      "description": "Display level on weapons",
      "defaultValue": true
    },
    {
      "type": "checkbox",
      "id": "showOnArmor",
      "name": "Show on Armor",
      "description": "Display level on armor",
      "defaultValue": true
    },
    {
      "type": "checkbox",
      "id": "showOnJewelry",
      "name": "Show on Jewelry",
      "description": "Display level on jewelry",
      "defaultValue": false
    }
  ]
}
```

**mod.js:**
```javascript
const config = D2RMM.getConfiguration();

if (config.showOnWeapons) {
  const weapons = D2RMM.readTsv('global\\excel\\weapons.txt');
  weapons.rows.forEach((row) => {
    row.ShowLevel = 1;
  });
  D2RMM.writeTsv('global\\excel\\weapons.txt', weapons);
}

if (config.showOnArmor) {
  const armor = D2RMM.readTsv('global\\excel\\armor.txt');
  armor.rows.forEach((row) => {
    row.ShowLevel = 1;
  });
  D2RMM.writeTsv('global\\excel\\armor.txt', armor);
}

if (config.showOnJewelry) {
  // ...
}
```

### GUI操作流程

1. **添加Mod**
   - 点击"➕ 添加Mod文件夹"
   - 选择ShowItemLevel文件夹
   - Mod自动加载,user_config初始化为默认值

2. **配置Mod**
   - 点击Mod名称或⚙按钮,打开配置面板
   - 勾选/取消勾选复选框
   - 配置自动保存到gui_config.json

3. **生成Mods**
   - 点击"🚀 生成Mods"
   - GUI创建临时配置文件
   - 调用CLI安装Mods
   - 用户配置传递到mod脚本

## 技术要点

### 1. 借用规则处理

由于egui的闭包借用规则,配置面板渲染时需要特殊处理:

```rust
fn render_config_panel(&mut self, ui: &mut egui::Ui) {
    // 先克隆配置,避免在闭包中借用self
    let mod_config = self.mods[index].load_config();
    let config_options = mod_config.config.clone();
    
    ui.group(|ui| {
        // 在闭包中访问 self.mods[index]
        for option in &config_options {
            match option {
                // 处理不同类型的配置项
            }
        }
    });
}
```

### 2. 临时值生命周期

```rust
// ❌ 错误:临时值生命周期太短
let mod_name = PathBuf::from(mod_path)
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("unknown");

// ✅ 正确:创建长生命周期绑定
let path_buf = PathBuf::from(mod_path);
let mod_name = path_buf
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("unknown");
```

### 3. 配置值变更检测

对于Number类型,需要正确处理变更:

```rust
let changed = ui.horizontal(|ui| {
    ui.label(name);
    if min.is_none() && max.is_none() {
        ui.add(egui::DragValue::new(&mut value)).changed()
    } else {
        ui.add(egui::Slider::new(&mut value, min..=max)).changed()
    }
}).inner;

if changed {
    mod_entry.user_config.insert(id.clone(), serde_json::json!(value));
    config_changed = true;
}
```

## 未来改进

### 1. 配置验证
- [ ] 添加值范围验证
- [ ] 添加必填项检查
- [ ] 添加配置冲突检测

### 2. UI增强
- [ ] 支持配置分组
- [ ] 支持配置搜索/过滤
- [ ] 支持配置重置为默认值
- [ ] 支持配置导入/导出

### 3. 类型扩展
- [ ] 支持Color类型(颜色选择器)
- [ ] 支持File类型(文件选择器)
- [ ] 支持Array类型(列表编辑)

### 4. 配置预设
- [ ] 支持保存配置预设
- [ ] 支持加载配置预设
- [ ] 支持分享配置预设

## 测试

### 手动测试清单

- [ ] 添加有配置选项的Mod
- [ ] 点击Mod名称,配置面板显示/隐藏
- [ ] 修改CheckBox,值正确保存
- [ ] 修改Number(有范围),Slider正常工作
- [ ] 修改Number(无范围),DragValue正常工作
- [ ] 修改Text,文本正确保存
- [ ] 修改Select,选项正确保存
- [ ] 关闭GUI重新打开,配置值保持
- [ ] 生成Mods,配置传递到脚本
- [ ] 查看输出文件,验证配置生效

### 自动化测试

TODO: 添加集成测试

## 相关文件

- `src/gui/app.rs` - GUI应用主逻辑
- `src/gui/main.rs` - GUI入口
- `src/mod_manager/config.rs` - 配置数据结构
- `docs/FIX_MIXED_LUA_JS_MODS.md` - 配置兼容性文档

## 参考资料

- [egui官方文档](https://docs.rs/egui/)
- [D2RMM配置格式](https://github.com/olegbl/d2rmm)
- [serde JSON序列化](https://docs.rs/serde_json/)
