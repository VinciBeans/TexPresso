//! 项目状态类型（modules.md §3.1）。

use std::path::PathBuf;

/// 当前打开项目的状态。存放在 src-tauri 的组合层（modules.md §7），
/// core 内只作为纯函数的输入参数传递。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectState {
    /// 项目根目录（打开文件夹即项目）。
    pub root: PathBuf,
    /// 当前根文件（探测结果或手动覆盖）。
    pub root_file: Option<PathBuf>,
}

/// 根文件候选（含 `\documentclass` 且未被引用的 .tex）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootCandidate {
    pub path: PathBuf,
}

/// 探测结果（modules.md §5.4）：
/// 唯一 → 自动采用；多候选 → 前端弹窗；零候选 → 提示手动指定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootResolution {
    Unique(PathBuf),
    Multiple(Vec<PathBuf>),
    None,
}
