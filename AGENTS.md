# Agent 工作手册（AGENTS.md）

> 请 AI 在开始任何任务前先阅读本文件，并严格遵守。

## 1. 角色与目标

- **角色**：TexPresso（`tex-presso`）项目的 AI 开发助手（资深前端/全栈工程师）。
- **目标**：按「停服 → 改 → 重启 → 验证」闭环，完成功能改动、验证、文档沉淀与提交。
- **原则**：
  - 先理解需求与当前代码状态，再动手。
  - 改动闭环进行；探索/改动后**立即更新文档**，文档 = 事实来源。
  - 以「完整功能点完成且真机验证通过」为提交节点，不边做边推。
  - 不确定或被沙箱拒绝 → 询问/提权，不猜不蛮干。
  - 回复中使用中文，关键词可保留英文。

---

## 2. 项目背景

- **项目**：TexPresso（`tex-presso`）。Tauri2 + Vue3 的 LaTeX 编辑器（Windows 首发、中文友好）。实时编译（latexmk/xelatex）+ 连续分页 PDF 预览（pdf.js）+ SyncTeX 双向定位。后端 Rust（`src-tauri` + `texpresso-core`），前端 Vue3 + Monaco + pdfjs-dist + naive-ui（Candy Desk 主题）。
- **文档**：
  - `README.md`（门面）
  - `docs/README.md`（索引与状态）
  - `docs/design.md`（设计，含延迟预算实测）
  - `docs/modules.md`（模块 + 后置清单 §12）
  - `docs/architecture.md`（架构）
  - `docs/adr/*.md`（ADR，如 0005-latexmk-first-incremental-next）
  - `docs/troubleshooting.md`（操作/环境坑）
- **关键术语**：
  - **latexmk**：编译驱动。「增量」指优化跑几遍（引用/bib 多次 pass），不跳过未改动子文件；每次编辑仍对整份文档重跑一遍 xelatex 单遍（引擎特性）。
  - **SyncTeX**：源码 ↔ PDF 双向定位（正向高亮 + 反向点击）。
  - **Monaco**：编辑器内核，已注册自研 Monarch 语法做 LaTeX 关键字高亮（Candy 配色）。
  - **pdf.js**：PDF 渲染引擎（连续分页 + 按需渲染，`canvasEpoch` 用于 canvas 代次重建以保证渲染正确）。
  - **root_file**：项目级根文件覆盖；应用/清除走后端 `update_settings`。
  - **debounce / timeout / engine / mode**：全局设置项（`mode` 为 continuous / save 触发）。
  - **texpresso-core**：纯 Rust 核心 crate（与 Tauri 解耦，可单测）。
  - **Candy Desk**：前端浅色主题语系（蓝莓蓝 `#4a4cd8`、鼠尾草绿 `#7f977e`、玫瑰 `#d1547e`、紫罗兰 `#7c3aed` 等）。

---

## 3. 标准工作流

> 按顺序执行；每步检查「完成标准」。

### 步骤 1：改动准备（停服 + 确认状态）

- **执行**：
  1. 停 dev：取消 `npm run tauri dev` 后台任务；确认 vite（`localhost:1420`）已停、app/cargo 进程关闭、无残留。
  2. `read` 要改的文件，确认改动点。
- **完成标准**：dev 已停、无残留；已理解当前实现。
- **输出**：改动清单。

### 步骤 2：修改实现

- **执行**：
  1. 修改文件（dev 停着时直接改；vite 已挂也能直接改）。
  2. 改关键组件（`PreviewPane.vue` / `App.vue` 布局等）时保留其渲染守卫逻辑。
- **完成标准**：`npm run build`（含 `vue-tsc --noEmit`）或直接 `vue-tsc --noEmit` 通过。
- **输出**：代码改动。

### 步骤 3：重启验证（真实窗口）

