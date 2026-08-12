//! 文件监视与触发组合（modules.md §7）。
//!
//! - notify 事件 → 规范化 → 分类（.tex / settings.json / 其余）；
//! - .tex 变化 → 组合层翻译成 CompileRequest → 调度器（设计决策 D3）；
//! - 旁路广播 files-changed（前端文件树防抖重建）。

use crate::events::{FilesChangedEvent, SettingsChangedEvent};
use crate::storage::SettingsStorage;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use texpresso_core::compose::{compile_request_for_change, ComposeContext};
use texpresso_core::project::{is_ignored, is_tree_excluded, ProjectState};
use texpresso_core::scheduler::SchedulerHandle;
use texpresso_core::settings::Settings;
use texpresso_core::types::FilesChanged; // 保留：broadcast_files_changed 用
use tauri_specta::Event;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

/// 监视任务命令（模块间通信只经此通道，信息局部性）。
pub enum WatchCommand {
    /// 切换项目根（打开项目时调用；None = 关闭）。
    SetProjectRoot(Option<PathBuf>),
}

/// 外部唯一入口：只暴露"切换项目根"意图。
#[derive(Clone)]
pub struct WatchHandle {
    tx: mpsc::UnboundedSender<WatchCommand>,
}

impl WatchHandle {
    pub fn set_project_root(&self, root: Option<PathBuf>) {
        let _ = self.tx.send(WatchCommand::SetProjectRoot(root));
    }
}

pub struct WatchState {
    pub project: Arc<RwLock<Option<ProjectState>>>,
    pub settings: Arc<RwLock<Settings>>,
    pub scheduler: SchedulerHandle,
    pub storage: Arc<SettingsStorage>,
    /// 项目覆盖（update_settings 的 root_file 写这里）。
    pub overrides: Arc<RwLock<texpresso_core::settings::ProjectOverrides>>,
    pub app: tauri::AppHandle,
}

/// 启动监视任务：维护 notify watcher，把事件路由到分类处理。
pub fn spawn_watcher(
    global_settings_dir: PathBuf,
    state: Arc<WatchState>,
) -> WatchHandle {
    let (tx, mut rx) = mpsc::unbounded_channel::<WatchCommand>();

    std::thread::spawn(move || {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher};

        let (event_tx, event_rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher: RecommendedWatcher =
            match notify::recommended_watcher(move |res| {
                let _ = event_tx.send(res);
            }) {
                Ok(w) => w,
                Err(e) => {
                    warn!("notify 初始化失败，文件监视不可用：{e}");
                    return;
                }
            };

        // 固定监视全局设置目录（热更新，modules.md §6）
        if let Err(e) = watcher.watch(&global_settings_dir, RecursiveMode::NonRecursive) {
            warn!("监视全局设置目录失败：{e}");
        }

        let mut project_root: Option<PathBuf> = None;

        loop {
            // 非阻塞消费命令
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    WatchCommand::SetProjectRoot(Some(root)) => {
                        if let Some(old) = &project_root {
                            let _ = watcher.unwatch(old);
                        }
                        match watcher.watch(&root, RecursiveMode::Recursive) {
                            Ok(()) => {
                                project_root = Some(root);
                                debug!("开始监视项目：{}", project_root.as_ref().unwrap().display());
                            }
                            Err(e) => warn!("监视项目失败（{}）：{e}", root.display()),
                        }
                    }
                    WatchCommand::SetProjectRoot(None) => {
                        if let Some(old) = project_root.take() {
                            let _ = watcher.unwatch(&old);
                        }
                    }
                }
            }

            // 阻塞等事件
            match event_rx.recv() {
                Ok(Ok(event)) => handle_event(&event, state.clone(), project_root.as_deref()),
                Ok(Err(e)) => warn!("notify 事件错误：{e}"),
                Err(_) => break, // 通道关闭
            }
        }
    });

    WatchHandle { tx }
}

