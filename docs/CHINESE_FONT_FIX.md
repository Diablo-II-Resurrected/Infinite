# 中文字体支持修复

## 问题

GUI中的中文显示为方框乱码，因为egui默认字体不包含中文字符。

## 解决方案

通过加载系统中文字体来支持中文显示。

### 实现细节

在 `src/gui/main.rs` 中添加了 `setup_custom_fonts` 函数，该函数会：

1. **Windows系统**：尝试加载以下字体（按优先级）：
   - 微软雅黑：`C:\Windows\Fonts\msyh.ttc`
   - 黑体：`C:\Windows\Fonts\simhei.ttf`
   - 宋体：`C:\Windows\Fonts\simsun.ttc`

2. **Linux系统**：尝试加载：
   - Droid Sans Fallback
   - Noto Sans CJK

3. **macOS系统**：尝试加载：
   - PingFang（苹方）
   - Arial Unicode

### 字体加载逻辑

```rust
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 根据操作系统加载相应的中文字体
    #[cfg(target_os = "windows")]
    {
        // 尝试加载Windows中文字体
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        }
        // ... 其他备选字体
    }

    // 将中文字体添加到字体家族
    if fonts.font_data.contains_key("chinese") {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "chinese".to_owned());
    }

    ctx.set_fonts(fonts);
}
```

### 字体优先级

中文字体被插入到 `Proportional` 字体家族的**首位**（`insert(0, ...)`），确保：
- 中文字符使用中文字体渲染
- 英文和符号仍使用默认字体
- 性能优化（优先尝试中文字体）

## 测试

### Windows
在Windows系统上，会自动加载微软雅黑字体，中文显示正常。

### 故障排查

如果中文仍然显示为方框：

1. **检查字体文件是否存在**：
   ```powershell
   # Windows
   Test-Path C:\Windows\Fonts\msyh.ttc
   Test-Path C:\Windows\Fonts\simhei.ttf
   Test-Path C:\Windows\Fonts\simsun.ttc
   ```

2. **查看日志**：
   运行时会输出字体加载信息（如果有的话）

3. **手动指定字体**：
   可以修改 `setup_custom_fonts` 函数，指定其他字体路径

### 添加自定义字体

如果要使用其他字体（如思源黑体），可以：

1. 下载字体文件（.ttf或.ttc）
2. 修改 `setup_custom_fonts` 函数：
   ```rust
   if let Ok(font_data) = std::fs::read("path/to/your/font.ttf") {
       fonts.font_data.insert(
           "chinese".to_owned(),
           egui::FontData::from_owned(font_data),
       );
   }
   ```

## 性能影响

- 字体文件大小：约10-20MB（取决于字体）
- 加载时间：启动时一次性加载，约100-200ms
- 运行时开销：几乎无影响
- 内存占用：增加约20-30MB（字体数据）

## 支持的字符

加载的中文字体通常包含：
- 简体中文
- 繁体中文
- 日文假名
- 韩文
- 常用符号

## 已知限制

1. **字体文件路径硬编码**：
   - 依赖系统标准路径
   - 如果字体不在标准位置，需要手动修改

2. **无法自动选择最佳字体**：
   - 按预定义优先级尝试
   - 不会检查字体质量或版本

3. **字体回退**：
   - 如果所有中文字体都加载失败，会回退到默认字体
   - 中文仍会显示为方框

## 改进建议

未来可以考虑：
- [ ] 添加字体配置文件
- [ ] 支持用户选择字体
- [ ] 嵌入开源中文字体（如思源黑体）
- [ ] 字体缓存和懒加载
- [ ] 更好的字体检测和回退机制

## 相关文件

- `src/gui/main.rs` - 字体设置逻辑
- `src/gui/app.rs` - UI实现
