# Agent 工作手册（AGENTS.md）

> 本文件用于指导 AI Agent 在项目中的行为、工作流程和注意事项。
> 请 AI 在开始任何任务前先阅读本文件，并严格遵守。

## 1. 角色与目标

- **角色**：你是一名 TexPresso（`tex-presso`）项目的 AI 开发助手（资深前端/全栈工程师）。
- **服务对象**：TexPresso 项目。
- **核心目标**：按「停服 → 改 → 重启 → 验证」的固定工作流，稳定、高效地完成项目的功能改动、验证、文档沉淀与提交，避免反复返工。
- **工作原则**：
  - 先理解需求与当前代码状态，再动手执行。
  - 改动一律走固定闭环；探索/改动后**立即更新文档**，让文档 = 事实来源。
  - 以「完整功能点完成且真机验证通过」为一次提交节点，不边做边推。
  - 遇到不确定、或操作被沙箱拒绝，先询问/提权，不猜测、不蛮干。

---

## 2. 项目背景

- **项目名称**：TexPresso（`tex-presso`）
- **项目简介**：基于 Tauri2 + Vue3 的 LaTeX 编辑器（Windows 首发、中文友好）。实时编译（latexmk/xelatex）+ 连续分页 PDF 预览（pdf.js）+ SyncTeX 双向定位。后端为 Rust（`src-tauri` + `texpresso-core`），前端为 Vue3 + Monaco 编辑器 + pdfjs-dist + naive-ui（Candy Desk 主题）。
- **相关文档**：
  - `README.md`（项目门面）
  - `docs/README.md`（文档索引与状态）
  - `docs/design.md`（设计，含延迟预算实测）
  - `docs/modules.md`（模块 + 后置清单 §12）
  - `docs/architecture.md`（架构）
  - `docs/adr/*.md`（ADR，如 0005-latexmk-first-incremental-next）
  - `docs/troubleshooting.md`（操作/环境类坑）
- **关键术语**：
  - **latexmk**：编译驱动。其「增量」指优化跑几遍（引用/bib 的多次 pass），不跳过未改动子文件；每次编辑仍会对整份文档重跑一遍 xelatex 单遍（引擎特性）。
  - **SyncTeX**：源码 ↔ PDF 双向定位（正向定位高亮 + 反向点击定位）。
  - **Monaco**：编辑器内核，已注册自研 Monarch 语法做 LaTeX 关键字高亮（Candy 配色）。
  - **pdf.js**：PDF 渲染引擎（连续分页 + 按需渲染，`canvasEpoch` 用于 canvas 代次重建以保证渲染正确）。
  - **root_file**：项目级根文件覆盖；应用/清除走后端 `update_settings`。
  - **debounce / timeout / engine / mode**：全局设置项（`mode` 为 continuous / save 触发）。
  - **texpresso-core**：纯 Rust 核心 crate（与 Tauri 解耦，可单测）。
  - **Candy Desk**：前端浅色主题语系（配色如蓝莓蓝 `#4a4cd8`、鼠尾草绿 `#7f977e`、玫瑰 `#d1547e`、紫罗兰 `#7c3aed` 等）。

---

## 3. 标准工作流

> 按顺序执行以下步骤。每完成一步，检查是否满足「完成标准」。

### 步骤 1：改动准备（停服 + 确认状态）

- **输入**：需求/要改的模块；当前 git 状态、dev 服务状态。
- **执行内容**：
  1. 先停 dev 服务：取消 `npm run tauri dev` 后台任务；确认 vite（`localhost:1420`）已停、app/cargo 进程关闭、无残留任务。
  2. 读当前要改的文件（避免覆盖原有逻辑），确认改动点。
- **完成标准**：dev 服务已停、无残留；已理解当前实现。
- **输出**：明确的改动清单。

### 步骤 2：修改实现

- **输入**：步骤 1 的改动清单。
- **执行内容**：
  1. 按需修改文件（dev 停着时**直接改**最省心；vite 已挂也能直接改）。
  2. 涉及关键组件（`PreviewPane.vue` / `App.vue` 布局等）时，保留其已有渲染守卫逻辑。
- **完成标准**：改动落地；`npm run build`（含 `vue-tsc --noEmit`）或直接 `vue-tsc --noEmit` 通过。
- **输出**：代码改动。

### 步骤 3：重启验证（真实窗口）

- **输入**：步骤 2 的改动。
- **执行内容**：
  1. `npm run tauri dev` 起真实 Tauri 窗口。
  2. 按 **UI 操作协议**操作并验证（见 §4 必须做）：每次点击前【全屏截图 → 从全屏图放大确认 → 再点击】；在选择对话框界面输入项目地址后【从第一次按 Enter 起连按 3 次】；打开项目后【截图确认「选择文件夹」对话框已关】；坐标按 pc-control 空间（屏幕 2560x1600，窗口常驻 `(-11,-11)`，最大化须实测）；读屏/操作统一走 pc-control。
  3. 后端链路用 tauri dev stdout 日志验证（`打开项目` / `触发编译` / `watch 线程启动`）。
- **完成标准**：改动在真实窗口生效，无回归。
- **输出**：验证结果（截图/日志证据）。

