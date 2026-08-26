# TexPresso 全仓 Code Review

> 审查范围：整个 `tex-presso` 仓库（前端 Vue3 + TS / Tauri 后端 / `texpresso-core` Rust 核心 / 测试 / 文档）。
> 审查方式：5 个并行静态审查（核心 crate、Tauri 后端、前端 store/service、前端组件、测试+文档一致性）+ 交叉验证（`vue-tsc`、`cargo test`、手册级前后端契约核对）。
> Rust 核心的逐文件深挖单独沉淀在 [`docs/core-review.md`](./core-review.md)。

## 0. 结论总览

整体是一套**结构清晰、纪律良好**的代码：分层单向（前端 → IPC 契约 → commands → 领域 ← 基础设施）、`ipc.ts`/`events.ts` 是唯一触 IPC/事件分发的层、错误契约 `{ code, message }` 干净一致、命令面/事件面（12 命令 5 事件）与 `src/bindings.ts` 一一对应、`texpresso-core` 无 Tauri/IO 依赖、**全仓无 `unsafe`**、生产路径没有会被真实输入触发的 `unwrap/expect`、调度器合并队列类型上强制最多一个待处理条目。

但也存在 **4 项必须处理的 High 级问题**（2 项跨项目设置污染/路径越界、1 项嵌套根文件编译失败、1 项前端自动保存数据丢失竞态），以及若干 Medium 级竞态/清理/防环问题。`src-tauri` 接线层几乎零测试，属于最大覆盖盲区。

## 1. 验证信号（客观）

| 检查 | 结果 |
|---|---|
| `npx vue-tsc --noEmit` | ✅ 通过（无类型错误） |
| `cargo test -p texpresso-core` | ✅ 93 passed / 0 failed |
| `npm run test`（vitest） | ⚠️ **沙箱内无法运行**：esbuild worker `spawn EPERM`（沙箱限制管道 stdio，非代码缺陷）。前端单测以静态方式审查 |

> 说明：`src-tauri` 的真实 latexmk/synctex 集成测试全部为 `#[ignore]`（需 TeX Live），`cargo test` 不会执行它们。

## 2. 【High】必须修复

### H1. `open_project` 把“合并后设置”当作“全局”，跨项目污染
`src-tauri/src/commands.rs:99-102`
- 首次打开时 `state.settings` 是纯全局（`lib.rs` 用 `block_on(storage.load_global)` 初始化）。`open_project` 第 99 行 `state.settings.read()` 取到的是**上次合并后的 effective**，第 102 行又把它写回 `state.settings`。
- 再打开第二个项目 B 时，`effective(effective(global, overrides_A), overrides_B)` 会把项目 A 的 `root_file`/`mode` 等覆盖值带进 B（B 无覆盖时尤为明显）。
- `update_settings`（commands.rs:366）刻意从磁盘重读**纯全局**规避此问题，`open_project` 应同样处理。
- **建议**：改为 `let global = state.storage.load_global(state.fs.as_ref()).await;`（与 366 行一致）。

### H2. 嵌套 `root_file` 导致编译失败/产物位置错乱
`src-tauri/src/runner.rs:31-42, 74-75`（触发点 `commands.rs:106`）
- runner 用 `root_stem(req.root_file)` 只取文件名 stem，第 41 行拼 `format!("{stem}.tex")` 且 `current_dir=项目根`，丢弃了 `root_file` 的目录成分。
- `root_file = proj/css/thesis.tex` 时实际执行 `latexmk thesis.tex`（在 `proj/` 下，文件不存在）→ 编译失败；`pdf_dst`（33 行）/`pdf_src`（74 行）也落在顶层 `proj/thesis.pdf`。
- 顶层 `main.tex` 恰好可用，故漏测。
- **建议**：传入输入用 `root_file.strip_prefix(project_root)` 的完整相对路径；`pdf_dst`/`pdf_src` 按同一相对路径计算。

