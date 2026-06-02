use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use anyhow::Result;

/// Calculate SHA256 hash of a file
pub fn calculate_file_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    
    let mut buffer = [0u8; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

/// Calculate hash from bytes
pub fn calculate_bytes_hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Quick hash using file metadata (faster but less accurate)
pub fn calculate_quick_hash<P: AsRef<Path>>(path: P) -> Result<String> {
    let metadata = std::fs::metadata(&path)?;
    let modified = metadata.modified()?;
    let size = metadata.len();
    
    let path_str = path.as_ref().to_string_lossy();
    let data = format!("{}:{}:{:?}", path_str, size, modified);
    
    Ok(calculate_bytes_hash(data.as_bytes()))
}

/// File info with hash for comparison
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub hash: String,
    pub quick_hash: String,
}

impl FileInfo {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        let full_hash = calculate_file_hash(&path)?;
        let quick = calculate_quick_hash(&path)?;
        
        Ok(Self {
            path: path.as_ref().to_string_lossy().to_string(),
            size: metadata.len(),
            modified: metadata.modified()?,
            hash: full_hash,
            quick_hash: quick,
        })
    }
    
    /// Check if file has changed by comparing quick hash
    pub fn has_changed(&self, other: &FileInfo) -> bool {
        self.quick_hash != other.quick_hash
    }
    
    /// Check if files are identical by comparing full hash
    pub fn is_identical(&self, other: &FileInfo) -> bool {
        self.hash == other.hash
    }
}

/// Hash cache for tracking file changes
pub struct HashCache {
    entries: std::collections::HashMap<String, FileInfo>,
}

impl HashCache {
    pub fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
        }
    }
    
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }
        
        let content = std::fs::read_to_string(path)?;
        let entries: std::collections::HashMap<String, FileInfo> = 
            serde_json::from_str(&content)?;
        
        Ok(Self { entries })
    }
    
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.entries)?;
        std::fs::write(path, content)?;
        Ok(())
    }
    
    pub fn get(&self, path: &str) -> Option<&FileInfo> {
        self.entries.get(path)
    }
    
    pub fn insert(&mut self, path: String, info: FileInfo) {
        self.entries.insert(path, info);
    }
    
    pub fn is_changed<P: AsRef<Path>>(&self, path: P) -> Result<bool> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        
        if let Some(cached) = self.entries.get(&path_str) {
            let current = FileInfo::from_path(&path)?;
            Ok(cached.has_changed(&current))
        } else {
            Ok(true) // New file
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;
    
    #[test]
    fn test_calculate_bytes_hash() {
        let hash1 = calculate_bytes_hash(b"hello");
        let hash2 = calculate_bytes_hash(b"hello");
        let hash3 = calculate_bytes_hash(b"world");
        
        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
    }
    
    #[test]
    fn test_file_hash() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(b"test content")?;
        
        let hash = calculate_file_hash(temp_file.path())?;
        assert_eq!(hash.len(), 64);
        
        Ok(())
    }
}
