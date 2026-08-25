# TeXPresso 端到端 GUI 测试（WebDriver）

按 **testing-tauri-apps** skill 的 WebDriver 方案：`tauri-driver`（中介）+ `msedgedriver`（Windows 底层，wry=WebView2/Edge）+ **WebdriverIO**。

目标：
- **冒烟**：验证 app 启动、工具条/预览面板渲染。
- **测 pdf.js 重载耗时**：自动打开项目 → 点「编译」→ 从 `window.__previewLastReload` 读取 `fetch/parse/render/total/pages/pagesRendered`，用于判断预览重载瓶颈。

## 前置条件（一次性）

```powershell
# 1. tauri-driver（本项目已装，若缺失）—— 需要网络
cargo install tauri-driver --locked

# 2. Windows Edge Driver（当前未装）—— 需要网络，版本需匹配本机 Edge
cargo install --git https://github.com/chippers/msedgedriver-tool
& "$HOME\.cargo\bin\msedgedriver-tool.exe"   # 安装 msedgedriver
msedgedriver --version                        # 验证
```

## 运行步骤

```powershell
# 3. 构建 Rust debug 二进制（WebDriver 拉起的就是它）
cargo build --manifest-path src-tauri/Cargo.toml

# 4. 安装 e2e 依赖
cd e2e-tests
npm install
cd ..

# 5. 起 vite dev（app 的 debug 二进制从 devUrl 加载前端，必须开着）
#    冒烟即可：
npm run dev
#    —— 若要“自动打开项目 + 测量重载耗时”，用绝对路径启动 vite：
$env:VITE_TEXPRESSO_PROJECT="C:\path\to\某个tex项目"   # 例如 ...\tex-presso\000test
npm run dev

# 6. 跑测试
cd e2e-tests
npm test
```

## 说明

- `tauri:options.application` 指向 `src-tauri/target/debug/texpresso.exe`（debug 构建从 `http://localhost:1420` 加载前端，**一定要先起 vite dev**）。
- `VITE_TEXPRESSO_PROJECT`：仅 dev/测试钩子（`src/App.vue` 里读取），设置了就自动打开该项目、绕过原生目录弹窗（原生弹窗 WebDriver 无法驱动）。生产不设置，行为不变。
- 测试 3 会点「编译」，等待 `window.__previewLastReload` 出现并打印每次重载耗时 JSON，输出形如：
  `PDF_RELOAD {"reload":3,"file":"main.pdf","pages":12,"bytes":182340,"fetch":8,"parse":142,"render":612,"total":762,"pagesRendered":5}`
- 若 `msedgedriver` 版本与 Edge 不匹配，`tauri-driver` 连接会超时（`WebDriver connection timeout`）——安装匹配版本即可。
