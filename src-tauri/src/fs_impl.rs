//! tokio::fs 实现 core 的 FileSystem trait（modules.md §3.2 / D4）。

use async_trait::async_trait;
use std::io;
use std::path::{Path, PathBuf};
use texpresso_core::project::{DirEntry, FileSystem};

/// Windows：`canonicalize` 返回 `\\?\` 前缀的 verbatim 路径，
/// 会破坏前端 `resolvePath` 的绝对路径判定（WSL 遗留：前端只认 `/`/盘符 开头）。
/// 对外暴露（root / root_file / 目录项）前统一剥掉该前缀；内部校验仍可 canonicalize。
pub fn strip_verbatim(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        // \\?\UNC\server\share\... → \\server\share\...
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    p.to_path_buf()
}

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
                path: strip_verbatim(&entry.path()),
                is_dir: file_type.is_dir(),
            });
        }
        Ok(out)
    }

    async fn read_to_string(&self, path: &Path) -> io::Result<String> {
        tokio::fs::read_to_string(path).await
    }
}
