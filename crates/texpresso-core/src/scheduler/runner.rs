//! `CompileRunner` 接口（modules.md §2.4）。
//!
//! 超时检测、进程树杀、PDF 拷贝全部在 runner 内（设计决策 D2）：
//! 调度器无时钟、无进程概念，单测只需喂假 `CompileOutcome`。

use crate::types::{CompileOutcome, CompileRequest};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

/// 编译执行抽象：core 唯一知道的"怎么编译"。
///
/// 契约：
/// - `cancel` 被取消后必须尽快终止（树杀）并返回 `Aborted`；
/// - 超时由 runner 自行检测（`req.timeout`），超时后树杀并返回 `Timeout`；
/// - 成功时 PDF 必须已拷贝到项目根（`Success.pdf_path`）。
#[async_trait]
pub trait CompileRunner: Send + Sync {
    async fn compile(&self, req: CompileRequest, cancel: CancellationToken) -> CompileOutcome;
}
