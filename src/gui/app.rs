use eframe::egui;
use infinite::ModConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// GUI应用状态
pub struct InfiniteApp {
    // 游戏路径
    game_path: String,
    // Mod列表
    mods: Vec<ModEntry>,
    // 当前选中的mod索引（用于显示配置面板）
    selected_mod_index: Option<usize>,
    // 状态
    status_message: Arc<Mutex<String>>,
    is_processing: Arc<Mutex<bool>>,
    // 进度信息
    progress: Arc<Mutex<Option<String>>>,
    // GitHub对话框状态
    github_dialog: Option<GitHubDialog>,
    // GitHub Token (可选)
    github_token: Option<String>,
    // GitHub API 速率限制信息
    github_rate_limit: Arc<Mutex<Option<GitHubRateLimit>>>,
    // 是否显示设置对话框
    show_settings: bool,
}

/// GitHub API 速率限制信息
#[derive(Clone)]
struct GitHubRateLimit {
    remaining: u32,
    limit: u32,
    reset_time: std::time::SystemTime,
}

/// GitHub Mod添加对话框
struct GitHubDialog {
    repo_url: String,
    branches: Arc<Mutex<Vec<String>>>,
    selected_branch: Option<String>,
    subdirs: Arc<Mutex<Vec<String>>>,
    selected_subdir: Option<String>,
    is_loading: Arc<Mutex<bool>>,
    is_loading_dirs: Arc<Mutex<bool>>,
    error_message: Arc<Mutex<Option<String>>>,
}

/// 配置加载状态
#[derive(Clone, Default)]
enum ConfigLoadState {
    #[default]
    NotLoaded,
    Loading,
    Loaded(ModConfig),
    Failed(String),
}

#[derive(Clone, Serialize, Deserialize)]
struct ModEntry {
    path: String,
    enabled: bool,
    name: String,
    /// 用户配置值（配置项ID -> 值）
    #[serde(default)]
    user_config: HashMap<String, serde_json::Value>,
    /// 缓存的配置加载状态 (不持久化)
    #[serde(skip)]
    config_state: Arc<Mutex<ConfigLoadState>>,
}

impl ModEntry {
    /// 从路径加载ModConfig
    fn load_config(&self, ctx: Option<egui::Context>, github_token: Option<String>) -> Option<ModConfig> {
        // 检查缓存状态
        let state = self.config_state.lock().unwrap().clone();
        match state {
            ConfigLoadState::Loaded(config) => return Some(config),
            ConfigLoadState::Failed(_) => return None, // 已经失败过,不再重试
            ConfigLoadState::Loading => return None, // 正在加载中
            ConfigLoadState::NotLoaded => {
                // 需要加载
            }
        }

        // 标记为正在加载
        *self.config_state.lock().unwrap() = ConfigLoadState::Loading;

        let result = if self.path.starts_with("github:") {
            // 尝试从缓存加载
            if let Some(cache_path) = self.resolve_github_path() {
                let mod_json = cache_path.join("mod.json");
                if mod_json.exists() {
                    if let Ok(content) = std::fs::read_to_string(&mod_json) {
                        if let Ok(config) = serde_json::from_str::<ModConfig>(&content) {
                            *self.config_state.lock().unwrap() = ConfigLoadState::Loaded(config.clone());
                            return Some(config);
                        }
                    }
                }
            }

            // 缓存不存在,启动异步任务从 GitHub API 获取
            self.load_config_from_github_async(ctx, github_token);
            None
        } else {
            let mod_json_path = PathBuf::from(&self.path).join("mod.json");
            if let Ok(content) = std::fs::read_to_string(&mod_json_path) {
                let config: Option<ModConfig> = serde_json::from_str(&content).ok();
                if let Some(ref cfg) = config {
                    *self.config_state.lock().unwrap() = ConfigLoadState::Loaded(cfg.clone());
                } else {
                    *self.config_state.lock().unwrap() = ConfigLoadState::Failed("Failed to parse config".to_string());
                }
                config
            } else {
                *self.config_state.lock().unwrap() = ConfigLoadState::Failed("Config file not found".to_string());
                None
            }
        };

        result
    }

