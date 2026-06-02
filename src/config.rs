use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Source directory to sync
    pub source: PathBuf,
    
    /// Destination (local path, s3://bucket, sftp://host)
    pub destination: String,
    
    /// Sync mode: bidirectional, push, pull
    #[serde(default = "default_mode")]
    pub mode: String,
    
    /// Files/directories to exclude
    #[serde(default)]
    pub exclude: Vec<String>,
    
    /// Conflict resolution strategy
    #[serde(default = "default_conflict_strategy")]
    pub conflict_strategy: String,
    
    /// Enable compression for network transfers
    #[serde(default = "default_compression")]
    pub compression: bool,
    
    /// Number of concurrent transfers
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    
    /// S3 configuration
    #[serde(default)]
    pub s3: Option<S3Config>,
    
    /// SFTP configuration
    #[serde(default)]
    pub sftp: Option<SftpConfig>,
    
    /// Backup settings
    #[serde(default)]
    pub backup: BackupConfig,
    
    /// Additional options
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub endpoint: Option<String>,
    pub bucket: String,
    pub prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SftpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<PathBuf>,
    pub remote_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    #[serde(default = "default_backup_enabled")]
    pub enabled: bool,
    
    #[serde(default = "default_backup_location")]
    pub location: PathBuf,
    
    #[serde(default = "default_max_backups")]
    pub max_backups: usize,
    
    #[serde(default)]
    pub auto_cleanup: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            enabled: default_backup_enabled(),
            location: default_backup_location(),
            max_backups: default_max_backups(),
            auto_cleanup: false,
        }
    }
}

fn default_mode() -> String {
    "bidirectional".to_string()
}

fn default_conflict_strategy() -> String {
    "timestamp".to_string()
}

fn default_compression() -> bool {
    true
}

fn default_concurrency() -> usize {
    4
}

fn default_backup_enabled() -> bool {
    true
}

fn default_backup_location() -> PathBuf {
    PathBuf::from(".synclite/backups")
}

fn default_max_backups() -> usize {
    10
}

impl Config {
    pub fn new(source: PathBuf, destination: String, mode: String) -> Self {
        Self {
            source,
            destination,
            mode,
            exclude: vec![
                ".git".to_string(),
                ".synclite".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".DS_Store".to_string(),
                "*.tmp".to_string(),
                "*.log".to_string(),
            ],
            conflict_strategy: default_conflict_strategy(),
            compression: default_compression(),
            concurrency: default_concurrency(),
            s3: None,
            sftp: None,
            backup: BackupConfig::default(),
            options: HashMap::new(),
        }
    }
    
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    /// Check if destination is S3
    pub fn is_s3(&self) -> bool {
        self.destination.starts_with("s3://")
    }
    
    /// Check if destination is SFTP
    pub fn is_sftp(&self) -> bool {
        self.destination.starts_with("sftp://")
    }
    
    /// Get destination type
    pub fn destination_type(&self) -> DestinationType {
        if self.is_s3() {
            DestinationType::S3
        } else if self.is_sftp() {
            DestinationType::Sftp
        } else {
            DestinationType::Local
        }
    }
    
    /// Parse S3 destination into bucket and prefix
    pub fn parse_s3_dest(&self) -> Option<(String, String)> {
        if !self.is_s3() {
            return None;
        }
        
        let dest = self.destination.trim_start_matches("s3://");
        let parts: Vec<&str> = dest.splitn(2, '/').collect();
        
        if parts.is_empty() {
            return None;
        }
        
        let bucket = parts[0].to_string();
        let prefix = if parts.len() > 1 {
            parts[1].to_string()
        } else {
            String::new()
        };
        
        Some((bucket, prefix))
    }
    
    /// Parse SFTP destination
    pub fn parse_sftp_dest(&self) -> Option<(String, u16, String)> {
        if !self.is_sftp() {
            return None;
        }
        
        let dest = self.destination.trim_start_matches("sftp://");
        let parts: Vec<&str> = dest.splitn(2, '/').collect();
        
        if parts.is_empty() {
            return None;
        }
        
        let host_port: Vec<&str> = parts[0].splitn(2, ':').collect();
        let host = host_port[0].to_string();
        let port = if host_port.len() > 1 {
            host_port[1].parse().unwrap_or(22)
        } else {
            22
        };
        
        let path = if parts.len() > 1 {
            format!("/{})", parts[1])
        } else {
            "/".to_string()
        };
        
        Some((host, port, path))
    }
    
    /// Check if a path should be excluded
    pub fn should_exclude(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        let file_name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        for pattern in &self.exclude {
            // Check exact match
            if path_str.contains(pattern) || file_name == *pattern {
                return true;
            }
            
            // Check glob pattern (simple implementation)
            if pattern.contains('*') {
                let regex_pattern = pattern.replace(".", "\\.").replace("*", ".*");
                if let Ok(regex) = regex::Regex::new(&regex_pattern) {
                    if regex.is_match(&file_name) {
                        return true;
                    }
                }
            }
        }
        
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DestinationType {
    Local,
    S3,
    Sftp,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_config_serialization() {
        let config = Config::new(
            PathBuf::from("/home/user/docs"),
            "s3://mybucket/backups".to_string(),
            "push".to_string(),
        );
        
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("source"));
        assert!(toml_str.contains("destination"));
    }
    
    #[test]
    fn test_parse_s3_dest() {
        let config = Config::new(
            PathBuf::from("/src"),
            "s3://mybucket/prefix/path".to_string(),
            "push".to_string(),
        );
        
        let (bucket, prefix) = config.parse_s3_dest().unwrap();
        assert_eq!(bucket, "mybucket");
        assert_eq!(prefix, "prefix/path");
    }
    
    #[test]
    fn test_should_exclude() {
        let config = Config::new(
            PathBuf::from("/src"),
            "/dest".to_string(),
            "push".to_string(),
        );
        
        assert!(config.should_exclude(Path::new("/src/.git")));
        assert!(config.should_exclude(Path::new("/src/test.tmp")));
        assert!(!config.should_exclude(Path::new("/src/file.txt")));
    }
}
