use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// GUI应用状态
pub struct InfiniteApp {
    // 游戏路径
    game_path: String,
    // Mod列表
    mods: Vec<ModEntry>,
    // 状态
    status_message: Arc<Mutex<String>>,
    is_processing: Arc<Mutex<bool>>,
    // 进度信息
    progress: Arc<Mutex<Option<String>>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ModEntry {
    path: String,
    enabled: bool,
    name: String,
}

/// 持久化配置
#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    game_path: String,
    mods: Vec<ModEntry>,
}

impl AppConfig {
    /// 获取配置文件路径
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("infinite");
        path.push("gui_config.json");
        path
    }

    /// 从文件加载配置
    fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// 保存配置到文件
    fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}

impl Default for InfiniteApp {
    fn default() -> Self {
        Self::new()
    }
}

impl InfiniteApp {
    pub fn new() -> Self {
        // 加载保存的配置
        let config = AppConfig::load();

        Self {
            game_path: config.game_path,
            mods: config.mods,
            status_message: Arc::new(Mutex::new("准备就绪".to_string())),
            is_processing: Arc::new(Mutex::new(false)),
            progress: Arc::new(Mutex::new(None)),
        }
    }

    /// 保存当前配置
    fn save_config(&self) {
        let config = AppConfig {
            game_path: self.game_path.clone(),
            mods: self.mods.clone(),
        };

        if let Err(e) = config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }

    fn select_game_path(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择暗黑破坏神2重制版游戏目录")
            .pick_folder()
        {
            self.game_path = path.to_string_lossy().to_string();
            *self.status_message.lock().unwrap() = format!("已选择游戏路径: {}", self.game_path);
            self.save_config();
        }
    }

