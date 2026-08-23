//! 命令面实现（modules.md §8）：DTO 进出、无业务逻辑。
//!
//! 安全底线（设计决策 D8）：路径类参数一律校验在项目根内
//! （自建命令没有 Tauri 权限模型兜底，必须自守）。

use crate::events::SettingsChangedEvent;
use crate::fs_impl::strip_verbatim;
use tauri_specta::Event;
use crate::storage::SettingsStorage;
use serde::Serialize;
use specta::Type;
use std::path::{Path, PathBuf};
use tauri::State;
use texpresso_core::compose::compile_request_manual;
use texpresso_core::project::{collect_tex_files, find_candidates, resolve, ProjectState, RootResolution};
use texpresso_core::settings::{apply_patch, validate_overrides, ProjectOverrides, Settings, SettingsPatch};
use texpresso_core::synctex::SourcePosition;
use texpresso_core::types::{
    DirEntryInfo, FileContent, ProjectInfo, SourcePositionDto, SyncTexTarget,
};
use texpresso_core::project::FileSystem;
use texpresso_core::scheduler::SchedulerHandle;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// 命令错误契约：{ code, message }（modules.md §4 错误模型）。
#[derive(Debug, Error, Serialize, Type)]
#[serde(tag = "code", content = "message")]
pub enum CmdError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Internal(String),
}

impl From<std::io::Error> for CmdError {
    fn from(e: std::io::Error) -> Self {
        CmdError::Internal(e.to_string())
    }
}

/// 应用状态（Tauri managed state，§1 全局状态清单）。
pub struct AppState {
    pub fs: Arc<dyn FileSystem>,
    pub scheduler: SchedulerHandle,
    pub sync: Arc<dyn texpresso_core::synctex::SyncTexProvider>,
    pub project: Arc<RwLock<Option<ProjectState>>>,
    pub settings: Arc<RwLock<Settings>>,
    pub overrides: Arc<RwLock<ProjectOverrides>>,
    pub storage: Arc<SettingsStorage>,
    pub watch: crate::watch::WatchHandle,
    pub app: tauri::AppHandle,
}

/// 路径校验（D8）：canonicalize 后必须落在项目根内。
async fn validate_in_project(state: &AppState, path: &Path) -> Result<PathBuf, CmdError> {
    let project = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?;
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| CmdError::NotFound(format!("路径不存在：{}", path.display())))?;
    let root = tokio::fs::canonicalize(&project.root)
        .await
        .map_err(|e| CmdError::Internal(format!("项目根不可访问：{e}")))?;
    if canonical.starts_with(&root) {
        Ok(canonical)
    } else {
        Err(CmdError::Invalid(format!(
            "路径在项目外：{}",
            path.display()
        )))
    }
}

// ---------------------------------------------------------------- 项目

#[tauri::command]
#[specta::specta]
pub async fn open_project(folder: String, state: State<'_, AppState>) -> Result<ProjectInfo, CmdError> {
    let folder = PathBuf::from(&folder);
    // 校验目录存在且可读
    let canonical = tokio::fs::canonicalize(&folder)
        .await
        .map_err(|_| CmdError::NotFound(format!("目录不存在：{}", folder.display())))?;
    if !canonical.is_dir() {
        return Err(CmdError::Invalid(format!("不是目录：{}", folder.display())));
    }

    // 项目设置：覆盖 + 合并（modules.md §6）
    let overrides = state.storage.load_overrides(state.fs.as_ref(), &canonical).await;
    let global = state.settings.read().await.clone();
    let settings = SettingsStorage::effective(&global, &overrides);
    *state.overrides.write().await = overrides;
    *state.settings.write().await = settings.clone();

    // 根文件：手动覆盖优先；否则探测（modules.md §5.4）
    let root_file = match settings.root_file.clone() {
        Some(override_path) => Some(strip_verbatim(&canonical.join(override_path))),
        None => {
            let files = collect_tex_files(state.fs.as_ref(), &canonical)
                .await
                .map_err(|e| CmdError::Internal(format!("扫描项目失败：{e}")))?;
            let resolution = resolve(find_candidates(&files, &canonical, |p| {
                // 同步读内容：探测是小文件、低频操作
                std::fs::read_to_string(p).ok()
            }));
            match resolution {
                RootResolution::Unique(p) => Some(strip_verbatim(&p)),
                RootResolution::Multiple(_) | RootResolution::None => None, // 前端弹窗/手动指定
            }
        }
    };

    let root = strip_verbatim(&canonical);
    let project = ProjectState {
        root: root.clone(),
        root_file,
    };
    *state.project.write().await = Some(project.clone());
    state.watch.set_project_root(Some(root.clone()));
    info!("打开项目：{}", root.display());

    Ok(ProjectInfo {
        root: project.root,
        root_file: project.root_file,
    })
}