### H3. `root_file` 覆盖不做路径安全校验，可越权逃逸
`src-tauri/src/commands.rs:98-106` + `crates/texpresso-core/src/settings/validate.rs:35-64`
- `open_project` 第 106 行 `canonical.join(override_path)` 无 `starts_with(root)` 检查；`validate_overrides`（354 行调用）只校验 `compile.*` 区间，完全不含 `root_file`。
- 项目 `.texpresso/settings.json` 中的 `../` 遍历或绝对路径 `root_file` 会被当作编译根，逃出项目盘符（与自守设计 D8 相悖）。
- **建议**：解析覆盖值后断言其 canonical 化后 `starts_with(root)` 且为 `.tex`；把 `root_file` 纳入 `validate_overrides`。

### H4. `useAutoSave` 陈旧保存竞态 → 数据丢失
`src/composables/useAutoSave.ts:21-35` + `src/stores/editor.ts:59-65`
- `run()` 先快照 `files`（内容 A），`await ipc.saveAll` 期间用户继续输入内容 B（`markDirty` 重写 `buffers[p1]=B`），随后 `markSaved` 无条件 `dirty.delete(p1)`（editor.ts:62）。
- 结果：`buffers` 里是最新内容 B，但 `dirty` 已被清空。若在重新防抖触发前 `flush()`（关标签/关窗），`dirty` 为空会跳过保存 → B 丢失。
- **建议**：保存前按路径记录内容快照/版本；仅对“缓冲区未变”的路径 `markSaved`，其余重新 `markDirty` 并 `schedule()`。

## 3. 【Medium】建议处理

### 功能缺口
- **`on_save` 模式实际未实现**（`App.vue:76-78` + `useAutoSave.ts:14-18` + `src-tauri` watch 触发）：前端 `onEditorChange` 无条件 `autoSave.schedule()`，且 `useAutoSave` 只读 `debounce_ms`；`writeFile`/`saveFile` 全仓无调用。后端编译触发（watch）不读 `mode`，也无时间防抖。文档承诺的“保存触发编译”与连续模式在功能上等同。**需确认产品语义**：要么前端按 `mode` 决定是否自动写盘，要么后端按 `mode` 门控触发。

### 竞态 / 状态
- **`openFile` 去重竞态**（`editor.ts:29-33`）：去重判断在 `await readFile` 之前，树节点快速双击会重复开标签。把去重移到 await 之后或串行化。
- **`onFilesChanged` check-then-await 竞态**（`editor.ts:93-99`）：检查 `dirty` 后 `await readFile`，期间用户输入则磁盘快照覆盖刚输入内容而 `dirty` 仍为 true。读取后复检 `dirty`，脏则丢弃重载。
- **`compileStore` 成功态不清空错误**（`compile.ts:12-23`）：`setStatus("success")` 对 `errors`/`hasError` 无操作；仅当无 `running` 前置时，旧错误会残留到“就绪”。`success` 时显式清空（或依赖并文档化事件顺序）。
- **`App.vue:43` `settings.init()` 未 try/catch**：异步 `onMounted` 中 `await` 未包，`get_settings` 拒绝会成未处理 rejection 且跳过后续自动打开项目。