    /// 异步从 GitHub API 加载配置
    fn load_config_from_github_async(&self, ctx: Option<egui::Context>, github_token: Option<String>) {
        if !self.path.starts_with("github:") {
            return;
        }

        println!("🌐 Fetching mod.json from GitHub for: {}", self.path);

        // 克隆必要的数据
        let path = self.path.clone();
        let config_state = self.config_state.clone();

        // 在后台线程中执行
        std::thread::spawn(move || {
            // 解析 GitHub 路径
            let path_str = &path[7..];
            let (path_without_branch, branch_opt) = if let Some(at_pos) = path_str.rfind('@') {
                (&path_str[..at_pos], Some(&path_str[at_pos + 1..]))
            } else {
                (path_str, None)
            };

            let (repo, subdir) = if let Some(colon_pos) = path_without_branch.find(':') {
                (&path_without_branch[..colon_pos], Some(&path_without_branch[colon_pos + 1..]))
            } else {
                (path_without_branch, None)
            };

            // 如果没有指定分支,先获取仓库的默认分支
            let branch = if let Some(b) = branch_opt {
                b.to_string()
            } else {
                // 查询仓库信息获取默认分支
                let repo_url = format!("https://api.github.com/repos/{}", repo);
                let mut repo_request = reqwest::blocking::Client::new()
                    .get(&repo_url)
                    .header("User-Agent", "infinite-mod-manager");
                
                if let Some(ref token) = github_token {
                    repo_request = repo_request.header("Authorization", format!("Bearer {}", token));
                }

                match repo_request.send() {
                    Ok(response) if response.status().is_success() => {
                        if let Ok(repo_info) = response.json::<serde_json::Value>() {
                            repo_info
                                .get("default_branch")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| "main".to_string())
                        } else {
                            "main".to_string()
                        }
                    }
                    _ => {
                        // 如果获取失败,回退到 main
                        eprintln!("⚠️ Failed to get default branch, trying 'main'");
                        "main".to_string()
                    }
                }
            };

            // 构建 GitHub API URL
            let file_path = if let Some(subdir) = subdir {
                format!("{}/mod.json", subdir)
            } else {
                "mod.json".to_string()
            };

            let url = format!(
                "https://api.github.com/repos/{}/contents/{}?ref={}",
                repo, file_path, branch
            );

            // 构建请求
            let mut request = reqwest::blocking::Client::new()
                .get(&url)
                .header("User-Agent", "infinite-mod-manager");

            // 如果有 token,添加认证
            if let Some(token) = github_token {
                request = request.header("Authorization", format!("Bearer {}", token));
            }

            // 尝试从 GitHub API 获取
            match request.send() {
                Ok(response) => {
                    // 检查速率限制
                    if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                        if let Ok(remaining_str) = remaining.to_str() {
                            if let Ok(remaining_num) = remaining_str.parse::<u32>() {
                                if remaining_num < 10 {
                                    eprintln!("⚠️ GitHub API rate limit warning: {} requests remaining", remaining_num);
                                }
                            }
                        }
                    }

                    if response.status().is_success() {
                        if let Ok(content_json) = response.json::<serde_json::Value>() {
                            // GitHub API 返回 base64 编码的内容
                            if let Some(content_b64) = content_json.get("content").and_then(|c| c.as_str()) {
                                // 移除换行符
                                let content_b64 = content_b64.replace("\n", "");
                                use base64::Engine;
                                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(&content_b64) {
                                    if let Ok(content_str) = String::from_utf8(decoded) {
                                        if let Ok(config) = serde_json::from_str(&content_str) {
                                            println!("✅ Successfully loaded mod.json from GitHub");
                                            *config_state.lock().unwrap() = ConfigLoadState::Loaded(config);
                                            // 请求重绘
                                            if let Some(ctx) = ctx {
                                                ctx.request_repaint();
                                            }
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                    } else if response.status().as_u16() == 403 {
                        eprintln!("⚠️ GitHub API rate limit exceeded. Consider adding a GitHub token in settings.");
                    } else {
                        eprintln!("⚠️ GitHub API error: {}", response.status());
                    }
                }
                Err(e) => {
                    eprintln!("❌ Failed to fetch mod.json from GitHub: {}", e);
                }
            }

            // 失败情况
            *config_state.lock().unwrap() = ConfigLoadState::Failed("Failed to load config from GitHub".to_string());
            if let Some(ctx) = ctx {
                ctx.request_repaint();
            }
        });
    }

    /// 解析 GitHub 路径到实际的缓存路径
    /// github:owner/repo:subdir@branch -> <config_dir>/infinite/mod_cache/owner/repo/branch/subdir
    fn resolve_github_path(&self) -> Option<PathBuf> {
        if !self.path.starts_with("github:") {
            return None;
        }

        // 移除 "github:" 前缀
        let path = &self.path[7..];

        // 分离分支 (如果有 @)
        let (path_without_branch, branch) = if let Some(at_pos) = path.rfind('@') {
            let branch = &path[at_pos + 1..];
            let path = &path[..at_pos];
            (path, branch)
        } else {
            (path, "main")
        };

        // 分离子目录 (如果有 :)
        let (repo, subdir) = if let Some(colon_pos) = path_without_branch.find(':') {
            let repo = &path_without_branch[..colon_pos];
            let subdir = &path_without_branch[colon_pos + 1..];
            (repo, Some(subdir))
        } else {
            (path_without_branch, None)
        };

        // 解析 owner/repo
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        // 构建缓存路径: <config_dir>/infinite/mod_cache/owner/repo/branch/subdir
        let cache_dir = AppConfig::cache_dir();
        let mut target_dir = cache_dir.join(parts[0]).join(parts[1]).join(branch);

        if let Some(subdir) = subdir {
            target_dir = target_dir.join(subdir);
        }

        Some(target_dir)
    }

    /// 初始化用户配置（使用默认值）
    fn init_user_config(&mut self) {
        if let Some(mod_config) = self.load_config(None, None) {
            for option in &mod_config.config {
                // 获取配置项的ID和默认值
                let (id, default_value) = match option {
                    infinite::mod_manager::config::ConfigOption::CheckBox {
                        id, default, ..
                    } => (id.clone(), serde_json::json!(default)),
                    infinite::mod_manager::config::ConfigOption::Number { id, default, .. } => {
                        (id.clone(), serde_json::json!(default))
                    }
                    infinite::mod_manager::config::ConfigOption::Text { id, default, .. } => {
                        (id.clone(), serde_json::json!(default))
                    }
                    infinite::mod_manager::config::ConfigOption::Select { id, default, .. } => {
                        (id.clone(), serde_json::json!(default))
                    }
                };

                // 如果用户配置中没有这个选项，使用默认值
                if !self.user_config.contains_key(&id) {
                    self.user_config.insert(id, default_value);
                }
            }
        }
    }

    /// 生成用户配置的JSON
    fn generate_user_config_json(&self) -> serde_json::Value {
        serde_json::to_value(&self.user_config).unwrap_or(serde_json::json!({}))
    }
}

/// 持久化配置
#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    game_path: String,
    mods: Vec<ModEntry>,
    #[serde(default)]
    github_token: Option<String>,
}

impl AppConfig {
    /// 获取数据目录路径
    fn data_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("infinite");
        path
    }

    /// 获取配置文件路径
    fn config_path() -> PathBuf {
        let mut path = Self::data_dir();
        path.push("gui_config.json");
        path
    }

    /// 获取 mod 缓存目录路径
    fn cache_dir() -> PathBuf {
        let mut path = Self::data_dir();
        path.push("mod_cache");
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
            game_path: config.game_path.clone(),
            mods: config.mods,
            selected_mod_index: None,
            status_message: Arc::new(Mutex::new("准备就绪".to_string())),
            is_processing: Arc::new(Mutex::new(false)),
            progress: Arc::new(Mutex::new(None)),
            github_dialog: None,
            github_token: config.github_token,
            github_rate_limit: Arc::new(Mutex::new(None)),
            show_settings: false,
        }
    }