#[tauri::command]
#[specta::specta]
pub async fn list_dir(path: String, state: State<'_, AppState>) -> Result<Vec<DirEntryInfo>, CmdError> {
    let path = validate_in_project(&state, Path::new(&path)).await?;
    let root = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?
        .root;
    // 递归收集（排除 tmp/ 与隐藏项；树需要所有扩展名）
    let mut out = Vec::new();
    let mut stack = vec![path];
    while let Some(dir) = stack.pop() {
        for entry in state.fs.read_dir(&dir).await? {
            if texpresso_core::project::is_tree_excluded(&entry.path, &root) {
                continue;
            }
            out.push(DirEntryInfo {
                name: entry
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: entry.path.clone(),
                is_dir: entry.is_dir,
            });
            if entry.is_dir {
                stack.push(entry.path);
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------- 文件

#[tauri::command]
#[specta::specta]
pub async fn read_file(path: String, state: State<'_, AppState>) -> Result<String, CmdError> {
    let path = validate_in_project(&state, Path::new(&path)).await?;
    state.fs.read_to_string(&path).await.map_err(CmdError::from)
}

#[tauri::command]
#[specta::specta]
pub async fn write_file(path: String, content: String, state: State<'_, AppState>) -> Result<(), CmdError> {
    save_content(&state, Path::new(&path), &content).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_file(path: String, content: String, state: State<'_, AppState>) -> Result<(), CmdError> {
    save_content(&state, Path::new(&path), &content).await
}

#[tauri::command]
#[specta::specta]
pub async fn save_all(files: Vec<FileContent>, state: State<'_, AppState>) -> Result<(), CmdError> {
    for f in &files {
        save_content(&state, Path::new(&f.path), &f.content).await?;
    }
    Ok(())
}

async fn save_content(state: &AppState, path: &Path, content: &str) -> Result<(), CmdError> {
    // 目标可能不存在：对父目录 canonicalize 后拼接
    let project = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?;
    let parent = path
        .parent()
        .ok_or_else(|| CmdError::Invalid("非法路径".into()))?;
    let canonical_parent = tokio::fs::canonicalize(parent)
        .await
        .map_err(|_| CmdError::NotFound(format!("目录不存在：{}", parent.display())))?;
    let root = tokio::fs::canonicalize(&project.root).await?;
    let target = canonical_parent.join(
        path.file_name()
            .ok_or_else(|| CmdError::Invalid("非法路径".into()))?,
    );
    if !target.starts_with(&root) {
        return Err(CmdError::Invalid(format!("路径在项目外：{}", path.display())));
    }
    tokio::fs::write(&target, content).await?;
    Ok(())
}

// ---------------------------------------------------------------- 编译

#[tauri::command]
#[specta::specta]
pub async fn compile_now(state: State<'_, AppState>) -> Result<(), CmdError> {
    let project = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?;
    let settings = state.settings.read().await.clone();
    let ctx = texpresso_core::compose::ComposeContext {
        project: &project,
        settings: &settings,
    };
    match compile_request_manual(ctx) {
        Some(req) => {
            debug!("手动编译: root={}", req.root_file.display());
            state.scheduler.compile(req);
            Ok(())
        }
        None => Err(CmdError::Invalid("未确定根文件，无法编译".into())),
    }
}

#[tauri::command]
#[specta::specta]
pub async fn abort_compile(state: State<'_, AppState>) -> Result<(), CmdError> {
    state.scheduler.abort();
    Ok(())
}

// ---------------------------------------------------------------- SyncTeX

fn pdf_path_for_root(project: &ProjectState) -> PathBuf {
    let stem = project
        .root_file
        .as_deref()
        .and_then(|f| f.file_stem())
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".into());
    project.root.join(format!("{stem}.pdf"))
}

#[tauri::command]
#[specta::specta]
pub async fn synctex_forward(
    file: String,
    line: u32,
    column: u32,
    state: State<'_, AppState>,
) -> Result<SyncTexTarget, CmdError> {
    let project = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?;
    let src = SourcePosition {
        file: PathBuf::from(&file),
        line,
        column: column as i32,
    };
    state
        .sync
        .forward(&src, &pdf_path_for_root(&project))
        .await
        .map(|p| SyncTexTarget {
            page: p.page,
            x: p.x,
            y: p.y,
        })
        .map_err(|e| CmdError::Internal(e.to_string()))
}

#[tauri::command]
#[specta::specta]
pub async fn synctex_inverse(
    page: u32,
    x: f32,
    y: f32,
    state: State<'_, AppState>,
) -> Result<SourcePositionDto, CmdError> {
    let project = state
        .project
        .read()
        .await
        .clone()
        .ok_or_else(|| CmdError::Invalid("尚未打开项目".into()))?;
    let pos = texpresso_core::synctex::SyncTexPosition { page, x, y };
    state
        .sync
        .inverse(&pos, &pdf_path_for_root(&project))
        .await
        .map(|p| SourcePositionDto {
            file: p.file.to_string_lossy().into_owned(),
            line: p.line,
            column: p.column,
        })
        .map_err(|e| CmdError::Internal(e.to_string()))
}

// ---------------------------------------------------------------- 设置

#[tauri::command]
#[specta::specta]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, CmdError> {
    Ok(state.settings.read().await.clone())
}

/// 局部更新（modules.md §6）：mode/debounce/timeout/engine → 全局文件；
/// root_file → 项目覆盖文件；随后重算有效设置并广播 settings-changed。
#[tauri::command]
#[specta::specta]
pub async fn update_settings(
    patch: SettingsPatch,
    state: State<'_, AppState>,
) -> Result<Settings, CmdError> {
    let project = state.project.read().await.clone();

    // 1. root_file 走项目覆盖
    if let Some(root_file) = &patch.root_file {
        let mut overrides = state.overrides.write().await.clone();
        overrides.root_file = root_file.clone();
        validate_overrides(&overrides)
            .map_err(|errs| CmdError::Invalid(errs.join("；")))?;
        if let Some(project) = &project {
            state.storage.save_overrides(&project.root, &overrides).await;
        }
        *state.overrides.write().await = overrides;
    }

    // 2. 其余字段走全局设置
    //    注意：必须读**纯全局**（磁盘 settings.json），而不能用 state.settings（合并后的
    //    effective）——否则 effective 里的项目覆盖值会被当作“全局”参与 merge 的继承语义，
    //    清除（root_file=null）后生效值仍是旧的（清不掉），正是“覆盖无法清除”的根因。
    let mut global = state.storage.load_global(state.fs.as_ref()).await;
    let mut patch_global = patch.clone();
    patch_global.root_file = None; // root_file 已在上一步处理
    if patch_global != SettingsPatch::default() {
        apply_patch(&mut global, &patch_global)
            .map_err(|errs| CmdError::Invalid(errs.join("；")))?;
        state.storage.save_global(&global).await;
    }

    // 3. 重算有效设置
    let overrides = state.overrides.read().await.clone();
    let effective = SettingsStorage::effective(&global, &overrides);
    *state.settings.write().await = effective.clone();

    let _ = SettingsChangedEvent(effective.clone()).emit(&state.app);
    debug!("设置已更新：{:?}", effective.compile);
    Ok(effective)
}