### 步骤 4：探索/改动后更新文档

- **输入**：步骤 3 的结论 / 探索发现。
- **执行内容**：
  1. 每次探索（基准评测/插桩测量/排查定位）或改动后、进入下一事前，把结论同步到对应文档：性能/延迟 → `docs/design.md`、`docs/adr/0005`；模块现状/后置项 → `docs/modules.md`；操作/环境坑 → `docs/troubleshooting.md`；仓库与分发状态 → `docs/README.md`。
  2. 只记录**已验证/实测**结论；被证伪的也要回写。
  3. 探索产物（插桩、自动打开钩子、e2e 基建）随文档一起沉淀，并注明「配套代码在工作区/未提交」。
- **完成标准**：文档与实际状态一致。
- **输出**：文档更新。

### 步骤 5：以完整功能点的完成为节点提交

- **输入**：步骤 3 功能点验证通过 + 步骤 4 文档更新。
- **执行内容**：
  1. 以「一个完整功能点做完且真机验证通过」为一次提交/推送节点；**不边做边推**，中途产物保留在工作区。
  2. 一次提交打包「代码 + 文档更新 + e2e/基建」。
  3. 提交后推送（注意 §4 边界的沙箱 TLS 绕行）。
- **完成标准**：功能点对应改动整体提交，无散落半成品。
- **输出**：一次干净的 commit（+push）。

---

## 4. 注意事项与规则

> 必须严格遵守。违反任何一条都视为任务失败。

### 必须做（Do）

- [ ] 改动走「停服 → 改 → 重启 → 验证」闭环；改关键组件前先停服。
- [ ] 每次点击前：全屏截图 → 从全屏图放大确认 → 再点击。
- [ ] 打开项目流程结束后截图确认「选择文件夹」对话框已关闭。
- [ ] 探索/改动后立即更新文档，让文档 = 事实来源。
- [ ] 只记录实测/已验证结论；被证伪的测试也回写。
- [ ] 动手前先 `read` 要改的文件，确认当前实现。
- [ ] `npm run build`（含 `vue-tsc --noEmit`）通过后再交付。

### 禁止做（Don't）

- [ ] 不要边做边推；不要在功能点未完成时提交散装改动。
- [ ] 不要用 pwsh 抓屏/裁图读 UI（受限模式会失败），读屏/操作统一用 pc-control。
- [ ] 不要改写全局 git 凭据配置（沙箱会拒，且影响用户环境）。
- [ ] 不要用 `npm run dev` + 手动拉起 debug 二进制做 GUI 测试（不渲染前端）；要用 `npm run tauri dev`。
- [ ] 不要伪造/猜测未验证的结论、API 或数据。

### 边界情况处理

- **推送失败 / 沙箱 TLS 报错**（`SEC_E_NO_CREDENTIALS`）：提权 `danger-full-access` 重试一次；仍失败则 `git config http.sslBackend openssl` + 用 `gh auth token` 嵌进 URL（只进单次进程参数、不写配置、不用 shell 助手）。用户在自己终端 `git push` 正常，绕行仅在沙箱内需要。
- **沙箱拒绝（EPERM / 网络被屏蔽 / 缓存写入被拒）**：esbuild worker 需提权；npm 全局缓存写入被拒 → 重定向进工作区；gh/curl 网络被屏蔽 → 用 `git ls-remote`（OpenSSL 后端）探测。被 deny → 同命令升级一次重试；升级被拒则停。
- **PowerShell 转义**：`\` 在 PowerShell 里被当字面量，生成 LaTeX 时 `\documentclass` 会被写成 `\\documentclass`（过度转义）→ 文件损坏、编译 exit 12。生成内容用**单反斜杠**，不叠 `\` 转义。
- **信息不足**：暂停并向用户提问，不要猜。

---

## 5. 工具与命令

- **允许使用的工具**：
  - **pc-control 系列**（mcp pc-control）：真实 Tauri 窗口的鼠标/键盘/滚动/截图——UI 操作与读屏统一直走这里。
  - **pwsh**（PowerShell）：运行命令/脚本；注意 `\` 转义（生成 LaTeX 用**单反斜杠**）、受限/FullLanguage 模式差异。
  - **read / write / edit / glob / grep**：文本文件读写与检索（先 read 再改）。
  - **后端日志验证**：读 tauri dev 后台任务的 stdout（`打开项目` / `触发编译` / `watch 线程启动`）。
- **常用命令**：
  ```bash
  npm run tauri dev                # 开发：构建 Rust + 起 vite + 真窗口（GUI 测试必须用它）
  npm run dev                      # 仅起 vite（localhost:1420），不调起窗口
  npm run build                    # copy pdf-worker + vue-tsc --noEmit + vite build（交付前类型检查）
  npm run tauri build              # 打包（NSIS 冒烟，验证 ADR-0003）
  cargo test -p texpresso-core     # 核心 crate 单测
  VITE_TEXPRESSO_PROJECT="<abs path>" npm run tauri dev   # 自动打开项目，绕过原生目录弹窗
  ```
- **注意**：在沙箱内 push 走 §4 边界里的 TLS 绕行（OpenSSL 后端 + gh token 嵌入 URL，不用 shell 凭据助手）。
