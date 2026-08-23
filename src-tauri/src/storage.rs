//! 设置存储（modules.md §6）：全局 settings.json + 项目 .texpresso/settings.json。
//!
//! - 原子写：临时文件 + rename（防崩溃截断）；
//! - 自写盘过滤：写入时记录内容 hash，watch 事件比对后消费（设计决策 D6）。

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use texpresso_core::project::FileSystem;
use texpresso_core::settings::{merge, ProjectOverrides, Settings};
use tracing::error;

pub struct SettingsStorage {
    /// 全局设置文件路径（app_config_dir/settings.json）。
    global_path: PathBuf,
    /// 最近一次写入的内容 hash（自写盘过滤，D6）。
    last_write: Mutex<HashMap<PathBuf, u64>>,
}

impl SettingsStorage {
    pub fn new(global_path: PathBuf) -> Self {
        Self {
            global_path,
            last_write: Mutex::new(HashMap::new()),
        }
    }

    pub fn global_path(&self) -> &Path {
        &self.global_path
    }

    fn hash(content: &str) -> u64 {
        let mut h = DefaultHasher::new();
        content.hash(&mut h);
        h.finish()
    }

    /// 读取全局设置；缺失 → 默认值并落盘；损坏 → 默认值（错误记录，不阻塞启动）。
    pub async fn load_global(&self, fs: &dyn FileSystem) -> Settings {
        match fs.read_to_string(&self.global_path).await {
            Ok(text) => match serde_json::from_str::<Settings>(&text) {
                Ok(s) => s,
                Err(e) => {
                    error!("全局设置损坏，使用默认值：{e}");
                    let d = Settings::default();
                    self.save_global(&d).await;
                    d
                }
            },
            Err(_) => {
                let d = Settings::default();
                self.save_global(&d).await;
                d
            }
        }
    }

    /// 原子写全局设置，并记录内容 hash 供自写盘过滤。
    pub async fn save_global(&self, s: &Settings) {
        let text = match serde_json::to_string_pretty(s) {
            Ok(t) => t,
            Err(e) => {
                error!("全局设置序列化失败：{e}");
                return;
            }
        };
        let hash = Self::hash(&text);
        if let Err(e) = atomic_write(&self.global_path, &text).await {
            error!("全局设置写盘失败：{e}");
            return;
        }
        self.last_write
            .lock()
            .unwrap()
            .insert(self.global_path.clone(), hash);
    }

    /// 读取项目覆盖；缺失 → 空覆盖（全继承全局）。
    pub async fn load_overrides(&self, fs: &dyn FileSystem, project_root: &Path) -> ProjectOverrides {
        let path = project_overrides_path(project_root);
        match fs.read_to_string(&path).await {
            Ok(text) => serde_json::from_str::<ProjectOverrides>(&text).unwrap_or_else(|e| {
                error!("项目设置损坏，忽略覆盖：{e}");
                ProjectOverrides::default()
            }),
            Err(_) => ProjectOverrides::default(),
        }
    }

    /// 原子写项目覆盖 + 记录 hash。
    pub async fn save_overrides(&self, project_root: &Path, o: &ProjectOverrides) {
        let path = project_overrides_path(project_root);
        let text = match serde_json::to_string_pretty(o) {
            Ok(t) => t,
            Err(e) => {
                error!("项目设置序列化失败：{e}");
                return;
            }
        };
        let hash = Self::hash(&text);
        if let Err(e) = atomic_write(&path, &text).await {
            error!("项目设置写盘失败：{e}");
            return;
        }
        self.last_write.lock().unwrap().insert(path, hash);
    }

    /// 自写盘过滤（D6）：watch 事件到达时，若内容 hash 与上次写入一致则消费并跳过。
    pub fn is_self_write(&self, path: &Path, content: &str) -> bool {
        let mut map = self.last_write.lock().unwrap();
        let expected = map.remove(path);
        match expected {
            Some(h) => h == Self::hash(content),
            None => false,
        }
    }

    /// 计算有效设置：全局 + 项目覆盖（modules.md §6 merge）。
    pub fn effective(global: &Settings, overrides: &ProjectOverrides) -> Settings {
        merge(global, overrides)
    }
}

/// 项目覆盖文件路径（.texpresso/settings.json）。
pub fn project_overrides_path(project_root: &Path) -> PathBuf {
    project_root.join(".texpresso").join("settings.json")
}

/// 原子写：临时文件 + rename（防崩溃截断；modules.md §6）。
/// 写入属于 src-tauri 基础设施（FileSystem trait 只有读接口，core 纯度不受影响）。
async fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    // 父目录可能不存在（如项目覆盖 .texpresso/）：先创建，否则 write 直接失败，
    // 覆盖只留在内存（重启即丢），表现为“设置/清除不持久”。
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let tmp = path.with_extension("json.tmp");
    tokio::fs::write(&tmp, content).await?;
    tokio::fs::rename(&tmp, path).await?;
    Ok(())
}
