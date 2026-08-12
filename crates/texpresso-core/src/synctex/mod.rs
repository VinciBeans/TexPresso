//! SyncTeX（modules.md §5 / ADR-0008：走 synctex CLI + 接口抽象）。

pub mod model;
pub mod provider;

pub use model::{SourcePosition, SyncTexError, SyncTexPosition};
pub use provider::{parse_forward_output, parse_inverse_output, SyncTexProvider};