### Rust / 后端健壮性
- **Abort 竞态**（`scheduler/actor.rs:121-161`）：abort 只取消 token + 清队；若 runner 忽略取消而返回成功/超时，调度器在 abort 后仍发 Success；且排队的 abort 可能取消刚从 pending 启动的新任务。应增加“取消意图”守卫。
- **symlink 目录未防环 → `open_project` 递归挂起**（`project/scan.rs:35,41-52`）：注释声称“不跟随符号链接（防环）”，但 symlink 到目录仍返回 `is_dir` 并递归，导致无界递归/挂起。应在扫描时识别 symlink 或限制深度。
- **设置加载未校验范围**（`storage.rs` + `validate.rs`）：`load_global`/`load_overrides` 解析后不调用 `validate`；手工/陈旧 `settings.json` 填 `timeout_secs=1` 会被直接使用（1s 超时）。加载时校验，越界回退默认。
- **`kill_tree` 在 async 内同步阻塞**（`runner.rs:114-130`）：用 `std::process::Command::status()` 同步 `taskkill`，会占用执行器线程。改 `spawn_blocking` 或 `tokio::process::Command`。
- **PDF 拷贝非原子**（`runner.rs:75-79`）：`fs::copy` 原地覆盖目标，失败（磁盘满 / Windows 下预览 PDF 被锁）会留下残缺 PDF，且不发 `pdf-updated` → 预览陈旧。改临时文件 + rename。
- **app 退出时孤儿 latexmk**（`runner.rs:46` + `actor.rs`）：`Child` 未 `kill_on_drop`，无关闭钩子；应用退出时在跑的子进程可残留、持续写 `tmp/`、占用 PDF 锁。设 `kill_on_drop(true)` 或 teardown 时取消 token。
- **`root_detect` 正则不能匹配 `\documentclass` 前空格**（`project/root_detect.rs:27`）：合法的 `\documentclass {article}` 不匹配 → 根探测返回 None。放宽正则。
- **`synctex` 输出解析跨块混搭字段**（`synctex/provider.rs:30-48`）：注释是“取第一个完整块”，实际是各自取第一个出现的 Page/x/y，跨残缺块可能混搭。按块解析。

### 前端组件清理
- **Monaco 订阅未逐个销毁**（`EditorPane.vue:55-73`）：3 个 `onDidChange*`/`onMouseDown` 的 IDisposable 未保留，仅靠 `editor.dispose()` 级联。建议逐个保留并 dispose。
- **`PreviewPane.load()` 无 unmount 守卫**（`PreviewPane.vue:397-479`）：卸载后 load 仍在跑，会写 `window.__previewLastReload`/`document.title`，并可能 `doc.numPages` 抛 TypeError → 伪错。加 unmounted 标志。
- **`PreviewPane` 卸载不取消渲染任务**（`PreviewPane.vue:597-603`）：在途 RenderTask/`renderChainByPage` 未取消，只靠 `loadingTask.destroy()`。卸载时显式 `cancelAllRenders()`。
- **`onCanvasClick` 无 try/catch**（`PreviewPane.vue:547-560`）：卸载期间点击会 `getPage`/`inverse` 拒绝未处理。

## 4. 【Low】代码卫生

