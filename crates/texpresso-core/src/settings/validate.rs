//! 设置范围校验（modules.md §6）。

use super::model::{ProjectOverrides, Settings};

const TIMEOUT_SECS_RANGE: std::ops::RangeInclusive<u64> = 5..=600;
const DEBOUNCE_MS_RANGE: std::ops::RangeInclusive<u64> = 100..=2000;

/// 校验合并后的有效设置；返回全部违规项（空 = 通过）。
pub fn validate(s: &Settings) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    if !TIMEOUT_SECS_RANGE.contains(&s.compile.timeout_secs) {
        errs.push(format!(
            "compile.timeout_secs 必须在 {}..={} 之间（当前 {}）",
            TIMEOUT_SECS_RANGE.start(),
            TIMEOUT_SECS_RANGE.end(),
            s.compile.timeout_secs
        ));
    }
    if !DEBOUNCE_MS_RANGE.contains(&s.compile.debounce_ms) {
        errs.push(format!(
            "compile.debounce_ms 必须在 {}..={} 之间（当前 {}）",
            DEBOUNCE_MS_RANGE.start(),
            DEBOUNCE_MS_RANGE.end(),
            s.compile.debounce_ms
        ));
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

/// 校验项目覆盖（只查 Some 的键）。
pub fn validate_overrides(o: &ProjectOverrides) -> Result<(), Vec<String>> {
    let mut errs = Vec::new();
    if let Some(compile) = &o.compile {
        if let Some(v) = compile.timeout_secs {
            if !TIMEOUT_SECS_RANGE.contains(&v) {
                errs.push(format!(
                    "compile.timeout_secs 必须在 {}..={} 之间（当前 {}）",
                    TIMEOUT_SECS_RANGE.start(),
                    TIMEOUT_SECS_RANGE.end(),
                    v
                ));
            }
        }
        if let Some(v) = compile.debounce_ms {
            if !DEBOUNCE_MS_RANGE.contains(&v) {
                errs.push(format!(
                    "compile.debounce_ms 必须在 {}..={} 之间（当前 {}）",
                    DEBOUNCE_MS_RANGE.start(),
                    DEBOUNCE_MS_RANGE.end(),
                    v
                ));
            }
        }
    }
    if errs.is_empty() {
        Ok(())
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_passes() {
        assert!(validate(&Settings::default()).is_ok());
    }

    #[test]
    fn rejects_out_of_range_values() {
        let mut s = Settings::default();
        s.compile.timeout_secs = 9999;
        s.compile.debounce_ms = 1;
        let errs = validate(&s).unwrap_err();
        assert_eq!(errs.len(), 2); // 全部违规项都收集
        assert!(errs[0].contains("timeout_secs"));
        assert!(errs[1].contains("debounce_ms"));
    }

    #[test]
    fn boundary_values_accepted() {
        let mut s = Settings::default();
        s.compile.timeout_secs = 5;
        assert!(validate(&s).is_ok());
        s.compile.timeout_secs = 600;
        assert!(validate(&s).is_ok());
        s.compile.debounce_ms = 100;
        assert!(validate(&s).is_ok());
        s.compile.debounce_ms = 2000;
        assert!(validate(&s).is_ok());
    }

    #[test]
    fn validate_overrides_checks_some_only() {
        let o = ProjectOverrides {
            compile: Some(crate::settings::model::CompileOverrides {
                timeout_secs: Some(9999),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(validate_overrides(&o).is_err());

        let o2 = ProjectOverrides::default();
        assert!(validate_overrides(&o2).is_ok());
    }
}