    fn load_mod_list(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("打开Mod列表文件")
            .add_filter("文本文件", &["txt"])
            .pick_file()
        {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    self.mods.clear();
                    for line in content.lines() {
                        let line = line.trim();
                        if !line.is_empty() && !line.starts_with('#') {
                            // 尝试从路径提取mod名称
                            let name = self.get_mod_name(line);

                            self.mods.push(ModEntry {
                                path: line.to_string(),
                                enabled: true,
                                name,
                            });
                        }
                    }
                    *self.status_message.lock().unwrap() = format!("已加载 {} 个mod", self.mods.len());
                    self.save_config();
                }
                Err(e) => {
                    *self.status_message.lock().unwrap() = format!("加载失败: {}", e);
                }
            }
        }
    }

    fn get_mod_name(&self, path: &str) -> String {
        // 尝试读取mod.json获取名称
        let mod_json_path = PathBuf::from(path).join("mod.json");
        if let Ok(content) = std::fs::read_to_string(&mod_json_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                    return name.to_string();
                }
            }
        }

        // 如果无法读取，使用文件夹名称
        PathBuf::from(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string()
    }

    fn save_mod_list(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("保存Mod列表")
            .add_filter("文本文件", &["txt"])
            .set_file_name("mod_list.txt")
            .save_file()
        {
            let content: String = self.mods
                .iter()
                .filter(|m| m.enabled)
                .map(|m| m.path.clone())
                .collect::<Vec<_>>()
                .join("\n");

            match std::fs::write(&path, content) {
                Ok(_) => {
                    *self.status_message.lock().unwrap() = "Mod列表已保存".to_string();
                }
                Err(e) => {
                    *self.status_message.lock().unwrap() = format!("保存失败: {}", e);
                }
            }
        }
    }

    fn add_mod_folder(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("选择Mod文件夹")
            .pick_folder()
        {
            let path_str = path.to_string_lossy().to_string();
            let name = self.get_mod_name(&path_str);

            self.mods.push(ModEntry {
                path: path_str.clone(),
                enabled: true,
                name,
            });

            *self.status_message.lock().unwrap() = "已添加Mod".to_string();
            self.save_config();
        }
    }

    fn remove_mod(&mut self, index: usize) {
        if index < self.mods.len() {
            self.mods.remove(index);
            *self.status_message.lock().unwrap() = "已删除Mod".to_string();
            self.save_config();
        }
    }

    fn move_mod_up(&mut self, index: usize) {
        if index > 0 && index < self.mods.len() {
            self.mods.swap(index - 1, index);
            self.save_config();
        }
    }

    fn move_mod_down(&mut self, index: usize) {
        if index < self.mods.len().saturating_sub(1) {
            self.mods.swap(index, index + 1);
            self.save_config();
        }
    }

    fn generate_mods(&mut self, ctx: egui::Context) {
        if self.game_path.is_empty() {
            *self.status_message.lock().unwrap() = "请先选择游戏路径".to_string();
            return;
        }

        if self.mods.is_empty() {
            *self.status_message.lock().unwrap() = "请先添加Mod".to_string();
            return;
        }

        let enabled_mods: Vec<String> = self.mods
            .iter()
            .filter(|m| m.enabled)
            .map(|m| m.path.clone())
            .collect();

        if enabled_mods.is_empty() {
            *self.status_message.lock().unwrap() = "没有启用的Mod".to_string();
            return;
        }

        // 计算输出路径
        let output_path = format!("{}/Mods/Infinite/Infinite.mpq/data", self.game_path);

        *self.status_message.lock().unwrap() = format!("正在生成 {} 个mod...", enabled_mods.len());
        *self.is_processing.lock().unwrap() = true;
        *self.progress.lock().unwrap() = Some("初始化...".to_string());

        // 克隆必要的数据
        let game_path = self.game_path.clone();
        let mods = enabled_mods.clone();
        let status_msg = self.status_message.clone();
        let is_proc = self.is_processing.clone();
        let progress = self.progress.clone();

        // 在新线程中运行
        std::thread::spawn(move || {
            // 创建临时mod列表文件
            let temp_list = std::env::temp_dir().join("infinite_gui_mods.txt");
            if let Err(e) = std::fs::write(&temp_list, mods.join("\n")) {
                *status_msg.lock().unwrap() = format!("❌ 无法创建临时文件: {}", e);
                *is_proc.lock().unwrap() = false;
                *progress.lock().unwrap() = None;
                ctx.request_repaint();
                return;
            }

            *progress.lock().unwrap() = Some("正在处理mods...".to_string());
            ctx.request_repaint();

            // 查找infinite CLI可执行文件
            let cli_exe = if let Ok(current_exe) = std::env::current_exe() {
                // 尝试在同一目录下查找infinite.exe
                let exe_dir = current_exe.parent().unwrap();
                let infinite_exe = exe_dir.join("infinite.exe");
                if infinite_exe.exists() {
                    infinite_exe
                } else {
                    // 如果找不到，尝试使用PATH中的infinite命令
                    std::path::PathBuf::from("infinite")
                }
            } else {
                std::path::PathBuf::from("infinite")
            };

            // 调用infinite CLI（不指定output-path，使用默认路径）
            let result = std::process::Command::new(&cli_exe)
                .args(&[
                    "install",
                    "--game-path",
                    &game_path,
                    "--mod-list",
                    temp_list.to_str().unwrap(),
                ])
                .output();

            // 清理临时文件
            let _ = std::fs::remove_file(&temp_list);

            match result {
                Ok(output) => {
                    if output.status.success() {
                        *status_msg.lock().unwrap() = format!("✅ 成功生成到: {}", output_path);
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        *status_msg.lock().unwrap() = format!("❌ 生成失败: {}", stderr);
                    }
                }
                Err(e) => {
                    *status_msg.lock().unwrap() = format!("❌ 无法执行命令: {}", e);
                }
            }

            *is_proc.lock().unwrap() = false;
            *progress.lock().unwrap() = None;
            ctx.request_repaint();
        });
    }
}