/// 事件分类与路由（modules.md §7 过滤规则）。
fn handle_event(event: &notify::Event, state: Arc<WatchState>, project_root: Option<&Path>) {
    let paths = normalize_event_paths(event);
    for path in paths {
        // 1. 设置文件（全局或项目）→ 热更新
        if is_settings_path(&path, &state, project_root) {
            handle_settings_change(&path, state.clone());
            continue;
        }
        // 2. .tex（排除 tmp/、隐藏项）→ 编译触发 + 广播
        let Some(root) = project_root else { continue };
        if is_ignored(&path, root) {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) == Some("tex") {
            trigger_compile(&path, state.clone());
        }
        // 3. 其余（含非 .tex、目录变化）→ 文件树刷新广播
        if !is_tree_excluded(&path, root) {
            broadcast_files_changed(&[path], &state);
        }
    }
}

/// rename 事件同时产出 from/to；这里都视为"变化"。
fn normalize_event_paths(event: &notify::Event) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let notify::EventKind::Modify(notify::event::ModifyKind::Name(
        notify::event::RenameMode::Both,
    )) = event.kind
    {
        // from/to 成对出现，取 to（新路径）
        if let Some(to) = event.paths.last() {
            out.push(to.clone());
        }
        return out;
    }
    out.extend(event.paths.iter().cloned());
    out
}

fn is_settings_path(path: &Path, state: &WatchState, project_root: Option<&Path>) -> bool {
    let file_name = path.file_name().and_then(|n| n.to_str());
    if file_name != Some("settings.json") {
        return false;
    }
    if path == state.storage.global_path() {
        return true;
    }
    if let Some(root) = project_root {
        if path == crate::storage::project_overrides_path(root) {
            return true;
        }
    }
    false
}

/// 设置热更新（D6）：自写盘 hash 过滤 → 重载 → 广播 settings-changed。
fn handle_settings_change(path: &Path, state: Arc<WatchState>) {
    let path = path.to_path_buf();
    let rt = tauri::async_runtime::handle();
    rt.spawn(async move {
        let is_global = path == state.storage.global_path();
        let content = match tokio::fs::read_to_string(&path).await {
            Ok(c) => c,
            Err(_) => return,
        };
        if state.storage.is_self_write(&path, &content) {
            debug!("设置自写盘，跳过重载：{}", path.display());
            return;
        }
        if is_global {
            if let Ok(parsed) = serde_json::from_str::<Settings>(&content) {
                let overrides = state.overrides.read().await.clone();
                *state.settings.write().await = SettingsStorage::effective(&parsed, &overrides);
            } else {
                warn!("全局设置解析失败，忽略外部修改：{}", path.display());
                return;
            }
        } else {
            if let Ok(parsed) = serde_json::from_str::<texpresso_core::settings::ProjectOverrides>(&content) {
                *state.overrides.write().await = parsed;
                let merged = SettingsStorage::effective(&state.settings.read().await.clone(), &state.overrides.read().await.clone());
                *state.settings.write().await = merged;
            } else {
                warn!("项目设置解析失败，忽略外部修改：{}", path.display());
                return;
            }
        }
        let s = state.settings.read().await.clone();
        let _ = SettingsChangedEvent(s).emit(&state.app);
    });
}

/// 组合层翻译（D3）：.tex 变化 → CompileRequest → 调度器。
fn trigger_compile(path: &Path, state: Arc<WatchState>) {
    let path = path.to_path_buf();
    let rt = tauri::async_runtime::handle();
    rt.spawn(async move {
        let project = state.project.read().await.clone();
        let Some(project) = project else { return };
        let settings = state.settings.read().await.clone();
        let ctx = ComposeContext {
            project: &project,
            settings: &settings,
        };
        match compile_request_for_change(ctx, &path) {
            Some(req) => state.scheduler.compile(req),
            None => debug!("变化不触发编译：{}", path.display()),
        }
    });
}

fn broadcast_files_changed(paths: &[PathBuf], state: &WatchState) {
    let paths: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let _ = FilesChangedEvent(FilesChanged { paths }).emit(&state.app);
}
