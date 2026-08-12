//! 测试工具（仅 #[cfg(test)]）：FakeFS / FakeRunner / 事件收集器。
//!
//! 信息局部性纪律的检验场：core 的模块只依赖 trait 接口，
//! 测试用假实现注入，验证模块不偷偷依赖 IO 或全局状态。

use crate::project::{DirEntry, FileSystem};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

/// 内存文件系统：目录注册表 + 文件内容表。
#[derive(Default)]
pub struct FakeFS {
    dirs: HashSet<PathBuf>,
    files: HashMap<PathBuf, String>,
}

impl FakeFS {
    pub fn new() -> Self {
        Self::default()
    }

    /// 放一个文件（自动注册父目录链）。
    pub fn put_file(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        let path = path.into();
        self.files.insert(path.clone(), content.into());
        let mut parent = path.parent();
        while let Some(p) = parent {
            if p.as_os_str().is_empty() {
                break;
            }
            self.dirs.insert(p.to_path_buf());
            parent = p.parent();
        }
    }

    /// 放一个空目录。
    pub fn put_dir(&mut self, path: impl Into<PathBuf>) {
        self.dirs.insert(path.into());
    }

    pub fn file(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }
}

/// 同步执行一个 future（测试用单线程运行时）。
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

#[async_trait]
impl FileSystem for FakeFS {
    async fn read_dir(&self, path: &Path) -> io::Result<Vec<DirEntry>> {
        let mut out: Vec<DirEntry> = Vec::new();
        for d in self.dirs.iter() {
            if d.parent() == Some(path) {
                out.push(DirEntry {
                    path: d.clone(),
                    is_dir: true,
                });
            }
        }
        for f in self.files.keys() {
            if f.parent() == Some(path) {
                out.push(DirEntry {
                    path: f.clone(),
                    is_dir: false,
                });
            }
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        if out.is_empty() && !self.dirs.contains(path) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "目录不存在"));
        }
        Ok(out)
    }

    async fn read_to_string(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "文件不存在"))
    }
}

/// 可注入的编译结果队列（按调用顺序出队）。
pub type OutcomeQueue = std::collections::VecDeque<crate::types::CompileOutcome>;

/// 记录每次编译调用的参数。
#[derive(Debug, Clone, Default)]
pub struct CallRecord {
    pub requests: Vec<crate::types::CompileRequest>,
}
