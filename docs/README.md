# TeXPresso 文档索引

> 项目状态：**设计收敛**（grill-with-docs 会话产出），待开工。
> 开工条件以 MVP 边界为准，见 [design.md](./design.md)。

## 文档结构

| 文件 | 内容 |
|---|---|
| [CONTEXT.md](../CONTEXT.md) | 术语表（ubiquitous language） |
| [design.md](./design.md) | 完整设计：产品定位、技术栈、编译子系统、MVP 边界、分发与质量底线 |
| [adr/](./adr/) | 决策记录（ADR），当前 5 项 |

## 阅读顺序

新成员：`CONTEXT.md → design.md → adr/0001-0005`

## 决策记录索引

- [0001 合并队列调度 + 延迟预算](./adr/0001-compile-scheduling-and-latency-budget.md)
- [0002 Tauri 2 + Vue 3 技术栈](./adr/0002-tauri-vue-tech-stack.md)
- [0003 Windows 首发 + 不签名分发策略](./adr/0003-windows-first-unsigned-distribution.md)
- [0004 MVP 边界](./adr/0004-mvp-scope.md)
- [0005 latexmk 起步，增量编译为首要后续任务](./adr/0005-latexmk-first-incremental-next.md)
