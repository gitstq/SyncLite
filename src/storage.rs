use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use crate::config::Config;

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// List files in the storage
    async fn list_files(&self, prefix: &str) -> Result<Vec<FileEntry>>;
    
    /// Download a file
    async fn download(&self, remote_path: &str, local_path: &Path) -> Result<()>;
    
    /// Upload a file
    async fn upload(&self, local_path: &Path, remote_path: &str) -> Result<()>;
    
    /// Delete a file
    async fn delete(&self, remote_path: &str) -> Result<()>;
    
    /// Check if file exists
    async fn exists(&self, remote_path: &str) -> Result<bool>;
    
    /// Get file metadata
    async fn metadata(&self, remote_path: &str) -> Result<Option<FileMetadata>>;
}

/// File entry in storage
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    pub is_directory: bool,
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    pub checksum: Option<String>,
}

/// Local filesystem storage
pub struct LocalStorage {
    base_path: PathBuf,
}

impl LocalStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    fn resolve_path(&self, remote_path: &str) -> PathBuf {
        self.base_path.join(remote_path.trim_start_matches('/'))
    }
}

#[async_trait]
impl StorageBackend for LocalStorage {
    async fn list_files(&self, prefix: &str) -> Result<Vec<FileEntry>> {
        let path = self.resolve_path(prefix);
        let mut entries = Vec::new();
        
        if !path.exists() {
            return Ok(entries);
        }
        
        for entry in walkdir::WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let metadata = entry.metadata()?;
            let relative_path = entry.path()
                .strip_prefix(&self.base_path)?
                .to_string_lossy()
                .to_string();
            
            entries.push(FileEntry {
                path: relative_path,
                size: metadata.len(),
                modified: metadata.modified()
                    .ok()
                    .map(|t| chrono::DateTime::from(t)),
                is_directory: metadata.is_dir(),
            });
        }
        
        Ok(entries)
    }
    
    async fn download(&self, remote_path: &str, local_path: &Path) -> Result<()> {
        let source = self.resolve_path(remote_path);
        
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::copy(&source, local_path).await?;
        Ok(())
    }
    
    async fn upload(&self, local_path: &Path, remote_path: &str) -> Result<()> {
        let dest = self.resolve_path(remote_path);
        
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        tokio::fs::copy(local_path, &dest).await?;
        Ok(())
    }
    
    async fn delete(&self, remote_path: &str) -> Result<()> {
        let path = self.resolve_path(remote_path);
        
        if path.is_dir() {
            tokio::fs::remove_dir_all(path).await?;
        } else {
            tokio::fs::remove_file(path).await?;
        }
        
        Ok(())
    }
    
    async fn exists(&self, remote_path: &str) -> Result<bool> {
        let path = self.resolve_path(remote_path);
        Ok(path.exists())
    }
    
    async fn metadata(&self, remote_path: &str) -> Result<Option<FileMetadata>> {
        let path = self.resolve_path(remote_path);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let metadata = tokio::fs::metadata(&path).await?;
        
        Ok(Some(FileMetadata {
            size: metadata.len(),
            modified: metadata.modified()
                .ok()
                .map(|t| chrono::DateTime::from(t)),
            checksum: None,
        }))
    }
}

/// Storage factory
pub struct StorageFactory;

impl StorageFactory {
    pub fn create(config: &Config) -> Result<Box<dyn StorageBackend>> {
        if config.is_s3() {
            #[cfg(feature = "s3")]
            {
                Ok(Box::new(S3Storage::new(config)?))
            }
            #[cfg(not(feature = "s3"))]
            {
                anyhow::bail!("S3 support not enabled. Build with --features s3")
            }
        } else if config.is_sftp() {
            #[cfg(feature = "sftp")]
            {
                Ok(Box::new(SftpStorage::new(config)?))
            }
            #[cfg(not(feature = "sftp"))]
            {
                anyhow::bail!("SFTP support not enabled. Build with --features sftp")
            }
        } else {
            let dest_path = PathBuf::from(&config.destination);
            Ok(Box::new(LocalStorage::new(dest_path)))
        }
    }
}

/// S3 storage backend
#[cfg(feature = "s3")]
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    prefix: String,
}

