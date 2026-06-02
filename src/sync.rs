use anyhow::Result;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use walkdir::WalkDir;

use crate::config::{Config, DestinationType};
use crate::hash::{FileInfo, HashCache};
use crate::storage::{FileEntry, StorageBackend, StorageFactory};

/// Sync operation types
#[derive(Debug, Clone, PartialEq)]
pub enum SyncOp {
    Upload(PathBuf),
    Download(PathBuf),
    DeleteLocal(PathBuf),
    DeleteRemote(String),
    Conflict(PathBuf, ConflictResolution),
    Skip(PathBuf, String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    KeepLocal,
    KeepRemote,
    KeepBoth,
}

/// Sync engine
pub struct SyncEngine {
    config: Config,
    hash_cache: HashCache,
}

impl SyncEngine {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            hash_cache: HashCache::new(),
        }
    }
    
    /// Perform synchronization
    pub async fn sync(&self, dry_run: bool, force: bool) -> Result<()> {
        let storage = StorageFactory::create(&self.config)?;
        
        println!("{}", "🔍 Scanning files...".cyan());
        
        // Collect local files
        let local_files = self.collect_local_files().await?;
        println!("   Found {} local files", local_files.len().to_string().cyan());
        
        // Collect remote files
        let remote_files = storage.list_files("").await?;
        println!("   Found {} remote files", remote_files.len().to_string().cyan());
        
        // Determine operations
        let ops = self.determine_operations(&local_files, &remote_files, force).await?;
        
        if ops.is_empty() {
            println!("{}", "✅ Everything is up to date!".green());
            return Ok(());
        }
        
        println!("\n{}", "📋 Sync plan:".yellow().bold());
        self.print_operations(&ops);
        
        if dry_run {
            println!("\n{}", "🔍 Dry run - no changes made".yellow());
            return Ok(());
        }
        
        // Execute operations
        self.execute_operations(&*storage, ops).await?;
        
        Ok(())
    }
    
    /// Show sync status
    pub async fn status(&self) -> Result<()> {
        println!("{}", "📊 Sync Status".cyan().bold());
        println!("   Source: {}", self.config.source.display().to_string().cyan());
        println!("   Destination: {}", self.config.destination.cyan());
        println!("   Mode: {}", self.config.mode.cyan());
        
        let storage = StorageFactory::create(&self.config)?;
        
        let local_files = self.collect_local_files().await?;
        let remote_files = storage.list_files("").await?;
        
        let local_size: u64 = local_files.values().map(|f| f.size).sum();
        let remote_size: u64 = remote_files.iter().map(|f| f.size).sum();
        
        println!("\n{}", "Local:".green());
        println!("   Files: {}", local_files.len());
        println!("   Size: {}", format_size(local_size));
        
        println!("\n{}", "Remote:".blue());
        println!("   Files: {}", remote_files.len());
        println!("   Size: {}", format_size(remote_size));
        
        // Calculate pending changes
        let ops = self.determine_operations(&local_files, &remote_files, false).await?;
        if !ops.is_empty() {
            println!("\n{}", format!("Pending changes: {}", ops.len()).yellow());
        } else {
            println!("\n{}", "In sync ✓".green());
        }
        
        Ok(())
    }
    
    /// Collect local files with hashes
    async fn collect_local_files(&self) -> Result<HashMap<String, FileInfo>> {
        let mut files = HashMap::new();
        
        for entry in WalkDir::new(&self.config.source)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }
            
            if self.config.should_exclude(path) {
                debug!("Excluding: {:?}", path);
                continue;
            }
            
            let relative_path = path
                .strip_prefix(&self.config.source)?
                .to_string_lossy()
                .to_string();
            
            match FileInfo::from_path(path) {
                Ok(info) => {
                    files.insert(relative_path, info);
                }
                Err(e) => {
                    warn!("Failed to hash {:?}: {}", path, e);
                }
            }
        }
        
        Ok(files)
    }
    
    /// Determine sync operations
    async fn determine_operations(
        &self,
        local_files: &HashMap<String, FileInfo>,
        remote_files: &[FileEntry],
        force: bool,
    ) -> Result<Vec<SyncOp>> {
        let mut ops = Vec::new();
        let remote_map: HashMap<String, &FileEntry> = remote_files
            .iter()
            .map(|f| (f.path.clone(), f))
            .collect();
        
        match self.config.mode.as_str() {
            "push" => {
                // Upload new and changed files
                for (rel_path, local_info) in local_files {
                    if let Some(remote_entry) = remote_map.get(rel_path) {
                        if local_info.size != remote_entry.size {
                            if force {
                                ops.push(SyncOp::Upload(PathBuf::from(rel_path)));
                            } else {
                                ops.push(SyncOp::Conflict(
                                    PathBuf::from(rel_path),
                                    ConflictResolution::KeepLocal,
                                ));
                            }
                        }
                    } else {
                        ops.push(SyncOp::Upload(PathBuf::from(rel_path)));
                    }
                }
                
                // Delete remote files not in local
                for rel_path in remote_map.keys() {
                    if !local_files.contains_key(rel_path) {
                        ops.push(SyncOp::DeleteRemote(rel_path.clone()));
                    }
                }
            }
            "pull" => {
                // Download new and changed files
                for remote_entry in remote_files {
                    let rel_path = &remote_entry.path;
                    if let Some(local_info) = local_files.get(rel_path) {
                        if local_info.size != remote_entry.size {
                            if force {
                                ops.push(SyncOp::Download(PathBuf::from(rel_path)));
                            } else {
                                ops.push(SyncOp::Conflict(
                                    PathBuf::from(rel_path),
                                    ConflictResolution::KeepRemote,
                                ));
                            }
                        }
                    } else {
                        ops.push(SyncOp::Download(PathBuf::from(rel_path)));
                    }
                }
                
                // Delete local files not in remote
                for rel_path in local_files.keys() {
                    if !remote_map.contains_key(rel_path) {
                        ops.push(SyncOp::DeleteLocal(PathBuf::from(rel_path)));
                    }
                }
            }
            "bidirectional" | _ => {
                // Bidirectional sync with timestamp-based conflict resolution
                for (rel_path, local_info) in local_files {
                    if let Some(remote_entry) = remote_map.get(rel_path) {
                        if local_info.size != remote_entry.size {
                            let local_modified = local_info.modified;
                            let remote_modified = remote_entry.modified
                                .map(|d| std::time::SystemTime::from(d));
                            
                            match (local_modified, remote_modified) {
                                (Some(l), Some(r)) if l > r => {
                                    ops.push(SyncOp::Upload(PathBuf::from(rel_path)));
                                }
                                (Some(l), Some(r)) if l < r => {
                                    ops.push(SyncOp::Download(PathBuf::from(rel_path)));
                                }
                                _ => {
                                    if force {
                                        ops.push(SyncOp::Upload(PathBuf::from(rel_path)));
                                    } else {
                                        ops.push(SyncOp::Conflict(
                                            PathBuf::from(rel_path),
                                            ConflictResolution::KeepBoth,
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        ops.push(SyncOp::Upload(PathBuf::from(rel_path)));
                    }
                }
                
                // Download remote-only files
                for remote_entry in remote_files {
                    if !local_files.contains_key(&remote_entry.path) {
                        ops.push(SyncOp::Download(PathBuf::from(&remote_entry.path)));
                    }
                }
            }
        }
        
        Ok(ops)
    }
    
    /// Print operations summary
    fn print_operations(&self, ops: &[SyncOp]) {
        let mut uploads = 0;
        let mut downloads = 0;
        let mut deletes = 0;
        let mut conflicts = 0;
        
        for op in ops {
            match op {
                SyncOp::Upload(_) => uploads += 1,
                SyncOp::Download(_) => downloads += 1,
                SyncOp::DeleteLocal(_) | SyncOp::DeleteRemote(_) => deletes += 1,
                SyncOp::Conflict(_, _) => conflicts += 1,
                _ => {}
            }
        }
        
        if uploads > 0 {
            println!("   {} Upload: {}", "↑".green(), uploads.to_string().cyan());
        }
        if downloads > 0 {
            println!("   {} Download: {}", "↓".blue(), downloads.to_string().cyan());
        }
        if deletes > 0 {
            println!("   {} Delete: {}", "×".red(), deletes.to_string().cyan());
        }
        if conflicts > 0 {
            println!("   {} Conflict: {}", "!".yellow(), conflicts.to_string().cyan());
        }
    }
    
    /// Execute sync operations
    async fn execute_operations(
        &self,
        storage: &dyn StorageBackend,
        ops: Vec<SyncOp>,
    ) -> Result<()> {
        let pb = ProgressBar::new(ops.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        
        for op in ops {
            match op {
                SyncOp::Upload(rel_path) => {
                    let local_path = self.config.source.join(&rel_path);
                    let remote_path = rel_path.to_string_lossy();
                    
                    pb.set_message(format!("Uploading {}", rel_path.display()));
                    
                    if let Err(e) = storage.upload(&local_path, &remote_path).await {
                        warn!("Failed to upload {:?}: {}", rel_path, e);
                    }
                }
                SyncOp::Download(rel_path) => {
                    let local_path = self.config.source.join(&rel_path);
                    let remote_path = rel_path.to_string_lossy();
                    
                    pb.set_message(format!("Downloading {}", rel_path.display()));
                    
                    if let Err(e) = storage.download(&remote_path, &local_path).await {
                        warn!("Failed to download {:?}: {}", rel_path, e);
                    }
                }
                SyncOp::DeleteLocal(rel_path) => {
                    let local_path = self.config.source.join(&rel_path);
                    
                    pb.set_message(format!("Deleting local {}", rel_path.display()));
                    
                    if let Err(e) = tokio::fs::remove_file(&local_path).await {
                        warn!("Failed to delete {:?}: {}", local_path, e);
                    }
                }
                SyncOp::DeleteRemote(rel_path) => {
                    pb.set_message(format!("Deleting remote {}", rel_path));
                    
                    if let Err(e) = storage.delete(&rel_path).await {
                        warn!("Failed to delete remote {:?}: {}", rel_path, e);
                    }
                }
                SyncOp::Conflict(rel_path, resolution) => {
                    pb.set_message(format!("Resolving conflict {}", rel_path.display()));
                    
                    match resolution {
                        ConflictResolution::KeepLocal => {
                            let local_path = self.config.source.join(&rel_path);
                            let remote_path = rel_path.to_string_lossy();
                            let _ = storage.upload(&local_path, &remote_path).await;
                        }
                        ConflictResolution::KeepRemote => {
                            let local_path = self.config.source.join(&rel_path);
                            let remote_path = rel_path.to_string_lossy();
                            let _ = storage.download(&remote_path, &local_path).await;
                        }
                        ConflictResolution::KeepBoth => {
                            // Rename local file and download remote
                            let local_path = self.config.source.join(&rel_path);
                            let backup_path = self.config.source.join(format!(
                                "{}.local",
                                rel_path.display()
                            ));
                            let _ = tokio::fs::rename(&local_path, &backup_path).await;
                            
                            let remote_path = rel_path.to_string_lossy();
                            let _ = storage.download(&remote_path, &local_path).await;
                        }
                    }
                }
                _ => {}
            }
            
            pb.inc(1);
        }
        
        pb.finish_with_message("Done");
        Ok(())
    }
}

/// Format byte size to human readable
fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let exp = (bytes as f64).log(1024.0).min(UNITS.len() as f64 - 1.0) as usize;
    let size = bytes as f64 / 1024f64.powi(exp as i32);
    
    format!("{:.2} {}", size, UNITS[exp])
}