impl eframe::App for InfiniteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let is_processing = *self.is_processing.lock().unwrap();
        let status_message = self.status_message.lock().unwrap().clone();
        let progress = self.progress.lock().unwrap().clone();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Infinite - Diablo II: Resurrected Mod Manager");
            ui.add_space(10.0);

            // 游戏路径选择
            ui.horizontal(|ui| {
                ui.label("游戏路径:");
                if ui.button("📁 选择游戏目录").clicked() {
                    self.select_game_path();
                }
                ui.add_space(10.0);
                if !self.game_path.is_empty() {
                    ui.label(&self.game_path);
                } else {
                    ui.label("未选择");
                }
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // Mod列表管理
            ui.horizontal(|ui| {
                ui.heading("Mod 列表");
                ui.add_space(20.0);

                if ui.button("📂 打开Mod列表").clicked() && !is_processing {
                    self.load_mod_list();
                }

                if ui.button("💾 保存Mod列表").clicked() && !is_processing {
                    self.save_mod_list();
                }

                if ui.button("➕ 添加Mod文件夹").clicked() && !is_processing {
                    self.add_mod_folder();
                }
            });

            ui.add_space(10.0);

            // Mod列表显示
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if self.mods.is_empty() {
                        ui.label(
                            egui::RichText::new("没有mod，请添加或打开mod列表")
                                .italics()
                                .color(egui::Color32::GRAY)
                        );
                    } else {
                        let mut to_remove = None;
                        let mut to_move_up = None;
                        let mut to_move_down = None;
                        let mut config_changed = false;

                        for (index, mod_entry) in self.mods.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                // 启用/禁用复选框
                                if ui.checkbox(&mut mod_entry.enabled, "").changed() {
                                    config_changed = true;
                                }

                                // Mod名称
                                ui.label(&mod_entry.name);

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    // 删除按钮
                                    if ui.button("🗑").clicked() && !is_processing {
                                        to_remove = Some(index);
                                    }

                                    // 下移按钮
                                    if ui.button("⬇").clicked() && !is_processing {
                                        to_move_down = Some(index);
                                    }

                                    // 上移按钮
                                    if ui.button("⬆").clicked() && !is_processing {
                                        to_move_up = Some(index);
                                    }

                                    // 路径显示
                                    ui.label(
                                        egui::RichText::new(&mod_entry.path)
                                            .small()
                                            .color(egui::Color32::GRAY)
                                    );
                                });
                            });
                            ui.add_space(5.0);
                        }

                        // 处理操作
                        if let Some(index) = to_remove {
                            self.remove_mod(index);
                        }
                        if let Some(index) = to_move_up {
                            self.move_mod_up(index);
                        }
                        if let Some(index) = to_move_down {
                            self.move_mod_down(index);
                        }

                        // 如果复选框状态改变，保存配置
                        if config_changed {
                            self.save_config();
                        }
                    }
                });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            // 进度显示
            if let Some(prog) = progress {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(&prog);
                });
                ui.add_space(5.0);
            }

            // 生成按钮
            ui.horizontal(|ui| {
                let enabled = !is_processing
                    && !self.game_path.is_empty()
                    && !self.mods.is_empty()
                    && self.mods.iter().any(|m| m.enabled);

                ui.add_enabled_ui(enabled, |ui| {
                    let button = egui::Button::new(
                        egui::RichText::new("🚀 生成Mods")
                            .size(20.0)
                    );

                    if ui.add_sized([150.0, 40.0], button).clicked() {
                        self.generate_mods(ctx.clone());
                    }
                });

                ui.add_space(20.0);

                // 显示输出路径
                if !self.game_path.is_empty() {
                    let output_path = format!("{}/Mods/Infinite/Infinite.mpq/data", self.game_path);
                    ui.label(
                        egui::RichText::new(format!("输出路径: {}", output_path))
                            .small()
                            .color(egui::Color32::LIGHT_GRAY)
                    );
                }
            });

            ui.add_space(10.0);

            // 状态栏
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("状态:");
                ui.label(
                    egui::RichText::new(&status_message)
                        .color(if is_processing {
                            egui::Color32::YELLOW
                        } else if status_message.starts_with("✅") {
                            egui::Color32::GREEN
                        } else if status_message.starts_with("❌") {
                            egui::Color32::RED
                        } else {
                            egui::Color32::LIGHT_BLUE
                        })
                );
            });
        });
    }
}