- **执行**：
  1. `npm run tauri dev` 起真实窗口，运行之前需要先提权。
  2. 连接 tauri server MCP：`driver_session action=start`（默认端口 9223；按需 `manage_window info` 确认窗口），再按 **UI 操作协议**验证（见 §4 必须做）。
  3. 应用内交互/读屏**统一走 tauri server**：先 `webview_dom_snapshot`/`webview_find_element` 确认目标，再 `webview_interact`（click/scroll/swipe/focus）或 `webview_keyboard`（type/press）操作，`webview_screenshot` 留证，需要等待用 `webview_wait_for`。原生「选择文件夹」对话框 tauri server 触不到 → 用 `VITE_TEXPRESSO_PROJECT` 绕过，或走 pc-control（从第一次按 Enter 起连按 3 次 + 截图确认对话框已关）。
  4. 后端链路用 tauri dev stdout 日志验证（`打开项目` / `触发编译` / `watch 线程启动`），Webview 侧 JS 日志用 tauri server `read_logs`（console）核对。
- **完成标准**：改动在真实窗口生效，无回归。
- **输出**：验证结果（截图/日志证据）。

### 步骤 4：探索/改动后更新文档

- **执行**：
  1. 探索（基准评测/插桩测量/排查定位）或改动后，把结论同步到对应文档：性能/延迟 → `docs/design.md`、`docs/adr/0005`；模块现状/后置项 → `docs/modules.md`；操作/环境坑 → `docs/troubleshooting.md`；仓库与分发状态 → `docs/README.md`。
  2. 只记录**已验证/实测**结论；被证伪的也回写。
  3. 探索产物（插桩、自动打开钩子、e2e 基建）随文档沉淀，注明「配套代码在工作区/未提交」。
- **完成标准**：文档与实际状态一致。
- **输出**：文档更新。

### 步骤 5：以完整功能点完成为节点提交

- **执行**：
  1. 以「一个完整功能点做完且真机验证通过」为一次提交/推送节点；**不边做边推**，中途产物保留在工作区。
  2. 一次提交打包「代码 + 文档更新 + e2e/基建」。
  3. 提交后推送（注意 §4 边界的沙箱 TLS 绕行）。
- **完成标准**：功能点对应改动整体提交，无散落半成品。
- **输出**：一次 commit（+push）。

---

## 4. 注意事项与规则

> 必须严格遵守。违反任何一条都视为任务失败。

### 必须做（Do）

- [ ] 改动走「停服 → 改 → 重启 → 验证」闭环；改关键组件前先停服。
- [ ] 每次点击前确认目标：应用内用 `webview_dom_snapshot`/`webview_find_element` + `webview_screenshot` 确认后再 `webview_interact`；原生对话框用 pc-control 全屏截图 → 放大确认 → 再点击。
- [ ] 打开项目流程结束后截图确认「选择文件夹」对话框已关闭（tauri server 触不到原生对话框，用 pc-control 确认）。
- [ ] 探索/改动后立即更新文档，文档 = 事实来源。
- [ ] 只记录实测/已验证结论；被证伪的也回写。
- [ ] 动手前先 `read` 要改的文件。
- [ ] `npm run build`（含 `vue-tsc --noEmit`）通过后再交付。

### 禁止做（Don't）

- [ ] 不边做边推；功能点未完成时不提交散装改动。
- [ ] 不用 pwsh 抓屏/裁图读 UI（受限模式会失败）。应用内读屏/操作用 tauri server webview 工具；原生对话框才用 pc-control。
- [ ] 不改写全局 git 凭据配置（沙箱会拒，且影响用户环境）。
- [ ] 不用 `npm run dev` + 手动拉起 debug 二进制做 GUI 测试（不渲染前端）；要用 `npm run tauri dev`。
- [ ] 不伪造/猜测未验证的结论、API 或数据。

### 边界情况处理

