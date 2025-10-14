use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use infinite::cli::Cli;
use infinite::file_system::FileManager;
use infinite::mod_manager::ModLoader;
use infinite::runtime::{Context, ModExecutor};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

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
            output_path,
            dry_run,
        } => {
            install_mods(&game_path, &mods_path, &output_path, dry_run).await?;
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
    mods_path: &str,
    output_path: &str,
    dry_run: bool,
) -> Result<()> {
    println!("\n{}", "🎮 infinite CLI - Installing Mods".bright_cyan().bold());
    println!("{}", "═".repeat(50).bright_black());
    println!("  {}  {}", "Game:".bright_white(), game_path);
    println!("  {}  {}", "Mods:".bright_white(), mods_path);
    println!("  {} {}", "Output:".bright_white(), output_path);
    if dry_run {
        println!("  {}  {}", "Mode:".bright_white(), "DRY RUN".bright_yellow());
    }
    println!("{}\n", "═".repeat(50).bright_black());

    let start_time = Instant::now();

    // Load all mods
    let loader = ModLoader::new(mods_path);
    let mods = loader.load_all()?;

    if mods.is_empty() {
        println!("{}", "⚠️  No mods found!".bright_yellow());
        return Ok(());
    }

    println!("📦 Found {} mod(s)\n", mods.len());

    // Create shared file manager
    let file_manager = Arc::new(RwLock::new(FileManager::new()));

    // Create mod executor
    let executor = ModExecutor::new()?;

    // Install each mod
    for (idx, mod_data) in mods.iter().enumerate() {
        let mod_start = Instant::now();

        println!(
            "{} {}/{} - {} {}",
            "⚙️".bright_blue(),
            (idx + 1).to_string().bright_white(),
            mods.len(),
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

        // Execute mod
        match executor.execute_mod(mod_data, context).await {
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
