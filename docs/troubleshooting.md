# Troubleshooting

## 白屏：无 GPU 虚拟机环境的首帧呈现竞态

**现象**：`npm run tauri dev` 启动应用进程时，窗口偶发白屏（webview 页面已加载、JS 正常、devtools Console 无报错，但首帧未呈现）。**右键 → Reload 后立即正常**。

**边界**（实测）：
- 纯前端改动（Vite HMR）不触发
- Rust 改动触发的进程重启不触发
- 仅**进程冷启动**时偶发（不是每次）
- 与 `vite.svg` 的 404 无关（模板遗留文件，品牌化后已移除；当时属重启瞬间的瞬态请求）

**已尝试无效**：`WEBKIT_DISABLE_DMABUF_RENDERER=1`、`LIBGL_ALWAYS_SOFTWARE=1`、`WEBKIT_DISABLE_COMPOSITING_MODE=1`。

**处置**：白屏时右键 → Reload。开发环境怪癖，仅影响无 GPU 虚拟机；目标平台 Windows（WebView2/DirectX 硬件路径）预计不受影响，真机验证时复查。

## 幽灵窗口：WSLg 下 WebKitGTK 完全不渲染（2026-08 实测，已修复）

**现象**：`npm run tauri:dev` 进程正常启动（无 panic），窗口创建（window-state 插件有记录、任务栏有缩略图）但内容完全不渲染——静态 HTML 页面、前端代码全部排除后依旧。

**根因**：WSLg（无 GPU）与 WebKitGTK 渲染栈的版本兼容问题。特征为 `libEGL warning` + `ZINK: vkCreateInstance failed`（无 GPU 驱动时 ZINK/Vulkan 初始化失败，软件渲染兜底也未生效），且**与项目代码无关**——全新 Tauri 模板（vanilla/Vue，零业务代码）同样幽灵。

**诊断链路**（遇到同样现象时按此排查，全部实测过）：
- 进程存活、`WebKitWebProcess`/`WebKitNetworkProcess` 正常；`WEBKIT_FORCE_SANDBOX=0` 无效
- 网络层正常：`ss -tnp` 显示 webview 与 vite 已建立 `[::1]:1420` 连接，无代理
- **决定性实验**：脱离 Tauri 的最小 WebKitGTK 程序（python-gobject）同样不渲染，仅 `libEGL` 警告
- 环境特征：Arch WSL + WSLg + webkit2gtk 2.52.x + mesa 26

**已尝试无效**：`WEBKIT_DISABLE_DMABUF_RENDERER=1`、`WEBKIT_DISABLE_COMPOSITING_MODE=1`、`WEBKIT_FORCE_SANDBOX=0`、`LIBGL_ALWAYS_SOFTWARE=1`、`GDK_BACKEND=x11`、窗口位置钉主屏、CSP 置 null、静态 HTML 页面。

**修复方法（实测有效）**：
1. **升级 WSL**（Windows 侧 PowerShell/CMD）：`wsl --update`
2. **重装 WebKitGTK**（WSL 内）：`sudo pacman -S webkit2gtk-4.1`（若版本未变可先 `sudo pacman -Rns webkit2gtk-4.1` 再装）

**处置**：修复后 `npm run tauri:dev` 正常显示。若仍异常，再走 Windows 真机（WebView2）验证——目标平台不受此问题影响。

**附：WSLg 多显示器窗口跑到外接屏**：window-state 插件会保存并恢复跨屏位置；窗口“消失”时先 `rm ~/.config/com.texpresso.app/.window-state.json`。真机多显示器拔出后同样可能出现，后续可加“位置越界则居中”保护。

## GUI/端到端测试链路（WebDriver + pc-control，2026-08）

**方案**：`test_file/e2e/` 用 **WebdriverIO**（`browserName:'wry'` + `tauri:options.application`）驱动 `tauri-driver`（中介）+ `msedgedriver`（Windows，需 `msedgedriver-tool` 安装并匹配 Edge 版本，`--native-driver` 指路径）。`src/App.vue` 加了 `VITE_TEXPRESSO_PROJECT` 钩子自动打开项目（绕过原生目录弹窗，WebDriver 无法驱动）；`src/components/PreviewPane.vue` 加了重载耗时插桩。

**关键教训（均实测）**：
- **手动拉起 debug 二进制 ≠ 可用**：`npm run dev`（只起 vite）+ 手动 `target/debug/texpresso.exe`，前端**不渲染**（`button.btn.primary` 找不到）。必须 **`npm run tauri dev`**（正确构建 Rust + 起 vite + 真正调起 Tauri 窗口），问题即消失。
- **tauri-driver 不打印 "listening"**：`beforeSession` 靠字符串匹配会永久卡住；改成**轮询 127.0.0.1:4444 是否可连**（`net.connect`）。
- **只读沙箱限制**：vite/esbuild 的 worker 子进程需创建命名管道，`workspace-write` 下会 `EPERM`；需 `danger-full-access` 才能跑 `vite dev`/WebDriver/调起 WebView2 窗口。npm 缓存写 `%LocalAppData%` 也被拒，需把 `NPM_CONFIG_CACHE` 指到工作区。
- **读取前端控制台**：pc-control 的 `screenshot` 返回 base64，需解码成图片再读；devtools 快捷键（F12/Ctrl+Shift+I）需窗口聚焦才生效。窗口标题不随 `document.title` 改变（Tauri 不同步），**不要**用它做观测通道。
- **`webview_keyboard` 无法向 Monaco 编辑器插入文本（2026-08-25 实测定级）**：**不是 MCP 服务 bug**（`webview_keyboard type` 能在普通 `<input>` 上写入 `hello`，实测）；**是 Monaco（EditContext 架构）对合成事件不响应的限制**。Monaco 可编辑区是 `.native-edit-context`（DIV，`role=textbox`，走 EditContext）；`press` 只派发 `keydown/keypress/keyup`（**全部 `isTrusted:false`，且不产生 `beforeinput`/`input`**），合成键不触发浏览器默认文本插入（普通 input 上 `press` 同样不插入，实测），故 Monaco 不写入。`type` 靠 set `value` + `input` 事件，仅对原生 input/textarea 有效，且对 Monaco 唯一 textarea `.ime-text-area`（**readonly** 的 IME 缓冲）无效。**凡是涉及 Monaco 文本编辑的自动化，用真实 OS 按键（pc-control，`isTrusted:true` 触发 EditContext 插入，已实测 `Q` 写入），或改用 Monaco `executeEdits` API**（需实例，MCP 触不到）。
- **e2e 操作要点（2026-08-25 完整 e2e 实测）**：① **pc-control 打字前先 `Ctrl+Space` 关闭中文输入法**——否则 IME 会截走按键转义（实测 `% E2E_MARKER` 被 IME 吞掉/乱插）；② **状态栏「外部修改：文件名」可点击**——点击即从磁盘重载该文件（`acceptExternal`），用于外部修改冲突恢复（v1 无独立「重载」按钮，此 span 即入口）；③ 原生目录对话框流程：`Ctrl+L` 聚焦地址栏 → 粘贴路径 → Enter → 再补 `Enter`（连按确认）即可选中并关闭（实测成功打开 multifile 工程）。

**注意**：`test_file/e2e/drivers/`（msedgedriver 二进制）与 `test_file/e2e/node_modules/` 已 gitignore。
