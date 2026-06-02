use anyhow::Result;
use chrono::Local;
use colored::*;
use cron::Schedule;
use std::str::FromStr;
use std::time::Duration;
use tracing::{info, warn};

use crate::config::Config;
use crate::sync::SyncEngine;

/// List scheduled sync tasks
pub async fn list_schedules() -> Result<()> {
    println!("{}", "📅 Scheduled Sync Tasks".cyan().bold());
    println!("\nNote: Scheduled tasks are managed by your system's cron/job scheduler.");
    println!("Use the following commands to set up scheduling:\n");
    
    println!("{}", "Linux/macOS (cron):".yellow());
    println!("  crontab -e");
    println!("  # Add line: */30 * * * * cd /path && synclite sync");
    
    println!("\n{}", "Windows (Task Scheduler):".yellow());
    println!("  schtasks /create /tn SyncLite /tr \"synclite sync\" /sc minute /mo 30");
    
    Ok(())
}

/// Add a cron-based schedule
pub async fn add_cron_schedule(config: &Config, cron_expr: &str) -> Result<()> {
    // Validate cron expression
    let schedule = Schedule::from_str(cron_expr)
        .map_err(|e| anyhow::anyhow!("Invalid cron expression: {}", e))?;
    
    println!("{}", format!("📅 Valid cron expression: {}", cron_expr).green());
    println!("\nNext 5 scheduled runs:");
    
    for datetime in schedule.upcoming(Local).take(5) {
        println!("  {}", datetime.format("%Y-%m-%d %H:%M:%S"));
    }
    
    println!("\n{}", "To add this to your crontab:".yellow());
    println!("  crontab -e");
    println!("  # Add: {} cd {} && synclite sync", cron_expr, config.source.display());
    
    Ok(())
}

/// Add an interval-based schedule
pub async fn add_interval_schedule(config: &Config, minutes: u64) -> Result<()> {
    println!("{}", format!("📅 Interval schedule: every {} minutes", minutes).green());
    
    println!("\n{}", "Linux/macOS (cron):".yellow());
    println!("  crontab -e");
    println!("  # Add: */{} * * * * cd {} && synclite sync", minutes, config.source.display());
    
    println!("\n{}", "Windows (Task Scheduler):".yellow());
    println!(
        "  schtasks /create /tn SyncLite /tr \"cd {} && synclite sync\" /sc minute /mo {}",
        config.source.display(),
        minutes
    );
    
    Ok(())
}

/// Run scheduler daemon (keeps running and executes sync on schedule)
pub async fn run_scheduler(config: Config, cron_expr: String) -> Result<()> {
    let schedule = Schedule::from_str(&cron_expr)?;
    let engine = SyncEngine::new(config);
    
    info!("Scheduler started with cron: {}", cron_expr);
    println!("{}", format!("📅 Scheduler started: {}", cron_expr).cyan().bold());
    println!("   Press Ctrl+C to stop\n");
    
    loop {
        let next = schedule.upcoming(Local).next();
        
        if let Some(next_time) = next {
            let now = Local::now();
            let duration = next_time.signed_duration_since(now);
            
            info!("Next sync scheduled at: {} (in {} seconds)", next_time, duration.num_seconds());
            
            // Sleep until next scheduled time
            tokio::time::sleep(Duration::from_secs(duration.num_seconds() as u64)).await;
            
            println!("{}", format!("🔄 Running scheduled sync at {}", Local::now().format("%H:%M:%S")).cyan());
            
            if let Err(e) = engine.sync(false, false).await {
                warn!("Scheduled sync failed: {}", e);
                println!("{}", format!("❌ Sync failed: {}", e).red());
            } else {
                println!("{}", "✅ Scheduled sync completed".green());
            }
        } else {
            warn!("No upcoming scheduled times found");
            break;
        }
    }
    
    Ok(())
}
