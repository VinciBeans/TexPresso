# Tauri 2 + Vue 3 技术栈

桌面完整 IDE 需要 Rust 后端（编译进程监督、调度）与成熟前端组件（Monaco 编辑器、pdf.js 预览），因此采用 **Tauri 2**（Rust 后端 + webview 前端）+ **Vue 3 + TypeScript + Vite**。

- **状态**：已接受
- **备选方案**：纯 Rust + egui（否决：生态无生产级代码编辑器组件，PDF 渲染需自接 pdfium/lopdf，工期翻倍）
- **影响**：锁定双端结构；Windows 首发下 webview 中文 IME 组合输入是已知坑，v1 必须验证；前端框架随开发者熟练度可换（Monaco/pdf.js 框架无关）
