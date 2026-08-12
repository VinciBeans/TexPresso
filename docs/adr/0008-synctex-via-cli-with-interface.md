# SyncTeX 走 synctex CLI + 接口抽象

MVP 需双向 SyncTeX（design.md v1 必须），但 Rust 生态**无维护中的解析库**（crates.io 上唯一 synctex_sys 是休眠的 unsafe FFI 绑定）。采用：core 定义 `SyncTexProvider` 接口，src-tauri 实现为调用系统 **synctex CLI**（随 TeX Live/MiKTeX 分发）并解析其输出。风险：CLI 输出契约需 Windows 实测；接口抽象保证可替换为自研实现而不动上层。

- **状态**：已接受
- **备选方案**：自研 Rust 解析器（否决：格式官方明言"不应视为公开"，漂移风险；留作 CLI 出问题时的替换路径）；内嵌 C 解析器（TeXstudio 做法，否决：引入 C 工具链）；synctex_sys FFI（否决：休眠且 unsafe）
- **影响**：`-synctex=1` 进 latexmk 命令；CLI 指向 `tmp/` 下的 .synctex.gz；`synctex` 模块在 core、`sync_cli` 在 src-tauri
