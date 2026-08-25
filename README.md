# TeXPresso

> 一杯浓缩咖啡的时间，实时编译出你的第一篇 LaTeX 成品。

TeXPresso 是一款 **Windows 首发**的中文友好 **LaTeX 编辑器**——面向论文、笔记与书稿写作的桌面 GUI 应用：
**编辑即编译**（连续模式/保存触发），内嵌 PDF 预览与 **SyncTeX 双向定位**，基于 Tauri 2 + Vue 3 + Rust。

## ✨ 特性

- **项目管理**：打开文件夹 → 文件树 → 多标签页；自动保存（防抖）；外部修改检测与冲突提示；根文件自动探测（正则启发式）+ 手动覆盖
- **编辑**：Monaco 编辑器 + **自研 LaTeX Monarch 语法**（命令/环境/数学/注释/URL 分层上色，Candy 配色）；`%` 注释与括号自动配对；错误点击跳转源码
- **编译**：`latexmk` 驱动（增量），默认 **XeLaTeX**（可切换 LuaLaTeX/pdfLaTeX）；连续编译（500ms 防抖）或保存触发；调度队列（合并重复请求）、**超时自动终止**、手动终止、错误分类；错误列表**同源去重/截断**（错误雪崩不刷屏）
- **预览**：pdf.js 内嵌 **连续分页**（视口感知按需渲染）、缩放（±/适应宽度/Ctrl+滚轮）、**SyncTeX 正向（Ctrl+点击）+ 反向（点击 PDF）**、重载后滚动/页码保持、高 DPI 渲染修复
- **设置**：引擎/编译模式/防抖/超时/根文件覆盖，全局 + 项目（`.texpresso/settings.json`）两层，改即生效
- **界面**：Candy Desk 设计系统（纸面 + 蓝莓/珊瑚/芒果/薄荷），完整品牌素材（logo 全家桶）
- **工程**：Windows 原生路径全链路修复（`\\?\` verbatim 剥离）、WSL 时代路径兼容、去中心化的守护日志

## 🚀 快速开始

### 环境要求

| 依赖 | 说明 |
|---|---|
| Windows 10/11 | 首发平台（建议） |
| Node.js ≥ 18 | 前端构建（npm） |
| Rust（stable-x86_64-pc-windows-msvc） | 需要 **VS Build Tools C++ 工具链**（link.exe + Windows SDK） |
| TeX Live / MiKTeX | 提供 `latexmk`、`xelatex`（缺失时应用会提示安装） |

### 开发

```bash
npm install        # 安装依赖
npm run tauri dev  # 启动开发（vite + Tauri，首次会编译 Rust ~数分钟）
```

常用脚本：`npm run dev`（仅 vite）、`npm run tauri build`（构建安装包）、`npm run build`（前端类型检查 + 构建）。

### 使用

1. 点「打开项目」选择一个含 `.tex` 的文件夹（如 `000test/`，仓库内本地测试工程）；
2. 编辑 `main.tex` → 自动编译 → 右侧 PDF 实时更新（首次自动"适应宽度"）；
3. `Ctrl+点击` 源码 → PDF 高亮；点击 PDF 任意位置 → 跳回源码；
4. 「编译」按钮手动编译；「⚙ 设置」调整引擎/模式/超时等。

## 🗂️ 仓库结构

```
├── src-tauri/            # Tauri 壳：命令面（commands）、文件监视（watch）、
│                         #   latexmk 运行器（runner）、设置存储（storage）、SyncTeX CLI
├── crates/texpresso-core # 核心（纯逻辑，可测）：编译调度器、.log 解析器、设置模型与合并、根文件探测
├── src/                  # Vue 前端：stores（Pinia）/ 组件 / Monaco 语法 / IPC 服务层
├── docs/                 # 设计文档：design.md / architecture.md / modules.md / ADR（9 项）
├── public/               # 图标资源与 pdf.js 产物（cmaps/worker，npm 脚本生成）
└── scripts/              # 工具脚本（pdf.js 拷贝、logo 导出）
```

**架构要点**：Rust 核心零 UI 依赖；命令面做路径校验（D8）；文件系统为内容真相源（D7）；事件单向分发（D9 见文档）。详见 [docs/](./docs/)。

## 🧪 测试与质量

- `cargo test`（core + src-tauri）：调度器语义、日志解析快照、设置合并、路径校验（含 Windows verbatim 探测）
- 前端：`npm run test`（vitest 单测：stores 路径归一化 / editor 自保存过滤与冲突 / useAutoSave 防抖）+ `npm run build` 类型检查；手动验收清单见 [docs/design.md](./docs/design.md)（打开→编译→报错→修改→恢复→预览全链路）

## 📚 文档索引

| 文档 | 内容 |
|---|---|
| [docs/design.md](./docs/design.md) | 完整设计（产品/编译子系统/延迟预算/分发） |
| [docs/architecture.md](./docs/architecture.md) | 分层与接口契约 |
| [docs/modules.md](./docs/modules.md) | 模块详细设计与决策画像 |
| [docs/adr/](./docs/adr/) | 9 项决策记录 |
| [CONTEXT.md](./CONTEXT.md) | 术语表 |

## 🛣️ Roadmap（后置清单）

增量编译策略深化 → 错误列表（已完成去重/截断）→ texlab LSP（\ref/\cite 补全、hover、跳转定义）→ 设置页（已完成 v1）→ 文件树增量刷新 → 代码片段/拼写检查 → 外部 PDF 查看器 → TinyTeX 内嵌兜底 → 引擎按系统语言自适应 → 自动更新（tauri-updater，不强制）→ 多窗口与多项目。

> 详细规划见 [docs/design.md](./docs/design.md)「后置/未决清单」。

## 📄 许可

[MIT](./LICENSE) © 2026 WenqiBian
