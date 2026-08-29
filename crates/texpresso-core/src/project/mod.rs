//! 项目子系统（modules.md §3）。
//!
//! - [`fs`]：`FileSystem` trait——core 唯一的 IO 抽象（设计决策 D4）
//! - [`model`]：项目状态与探测结果类型
//! - [`scan`]：.tex 文件收集与忽略规则
//! - [`root_detect`]：根文件探测（正则启发式，ADR-0009）

pub mod fs;
pub mod model;
pub mod root_detect;
pub mod scan;

pub use fs::{DirEntry, FileSystem};
pub use model::{ProjectState, RootCandidate, RootResolution};
pub use scan::{collect_tex_files, is_ignored, is_tree_excluded};
pub use root_detect::{find_candidates, resolve};
