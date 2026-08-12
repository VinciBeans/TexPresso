//! .tex 文件收集与忽略规则（modules.md §3.3）。
//!
//! 忽略规则是**单一事实来源**：编译监视（watch）与文件收集共用同一函数。

use super::fs::FileSystem;
use std::path::{Path, PathBuf};

/// 相对项目根的组件中是否有 `tmp/` 或隐藏项（.git 等）。
/// 项目根**之前**的隐藏父目录不算（如 `~/.projects/foo` 是合法项目根）。
pub fn is_hidden_or_tmp(path: &Path, root: &Path) -> bool {
    let rel = path.strip_prefix(root).unwrap_or(path);
    for comp in rel.components() {
        let name = comp.as_os_str().to_str().unwrap_or("");
        if name == "tmp" || (name.starts_with('.') && name.len() > 1) {
            return true;
        }
    }
    false
}

/// 编译监视/收集忽略规则：tmp/、隐藏项、非 .tex（modules.md §3.3）。
/// 注意 `.texpresso/settings.json` 由 watch 在调用本函数**之前**单独分支处理。
pub fn is_ignored(path: &Path, root: &Path) -> bool {
    if is_hidden_or_tmp(path, root) {
        return true;
    }
    match path.extension().and_then(|e| e.to_str()) {
        Some("tex") => false,
        _ => true,
    }
}

/// 文件树展示忽略规则：只藏 tmp/ 与隐藏项（树需要展示所有扩展名）。
pub fn is_tree_excluded(path: &Path, root: &Path) -> bool {
    is_hidden_or_tmp(path, root)
}

/// 递归收集项目内全部 .tex（排除 tmp/ 与隐藏目录），不跟随符号链接（防环）。
///
/// 每次调用全量扫描、无缓存（modules.md §3.3：目录量大时再优化增量）。
pub async fn collect_tex_files(fs: &dyn FileSystem, root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs.read_dir(&dir).await? {
            let path = entry.path;
            if is_hidden_or_tmp(&path, root) {
                continue;
            }
            if entry.is_dir {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("tex") {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::FakeFS;
    use std::path::Path;

    fn root() -> &'static Path {
        Path::new("proj")
    }

    #[test]
    fn tmp_dir_ignored() {
        assert!(is_ignored(Path::new("proj/tmp/main.aux"), root()));
        assert!(is_ignored(Path::new("proj/tmp/main.tex"), root()));
        assert!(is_hidden_or_tmp(Path::new("proj/tmp/x"), root()));
    }

    #[test]
    fn hidden_entries_ignored() {
        assert!(is_ignored(Path::new("proj/.git/config"), root()));
        assert!(is_ignored(Path::new("proj/.texpresso/settings.json"), root()));
        assert!(is_hidden_or_tmp(Path::new("proj/.git"), root()));
    }

    #[test]
    fn non_tex_ignored_for_compile_but_not_for_tree() {
        assert!(is_ignored(Path::new("proj/notes.md"), root()));
        assert!(!is_tree_excluded(Path::new("proj/notes.md"), root()));
        assert!(!is_ignored(Path::new("proj/main.tex"), root()));
    }

    #[test]
    fn hidden_parent_before_root_not_ignored() {
        // 项目根本身在隐藏目录里是合法场景（~/.projects/foo）
        let root = Path::new("/home/u/.projects/foo");
        assert!(!is_hidden_or_tmp(&root.join("main.tex"), root));
        assert!(is_hidden_or_tmp(&root.join(".git"), root));
    }

    #[test]
    fn collect_tex_files_dfs_with_filters() {
        let mut fs = FakeFS::new();
        fs.put_file("proj/main.tex", "\\documentclass{article}");
        fs.put_file("proj/chapters/intro.tex", "intro");
        fs.put_file("proj/chapters/deep/extra.tex", "extra");
        fs.put_file("proj/tmp/main.aux", "aux"); // 排除
        fs.put_file("proj/tmp/main.tex", "tmp tex"); // 排除
        fs.put_file("proj/.git/HEAD", "ref"); // 排除
        fs.put_file("proj/notes.md", "md"); // 非 tex 排除

        let files = crate::testutil::block_on(collect_tex_files(&fs, Path::new("proj")));
        let files: Vec<String> = files
            .unwrap()
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            files,
            vec![
                "proj/chapters/deep/extra.tex",
                "proj/chapters/intro.tex",
                "proj/main.tex",
            ]
        );
    }

    #[test]
    fn collect_tex_files_missing_root_errors() {
        let fs = FakeFS::new();
        let result = crate::testutil::block_on(collect_tex_files(&fs, Path::new("nope")));
        assert!(result.is_err());
    }
}