#[cfg(feature = "s3")]
impl S3Storage {
    pub fn new(config: &Config) -> Result<Self> {
        use aws_config::BehaviorVersion;
        
        let (bucket, prefix) = config.parse_s3_dest()
            .ok_or_else(|| anyhow::anyhow!("Invalid S3 destination"))?;
        
        // This would need proper async initialization in real implementation
        // For now, we'll use a placeholder
        let sdk_config = tokio::runtime::Handle::current()
            .block_on(async {
                aws_config::defaults(BehaviorVersion::latest())
                    .region(aws_sdk_s3::config::Region::new(
                        config.s3.as_ref().map(|s| s.region.clone())
                            .unwrap_or_else(|| "us-east-1".to_string())
                    ))
                    .load()
                    .await
            });
        
        let client = aws_sdk_s3::Client::new(&sdk_config);
        
        Ok(Self {
            client,
            bucket,
            prefix,
        })
    }
    
    fn resolve_key(&self, path: &str) -> String {
        if self.prefix.is_empty() {
            path.trim_start_matches('/').to_string()
        } else {
            format!("{}/{}", self.prefix, path.trim_start_matches('/'))
        }
    }
}

#[cfg(feature = "s3")]
#[async_trait]
impl StorageBackend for S3Storage {
    async fn list_files(&self, prefix: &str) -> Result<Vec<FileEntry>> {
        let key_prefix = self.resolve_key(prefix);
        
        let result = self.client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&key_prefix)
            .send()
            .await?;
        
        let entries = result.contents()
            .map(|objects| {
                objects.iter()
                    .filter_map(|obj| {
                        Some(FileEntry {
                            path: obj.key()?.to_string(),
                            size: obj.size() as u64,
                            modified: obj.last_modified()
                                .map(|d| chrono::DateTime::from(d)),
                            is_directory: false,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(entries)
    }
    
    async fn download(&self, remote_path: &str, local_path: &Path) -> Result<()> {
        let key = self.resolve_key(remote_path);
        
        let result = self.client
            .get_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await?;
        
        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let bytes = result.body.collect().await?.into_bytes();
        tokio::fs::write(local_path, bytes).await?;
        
        Ok(())
    }
    
    async fn upload(&self, local_path: &Path, remote_path: &str) -> Result<()> {
        let key = self.resolve_key(remote_path);
        let body = tokio::fs::read(local_path).await?;
        
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(body.into())
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn delete(&self, remote_path: &str) -> Result<()> {
        let key = self.resolve_key(remote_path);
        
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await?;
        
        Ok(())
    }
    
    async fn exists(&self, remote_path: &str) -> Result<bool> {
        let key = self.resolve_key(remote_path);
        
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                if e.to_string().contains("NotFound") {
                    Ok(false)
                } else {
                    Err(e.into())
                }
            }
        }
    }
    
    async fn metadata(&self, remote_path: &str) -> Result<Option<FileMetadata>> {
        let key = self.resolve_key(remote_path);
        
        match self.client
            .head_object()
            .bucket(&self.bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(result) => {
                Ok(Some(FileMetadata {
                    size: result.content_length() as u64,
                    modified: result.last_modified()
                        .map(|d| chrono::DateTime::from(d)),
                    checksum: result.e_tag().map(|s| s.to_string()),
                }))
            }
            Err(_) => Ok(None),
        }
    }
}

/// SFTP storage backend
#[cfg(feature = "sftp")]
pub struct SftpStorage {
    // SFTP implementation would go here
    config: Config,
}

#[cfg(feature = "sftp")]
impl SftpStorage {
    pub fn new(config: &Config) -> Result<Self> {
        Ok(Self { config: config.clone() })
    }
}

#[cfg(feature = "sftp")]
#[async_trait]
impl StorageBackend for SftpStorage {
    async fn list_files(&self, _prefix: &str) -> Result<Vec<FileEntry>> {
        // SFTP implementation
        todo!("SFTP implementation")
    }
    
    async fn download(&self, _remote_path: &str, _local_path: &Path) -> Result<()> {
        todo!("SFTP implementation")
    }
    
    async fn upload(&self, _local_path: &Path, _remote_path: &str) -> Result<()> {
        todo!("SFTP implementation")
    }
    
    async fn delete(&self, _remote_path: &str) -> Result<()> {
        todo!("SFTP implementation")
    }
    
    async fn exists(&self, _remote_path: &str) -> Result<bool> {
        todo!("SFTP implementation")
    }
    
    async fn metadata(&self, _remote_path: &str) -> Result<Option<FileMetadata>> {
        todo!("SFTP implementation")
    }
}
