use std::path::{Path, PathBuf};

use async_trait::async_trait;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use crate::error::{AppError, Result};
use crate::ports::blob::{BlobReader, BlobStore};

pub struct FsBlobStore {
    root: PathBuf,
}

impl FsBlobStore {
    pub async fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&root).await?;
        Ok(FsBlobStore { root })
    }

    fn path(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }
}

#[async_trait]
impl BlobStore for FsBlobStore {
    async fn put(&self, data: Bytes) -> Result<String> {
        let key: String = Sha256::digest(&data)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let path = self.path(&key);
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return Ok(key);
        }
        tokio::fs::write(&path, &data).await?;
        Ok(key)
    }

    async fn get(&self, key: &str) -> Result<Bytes> {
        let mut file = tokio::fs::File::open(self.path(key))
            .await
            .map_err(|_| AppError::NotFound)?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        Ok(Bytes::from(buf))
    }

    async fn open(&self, key: &str) -> Result<BlobReader> {
        let file = tokio::fs::File::open(self.path(key))
            .await
            .map_err(|_| AppError::NotFound)?;
        Ok(Box::new(file))
    }

    async fn size(&self, key: &str) -> Result<u64> {
        let meta = tokio::fs::metadata(self.path(key))
            .await
            .map_err(|_| AppError::NotFound)?;
        Ok(meta.len())
    }

    async fn delete(&self, key: &str) -> Result<()> {
        match tokio::fs::remove_file(self.path(key)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