- `events.ts:16-34` 大量 `as any` 丢弃 Specta 类型；`ipc.ts` 未消费 `CmdError` 的字段（仅 rethrow）。
- `ipc.ts:19-20` `writeFile`/`saveFile` 死代码（全仓未调用；与 `save_file` 语义重复，见后端 L）。
- `project.ts` `treeVersion` 导出但组件未使用；`refreshTree().catch(()=>{})` 静默吞掉 `list_dir` 错误。
- `ipc.ts:26` `synctexInverse` 把 `x/y` 收窄为 `number`，比契约 `number | null` 更窄。
- `SettingsPanel.vue` `lastAppliedRoot` 死状态；`ErrorList.vue` `v-for :key="i"` 索引键 + `split('\n')[0]` 重复计算。
- `SplitPane.vue` `vertical` 方向命名反了、缺显式 `.split-pane.vertical` 规则。
- `PreviewPane.vue` 755 行单文件，滚动/`y` 翻转逻辑重复、硬编码高亮框；缺“已设 pdfPath 但 numPages=0”的 loading 态。
- Rust：`types.rs:15-24` Engine 多余 `rename_all`；`merge.rs:58-68` 死 `overrides_to_patch`；`SyncTexPosition/SyncTexTarget` 与 `SourcePosition/SourcePositionDto` 结构重复；`{CompileStatus 文档}` vs `{CompileStatusDto 代码}` 命名漂移；`select!` 分支不确定性；`.tex` 大小写敏感（`scan.rs:27`）；`compose.rs:28` `starts_with` 未归一化。
- 后端：`kill_tree` 阻塞（已列 Medium）；`atomic_write` 无 `fsync`；损坏的全局 `settings.json` 被直接覆盖默认且无备份；watcher 线程永不 join、通道无界无背压（tmp/ 递归监视吃事件）；Windows 下 `app_config_dir` 可能带 `\\?\` 前缀导致 `is_settings_path` 匹配失效；`open_project` 内阻塞 `read_to_string`；`pdf_path_for_root` 在 `root_file=None` 时默认 `main.pdf` 导致 `synctex_*` 返回 Internal 而非明确“未确定根文件”；`list_dir` 读两次项目根。
- 契约：`synctex_inverse` 的 `x/y`（bindings `number | null`）与 Rust 处理器 `x: f32, y: f32`（拒绝 null）不一致——目前 `ipc.ts` 强转可为掩盖。

## 5. 测试覆盖评估

**强项**
- `texpresso-core` 覆盖最好：调度器（queue merge / timeout+retry / abort / content+IO error / pending 语义）、`policy` 纯函数决策表、`log_parser` 真实日志快照、`root_detect`、`synctex` 前后向解析（含真实 Windows 样本）、`settings` merge/validate/patch。
- 前端 vitest 聚焦契约且隔离良好（`services/ipc` 全程 mock、`happy-dom`、每测新 `createPinia()`）；回归测试直接锁定真 bug（`./` 绝对路径去重、content-change 不覆盖结构性意图、防抖重置只存一次）。

**缺口（按风险）**
- **`src-tauri` 接线层几乎零测试**：`commands.rs`/`watch.rs`/`storage.rs`/`fs_impl.rs`/`sync_cli.rs`/`events.rs` 无 `#[cfg(test)]`。路径安全（`validate_in_project`/`save_content`）、设置热重载/自写哈希过滤（`is_self_write`）、`is_structural_event`、`strip_verbatim`、`atomic_write` 全未测。路径检查回归 = 任意文件读写（D8 底线）。**最高优先补测**。
- 真实 latexmk/synctex 进程契约全 `#[ignore]`（runner.rs:191-329），不跑进 CI；嵌套 `root_file`、绝对 Windows 路径冒号分割（ADR-0008 最大风险点）均未验。
- 调度器：panic runner 的 `JoinError→IoError` 路径、abort 竞态、Timeout-with-pending 的 actor 级、channel 中途关闭——未测。
- 前端：`compileStore`/`previewStore`/`settingsStore`、`events.ts` 事件分发、`useSyncTex`、`acceptExternal`/`markSaving`（文档中的竞态修复）无测试；`useAutoSave` 的“保存期间编辑”陈旧竞态（H4）也无测试。

## 6. 文档 vs 代码漂移

- **[模块 §2.3 决策表]** “`Aborted | — | — | Stay`” 与代码不符——实际无 pending 时返回 `Fail(Aborted)`，有 pending 时 `StartPending`（`policy.rs`, `integration_tests.rs:239`）。**文档错误**（且 §2.5 自相矛盾地写了 `ShowError(k)→Failed{kind:k}`）。
- **[模块 §2.3/§2.5 代码块]** `Decide{Start,Retry,ShowError,Stay}`、`decide(running,&outcome,has_pending)`、`Scheduler.spawn/emit/SchedulerCommand(无 JobFinished)` 均为过期签名；实为 `{StartPending,Retry,FinishOk,Fail}`、`decide(attempt,outcome,has_pending)`、`Scheduler::create()→(SchedulerHandle,Scheduler)`。**文档错误**。
- **[模块 §2.6]** `stdout(piped).stderr(piped)` 与代码 `Stdio::null()` + `-synctex=1` 不符。**文档错误（minor）**。
- **[架构 §4 命令表]** `save_all/save_file | [path]` 不准：`save_file(path,content)`、`save_all(Vec<FileContent>)`，且 `write_file`==`save_file` 为别名；`synctex_inverse` 输出漏 `column`；`kind` 是常驻（可空）非可选。**文档错误 / 轻微冗余**。
- **[模块 §8]** `compile_now` “compose.on_tex_changed(当前活动文件)” 不准，代码用 `compile_request_manual` 只看 `root_file`。**文档不精确**。
- **[设计 §"保存触发编译" 与 架构 §5.1]** 前端从未按 `mode` 分支（见 M1）。**高价值漂移（文档化功能未落地）**。
- **[模块 §9.2 previewStore]** 列 `pdfPath,page,scrollPos,highlight`，实际仅 `pdfPath,reloadKey,highlight`，滚动恢复在 PreviewPane 模块变量。**文档过期**。
- **[模块 §6]** 命名 `load_project/save_project/hot_reload`，代码为 `load_overrides/save_overrides/self_write`。**文档过期**。