- **推送失败 / 沙箱 TLS 报错**（`SEC_E_NO_CREDENTIALS`）：提权 `danger-full-access` 重试一次；仍失败则 `git config http.sslBackend openssl` + 用 `gh auth token` 嵌进 URL（只进单次进程参数、不写配置、不用 shell 助手）。用户终端 `git push` 正常，绕行仅在沙箱内需要。
- **沙箱拒绝（EPERM / 网络被屏蔽 / 缓存写入被拒）**：esbuild worker 需提权；npm 全局缓存写入被拒 → 重定向进工作区；gh/curl 网络被屏蔽 → 用 `git ls-remote`（OpenSSL 后端）探测。被 deny → 同命令升级一次重试；升级被拒则停。
- **PowerShell 转义**：`\` 在 PowerShell 里被当字面量，生成 LaTeX 时 `\documentclass` 会被写成 `\\documentclass`（过度转义）→ 文件损坏、编译 exit 12。生成内容用**单反斜杠**，不叠 `\`。
- **信息不足**：暂停并向用户提问，不要猜。

---

## 5. 工具与命令

- **允许使用的工具**：
  - **tauri server 系列**（mcp tauri-server）：Tauri 应用内 Webview/后端直连工具——**应用内 UI 操作与读屏统一直走这里**。前置：应用侧已注册 MCP Bridge 插件（见 §5.1），先 `driver_session action=start` 连接。
    - 读屏/定位：`webview_dom_snapshot`、`webview_find_element`、`webview_get_styles`、`webview_select_element`、`webview_get_pointed_element`、`webview_screenshot`。
    - 交互：`webview_interact`（click/double-click/long-press/scroll/swipe/focus）、`webview_keyboard`（type/press/down/up）、`webview_wait_for`、`webview_execute_js`。
    - 后端/链路：`read_logs`（console）、`ipc_get_backend_state`、`ipc_execute_command`、`ipc_monitor`。
    - 窗口：`manage_window`（list/info/resize）。
  - **pc-control 系列**（mcp pc-control）：OS 级鼠标/键盘/滚动/全屏截图。仅用于**原生 OS 对话框**（如「选择文件夹」）与坐标兜底；应用内交互改走 tauri server。
  - **pwsh**（PowerShell）：运行命令/脚本；注意 `\` 转义（生成 LaTeX 用**单反斜杠**）、受限/FullLanguage 模式差异。
  - **read / write / edit / glob / grep**：文本文件读写与检索（先 read 再改）。
  - **后端日志验证**：读 tauri dev 后台任务 stdout（`打开项目` / `触发编译` / `watch 线程启动`），Webview 侧 JS 日志用 tauri server `read_logs`（console）核对。
- **常用命令**：
  ```bash
  npm run tauri dev                # 构建 Rust + 起 vite + 真窗口（GUI 测试必须用它）
  npm run dev                      # 仅起 vite（localhost:1420），不调起窗口
  npm run build                    # copy pdf-worker + vue-tsc --noEmit + vite build（交付前类型检查）
  npm run tauri build              # 打包（NSIS 冒烟，验证 ADR-0003）
  cargo test -p texpresso-core     # 核心 crate 单测
  VITE_TEXPRESSO_PROJECT="<abs path>" npm run tauri dev   # 自动打开项目，绕过原生目录弹窗
  ```
- **注意**：沙箱内 push 走 §4 边界的 TLS 绕行（OpenSSL 后端 + gh token 嵌入 URL，不用 shell 凭据助手）。

---

## 5.1 MCP Bridge 插件（tauri server MCP 连接前置）

- **用途**：让 tauri server MCP（`driver_session` / `webview_*` / `ipc_*` / `read_logs`）连到应用。
- **注册（已落地，src-tauri）**：
  - `src-tauri/Cargo.toml` → `tauri-plugin-mcp-bridge = "0.12"`
  - `src-tauri/src/lib.rs` → `#[cfg(debug_assertions)]` 下 `.plugin(tauri_plugin_mcp_bridge::init())`（仅 debug 构建，不进生产）
  - `src-tauri/tauri.conf.json` → `app.withGlobalTauri = true`
  - `src-tauri/capabilities/default.json` → `mcp-bridge:default`
- **连接**：`driver_session action=start`（默认端口 `0.0.0.0:9223`）→ `action=status` 确认连上。
- **局限**：webview 工具只能操作 Webview 内部；**原生 OS 对话框**（选择文件夹等）触不到 → 仍走 pc-control，或用 `VITE_TEXPRESSO_PROJECT` 绕过。
