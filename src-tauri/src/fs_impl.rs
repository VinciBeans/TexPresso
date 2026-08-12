//! tokio::fs 实现 core 的 FileSystem trait（modules.md §3.2 / D4）。

use async_trait::async_trait;
use std::io;
use std::path::Path;
use texpresso_core::project::{DirEntry, FileSystem};

/// 真实文件系统实现（src-tauri 注入 core）。
pub struct TokioFs;

#[async_trait]
impl FileSystem for TokioFs {
    async fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut rd = tokio::fs::read_dir(path).await?;
        let mut out = Vec::new();
        while let Some(entry) = rd.next_entry().await? {
            let file_type = entry.file_type().await?;
            out.push(DirEntry {
                path: entry.path(),
                is_dir: file_type.is_dir(),
            });
        }
        Ok(out)
    }

    async fn read_to_string(&self, path: &Path) -> io::Result<String> {
        tokio::fs::read_to_string(path).await
    }
}