    /// 保存当前配置
    fn save_config(&self) {
        let config = AppConfig {
            game_path: self.game_path.clone(),
            mods: self.mods.clone(),
            github_token: self.github_token.clone(),
        };

        if let Err(e) = config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }

    /// 统一路径格式显示 - 将反斜杠转换为正斜杠
    fn normalize_path_display(path: &str) -> String {
        path.replace('\\', "/")
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

                            let mut mod_entry = ModEntry {
                                path: line.to_string(),
                                enabled: true,
                                name,
                                user_config: HashMap::new(),
                                config_state: Arc::new(Mutex::new(ConfigLoadState::NotLoaded)),
                            };
                            mod_entry.init_user_config();
                            self.mods.push(mod_entry);
                        }
                    }
                    *self.status_message.lock().unwrap() =
                        format!("已加载 {} 个mod", self.mods.len());
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
            let content: String = self
                .mods
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

            // 检查路径是否已存在
            if self.mods.iter().any(|m| m.path == path_str) {
                *self.status_message.lock().unwrap() = "⚠️ 该Mod已存在于列表中".to_string();
                return;
            }

            let name = self.get_mod_name(&path_str);

            let mut mod_entry = ModEntry {
                path: path_str.clone(),
                enabled: true,
                name,
                user_config: HashMap::new(),
                config_state: Arc::new(Mutex::new(ConfigLoadState::NotLoaded)),
            };
            mod_entry.init_user_config();
            self.mods.push(mod_entry);

            *self.status_message.lock().unwrap() = "已添加Mod".to_string();
            self.save_config();
        }
    }

    fn open_github_dialog(&mut self) {
        self.github_dialog = Some(GitHubDialog {
            repo_url: String::new(),
            branches: Arc::new(Mutex::new(Vec::new())),
            selected_branch: None,
            subdirs: Arc::new(Mutex::new(Vec::new())),
            selected_subdir: None,
            is_loading: Arc::new(Mutex::new(false)),
            is_loading_dirs: Arc::new(Mutex::new(false)),
            error_message: Arc::new(Mutex::new(None)),
        });
    }

    fn close_github_dialog(&mut self) {
        self.github_dialog = None;
    }

    fn parse_github_url(url: &str) -> Option<String> {
        // 支持的格式:
        // https://github.com/user/repo
        // github.com/user/repo
        // user/repo
        let url = url.trim();

        if url.contains("github.com/") {
            // 提取 user/repo 部分
            if let Some(idx) = url.find("github.com/") {
                let after = &url[idx + 11..];
                let parts: Vec<&str> = after.split('/').collect();
                if parts.len() >= 2 {
                    return Some(format!("{}/{}", parts[0], parts[1]));
                }
            }
        } else if url.contains('/') && !url.contains(':') {
            // 直接是 user/repo 格式
            let parts: Vec<&str> = url.split('/').collect();
            if parts.len() >= 2 {
                return Some(format!("{}/{}", parts[0], parts[1]));
            }
        }

        None
    }

    /// 解析 GitHub 路径到实际的缓存路径 (静态版本)
    /// github:owner/repo:subdir@branch -> <config_dir>/infinite/mod_cache/owner/repo/branch/subdir
    fn resolve_github_path_static(path: &str) -> Option<PathBuf> {
        if !path.starts_with("github:") {
            return None;
        }

        // 移除 "github:" 前缀
        let path = &path[7..];

        // 分离分支 (如果有 @)
        let (path_without_branch, branch) = if let Some(at_pos) = path.rfind('@') {
            let branch = &path[at_pos + 1..];
            let path = &path[..at_pos];
            (path, branch)
        } else {
            (path, "main")
        };

        // 分离子目录 (如果有 :)
        let (repo, subdir) = if let Some(colon_pos) = path_without_branch.find(':') {
            let repo = &path_without_branch[..colon_pos];
            let subdir = &path_without_branch[colon_pos + 1..];
            (repo, Some(subdir))
        } else {
            (path_without_branch, None)
        };

        // 解析 owner/repo
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return None;
        }

        // 构建缓存路径: <config_dir>/infinite/mod_cache/owner/repo/branch/subdir
        let cache_dir = AppConfig::cache_dir();
        let mut target_dir = cache_dir.join(parts[0]).join(parts[1]).join(branch);

        if let Some(subdir) = subdir {
            target_dir = target_dir.join(subdir);
        }

        Some(target_dir)
    }

    fn fetch_github_info(&mut self, ctx: egui::Context) {
        if let Some(dialog) = &mut self.github_dialog {
            let repo = match Self::parse_github_url(&dialog.repo_url) {
                Some(r) => r,
                None => {
                    *dialog.error_message.lock().unwrap() =
                        Some("无效的 GitHub URL 格式".to_string());
                    return;
                }
            };

            *dialog.is_loading.lock().unwrap() = true;
            *dialog.error_message.lock().unwrap() = None;

            let repo_clone = repo.clone();
            let branches_clone = dialog.branches.clone();
            let error_clone = dialog.error_message.clone();
            let is_loading_clone = dialog.is_loading.clone();
            let github_token = self.github_token.clone();
            let rate_limit_clone = self.github_rate_limit.clone();

            // 在新线程中获取分支信息
            std::thread::spawn(move || {
                // 使用 GitHub API 获取分支列表
                let url = format!("https://api.github.com/repos/{}/branches", repo_clone);

                let mut request = reqwest::blocking::Client::new()
                    .get(&url)
                    .header("User-Agent", "infinite-mod-manager");

                // 添加 token (如果有)
                if let Some(token) = github_token {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }

                match request.send() {
                    Ok(response) => {
                        // 更新速率限制信息
                        if let (Some(remaining), Some(limit), Some(reset)) = (
                            response.headers().get("x-ratelimit-remaining"),
                            response.headers().get("x-ratelimit-limit"),
                            response.headers().get("x-ratelimit-reset"),
                        ) {
                            if let (Ok(rem_str), Ok(lim_str), Ok(reset_str)) = (
                                remaining.to_str(),
                                limit.to_str(),
                                reset.to_str(),
                            ) {
                                if let (Ok(rem), Ok(lim), Ok(reset_ts)) = (
                                    rem_str.parse::<u32>(),
                                    lim_str.parse::<u32>(),
                                    reset_str.parse::<u64>(),
                                ) {
                                    *rate_limit_clone.lock().unwrap() = Some(GitHubRateLimit {
                                        remaining: rem,
                                        limit: lim,
                                        reset_time: std::time::UNIX_EPOCH + std::time::Duration::from_secs(reset_ts),
                                    });
                                }
                            }
                        }

                        let status = response.status();
                        if status.is_success() {
                            if let Ok(branches_json) = response.json::<serde_json::Value>() {
                                if let Some(branches_array) = branches_json.as_array() {
                                    let branch_list: Vec<String> = branches_array
                                        .iter()
                                        .filter_map(|b| b.get("name")?.as_str())
                                        .map(String::from)
                                        .collect();

                                    *branches_clone.lock().unwrap() = branch_list;
                                    *is_loading_clone.lock().unwrap() = false;
                                    ctx.request_repaint();
                                    return;
                                }
                            }
                        }

                        *error_clone.lock().unwrap() =
                            Some(format!("无法获取仓库信息: {}", status));
                        *is_loading_clone.lock().unwrap() = false;
                    }
                    Err(e) => {
                        *error_clone.lock().unwrap() = Some(format!("网络错误: {}", e));
                        *is_loading_clone.lock().unwrap() = false;
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn fetch_github_directories(&mut self, ctx: egui::Context) {
        if let Some(dialog) = &self.github_dialog {
            let repo = match Self::parse_github_url(&dialog.repo_url) {
                Some(r) => r,
                None => return,
            };

            let branch = match &dialog.selected_branch {
                Some(b) => b.clone(),
                None => return,
            };

            *dialog.is_loading_dirs.lock().unwrap() = true;
            *dialog.error_message.lock().unwrap() = None;

            let subdirs_clone = dialog.subdirs.clone();
            let error_clone = dialog.error_message.clone();
            let is_loading_dirs_clone = dialog.is_loading_dirs.clone();
            let github_token = self.github_token.clone();

            // 在新线程中获取目录树
            std::thread::spawn(move || {
                // 使用 GitHub API 获取目录树
                let url = format!(
                    "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
                    repo, branch
                );

                let mut request = reqwest::blocking::Client::new()
                    .get(&url)
                    .header("User-Agent", "infinite-mod-manager");

                // 添加 token (如果有)
                if let Some(token) = github_token {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }

                match request.send() {
                    Ok(response) => {
                        let status = response.status();
                        if status.is_success() {
                            if let Ok(tree_json) = response.json::<serde_json::Value>() {
                                if let Some(tree_array) =
                                    tree_json.get("tree").and_then(|t| t.as_array())
                                {
                                    let mut dirs: Vec<String> = tree_array
                                        .iter()
                                        .filter_map(|item| {
                                            // 只获取目录类型
                                            if item.get("type")?.as_str()? == "tree" {
                                                Some(item.get("path")?.as_str()?.to_string())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect();

                                    // 排序并添加根目录选项
                                    dirs.sort();
                                    dirs.insert(0, "(根目录)".to_string());

                                    *subdirs_clone.lock().unwrap() = dirs;
                                    *is_loading_dirs_clone.lock().unwrap() = false;
                                    ctx.request_repaint();
                                    return;
                                }
                            }
                        }

                        *error_clone.lock().unwrap() =
                            Some(format!("无法获取目录结构: {}", status));
                        *is_loading_dirs_clone.lock().unwrap() = false;
                    }
                    Err(e) => {
                        *error_clone.lock().unwrap() = Some(format!("网络错误: {}", e));
                        *is_loading_dirs_clone.lock().unwrap() = false;
                    }
                }
                ctx.request_repaint();
            });
        }
    }

    fn add_github_mod(&mut self) {
        if let Some(dialog) = &self.github_dialog {
            if let Some(repo) = Self::parse_github_url(&dialog.repo_url) {
                let mut github_path = format!("github:{}", repo);

                if let Some(subdir) = &dialog.selected_subdir {
                    // 忽略 "(根目录)" 选项
                    if !subdir.is_empty() && subdir != "(根目录)" {
                        github_path = format!("{}:{}", github_path, subdir);
                    }
                }

                if let Some(branch) = &dialog.selected_branch {
                    if branch != "main" && branch != "master" {
                        github_path = format!("{}@{}", github_path, branch);
                    }
                }

                // 检查路径是否已存在
                if self.mods.iter().any(|m| m.path == github_path) {
                    *self.status_message.lock().unwrap() = "⚠️ 该GitHub Mod已存在于列表中".to_string();
                    self.close_github_dialog();
                    return;
                }

                // 提取仓库名称作为 mod 名称
                let name = repo.split('/').last().unwrap_or(&repo).to_string();

                let mut mod_entry = ModEntry {
                    path: github_path,
                    enabled: true,
                    name,
                    user_config: HashMap::new(),
                    config_state: Arc::new(Mutex::new(ConfigLoadState::NotLoaded)),
                };
                mod_entry.init_user_config();
                self.mods.push(mod_entry);

                *self.status_message.lock().unwrap() = "已添加 GitHub Mod".to_string();
                self.save_config();
            }
        }

        self.close_github_dialog();
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

    /// 渲染Mod配置面板
    fn render_config_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if let Some(index) = self.selected_mod_index {
            if index < self.mods.len() {
                // 先加载配置,避免借用冲突
                let mod_config_opt = self.mods[index].load_config(Some(ctx.clone()), self.github_token.clone());
                let mod_name = self.mods[index].name.clone();

                if let Some(mod_config) = mod_config_opt {
                    let description = mod_config.description.clone();
                    let config_options = mod_config.config.clone();

                    ui.group(|ui| {
                        ui.heading(format!("⚙ {} - 配置", mod_name));

                        if let Some(desc) = description {
                            ui.label(egui::RichText::new(desc).small().color(egui::Color32::GRAY));
                            ui.add_space(5.0);
                        }

                        ui.separator();
                        ui.add_space(10.0);

                        let mut config_changed = false;

                        // 配置选项区域 - 不需要内部滚动,外层已经有了
                        let mod_entry = &mut self.mods[index];
                        ui.set_width(ui.available_width());

                        for option in &config_options {
                                    match option {
                                        infinite::mod_manager::config::ConfigOption::CheckBox {
                                            id,
                                            name,
                                            description,
                                            default,
                                        } => {
                                            let mut value = mod_entry
                                                .user_config
                                                .get(id)
                                                .and_then(|v| v.as_bool())
                                                .unwrap_or(*default);

                                            if ui.checkbox(&mut value, name).changed() {
                                                mod_entry
                                                    .user_config
                                                    .insert(id.clone(), serde_json::json!(value));
                                                config_changed = true;
                                            }

                                            if let Some(desc) = description {
                                                ui.label(
                                                    egui::RichText::new(desc)
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                            ui.add_space(8.0);
                                        }

                                        infinite::mod_manager::config::ConfigOption::Number {
                                            id,
                                            name,
                                            description,
                                            min,
                                            max,
                                            default,
                                        } => {
                                            let mut value = mod_entry
                                                .user_config
                                                .get(id)
                                                .and_then(|v| v.as_f64())
                                                .unwrap_or(*default);

                                            let changed = ui
                                                .horizontal(|ui| {
                                                    ui.label(name);

                                                    if min.is_none() && max.is_none() {
                                                        // 如果没有范围,使用 DragValue
                                                        ui.add(egui::DragValue::new(&mut value))
                                                            .changed()
                                                    } else {
                                                        // 使用 Slider
                                                        ui.add(egui::Slider::new(
                                                            &mut value,
                                                            min.unwrap_or(0.0)
                                                                ..=max.unwrap_or(100.0),
                                                        ))
                                                        .changed()
                                                    }
                                                })
                                                .inner;

                                            // 如果值改变了，更新配置
                                            if changed {
                                                mod_entry
                                                    .user_config
                                                    .insert(id.clone(), serde_json::json!(value));
                                                config_changed = true;
                                            }

                                            if let Some(desc) = description {
                                                ui.label(
                                                    egui::RichText::new(desc)
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                            ui.add_space(8.0);
                                        }

                                        infinite::mod_manager::config::ConfigOption::Text {
                                            id,
                                            name,
                                            description,
                                            default,
                                        } => {
                                            let mut value = mod_entry
                                                .user_config
                                                .get(id)
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(default)
                                                .to_string();

                                            ui.horizontal(|ui| {
                                                ui.label(name);
                                                if ui.text_edit_singleline(&mut value).changed() {
                                                    mod_entry.user_config.insert(
                                                        id.clone(),
                                                        serde_json::json!(value),
                                                    );
                                                    config_changed = true;
                                                }
                                            });

                                            if let Some(desc) = description {
                                                ui.label(
                                                    egui::RichText::new(desc)
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                            ui.add_space(8.0);
                                        }

                                        infinite::mod_manager::config::ConfigOption::Select {
                                            id,
                                            name,
                                            description,
                                            default,
                                            options,
                                        } => {
                                            let mut value = mod_entry
                                                .user_config
                                                .get(id)
                                                .and_then(|v| v.as_str())
                                                .unwrap_or(default)
                                                .to_string();

                                            ui.horizontal(|ui| {
                                                ui.label(name);
                                                egui::ComboBox::from_id_source(id)
                                                    .selected_text(&value)
                                                    .show_ui(ui, |ui| {
                                                        for opt in options {
                                                            if ui
                                                                .selectable_value(
                                                                    &mut value,
                                                                    opt.value.clone(),
                                                                    &opt.label,
                                                                )
                                                                .clicked()
                                                            {
                                                                mod_entry.user_config.insert(
                                                                    id.clone(),
                                                                    serde_json::json!(value),
                                                                );
                                                                config_changed = true;
                                                            }
                                                        }
                                                    });
                                            });

                                            if let Some(desc) = description {
                                                ui.label(
                                                    egui::RichText::new(desc)
                                                        .small()
                                                        .color(egui::Color32::GRAY),
                                                );
                                            }
                                            ui.add_space(8.0);
                                        }
                                    }
                                }

                        // 如果配置改变了,保存
                        if config_changed {
                            self.save_config();
                        }
                    });
                } else {
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new("该Mod没有配置选项")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
                    });
                }
            }
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

        // 收集启用的mods及其配置
        let enabled_mods: Vec<(String, HashMap<String, serde_json::Value>)> = self
            .mods
            .iter()
            .filter(|m| m.enabled)
            .map(|m| (m.path.clone(), m.user_config.clone()))
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
        let status_msg = self.status_message.clone();
        let is_proc = self.is_processing.clone();
        let progress = self.progress.clone();
        let github_token = self.github_token.clone();

        // 在新线程中运行
        std::thread::spawn(move || {
            // 创建临时mod列表文件
            let temp_list = std::env::temp_dir().join("infinite_gui_mods.txt");
            let mod_paths: Vec<String> =
                enabled_mods.iter().map(|(path, _)| path.clone()).collect();
            if let Err(e) = std::fs::write(&temp_list, mod_paths.join("\n")) {
                *status_msg.lock().unwrap() = format!("❌ 无法创建临时文件: {}", e);
                *is_proc.lock().unwrap() = false;
                *progress.lock().unwrap() = None;
                ctx.request_repaint();
                return;
            }

            // 创建临时配置映射文件 (用于 GitHub mod 的配置)
            let temp_config = std::env::temp_dir().join("infinite_gui_config.json");
            let config_map: HashMap<String, HashMap<String, serde_json::Value>> = enabled_mods
                .iter()
                .filter(|(path, config)| !config.is_empty())
                .map(|(path, config)| (path.clone(), config.clone()))
                .collect();
            if let Ok(config_json) = serde_json::to_string_pretty(&config_map) {
                let _ = std::fs::write(&temp_config, config_json);
            }

            // 保存每个mod的用户配置到mod目录 (仅限本地 mod 和已下载的 GitHub mod)
            for (mod_path, user_config) in &enabled_mods {
                if !user_config.is_empty() {
                    // 解析路径(支持GitHub路径)
                    let config_dir = if mod_path.starts_with("github:") {
                        // 解析 GitHub 路径到缓存目录
                        Self::resolve_github_path_static(mod_path)
                    } else {
                        Some(PathBuf::from(mod_path))
                    };

                    if let Some(dir) = config_dir {
                        // 检查目录是否存在,如果是 GitHub mod 且目录不存在,跳过保存
                        // (CLI 会在下载 mod 后处理配置)
                        if !dir.exists() {
                            if mod_path.starts_with("github:") {
                                println!("⏭ Skipping config save for {}: mod not downloaded yet", mod_path);
                                continue;
                            }
                        }

                        let config_file = dir.join("config.json");
                        if let Ok(config_json) = serde_json::to_string_pretty(user_config) {
                            // 确保目录存在
                            if let Err(e) = std::fs::create_dir_all(&dir) {
                                eprintln!("Warning: Failed to create directory for {}: {}", mod_path, e);
                                continue;
                            }

                            if let Err(e) = std::fs::write(&config_file, config_json) {
                                eprintln!("Warning: Failed to write config for {}: {}", mod_path, e);
                            } else {
                                println!("✓ Saved config to: {}", config_file.display());
                            }
                        }
                    }
                }
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
            let mut command = std::process::Command::new(&cli_exe);
            command.args(&[
                "install",
                "--game-path",
                &game_path,
                "--mod-list",
                temp_list.to_str().unwrap()
            ]);

            // 如果有 GitHub token,通过环境变量传递给 CLI
            if let Some(token) = github_token {
                command.env("GITHUB_TOKEN", token);
            }

            let result = command.output();

            // 清理临时文件
            let _ = std::fs::remove_file(&temp_list);
            let temp_config = std::env::temp_dir().join("infinite_gui_config.json");
            let _ = std::fs::remove_file(&temp_config);

            match result {
                Ok(output) => {
                    if output.status.success() {
                        *status_msg.lock().unwrap() = format!("✅ 成功生成到: {}", output_path);

                        // 成功后删除临时的 config.json 文件
                        for (mod_path, user_config) in &enabled_mods {
                            if !user_config.is_empty() {
                                let config_dir = if mod_path.starts_with("github:") {
                                    Self::resolve_github_path_static(mod_path)
                                } else {
                                    Some(PathBuf::from(mod_path))
                                };

                                if let Some(dir) = config_dir {
                                    let config_file = dir.join("config.json");
                                    // 只删除存在的文件
                                    if config_file.exists() {
                                        if let Err(e) = std::fs::remove_file(&config_file) {
                                            eprintln!("Warning: Failed to delete config.json for {}: {}", mod_path, e);
                                        } else {
                                            println!("🗑 Deleted temporary config: {}", config_file.display());
                                        }
                                    }
                                }
                            }
                        }
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
            ui.horizontal(|ui| {
                ui.heading("Infinite - Diablo II: Resurrected Mod Manager");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // GitHub API 速率限制显示
                    if let Some(rate_limit) = self.github_rate_limit.lock().unwrap().as_ref() {
                        let color = if rate_limit.remaining < 10 {
                            egui::Color32::RED
                        } else if rate_limit.remaining < 50 {
                            egui::Color32::YELLOW
                        } else {
                            egui::Color32::GREEN
                        };
                        ui.colored_label(
                            color,
                            format!("🔄 API: {}/{}", rate_limit.remaining, rate_limit.limit),
                        );
                    }

                    // 设置按钮
                    if ui.button("⚙ 设置").clicked() {
                        self.show_settings = true;
                    }
                });
            });
            ui.add_space(10.0);

            // 游戏路径选择
            ui.horizontal(|ui| {
                ui.label("游戏路径:");
                if ui.button("📁 选择游戏目录").clicked() {
                    self.select_game_path();
                }
                ui.add_space(10.0);
                if !self.game_path.is_empty() {
                    ui.label(Self::normalize_path_display(&self.game_path));
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

                if ui.button("🌐 添加GitHub Mod").clicked() && !is_processing {
                    self.open_github_dialog();
                }
            });

            ui.add_space(10.0);

            // Mod列表显示 - 固定高度避免向下顶出窗口
            ui.label(egui::RichText::new(format!("共 {} 个Mod", self.mods.len())).weak());
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .id_source("mod_list_scroll")
                .max_height(200.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.mods.is_empty() {
                        ui.label(
                            egui::RichText::new("没有mod，请添加或打开mod列表")
                                .italics()
                                .color(egui::Color32::GRAY),
                        );
                    } else {
                        let mut to_remove = None;
                        let mut to_move_up = None;
                        let mut to_move_down = None;
                        let mut config_changed = false;

                        for (index, mod_entry) in self.mods.iter_mut().enumerate() {
                            let is_selected = self.selected_mod_index == Some(index);

                            // 检查是否有配置选项
                            let has_config = mod_entry
                                .load_config(Some(ctx.clone()), self.github_token.clone())
                                .map(|cfg| !cfg.config.is_empty())
                                .unwrap_or(false);

                            ui.horizontal(|ui| {
                                // 启用/禁用复选框
                                if ui.checkbox(&mut mod_entry.enabled, "").changed() {
                                    config_changed = true;
                                }

                                // Mod名称 - 如果有配置,点击可选中/取消选中
                                if has_config {
                                    let name_response =
                                        ui.selectable_label(is_selected, &mod_entry.name);
                                    if name_response.clicked() {
                                        self.selected_mod_index =
                                            if is_selected { None } else { Some(index) };
                                    }
                                } else {
                                    ui.label(&mod_entry.name);
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
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

                                        // 配置按钮 - 只在有配置选项时显示
                                        if has_config {
                                            if ui.button("⚙").clicked() {
                                                self.selected_mod_index =
                                                    if is_selected { None } else { Some(index) };
                                            }
                                        }

                                        // 路径显示
                                        ui.label(
                                            egui::RichText::new(Self::normalize_path_display(&mod_entry.path))
                                                .small()
                                                .color(egui::Color32::GRAY),
                                        );
                                    },
                                );
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

            // Mod配置面板 - 限制高度避免超出窗口
            if self.selected_mod_index.is_some() {
                // 计算可用高度: 窗口高度 - 已用空间 - 底部按钮区域预留空间
                let available_height = ui.available_height();
                let max_config_height = (available_height - 150.0).max(150.0); // 至少150px,最多为可用高度-150px

                egui::ScrollArea::vertical()
                    .id_source("config_panel_scroll")
                    .max_height(max_config_height)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.render_config_panel(ui, ctx);
                    });
                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);
            }

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
                    let button = egui::Button::new(egui::RichText::new("🚀 生成Mods").size(20.0));

                    if ui.add_sized([150.0, 40.0], button).clicked() {
                        self.generate_mods(ctx.clone());
                    }
                });

                ui.add_space(20.0);

                // 显示输出路径
                if !self.game_path.is_empty() {
                    let output_path = format!("{}/Mods/Infinite/Infinite.mpq/data", self.game_path);
                    ui.label(
                        egui::RichText::new(format!("输出路径: {}", Self::normalize_path_display(&output_path)))
                            .small()
                            .color(egui::Color32::LIGHT_GRAY),
                    );
                }
            });

            ui.add_space(10.0);

            // 状态栏
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("状态:");
                ui.label(
                    egui::RichText::new(&status_message).color(if is_processing {
                        egui::Color32::YELLOW
                    } else if status_message.starts_with("✅") {
                        egui::Color32::GREEN
                    } else if status_message.starts_with("❌") {
                        egui::Color32::RED
                    } else {
                        egui::Color32::LIGHT_BLUE
                    }),
                );
            });
        });

        // GitHub 对话框
        let mut should_close = false;
        let mut should_add = false;
        let mut should_fetch = false;
        let mut should_fetch_dirs = false;

        if let Some(dialog) = &mut self.github_dialog {
            egui::Window::new("🌐 添加 GitHub Mod")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // 仓库 URL 输入
                        ui.horizontal(|ui| {
                            ui.label("仓库地址:");
                            ui.add_space(5.0);
                        });

                        ui.add(
                            egui::TextEdit::singleline(&mut dialog.repo_url)
                                .hint_text("user/repo 或 https://github.com/user/repo")
                                .desired_width(400.0),
                        );

                        ui.add_space(5.0);
                        ui.label(
                            egui::RichText::new("支持格式: user/repo 或 github.com/user/repo")
                                .small()
                                .color(egui::Color32::GRAY),
                        );

                        ui.add_space(10.0);

                        // 获取分支按钮
                        let is_loading = *dialog.is_loading.lock().unwrap();
                        if ui.button("🔍 获取分支信息").clicked() && !is_loading {
                            should_fetch = true;
                        }

                        ui.add_space(10.0);

                        // 加载状态
                        if is_loading {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("正在获取仓库信息...");
                            });
                        }

                        // 错误消息
                        if let Some(error) = dialog.error_message.lock().unwrap().clone() {
                            ui.colored_label(egui::Color32::RED, error);
                            ui.add_space(5.0);
                        }

                        // 分支选择
                        let branches = dialog.branches.lock().unwrap().clone();
                        if !branches.is_empty() {
                            ui.separator();
                            ui.add_space(5.0);

                            // 记录上一次的分支选择
                            let prev_branch = dialog.selected_branch.clone();

                            ui.horizontal(|ui| {
                                ui.label("分支:");
                                egui::ComboBox::from_id_source("branch_combo")
                                    .selected_text(
                                        dialog
                                            .selected_branch
                                            .as_ref()
                                            .unwrap_or(&"选择分支".to_string()),
                                    )
                                    .show_ui(ui, |ui| {
                                        for branch in &branches {
                                            ui.selectable_value(
                                                &mut dialog.selected_branch,
                                                Some(branch.clone()),
                                                branch,
                                            );
                                        }
                                    });
                            });

                            // 检测分支是否改变
                            if prev_branch != dialog.selected_branch
                                && dialog.selected_branch.is_some()
                            {
                                // 分支改变，需要获取目录结构
                                should_fetch_dirs = true;
                            }

                            ui.add_space(10.0);

                            // 子目录选择
                            let subdirs = dialog.subdirs.lock().unwrap().clone();
                            let is_loading_dirs = *dialog.is_loading_dirs.lock().unwrap();

                            if is_loading_dirs {
                                ui.horizontal(|ui| {
                                    ui.spinner();
                                    ui.label("正在获取目录结构...");
                                });
                            } else if !subdirs.is_empty() {
                                ui.horizontal(|ui| {
                                    ui.label("子目录:");
                                    egui::ComboBox::from_id_source("subdir_combo")
                                        .selected_text(
                                            dialog
                                                .selected_subdir
                                                .as_ref()
                                                .unwrap_or(&"(根目录)".to_string()),
                                        )
                                        .show_ui(ui, |ui| {
                                            for subdir in &subdirs {
                                                let display_text = subdir.clone();
                                                ui.selectable_value(
                                                    &mut dialog.selected_subdir,
                                                    Some(subdir.clone()),
                                                    display_text,
                                                );
                                            }
                                        });
                                });
                            } else if dialog.selected_branch.is_some() {
                                // 有分支但还没加载目录，显示手动输入框
                                ui.horizontal(|ui| {
                                    ui.label("子目录:");
                                    ui.add_space(5.0);
                                });

                                let mut subdir_text =
                                    dialog.selected_subdir.clone().unwrap_or_default();
                                ui.add(
                                    egui::TextEdit::singleline(&mut subdir_text)
                                        .hint_text("可选，例如: mods/my_mod")
                                        .desired_width(400.0),
                                );
                                dialog.selected_subdir = if subdir_text.is_empty() {
                                    None
                                } else {
                                    Some(subdir_text)
                                };

                                ui.add_space(5.0);
                                ui.label(
                                    egui::RichText::new("留空表示使用仓库根目录")
                                        .small()
                                        .color(egui::Color32::GRAY),
                                );
                            }
                        }

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // 按钮
                        ui.horizontal(|ui| {
                            let can_add = !dialog.repo_url.is_empty()
                                && !branches.is_empty()
                                && dialog.selected_branch.is_some()
                                && !is_loading;

                            ui.add_enabled_ui(can_add, |ui| {
                                if ui.button("✅ 添加").clicked() {
                                    should_add = true;
                                }
                            });

                            if ui.button("❌ 取消").clicked() {
                                should_close = true;
                            }
                        });
                    });
                });
        }

        // 处理对话框操作
        if should_fetch {
            self.fetch_github_info(ctx.clone());
        }
        if should_fetch_dirs {
            self.fetch_github_directories(ctx.clone());
        }
        if should_add {
            self.add_github_mod();
        }
        if should_close {
            self.close_github_dialog();
        }

        // 设置对话框
        if self.show_settings {
            let mut should_close_settings = false;

            egui::Window::new("⚙ 设置")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("GitHub Token");
                        ui.add_space(5.0);

                        ui.label(
                            egui::RichText::new(
                                "配置 GitHub Personal Access Token 可以提高 API 限额:\n\
                                 • 未认证: 60 请求/小时\n\
                                 • 认证后: 5000 请求/小时\n\n\
                                 创建 Token: https://github.com/settings/tokens\n\
                                 权限: 只需要 public_repo (读取公开仓库)"
                            )
                            .small()
                            .color(egui::Color32::GRAY),
                        );

                        ui.add_space(10.0);

                        let mut token_text = self.github_token.clone().unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Token:");
                            ui.add(
                                egui::TextEdit::singleline(&mut token_text)
                                    .password(true)
                                    .hint_text("ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx")
                                    .desired_width(300.0),
                            );
                        });

                        self.github_token = if token_text.is_empty() {
                            None
                        } else {
                            Some(token_text)
                        };

                        ui.add_space(10.0);

                        // 显示当前 API 状态
                        if let Some(rate_limit) = self.github_rate_limit.lock().unwrap().as_ref() {
                            ui.separator();
                            ui.label(format!("当前 API 限额: {}/{}", rate_limit.remaining, rate_limit.limit));

                            if let Ok(elapsed) = rate_limit.reset_time.elapsed() {
                                ui.label(format!("已过去: {} 秒", elapsed.as_secs()));
                            } else if let Ok(duration) = rate_limit.reset_time.duration_since(std::time::SystemTime::now()) {
                                ui.label(format!("重置时间: {} 秒后", duration.as_secs()));
                            }
                        }

                        ui.add_space(15.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.horizontal(|ui| {
                            if ui.button("✅ 保存").clicked() {
                                self.save_config();
                                should_close_settings = true;
                            }

                            if ui.button("❌ 取消").clicked() {
                                should_close_settings = true;
                            }
                        });
                    });
                });

            if should_close_settings {
                self.show_settings = false;
            }
        }
    }
}
