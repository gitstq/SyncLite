use anyhow::Result;
use chrono::Local;
use colored::*;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::Builder;
use tracing::{info, warn};
use walkdir::WalkDir;

use crate::config::Config;

/// Backup manager
pub struct BackupManager {
    config: Config,
}

impl BackupManager {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
    
    /// Create a backup
    pub async fn create(&self, tag: Option<String>, compress: bool) -> Result<()> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let backup_name = match tag {
            Some(t) => format!("{}_{}", timestamp, t),
            None => timestamp.to_string(),
        };
        
        let backup_dir = self.config.source.join(&self.config.backup.location);
        tokio::fs::create_dir_all(&backup_dir).await?;
        
        let backup_path = if compress {
            backup_dir.join(format!("{}.tar.gz", backup_name))
        } else {
            backup_dir.join(format!("{}", backup_name))
        };
        
        println!("{}", format!("📦 Creating backup: {}", backup_name).cyan());
        
        if compress {
            self.create_compressed_backup(&backup_path).await?;
        } else {
            self.create_directory_backup(&backup_path).await?;
        }
        
        // Save backup metadata
        let metadata = BackupMetadata {
            name: backup_name.clone(),
            timestamp: Local::now(),
            compressed: compress,
            tag,
        };
        
        let meta_path = backup_dir.join(format!("{}.json", backup_name));
        let meta_json = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(&meta_path, meta_json).await?;
        
        // Cleanup old backups if enabled
        if self.config.backup.auto_cleanup {
            self.cleanup_old_backups().await?;
        }
        
        println!("{}", format!("✅ Backup created: {:?}", backup_path).green());
        
