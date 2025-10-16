// 在 release 模式下禁用控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    // 设置日志
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 660.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(
                // 可以添加图标
                eframe::icon_data::from_png_bytes(&[])
                    .unwrap_or_default()
            ),
        ..Default::default()
    };

    eframe::run_native(
        "Infinite - D2R Mod Manager",
        options,
        Box::new(|cc| {
            // 设置样式
            cc.egui_ctx.set_visuals(egui::Visuals::dark());

            // 设置中文字体
            setup_custom_fonts(&cc.egui_ctx);

            Box::new(app::InfiniteApp::new())
        }),
    )
}

/// 设置自定义字体以支持中文
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 添加中文字体
    // 使用思源黑体或者Windows系统字体
    #[cfg(target_os = "windows")]
    {
        // 尝试加载Windows系统中文字体
        if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        } else if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\simhei.ttf") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        } else if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\simsun.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Linux系统字体路径
        if let Ok(font_data) = std::fs::read("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        } else if let Ok(font_data) = std::fs::read("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS系统字体路径
        if let Ok(font_data) = std::fs::read("/System/Library/Fonts/PingFang.ttc") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        } else if let Ok(font_data) = std::fs::read("/Library/Fonts/Arial Unicode.ttf") {
            fonts.font_data.insert(
                "chinese".to_owned(),
                egui::FontData::from_owned(font_data),
            );
        }
    }

    // 将中文字体添加到字体家族中（如果成功加载）
    if fonts.font_data.contains_key("chinese") {
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "chinese".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("chinese".to_owned());
    }

    ctx.set_fonts(fonts);
}
