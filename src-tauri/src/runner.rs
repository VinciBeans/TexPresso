//! Latexmk 执行器（modules.md §2.6 / 设计决策 D2）。
//!
//! 超时检测、进程树杀、PDF 拷贝全在这里；调度器无时钟、无进程概念。

use async_trait::async_trait;
use std::path::Path;
use texpresso_core::log_parser::parse_log;
use texpresso_core::project::FileSystem;
use texpresso_core::scheduler::CompileRunner;
use texpresso_core::types::{CompileOutcome, CompileRequest, ErrorEntry, ErrorKind};
use tokio_util::sync::CancellationToken;
use tracing::warn;

/// 编译中间目录（design.md：统一收纳 tmp/，与 core 忽略规则一致）。
const OUT_DIR: &str = "tmp";

pub struct LatexmkRunner {
    pub fs: std::sync::Arc<dyn FileSystem>,
}

fn root_stem(root_file: &Path) -> String {
    root_file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "main".to_string())
}

#[async_trait]
impl CompileRunner for LatexmkRunner {
    async fn compile(&self, req: CompileRequest, cancel: CancellationToken) -> CompileOutcome {
        let stem = root_stem(&req.root_file);
        let tmp_dir = req.project_root.join(OUT_DIR);
        let pdf_dst = req.project_root.join(format!("{stem}.pdf"));

        // 命令构造（modules.md §2.6 算法）：cwd = 项目根，相对 input/include 才能解析
        let mut cmd = tokio::process::Command::new("latexmk");
        cmd.arg(req.engine.latexmk_flag())
            .arg(format!("-outdir={OUT_DIR}"))
            .arg("-synctex=1")
            .arg("-interaction=nonstopmode")
            .arg(format!("{stem}.tex"))
            .current_dir(&req.project_root)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return CompileOutcome::IoError {
                    message: format!("无法启动 latexmk（TeX Live 未安装？）：{e}"),
                }
            }
        };
        let pid = child.id();

        let outcome = tokio::select! {
            _ = tokio::time::sleep(req.timeout) => {
                if let Some(pid) = pid {
                    warn!(pid, "编译超时（{}s），树杀进程", req.timeout.as_secs());
                    kill_tree(pid);
                }
                CompileOutcome::Timeout
            }
            _ = cancel.cancelled() => {
                if let Some(pid) = pid {
                    warn!(pid, "收到手动终止，树杀进程");
                    kill_tree(pid);
                }
                CompileOutcome::Aborted
            }
            status = child.wait() => match status {
                Ok(s) if s.success() => {
                    // 成功：tmp/<stem>.pdf → 项目根（design.md 产物位置）
                    let pdf_src = tmp_dir.join(format!("{stem}.pdf"));
                    match tokio::fs::copy(&pdf_src, &pdf_dst).await {
                        Ok(_) => CompileOutcome::Success { pdf_path: pdf_dst },
                        Err(e) => CompileOutcome::IoError {
                            message: format!("PDF 拷贝失败（{} → {}）：{e}", pdf_src.display(), pdf_dst.display()),
                        },
                    }
                }
                Ok(_) => {
                    // 非零退出：.log 是权威（modules.md §4：不做流式输出）
                    let log_path = tmp_dir.join(format!("{stem}.log"));
                    match self.fs.read_to_string(&log_path).await {
                        Ok(text) => CompileOutcome::ContentError {
                            errors: parse_log(&text)
                                .into_iter()
                                .map(|m| ErrorEntry {
                                    message: m.message,
                                    file: m.file,
                                    line: m.line,
                                    kind: ErrorKind::ContentError,
                                })
                                .collect(),
                        },
                        Err(e) => CompileOutcome::IoError {
                            message: format!("编译失败且无法读取日志（{}）：{e}", log_path.display()),
                        },
                    }
                }
                Err(e) => CompileOutcome::IoError {
                    message: format!("等待 latexmk 失败：{e}"),
                },
            },
        };
        outcome
    }
}

/// 进程树杀（modules.md §2.6）：
/// Windows：`taskkill /T /F /PID`（Child::kill 只杀直接子进程，latexmk 的子进程会存活）。
/// Unix：先试进程组（-pid），失败再试直接 kill（best-effort，与 LaTeX Workshop 相同的已知局限）。
fn kill_tree(pid: u32) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let group = std::process::Command::new("kill")
            .args(["-9", &format!("-{pid}")])
            .status();
        if group.map(|s| !s.success()).unwrap_or(true) {
            let _ = std::process::Command::new("kill").arg("-9").arg(pid.to_string()).status();
        }
    }
}
