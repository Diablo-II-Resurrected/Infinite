use anyhow::{Result, Context as AnyhowContext};
use clap::Parser;
use colored::Colorize;
use infinite::cli::Cli;
use infinite::casc::CascStorage;
use infinite::file_system::FileManager;
use infinite::github_downloader::GitHubDownloader;
use infinite::mod_manager::ModLoader;
use infinite::mod_sources::{ModList, ModSource};
use infinite::runtime::{Context, ModExecutor};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

/// 获取 mod 缓存目录路径
fn get_cache_dir() -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("infinite");
    path.push("mod_cache");
    path
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Execute command
    match cli.command {
        infinite::cli::commands::Commands::Install {
            game_path,
            mods_path,
            mod_list,
            output_path,
            dry_run,
            clear_cache,
        } => {
            // Use default output path if not specified
            let output = output_path.unwrap_or_else(|| {
                format!("{}/Mods/Infinite/Infinite.mpq/data", game_path)
            });
            install_mods(&game_path, mods_path.as_deref(), mod_list.as_deref(), &output, dry_run, clear_cache).await?;
        }
        infinite::cli::commands::Commands::List { mods_path } => {
            list_mods(&mods_path).await?;
        }
        infinite::cli::commands::Commands::Validate { mod_path } => {
            validate_mod(&mod_path).await?;
        }
    }

    Ok(())
}

