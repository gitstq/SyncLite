use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;
use tracing::{info, warn};

mod config;
mod sync;
mod storage;
mod watcher;
mod hash;
mod backup;
mod scheduler;

use config::Config;
use sync::SyncEngine;

#[derive(Parser)]
#[command(name = "synclite")]
#[command(about = "🔄 A lightweight, fast, and intelligent file synchronization tool")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Configuration file path
    #[arg(short, long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new sync configuration
    Init {
        /// Source directory
        #[arg(short, long, value_name = "DIR")]
        source: PathBuf,
        
        /// Destination (local path, s3://bucket, or sftp://host)
        #[arg(short, long, value_name = "DEST")]
        dest: String,
        
        /// Sync mode: bidirectional, push, or pull
        #[arg(short, long, default_value = "bidirectional")]
        mode: String,
    },
    
    /// Run synchronization
    Sync {
        /// Dry run - show what would be synced without making changes
        #[arg(short, long)]
        dry_run: bool,
        
        /// Force sync (ignore conflicts)
        #[arg(short, long)]
        force: bool,
    },
    
    /// Watch for changes and sync automatically
    Watch {
        /// Debounce duration in seconds
        #[arg(short, long, default_value = "5")]
        debounce: u64,
    },
    
    /// Create a backup snapshot
    Backup {
        /// Backup name/tag
        #[arg(short, long)]
        tag: Option<String>,
        
        /// Compress the backup
        #[arg(short, long)]
        compress: bool,
    },
    
    /// List backup history
    List,
    
    /// Restore from backup
    Restore {
        /// Backup timestamp or tag
        #[arg(value_name = "BACKUP")]
        backup: String,
        
        /// Restore destination (defaults to source)
        #[arg(short, long)]
        dest: Option<PathBuf>,
    },
    
    /// Show sync status
    Status,
    
    /// Schedule automatic sync
    #[cfg(feature = "scheduler")]
    Schedule {
        /// Cron expression (e.g., "0 */6 * * *" for every 6 hours)
        #[arg(short, long)]
        cron: Option<String>,
        
        /// Interval in minutes
        #[arg(short, long)]
        interval: Option<u64>,
        
        /// List scheduled tasks
        #[arg(short, long)]
        list: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(
            if cli.verbose {
                "synclite=debug"
            } else {
                "synclite=info"
            }
        )
        .init();
    
    match cli.command {
        Commands::Init { source, dest, mode } => {
            init_config(cli.config, source, dest, mode).await?;
        }
        Commands::Sync { dry_run, force } => {
            run_sync(cli.config, dry_run, force).await?;
        }
        Commands::Watch { debounce } => {
            watch_and_sync(cli.config, debounce).await?;
        }
        Commands::Backup { tag, compress } => {
            create_backup(cli.config, tag, compress).await?;
        }
        Commands::List => {
            list_backups(cli.config).await?;
        }
        Commands::Restore { backup, dest } => {
            restore_backup(cli.config, backup, dest).await?;
        }
        Commands::Status => {
            show_status(cli.config).await?;
        }
        #[cfg(feature = "scheduler")]
        Commands::Schedule { cron, interval, list } => {
            handle_schedule(cli.config, cron, interval, list).await?;
        }
    }
    
    Ok(())
}

async fn init_config(
    config_path: Option<PathBuf>,
    source: PathBuf,
    dest: String,
    mode: String,
) -> Result<()> {
    println!("{}", "🚀 Initializing SyncLite configuration...".cyan().bold());
    
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if config_file.exists() {
        warn!("Configuration file already exists: {:?}", config_file);
        print!("{}", "Overwrite existing config? [y/N]: ".yellow());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "Aborted.".red());
            return Ok(());
        }
    }
    
    let config = Config::new(source.clone(), dest.clone(), mode.clone());
    config.save(&config_file)?;
    
    println!("{}", "✅ Configuration created successfully!".green().bold());
    println!("   Source: {}", source.display().to_string().cyan());
    println!("   Destination: {}", dest.cyan());
    println!("   Mode: {}", mode.cyan());
    println!("\n{}", "Next steps:".yellow().bold());
    println!("  1. Edit {:?} to customize your sync settings", config_file);
    println!("  2. Run {} to perform the first sync", "synclite sync".cyan());
    
    Ok(())
}

async fn run_sync(config_path: Option<PathBuf>, dry_run: bool, force: bool) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}\nRun 'synclite init' first.", config_file);
    }
    
    let config = Config::load(&config_file)?;
    let engine = SyncEngine::new(config);
    
    if dry_run {
        println!("{}", "🔍 Dry run mode - no changes will be made".yellow());
    }
    
    engine.sync(dry_run, force).await?;
    
    if !dry_run {
        println!("{}", "✅ Sync completed successfully!".green().bold());
    }
    
    Ok(())
}

async fn watch_and_sync(config_path: Option<PathBuf>, debounce: u64) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    println!("{}", "👁️  Starting file watcher...".cyan().bold());
    println!("   Debounce: {} seconds", debounce);
    println!("   Press Ctrl+C to stop\n");
    
    let config = Config::load(&config_file)?;
    let watcher = watcher::FileWatcher::new(config, debounce);
    watcher.start().await?;
    
    Ok(())
}

async fn create_backup(
    config_path: Option<PathBuf>,
    tag: Option<String>,
    compress: bool,
) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    let config = Config::load(&config_file)?;
    let backup = backup::BackupManager::new(config);
    
    backup.create(tag, compress).await?;
    
    println!("{}", "✅ Backup created successfully!".green().bold());
    
    Ok(())
}

async fn list_backups(config_path: Option<PathBuf>) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    let config = Config::load(&config_file)?;
    let backup = backup::BackupManager::new(config);
    
    backup.list().await?;
    
    Ok(())
}

async fn restore_backup(
    config_path: Option<PathBuf>,
    backup_id: String,
    dest: Option<PathBuf>,
) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    let config = Config::load(&config_file)?;
    let backup = backup::BackupManager::new(config);
    
    println!("{}", format!("🔄 Restoring backup: {}", backup_id).cyan());
    
    backup.restore(&backup_id, dest).await?;
    
    println!("{}", "✅ Restore completed successfully!".green().bold());
    
    Ok(())
}

async fn show_status(config_path: Option<PathBuf>) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    let config = Config::load(&config_file)?;
    let engine = SyncEngine::new(config);
    
    engine.status().await?;
    
    Ok(())
}

#[cfg(feature = "scheduler")]
async fn handle_schedule(
    config_path: Option<PathBuf>,
    cron_expr: Option<String>,
    interval: Option<u64>,
    list: bool,
) -> Result<()> {
    let config_file = config_path.unwrap_or_else(|| PathBuf::from("synclite.toml"));
    
    if list {
        scheduler::list_schedules().await?;
        return Ok(());
    }
    
    if !config_file.exists() {
        anyhow::bail!("Configuration file not found: {:?}", config_file);
    }
    
    let config = Config::load(&config_file)?;
    
    if let Some(cron) = cron_expr {
        scheduler::add_cron_schedule(&config, &cron).await?;
        println!("{}", format!("📅 Scheduled sync with cron: {}", cron).green());
    } else if let Some(minutes) = interval {
        scheduler::add_interval_schedule(&config, minutes).await?;
        println!("{}", format!("📅 Scheduled sync every {} minutes", minutes).green());
    } else {
        println!("{}", "Usage: synclite schedule --cron \"0 */6 * * *\" OR --interval 60".yellow());
    }
    
    Ok(())
}
