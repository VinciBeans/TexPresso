# SyncTeX 走 synctex CLI + 接口抽象

MVP 需双向 SyncTeX（design.md v1 必须），但 Rust 生态**无维护中的解析库**（crates.io 上唯一 synctex_sys 是休眠的 unsafe FFI 绑定）。采用：core 定义 `SyncTexProvider` 接口，src-tauri 实现为调用系统 **synctex CLI**（随 TeX Live/MiKTeX 分发）并解析其输出。风险：CLI 输出契约需 Windows 实测；接口抽象保证可替换为自研实现而不动上层。

- **状态**：已接受
- **备选方案**：自研 Rust 解析器（否决：格式官方明言"不应视为公开"，漂移风险；留作 CLI 出问题时的替换路径）；内嵌 C 解析器（TeXstudio 做法，否决：引入 C 工具链）；synctex_sys FFI（否决：休眠且 unsafe）
- **影响**：`-synctex=1` 进 latexmk 命令；CLI 指向 `tmp/` 下的 .synctex.gz；`synctex` 模块在 core、`sync_cli` 在 src-tauri
- **实测落地（2026-08，WSL TeX Live synctex 1.21）**：① synctex 数据在 `tmp/` 而 PDF 在项目根，必须传 `-d <tmp 目录>` 参数，否则报 "No SyncTeX available"；② `edit` 输出源文件行前缀是 `Input:`（非 `File:`），解析器两者兼容；③ `Column` 可为 `-1`（未知列），类型用 i32；④ 坐标从页面左上起算，与 pdf.js 右下起算需翻转
- **Windows 实测定稿（2026-08-25，本机 TeX Live 2026 / synctex 1.5）**：
  - `view -i "line:col:path" -o <pdf> -d <tmp> -x`：可能返回**多个** `Output:` 块（一个源位置命中多个盒子，如数学箱体）。每块含 `Page:`/`x:`/`y:`/`h:`/`v:`/`W:`/`H:`/`before:`/`middle:`/`after:`/`offset:-1`。**解析器取第一个完整块**（主结果）；`-i` 的 path 传 `E:\...`、`E:/...`、`E:/..././...` 均能匹配。
  - `edit -o "page:x:y:pdf" -d <tmp>`：输出 `Input:<源文件>`（**正斜杠 + `./`**，如 `E:/Works/tex-presso/test_file/projects/multifile/./main.tex`）、`Line:<n>`、`Column:-1`、`Offset:0`、`Context:`。命中点可能指向生成文件（`.toc`/`.aux`，如点击目录区 → `main.toc`）。`Input` 路径需归一化（剥 `./`、统一正斜杠）才能与已打开的标签匹配——见 `project.resolvePath` 修复。
  - 定格：`parse_forward_output` 取首个完整块；`parse_inverse_output` 兼容 `Input:`/`File:`、i32 的 `-1` 列。两者均以真实 Windows 输出固化为单测（`provider.rs` synctex 模块）。