async fn install_mods(
    game_path: &str,
    mods_path: Option<&str>,
    mod_list: Option<&str>,
    output_path: &str,
    dry_run: bool,
    clear_cache: bool,
) -> Result<()> {
    println!("\n{}", "🎮 infinite CLI - Installing Mods".bright_cyan().bold());
    println!("{}", "═".repeat(50).bright_black());
    println!("  {}  {}", "Game:".bright_white(), game_path);

    // 尝试读取 GUI 传递的配置映射
    let temp_config_path = std::env::temp_dir().join("infinite_gui_config.json");
    let gui_config_map: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>> =
        if temp_config_path.exists() {
            match std::fs::read_to_string(&temp_config_path) {
                Ok(content) => {
                    match serde_json::from_str(&content) {
                        Ok(map) => {
                            tracing::info!("Loaded GUI config mapping with {} mod(s)",
                                std::collections::HashMap::<String, std::collections::HashMap<String, serde_json::Value>>::len(&map));
                            map
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse GUI config: {}", e);
                            std::collections::HashMap::new()
                        }
                    }
                }
                Err(_) => std::collections::HashMap::new()
            }
        } else {
            std::collections::HashMap::new()
        };

    // Determine mod sources
    let mod_dirs: Vec<PathBuf> = if let Some(list_path) = mod_list {
        println!("  {}  {}", "Mod List:".bright_white(), list_path);

        // Setup GitHub downloader with user data directory
        let cache_dir = get_cache_dir();
        let downloader = GitHubDownloader::new(cache_dir);

        if clear_cache {
            println!("  {} Clearing download cache...", "🗑️".bright_yellow());
            downloader.clear_cache().await?;
        }

        // Load mod list
        let mod_list = ModList::from_file(std::path::Path::new(list_path)).await?;
        println!("  {} Loaded {} mod source(s)", "📝".bright_cyan(), mod_list.sources.len());

        // Resolve all sources
        let mut dirs = Vec::new();
        for (idx, source) in mod_list.sources.iter().enumerate() {
            println!("\n  {} [{}/{}] Processing source...", "⬇️".bright_blue(), idx + 1, mod_list.sources.len());
            match source {
                ModSource::Local { path } => {
                    println!("    {} Local: {}", "📁".bright_green(), path.display());
                    dirs.push(path.clone());
                }
                ModSource::GitHub { repo, subdir, branch } => {
                    println!("    {} GitHub: {}", "🌐".bright_green(), repo);
                    if let Some(subdir) = subdir {
                        println!("      Subdirectory: {}", subdir);
                    }
                    if let Some(branch) = branch {
                        println!("      Branch: {}", branch);
                    }

                    let local_path = downloader
                        .download(repo, subdir.as_deref(), branch.as_deref())
                        .await?;

                    println!("    {} Downloaded to: {}", "✓".bright_green(), local_path.display());

                    // 检查是否有 GUI 传递的配置需要应用
                    // 构建 github: 格式的路径来匹配 GUI 配置
                    let mut github_path = format!("github:{}", repo);
                    if let Some(subdir) = subdir {
                        github_path = format!("{}:{}", github_path, subdir);
                    }
                    if let Some(branch) = branch {
                        if branch != "main" && branch != "master" {
                            github_path = format!("{}@{}", github_path, branch);
                        }
                    }

                    // 如果有配置,立即写入到下载的 mod 目录
                    if let Some(user_config) = gui_config_map.get(&github_path) {
                        if !user_config.is_empty() {
                            let config_file = local_path.join("config.json");
                            match serde_json::to_string_pretty(user_config) {
                                Ok(config_json) => {
                                    match std::fs::write(&config_file, config_json) {
                                        Ok(_) => {
                                            tracing::info!("Applied GUI config to: {}", config_file.display());
                                            println!("    {} Applied user configuration", "⚙️".bright_cyan());
                                        }
                                        Err(e) => {
                                            tracing::warn!("Failed to write config for {}: {}", github_path, e);
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!("Failed to serialize config for {}: {}", github_path, e);
                                }
                            }
                        }
                    }

                    dirs.push(local_path);
                }
            }
        }
        dirs
    } else if let Some(path) = mods_path {
        println!("  {}  {}", "Mods:".bright_white(), path);
        vec![PathBuf::from(path)]
    } else {
        anyhow::bail!("Either --mods-path or --mod-list must be specified");
    };

    println!("  {} {}", "Output:".bright_white(), output_path);
    if dry_run {
        println!("  {}  {}", "Mode:".bright_white(), "DRY RUN".bright_yellow());
    }
    println!("{}\n", "═".repeat(50).bright_black());

    let start_time = Instant::now();

    // Load all mods from all directories
    let mut all_mods = Vec::new();
    for mod_dir in &mod_dirs {
        // Check if this is a single mod or a mods directory
        let config_path = mod_dir.join("mod.json");
        if config_path.exists() {
            // This is a single mod directory
            let loader = ModLoader::new(mod_dir.parent().unwrap_or(&PathBuf::from(".")));
            match loader.load_mod(mod_dir) {
                Ok(mod_data) => all_mods.push(mod_data),
                Err(e) => eprintln!("Warning: Failed to load mod at {:?}: {}", mod_dir, e),
            }
        } else {
            // This is a directory containing multiple mods
            let loader = ModLoader::new(mod_dir);
            let mods = loader.load_all()?;
            all_mods.extend(mods);
        }
    }

    if all_mods.is_empty() {
        println!("{}", "⚠️  No mods found!".bright_yellow());
        return Ok(());
    }

    println!("📦 Found {} mod(s)\n", all_mods.len());

    // Clear output directory if it exists
    let output_path_buf = PathBuf::from(output_path);
    if output_path_buf.exists() {
        println!("  {} Clearing output directory...", "🗑️".bright_yellow());
        std::fs::remove_dir_all(&output_path_buf)
            .with_context(|| format!("Failed to clear output directory: {}", output_path))?;
        println!("  {} Output directory cleared", "✅".bright_green());
    }

    // Create shared file manager
    let mut file_manager = FileManager::new();
    file_manager.set_output_path(output_path);
    file_manager.set_game_path(game_path);

    // Try to open CASC storage
    match CascStorage::open(game_path) {
        Ok(casc) => {
            tracing::info!("CASC storage opened successfully");
            file_manager.set_casc_storage(Arc::new(casc));
        }
        Err(e) => {
            tracing::warn!("Failed to open CASC storage: {}. File extraction will be disabled.", e);
            tracing::warn!("Make sure the game path is correct and the game is installed.");
        }
    }

    let file_manager = Arc::new(RwLock::new(file_manager));

    // Install each mod
    for (idx, mod_data) in all_mods.iter().enumerate() {
        let mod_start = Instant::now();

        println!(
            "{} {}/{} - {} {}",
            "⚙️".bright_blue(),
            (idx + 1).to_string().bright_white(),
            all_mods.len(),
            mod_data.config.name.bright_green(),
            format!("v{}", mod_data.config.version).bright_black()
        );

        // Create execution context
        let context = Arc::new(Context {
            mod_id: mod_data.id.clone(),
            mod_path: mod_data.path.clone(),
            config: serde_json::to_value(&mod_data.user_config)?,
            file_manager: file_manager.clone(),
            game_path: game_path.into(),
            output_path: output_path.into(),
            dry_run,
        });

        // Execute mod (static method now)
        match ModExecutor::execute_mod(mod_data, context).await {
            Ok(_) => {
                let elapsed = mod_start.elapsed();
                println!(
                    "   {} Installed in {:.2}s\n",
                    "✅".bright_green(),
                    elapsed.as_secs_f64()
                );
            }
            Err(e) => {
                eprintln!(
                    "   {} Failed: {}\n",
                    "❌".bright_red(),
                    e.to_string().bright_red()
                );
                // Continue with next mod
            }
        }
    }

    let total_elapsed = start_time.elapsed();

    // Flush all cached file modifications to disk
    println!("\n{}", "💾 Flushing cached modifications...".bright_cyan());
    {
        let mut fm = file_manager.write().await;
        if let Err(e) = fm.flush_cache().await {
            eprintln!(
                "{} Failed to flush cache: {}",
                "⚠️".bright_yellow(),
                e.to_string().bright_red()
            );
        } else {
            println!("{} All modifications written to disk", "✅".bright_green());
        }
    }

    // Generate modinfo.json in parent directory of output_path
    if !dry_run {
        if let Some(parent_dir) = std::path::Path::new(output_path).parent() {
            let modinfo_path = parent_dir.join("modinfo.json");
            let modinfo_content = serde_json::json!({
                "name": "Infinite",
                "savepath": "Infinite/"
            });

            match std::fs::create_dir_all(parent_dir) {
                Ok(_) => {
                    match std::fs::write(&modinfo_path, serde_json::to_string_pretty(&modinfo_content)?) {
                        Ok(_) => {
                            println!("{} Generated modinfo.json at: {}", "✅".bright_green(), modinfo_path.display());
                        }
                        Err(e) => {
                            eprintln!(
                                "{} Failed to write modinfo.json: {}",
                                "⚠️".bright_yellow(),
                                e.to_string().bright_red()
                            );
                        }
                    }
                }
                Err(e) => {
                    eprintln!(
                        "{} Failed to create directory for modinfo.json: {}",
                        "⚠️".bright_yellow(),
                        e.to_string().bright_red()
                    );
                }
            }
        }
    }

    // Print summary
    println!("{}", "═".repeat(50).bright_black());
    let fm = file_manager.read().await;
    fm.print_summary();
    println!("\n{}", "═".repeat(50).bright_black());
    println!(
        "{} All mods processed in {:.2}s",
        "🎉".bright_green(),
        total_elapsed.as_secs_f64()
    );

    Ok(())
}

async fn list_mods(mods_path: &str) -> Result<()> {
    println!("\n{}", "📦 Available Mods".bright_cyan().bold());
    println!("{}\n", "═".repeat(50).bright_black());

    let loader = ModLoader::new(mods_path);
    let mods = loader.load_all()?;

    if mods.is_empty() {
        println!("{}", "No mods found.".bright_yellow());
        return Ok(());
    }

    for (idx, mod_data) in mods.iter().enumerate() {
        println!(
            "{}. {} {}",
            (idx + 1).to_string().bright_white(),
            mod_data.config.name.bright_green().bold(),
            format!("v{}", mod_data.config.version).bright_black()
        );

        if let Some(desc) = &mod_data.config.description {
            println!("   {}", desc.bright_black());
        }

        if let Some(author) = &mod_data.config.author {
            println!("   {} {}", "By:".bright_black(), author);
        }

        if !mod_data.config.config.is_empty() {
            println!(
                "   {} {} configuration option(s)",
                "⚙️".bright_blue(),
                mod_data.config.config.len()
            );
        }

        println!();
    }

    println!("{}", "═".repeat(50).bright_black());
    println!("Total: {} mod(s)", mods.len());

    Ok(())
}

async fn validate_mod(mod_path: &str) -> Result<()> {
    println!("\n{}", "🔍 Validating Mod".bright_cyan().bold());
    println!("{}\n", "═".repeat(50).bright_black());

    let loader = ModLoader::new(mod_path);
    let mod_data = loader.load_mod(std::path::Path::new(mod_path))?;

    println!("{} Mod configuration is valid!", "✅".bright_green());
    println!();
    println!("  {}  {}", "Name:".bright_white(), mod_data.config.name);
    println!(
        "  {} {}",
        "Version:".bright_white(),
        mod_data.config.version
    );

    if let Some(author) = &mod_data.config.author {
        println!("  {} {}", "Author:".bright_white(), author);
    }

    if let Some(desc) = &mod_data.config.description {
        println!("  {} {}", "Description:".bright_white(), desc);
    }

    if !mod_data.config.config.is_empty() {
        println!("\n  {} Configuration Options:", "⚙️".bright_blue());
        for opt in &mod_data.config.config {
            println!("    • {}", opt.id().bright_cyan());
        }
    }

    println!();
    Ok(())
}
