//! texpresso-core：TeXPresso 领域核心。
//!
//! 纪律（ADR-0006）：本 crate **不得**依赖 tauri、不得做 IO；
//! 一切 IO 经 [`project::FileSystem`] 等 trait 注入。

pub mod project;
pub mod settings;
pub mod types;

#[cfg(test)]
mod testutil;
