//! SyncTeX 接口与 CLI 输出解析（modules.md §5）。
//!
//! core 只定义接口与**纯解析函数**；进程调用在 src-tauri 的 `sync_cli` 实现。
//! 解析函数输出契约以 TeX Live synctex 1.5 输出为准（ADR-0008：需 Windows 实测）。

use super::model::{SourcePosition, SyncTexError, SyncTexPosition};
use async_trait::async_trait;
use std::path::Path;

/// 进程调用抽象：core 的 synctex 模块不碰进程，只依赖此接口。
#[async_trait]
pub trait SyncTexProvider: Send + Sync {
    /// 源码 → PDF：`synctex view -i "line:col:file" -o "<pdf>" -x`
    async fn forward(&self, src: &SourcePosition, pdf: &Path) -> Result<SyncTexPosition, SyncTexError>;
    /// PDF → 源码：`synctex edit -o "page:x:y:<pdf>"`
    async fn inverse(&self, pos: &SyncTexPosition, pdf: &Path) -> Result<SourcePosition, SyncTexError>;
}

/// 解析 `synctex view` 的 stdout（modules.md §5 算法）。
///
/// 期望行：`Page:<n>`、`x:<float>`、`y:<float>`；缺任一字段即 Parse 错误。
pub fn parse_forward_output(text: &str) -> Result<SyncTexPosition, SyncTexError> {
    let mut page = None;
    let mut x = None;
    let mut y = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("Page:") {
            page = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("x:") {
            x = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("y:") {
            y = v.trim().parse().ok();
        }
    }
    match (page, x, y) {
        (Some(page), Some(x), Some(y)) => Ok(SyncTexPosition { page, x, y }),
        _ => Err(SyncTexError::Parse(format!(
            "缺少 Page/x/y 字段（得到 {} 行）：{}",
            text.lines().count(),
            text.lines().next().unwrap_or("")
        ))),
    }
}

/// 解析 `synctex edit` 的 stdout。
///
/// 期望行：`Input:`（TeX Live 1.21 实测）或 `File:`（旧版本）为源文件，`Line:<n>`、`Column:<n>`。
/// ADR-0008 风险落地：2026-08 WSL 实测 synctex 1.21 输出 `Input:` 前缀。
pub fn parse_inverse_output(text: &str) -> Result<SourcePosition, SyncTexError> {
    let mut file = None;
    let mut line_no = None;
    let mut column = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("Input:") {
            file = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("File:") {
            file = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("Line:") {
            line_no = v.trim().parse().ok();
        } else if let Some(v) = line.strip_prefix("Column:") {
            column = v.trim().parse().ok();
        }
    }
    match (file, line_no, column) {
        (Some(file), Some(line), Some(column)) => Ok(SourcePosition {
            file: file.into(),
            line,
            column,
        }),
        _ => Err(SyncTexError::Parse(format!(
            "缺少 Input/File、Line、Column 字段（得到 {} 行）：{}",
            text.lines().count(),
            text.lines().next().unwrap_or("")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// synctex 1.5 `view -x` 的真实感输出（TeX Live 格式，ADR-0008 待实测）。
    const FORWARD_SAMPLE: &str = "This is SyncTeX command line utility, version 1.5\n\
        SyncTeX result begin\n\
        Output:/home/u/proj/main.pdf\n\
        Page:1\n\
        x:337.1231\n\
        y:720.9981\n\
        h:100.000000\n\
        v:50.000000\n\
        W:400.000000\n\
        H:300.000000\n\
        before:\\\\section{Intro}\n\
        after:\n\
        SyncTeX result end\n";

    #[test]
    fn parse_forward_sample() {
        let pos = parse_forward_output(FORWARD_SAMPLE).unwrap();
        assert_eq!(pos.page, 1);
        assert!((pos.x - 337.1231).abs() < 1e-4);
        assert!((pos.y - 720.9981).abs() < 1e-4);
    }

    #[test]
    fn parse_forward_missing_field_is_error() {
        let text = "Page:1\nx:10.0\n"; // 缺 y
        let err = parse_forward_output(text).unwrap_err();
        assert!(err.to_string().contains("解析失败"));
    }

    #[test]
    fn parse_forward_empty_is_error() {
        assert!(parse_forward_output("").is_err());
        assert!(parse_forward_output("SyncTeX result begin\nSyncTeX result end\n").is_err());
    }

    #[test]
    fn parse_forward_tolerates_extra_lines() {
        let text = "garbage\nPage:3\nnoise\nx:1.5\ny:2.5\n";
        let pos = parse_forward_output(text).unwrap();
        assert_eq!(pos.page, 3);
    }

    /// synctex `edit` 的真实感输出。
    const INVERSE_SAMPLE: &str = "This is SyncTeX command line utility, version 1.5\n\
        SyncTeX result begin\n\
        Output:main.pdf\n\
        File:main.tex\n\
        Line:12\n\
        Column:5\n\
        Offset:240\n\
        Context:\\\\section{Intro}\n\
        SyncTeX result end\n";

    #[test]
    fn parse_inverse_sample() {
        let pos = parse_inverse_output(INVERSE_SAMPLE).unwrap();
        assert_eq!(pos.file.to_string_lossy(), "main.tex");
        assert_eq!(pos.line, 12);
        assert_eq!(pos.column, 5);
    }

    #[test]
    fn parse_inverse_missing_field_is_error() {
        let err = parse_inverse_output("File:a.tex\nLine:1\n").unwrap_err();
        assert!(err.to_string().contains("Column"));
    }

    /// synctex 1.21 实测输出（2026-08 WSL）：`Input:` 前缀（ADR-0008 风险落地）。
    #[test]
    fn parse_inverse_input_prefix_from_real_cli() {
        let text = "This is SyncTeX command line utility, version 1.5\n\
            SyncTeX result begin\n\
            Output:main.pdf\n\
            Input:/home/u/proj/tmp/main.bbl\n\
            Line:8\n\
            Column:-1\n\
            Offset:0\n\
            Context:\n\
            SyncTeX result end\n";
        // 注：`\` 续行会连接行首空白，因此上面的字面量实际是每行前带缩进的单行文本；
        // 改用显式拼接验证多行解析：
        let text = [
            "This is SyncTeX command line utility, version 1.5",
            "SyncTeX result begin",
            "Output:main.pdf",
            "Input:/home/u/proj/tmp/main.bbl",
            "Line:8",
            "Column:-1",
            "Offset:0",
            "Context:",
            "SyncTeX result end",
        ]
        .join("\n");
        let pos = parse_inverse_output(&text).unwrap();
        assert_eq!(pos.file.to_string_lossy(), "/home/u/proj/tmp/main.bbl");
        assert_eq!(pos.line, 8);
    }

    #[test]
    fn parse_inverse_empty_is_error() {
        assert!(parse_inverse_output("").is_err());
    }
}
