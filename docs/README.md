# TeXPresso 文档索引

> 项目状态：**设计收敛**（grill-with-docs 会话产出），待开工。
> 开工条件以 MVP 边界为准，见 [design.md](./design.md)。

## 文档结构

| 文件 | 内容 |
|---|---|
| [CONTEXT.md](../CONTEXT.md) | 术语表（ubiquitous language） |
| [design.md](./design.md) | 完整设计：产品定位、技术栈、编译子系统、MVP 边界、分发与质量底线 |
| [architecture.md](./architecture.md) | 分层设计：Rust/前端模块、层间接口契约、数据流、技术栈冻结、安全与工程基建 |
| [modules.md](./modules.md) | 模块详细设计：大模块拆分、函数签名与算法、通信契约、信息局部性 |
| [adr/](./adr/) | 决策记录（ADR），当前 9 项 |

## 阅读顺序

新成员：`CONTEXT.md → design.md → architecture.md → modules.md → adr/0001-0009`

## 决策记录索引

- [0001 合并队列调度 + 延迟预算](./adr/0001-compile-scheduling-and-latency-budget.md)
- [0002 Tauri 2 + Vue 3 技术栈](./adr/0002-tauri-vue-tech-stack.md)
- [0003 Windows 首发 + 不签名分发策略](./adr/0003-windows-first-unsigned-distribution.md)
- [0004 MVP 边界](./adr/0004-mvp-scope.md)
- [0005 latexmk 起步，增量编译为首要后续任务](./adr/0005-latexmk-first-incremental-next.md)
- [0006 workspace 拆分：core 与 src-tauri](./adr/0006-workspace-split-core-and-tauri.md)
- [0007 文件系统为内容真相源](./adr/0007-filesystem-as-source-of-truth.md)
- [0008 SyncTeX 走 CLI + 接口抽象](./adr/0008-synctex-via-cli-with-interface.md)
- [0009 根文件探测正则启发式](./adr/0009-regex-heuristic-root-detection.md)
