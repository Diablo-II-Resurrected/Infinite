use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "infinite")]
#[command(about = "Diablo II: Resurrected Mod Manager CLI", long_about = None)]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install mods
    Install {
        /// Path to the game directory
        #[arg(short, long)]
        game_path: String,

        /// Path to the mods directory
        #[arg(short, long)]
        mods_path: String,

        /// Path to the output directory
        #[arg(short, long)]
        output_path: String,

        /// Dry run (don't write files)
        #[arg(long)]
        dry_run: bool,
    },

    /// List available mods
    List {
        /// Path to the mods directory
        #[arg(short, long)]
        mods_path: String,
    },

    /// Validate a mod
    Validate {
        /// Path to the mod directory
        #[arg(short, long)]
        mod_path: String,
    },
}
