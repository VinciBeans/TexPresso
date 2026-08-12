# 根文件探测用正则启发式，不做完整 TeX 解析

根文件探测（含 `\documentclass` 且不被 `\input`/`\include` 引用）v1 用正则匹配（`\\(input|include)\{...\}`、`\\documentclass[...]{...}`），而非完整 TeX 词法解析。已知局限：注释中的引用会误报、`\includeonly` 未处理、编码假定 UTF-8。手动覆盖（项目 settings.json 的 `root_file`）是逃生门。

- **状态**：已接受
- **备选方案**：完整 TeX 词法解析（否决：v1 成本收益不成比例，探测失败有逃生门；若误报在实践中成为问题，升级路径是替换 root_detect 内部实现，接口不变）
- **影响**：`root_detect.rs` 是纯函数组合（extract_includes → find_candidates → resolve），可单测；局限写入代码注释与测试用例
