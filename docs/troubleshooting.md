# Troubleshooting

## 白屏：无 GPU 虚拟机环境的首帧呈现竞态

**现象**：`npm run tauri dev` 启动应用进程时，窗口偶发白屏（webview 页面已加载、JS 正常、devtools Console 无报错，但首帧未呈现）。**右键 → Reload 后立即正常**。

**边界**（实测）：
- 纯前端改动（Vite HMR）不触发
- Rust 改动触发的进程重启不触发
- 仅**进程冷启动**时偶发（不是每次）
- 与 `vite.svg` 的 404 无关（public/ 中存在该文件，属重启瞬间的瞬态请求）

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

**附：WSLg 多显示器窗口跑到外接屏**：window-state 插件会保存并恢复跨屏位置；窗口“消失”时先 `rm ~/.config/com.texpresso.app/.window-state.json`。真机多显示器拔出后同样可能出现，后续可加“位置越界则居中”保护。
