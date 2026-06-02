use anyhow::Result;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use crate::config::Config;
use crate::sync::SyncEngine;

/// File watcher for automatic sync
pub struct FileWatcher {
    config: Config,
    debounce_secs: u64,
}

impl FileWatcher {
    pub fn new(config: Config, debounce_secs: u64) -> Self {
        Self {
            config,
            debounce_secs,
        }
    }
    
    /// Start watching for file changes
    pub async fn start(&self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<Event>(100);
        let config = self.config.clone();
        let debounce_secs = self.debounce_secs;
        
        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = tx.try_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(1)),
        )?;
        
        // Watch source directory
        watcher.watch(&self.config.source, RecursiveMode::Recursive)?;
        
        info!("Started watching: {:?}", self.config.source);
        
        let mut pending_sync = false;
        let mut last_event = std::time::Instant::now();
        
        loop {
            tokio::select! {
                Some(event) = rx.recv() => {
                    debug!("File event: {:?}", event);
                    
                    // Check if we should ignore this event
                    if should_ignore_event(&event, &config) {
                        continue;
                    }
                    
                    pending_sync = true;
                    last_event = std::time::Instant::now();
                    
                    println!("📁 Change detected: {:?}", event.paths.first());
                }
                _ = sleep(Duration::from_secs(1)) => {
                    if pending_sync && last_event.elapsed().as_secs() >= debounce_secs {
                        println!("\n{}", "🔄 Auto-syncing changes...".cyan());
                        
                        let engine = SyncEngine::new(config.clone());
                        if let Err(e) = engine.sync(false, false).await {
                            error!("Sync failed: {}", e);
                            println!("{}", format!("❌ Sync failed: {}", e).red());
                        } else {
                            println!("{}", "✅ Auto-sync completed".green());
                        }
                        
                        pending_sync = false;
                        println!("\n{}", "👁️  Watching for changes... (Ctrl+C to stop)".cyan());
                    }
                }
            }
        }
    }
}

/// Check if a file event should be ignored
fn should_ignore_event(event: &Event, config: &Config) -> bool {
    for path in &event.paths {
        let path_str = path.to_string_lossy();
        
        // Check against exclude patterns
        for pattern in &config.exclude {
            if path_str.contains(pattern) {
                return true;
            }
        }
        
        // Ignore synclite's own files
        if path_str.contains(".synclite") {
            return true;
        }
    }
    
    false
}
