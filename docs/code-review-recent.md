# 近期改动 Code Review（93e2e53..HEAD）

> 范围：全仓 code review 落盘后的一批修复（e229db0..e116187）+ 新增大纲功能（cbb904f、3d193e0）。
> 方法：逐文件通读 diff + 全量验证（vue-tsc / vitest / cargo test / cargo check --tests）。

## 0. 验证（HEAD 全绿）

| 检查 | 结果 |
|---|---|
| `npx vue-tsc --noEmit` | ✅ 通过 |
| `npm run test`（vitest） | ✅ 28 / 4（提权运行） |
| `cargo test -p texpresso-core` | ✅ 通过（97） |
| `cargo check -p texpresso --tests` | ✅ 通过（src-tauri 测试编译无误） |
| `git status` | 干净 |

## 1. 大纲功能（cbb904f、3d193e0）——整体良好

**强项**
- 分层清晰：`stores/outline.ts`（解析/建树/refresh/goTo）与 `components/OutlinePane.vue`（拍平渲染），符合项目"store 负责逻辑、组件负责 UI"的模式。
- include 图递归带 `visited` 去重（防环/防重复），按文档顺序嵌套；处理 `\section*` 与 `[short]`；`buildTree` 按栈正确落层（含乱序层级）。
- 内容来源正确：打开标签用 `editor.buffers` 实时缓冲，否则读盘——与"文件系统为事实来源 + 实况编辑"一致。
- `goTo`：`openFile(file,line)` 揭示源码 + SyncTeX 正向（失败仅走源码并 warn，符合"尽力而为"）。
- 接线完整：项目打开 / 编译成功 / 结构变化（files-changed structural）都触发刷新；渲染有 depth 缩进、level 色标、当前文件高亮。

**发现**
- `[Medium] stores/outline.ts:145-166 + services/events.ts` — **并发 refresh 竞态**：编译成功与结构变化都可能触发 `refresh()`；两个 refresh 并发时，后完成的旧结果会覆盖新结果（`items.value` 为无序 last-write-wins）。建议加 `loadSeq`/supersede 守卫（参照 `PreviewPane`）。此外 `events.ts` 对 `refresh()` 用 `void`（fire-and-forget），内部若抛错会成为未处理 rejection，建议 `.catch()`。
- `[Medium] services/events.ts:19` — **每次编译成功都全量重扫**：`dto.phase==="success"` → 递归读根文件 include 图下所有文件 + 正则逐行扫描。大项目/高频编译下 IO 与解析成本明显，与延迟预算主题相悖。建议：仅"结构命令所在文件"或"结构变化"时才重扫，或对刷新节流/复用已读内容。
- `[Low] OutlinePane.vue:61` — `:key="i"` 索引键：列表顺序变化时 DOM 复用可能导致高亮/缩进错位；建议用稳定 key（`file:line`）。
- `[Low] outline.ts:75-86` — `resolveInclude` 先从 fromFile 相对目录拼（往往是不存在的错误路径，浪费一次 readFile），再试项目根；逻辑正确但可先判定存在性。
- `[Low] outline.ts:98-116` — `SECTION_RE` 仍会把注释行（非行首 `%`）或 `\verb`/`\begin{verbatim}` 内的结构命令误命中（启发式限制，与 ADR-0009 同类）；可在 parseFile 里跳过 verbatim 环境。
- `[Low] components/SplitPane.vue` — 方向命名反直觉（`horizontal`=上下 column、`vertical`=左右 row），无显式 `.split-pane.vertical` 规则（依赖默认 row）。功能正确但脆弱；本次大纲布局又新增多处方向用法，建议后续补显式规则。

## 2. 本轮修复（e229db0..e116187）——自查

**正确性均验证**
- H1-H4、on_save、Rust Medium（root_detect 正则 / 设置加载校验 / PDF 原子拷贝 / kill_on_drop / synctex 分块 / abort 竞态 / kill_tree 异步化）、前端 Medium（store 竞态/组件生命周期）——均有对应回归测试且通过；`cargo check --tests` 确认 src-tauri 测试编译无误（运行受 tauri 测试二进制 loader 限制，见 troubleshooting.md）。

**边界/行为提示（非阻断）**
- `[Medium] storage.rs load_overrides` — 任一字段校验失败即丢弃**整个** overrides：若项目 `.texpresso/settings.json` 的 `root_file` 非法（如 `..`），连合法的 compile 覆盖也会被回退为全局。安全但偏粗。可改为仅忽略非法字段。此为 H3 引入的行为变化（此前 root_file 不参与校验）。
- `[Low] runner.rs` — `pdf_dst.with_extension("pdf.tmp")` 在项目根留下临时文件：进程在 copy 与 rename 之间崩溃会残留 `main.pdf.tmp`；下次运行会被覆盖，但可在启动时清理。
- `[Low] App.vue manualCompile` — on_save 与连续模式下都会先 `flush()` 再 `compile_now`；连续模式点「编译」会立即落盘（原本不保存），属预期收益但改变了原行为。

## 3. 建议优先级

1. **大纲**：加 `refresh()` 的并发守卫（loadSeq/supersede）+ `void refresh().catch()`；评估"编译成功全量重扫"是否节流。
2. **storage.load_overrides**：考虑"逐字段忽略非法值"而非整对象丢弃（避免连带丢弃合法编译覆盖）。
3. 大纲细节：`OutlinePane :key` 改稳定 key；`resolveInclude` 先判存在；可跳过 verbatim 环境。
4. SplitPane 补显式 `.split-pane.vertical` 规则（既有 Low）。

## 4. 修复状态（按优先级）

- **M1 大纲并发守卫 + .catch()**：已修——`outline.refresh()` 加 `loadSeq` supersede（旧刷新完成即弃），`events.ts` 两处 `void refresh()` 改 `.catch(() => {})`。
- **M2 编译成功全量重扫**：已评估——超驰守卫避免并发重扫互相覆盖/重复写；全量重扫由编译频率约束（必要：结构可能随编译变化），暂不加节流，成本可通过后续命中「结构命令所在文件」细化。
- **P2 load_overrides 逐字段清洗**：已修——core 增 `sanitize_overrides`（越界/非法根文件置 None 回退全局、保留合法覆盖），`load_overrides` 改为此（不再整包丢弃）。
- **P3 大纲细节**：已修——稳定 key（`file:line`）、`resolveInclude` 根相对优先、跳过 `verbatim` 环境。
- **P4 SplitPane.vertical**：已修——补显式 `.split-pane.vertical { flex-direction: row; }`。

验证：`vue-tsc --noEmit` / `npm run test`（28/4）/ `cargo test -p texpresso-core` / `cargo check -p texpresso --tests` 全绿。

## 5. 强项

- 修复均以"代码 + 回归测试 + 文档"打包提交，每步验证后落地；无散装半成品。
- abort 竞态守卫、synctex 分块解析、嵌套 root_file、SD 设置校验等修复点都有针对性单测，且未破坏既有 93 项测试（现 97）。
- 大纲功能在编码风格、数据来源、失效处理上与项目既有约定（store 分层、buffer 优先、系统文件为事实来源）一致。
