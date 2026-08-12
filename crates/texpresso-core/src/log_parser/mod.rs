//! 日志解析（modules.md §4）。

pub mod model;
pub mod scan;

pub use model::{LogMessage, MessageKind};
pub use scan::parse_log;