## 7. 已知问题状态（文档 vs 代码）

| 文档问题 | 状态 | 证据 |
|---|---|---|
| SyncTeX 正向定位未加载页跳不到位 | **已修复** | `renderPage` 返回 promise、`goToPage` 预热高度 + `behavior:"auto"`、高亮路径等待渲染（PreviewPane） |
| 页码跳转 | **已修复** | `.page-input` + `onPageInput → goToPage` |
| 文件树增量刷新 | **已修复 & 已测** | `FilesChanged.structural` + `is_structural_event` + `refreshTreeDebounced` + project.spec |
| PDF 消失 / 滚动条拖不动（缩放） | **已修复** | `.page-wrap` 高度按 `pageH1×scale`、`layoutRev`、`display:block` |
| 错误列表去重/截断 | **已修复** | `ErrorList` `MAX_DISPLAY=30` + `grouped` + `×N` |
| synctex 输出契约 | **已定稿 & 已测** | provider.rs + 真实 Windows 样本 |
| 冲突对话框 | **以状态栏点击重载替代，非对话框** | `acceptExternal` + StatusBar（设计已标明后置，一致） |

## 8. 修复优先级建议

1. **先修 4 个 High**：H1 设置污染、H2 嵌套根文件、H3 路径越界、H4 自动保存数据丢失。
2. **明确 `on_save` 语义**并落地（M1），否则删除/改文档为“仅连续模式”。
3. **补 `src-tauri` 接线层测试**（路径安全、设置热重载、`structural` 计算、`strip_verbatim`）——这是当前最大盲区。
4. 处理 Rust 治本类 Medium：symlink 防环、abort 竞态、设置加载校验、`kill_tree` 异步化、PDF 原子拷贝、进程孤儿、`root_detect`/`synctex` 解析修正。
5. 清理前端组件生命周期（Monaco 订阅、PreviewPane unmount 守卫与渲染取消）+ 状态竞态（openFile/onFilesChanged/compile 清错）。
6. 修正文档漂移（尤其 modules.md §2.3 Aborted 行、命令表、`on_save`）。
7. 补前端关键状态/事件/useSyncTex 测试，尤其 H4 陈旧竞态的回归用例。

## 9. 强项

- 分层单向、`ipc.ts`/`events.ts` 唯一触 IPC/事件；错误契约干净、命名/字段跨三端一致；路径安全总体扎实（canonicalize + `starts_with(root)` + 父目录再校验）。
- `texpresso-core` 无 Tauri/IO，全效果注入（`FileSystem`/`CompileRunner`/`SyncTexProvider` trait），单 owner 调度 actor，决策表纯函数、合并队列类型强制“最新胜出”，无 `unsafe`、生产无可达 `panic`。
- 前端 `PreviewPane` 渲染守卫（`renderedScale/structuralEpoch/pageH1 保留 + layoutRev`）、pdf.js 每页 FIFO + cancel-then-await + `loadSeq` 取代、按需渲染与高度保留虚拟化均良好；事件订阅一次注册 + 卸载清理正确。
- 路径归一化（`resolvePath`/`normalizePath`，含 `\\?\` 直通）与 Windows `strip_verbatim` 处理一致且有回归测试。
