//! IO 抽象：core 唯一的文件访问入口（modules.md §3.2 / 设计决策 D4）。

use async_trait::async_trait;
use std::io;
use std::path::PathBuf;

/// 目录项（携带 is_dir，避免二次 stat）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub path: PathBuf,
    pub is_dir: bool,
}

/// core 唯一的 IO 抽象。面最小：只两个方法。
/// src-tauri 用 tokio::fs 实现；测试用 FakeFS。
#[async_trait]
pub trait FileSystem: Send + Sync {
    /// 非递归列出目录子项。
    async fn read_dir(&self, path: &std::path::Path) -> io::Result<Vec<DirEntry>>;
    /// 读文本文件（UTF-8）。
    async fn read_to_string(&self, path: &std::path::Path) -> io::Result<String>;
}
