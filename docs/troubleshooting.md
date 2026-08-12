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

## 幽灵窗口：WSLg 下 WebKitGTK 完全不渲染（2026-08 实测）

**现象**：`npm run tauri:dev` 进程正常启动（无 panic），窗口创建（window-state 插件有记录、任务栏有缩略图）但内容完全不渲染——静态 HTML 页面、前端代码全部排除后依旧。

**诊断链路**（全部实测）：
- 进程存活、`WebKitWebProcess`/`WebKitNetworkProcess` 正常；`WEBKIT_FORCE_SANDBOX=0` 无效
- 网络层正常：`ss -tnp` 显示 webview 与 vite 已建立 `[::1]:1420` 连接，无代理
- **决定性实验**：脱离 Tauri 的最小 WebKitGTK 程序（python-gobject）同样不渲染，仅 `libEGL` 警告
- 环境：Arch WSL + WSLg 多显示器（窗口状态被保存到外接屏虚拟区域 x=3397）+ webkit2gtk 2.52.5 + mesa 26（无 GPU 驱动，ZINK/Vulkan 初始化失败，软件渲染兜底也未生效）

**结论**：WebKitGTK 2.52.x 与 WSLg（无 GPU）不兼容，属环境问题，与项目代码无关；模板期“白屏可 Reload 恢复”与本次“完全不渲染”是同一根因的恶化（Arch 滚动升级）。

**处置**：本机放弃 GUI 验证；目标平台 Windows（WebView2）不受影响。在有 TeX 的机器上先跑 `cargo test -p texpresso -- --ignored` 验证编译链路（无 GUI 依赖），再 `npm run tauri:dev` 做 GUI 全链路验收。

**附：WSLg 多显示器窗口跑到外接屏**：window-state 插件会保存并恢复跨屏位置；窗口“消失”时先 `rm ~/.config/com.texpresso.app/.window-state.json`。真机多显示器拔出后同样可能出现，后续可加“位置越界则居中”保护。