        Ok(())
    }
    
    /// Create compressed backup
    async fn create_compressed_backup(&self, backup_path: &Path) -> Result<()> {
        let tar_gz = File::create(backup_path)?;
        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = Builder::new(enc);
        
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
                continue;
            }
            
            if path.starts_with(&self.config.source.join(&self.config.backup.location)) {
                continue; // Don't backup the backups
            }
            
            let relative_path = path.strip_prefix(&self.config.source)?;
            
            match tar.append_path_with_name(path, relative_path) {
                Ok(_) => {}
                Err(e) => {
                    warn!("Failed to add {:?} to backup: {}", path, e);
                }
            }
        }
        
        tar.finish()?;
        Ok(())
    }
    
    /// Create directory backup (hard links)
    async fn create_directory_backup(&self, backup_path: &Path) -> Result<()> {
        tokio::fs::create_dir_all(backup_path).await?;
        
        for entry in WalkDir::new(&self.config.source)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if self.config.should_exclude(path) {
                continue;
            }
            
            if path.starts_with(&self.config.source.join(&self.config.backup.location)) {
                continue;
            }
            
            let relative_path = path.strip_prefix(&self.config.source)?;
            let dest_path = backup_path.join(relative_path);
            
            if path.is_dir() {
                tokio::fs::create_dir_all(&dest_path).await?;
            } else if path.is_file() {
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::copy(path, dest_path).await?;
            }
        }
        
        Ok(())
    }
    
    /// List all backups
    pub async fn list(&self) -> Result<()> {
        let backup_dir = self.config.source.join(&self.config.backup.location);
        
        if !backup_dir.exists() {
            println!("{}", "No backups found".yellow());
            return Ok(());
        }
        
        let mut backups = Vec::new();
        
        let mut entries = tokio::fs::read_dir(&backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = tokio::fs::read_to_string(&path).await?;
                if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&content) {
                    backups.push(metadata);
                }
            }
        }
        
        backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        if backups.is_empty() {
            println!("{}", "No backups found".yellow());
            return Ok(());
        }
        
        println!("{}", "📋 Backup History".cyan().bold());
        println!("{:<25} {:<15} {:<10} {}", "Timestamp", "Name", "Type", "Tag");
        println!("{}", "-".repeat(80));
        
        for backup in backups {
            let type_str = if backup.compressed { "compressed" } else { "directory" };
            let tag_str = backup.tag.as_deref().unwrap_or("-");
            
            println!(
                "{:<25} {:<15} {:<10} {}",
                backup.timestamp.format("%Y-%m-%d %H:%M:%S"),
                backup.name,
                type_str,
                tag_str
            );
        }
        
        Ok(())
    }
    
    /// Restore from backup
    pub async fn restore(&self, backup_id: &str, dest: Option<PathBuf>) -> Result<()> {
        let backup_dir = self.config.source.join(&self.config.backup.location);
        
        // Try to find backup by exact name or tag
        let backup_path = backup_dir.join(backup_id);
        let tar_path = backup_dir.join(format!("{}.tar.gz", backup_id));
        
        let restore_dest = dest.unwrap_or_else(|| self.config.source.clone());
        
        if tar_path.exists() {
            self.restore_compressed(&tar_path, &restore_dest).await?;
        } else if backup_path.exists() && backup_path.is_dir() {
            self.restore_directory(&backup_path, &restore_dest).await?;
        } else {
            // Try to find by tag
            let mut entries = tokio::fs::read_dir(&backup_dir).await?;
            let mut found = false;
            
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    let content = tokio::fs::read_to_string(&path).await?;
                    if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&content) {
                        if metadata.tag.as_deref() == Some(backup_id) {
                            let backup_file = if metadata.compressed {
                                backup_dir.join(format!("{}.tar.gz", metadata.name))
                            } else {
                                backup_dir.join(&metadata.name)
                            };
                            
                            if metadata.compressed {
                                self.restore_compressed(&backup_file, &restore_dest).await?;
                            } else {
                                self.restore_directory(&backup_file, &restore_dest).await?;
                            }
                            found = true;
                            break;
                        }
                    }
                }
            }
            
            if !found {
                anyhow::bail!("Backup not found: {}", backup_id);
            }
        }
        
        Ok(())
    }
    
    /// Restore from compressed backup
    async fn restore_compressed(&self, tar_path: &Path, dest: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;
        
        println!("{}", format!("📦 Extracting: {:?}", tar_path).cyan());
        
        let tar_gz = File::open(tar_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        
        tokio::fs::create_dir_all(dest).await?;
        archive.unpack(dest)?;
        
        Ok(())
    }
    
    /// Restore from directory backup
    async fn restore_directory(&self, backup_path: &Path, dest: &Path) -> Result<()> {
        println!("{}", format!("📁 Restoring from: {:?}", backup_path).cyan());
        
        for entry in WalkDir::new(backup_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let relative_path = path.strip_prefix(backup_path)?;
            let dest_path = dest.join(relative_path);
            
            if path.is_dir() {
                tokio::fs::create_dir_all(&dest_path).await?;
            } else if path.is_file() {
                if let Some(parent) = dest_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::copy(path, dest_path).await?;
            }
        }
        
        Ok(())
    }
    
    /// Cleanup old backups
    async fn cleanup_old_backups(&self) -> Result<()> {
        let backup_dir = self.config.source.join(&self.config.backup.location);
        
        if !backup_dir.exists() {
            return Ok(());
        }
        
        let mut backups = Vec::new();
        
        let mut entries = tokio::fs::read_dir(&backup_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = tokio::fs::read_to_string(&path).await?;
                if let Ok(metadata) = serde_json::from_str::<BackupMetadata>(&content) {
                    backups.push((metadata, path));
                }
            }
        }
        
        backups.sort_by(|a, b| b.0.timestamp.cmp(&a.0.timestamp));
        
        if backups.len() > self.config.backup.max_backups {
            let to_delete = &backups[self.config.backup.max_backups..];
            
            for (metadata, meta_path) in to_delete {
                info!("Removing old backup: {}", metadata.name);
                
                // Remove metadata file
                let _ = tokio::fs::remove_file(meta_path).await;
                
                // Remove backup data
                let backup_path = if metadata.compressed {
                    backup_dir.join(format!("{}.tar.gz", metadata.name))
                } else {
                    backup_dir.join(&metadata.name)
                };
                
                if metadata.compressed {
                    let _ = tokio::fs::remove_file(backup_path).await;
                } else {
                    let _ = tokio::fs::remove_dir_all(backup_path).await;
                }
            }
        }
        
        Ok(())
    }
}

/// Backup metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct BackupMetadata {
    name: String,
    timestamp: chrono::DateTime<Local>,
    compressed: bool,
    tag: Option<String>,
}
