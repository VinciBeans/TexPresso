# TeXPresso 模块详细设计（函数级）

> 上层设计见 [architecture.md](./architecture.md)，产品语义见 [design.md](./design.md)。
> 本文件把 architecture.md 的每个大模块拆到**小模块 → 函数**，给出每个函数的签名、算法与信息局部性，以及大模块之间的通信契约。

## 0. 设计原则与信息局部性总则

**两条硬原则**（本文件所有设计的判据）：

1. **最小外部依赖**：函数只接收它真正需要的参数；模块只依赖它真正需要的模块；一切外部信息经显式参数或接口传入，禁止隐式获取。
2. **最小全局状态**：core 内**零全局可变状态**；进程内唯一的"全局"是 Tauri managed state 里的**不可变句柄**（sender、trait 实现），所有可变状态收容在单一写者（scheduler task / settings 存储）内。

**信息局部性三级规则**：

| 层级 | 允许存放的信息 | 禁止 |
|---|---|---|
| 函数内 | 解析中间态、正则匹配结果、计时器、临时缓冲 | 任何跨调用存活的信息 |
| 小模块内 | 队列条目、运行句柄、监视过滤器、模块自己的缓存 | 项目状态、设置全量 |
| 大模块内 | 项目状态、设置快照、调度状态 | — |
| 大模块间 | 只经 DTO / 命令 / 事件（不可变值传递） | 共享可变引用、全局单例 |

**跨大模块的通信只允许两种**：同步命令调用（传 DTO、返回 DTO）与单向事件（不可变载荷）。模块间不得共享锁、不得互相持有内部对象。

## 1. 全局状态清单（进程内仅此一份）

| 状态 | 类型 | 写者 | 位置 |
|---|---|---|---|
| 调度命令通道 | `mpsc::UnboundedSender<SchedulerCommand>` | 组合层（watch/commands） | Tauri state |
| 设置存储 | `Arc<RwLock<Settings>>` | settings 存储模块 | Tauri state |
| 项目状态 | `Arc<RwLock<ProjectState>>` | open_project / 探测 | Tauri state |
| FS / Runner / SyncTex 实现 | `Arc<dyn Trait>`（不可变） | 启动时构造一次 | Tauri state |

core 内的调度器、探测、解析、合并全部是**无全局态**的纯逻辑或 task 内状态。

## 2. 编译子系统（scheduler 大模块）

### 2.1 拆分

```
scheduler（core）
├── queue.rs        —— 合并队列（纯逻辑，无 IO）
├── policy.rs       —— 失败语义决策表（纯函数）
├── runner.rs       —— CompileRunner trait + CompileRequest/CompileOutcome（core 定义）
└── scheduler.rs    —— actor 主循环（唯一写者，状态全在 task 内）

src-tauri
└── runner.rs       —— LatexmkRunner：CompileRunner 实现（tokio 进程、超时、树杀、PDF 拷贝）
```

### 2.2 queue.rs — 合并队列

```rust
/// 待编译条目：最多一个，总是最新（覆盖语义）。
pub struct Queue { pending: Option<CompileRequest> }

impl Queue {
    pub fn new() -> Self;
    /// 合并语义：新请求覆盖旧请求（ADR-0001）。
    pub fn push(&mut self, req: CompileRequest) { self.pending = Some(req); }
    pub fn take(&mut self) -> Option<CompileRequest>;
    pub fn clear(&mut self);
    pub fn is_empty(&self) -> bool;
}
```

**算法**：无。唯一规则是 `push` 覆盖——"最多一个、总是最新"由类型本身保证，无法构造出多条目状态。

**信息局部性**：`pending` 只存在于 Queue 内；Queue 只存在于 scheduler task 内。函数参数只有请求本身，不接收任何上下文。

### 2.3 policy.rs — 失败语义决策表（纯函数）

```rust
pub struct RunningJob {
    pub request: CompileRequest,
    pub attempt: u8,          // 0 = 首次，1 = 已重试一次
}

pub enum Decide {
    Start(CompileRequest),    // 有 pending，执行之
    Retry(CompileRequest),    // 超时且无 pending 且 attempt == 0
    ShowError(FailureKind),   // 展示错误，回到 Idle
    Stay,                     // 无事可做
}

/// 输入只有两个：当前状态 + 编译结果。不读任何外部信息。
pub fn decide(running: &RunningJob, outcome: &CompileOutcome, has_pending: bool) -> Decide;
```

**算法**（来自 design.md 失败语义表，逐一对应）：

| outcome | has_pending | attempt | 决策 |
|---|---|---|---|
| Success | — | — | 有 pending → `Start(pending)`；无 → `Stay` |
| Timeout | true | — | `Start(pending)`（跳过重试） |
| Timeout | false | 0 | `Retry(同请求, attempt+1)` |
| Timeout | false | 1 | `ShowError(Timeout)` |
| ContentError | true | — | `Start(pending)`（不重试） |
| ContentError | false | — | `ShowError(ContentError)` |
| Aborted | — | — | `Stay`（队列已在 abort 时清空） |
| IoError | — | — | 视同 ContentError（不重试） |

**信息局部性**：`decide` 只读 `running` 与 `outcome`，不碰队列、不碰设置、不碰时间。重试计数是唯一跨调用信息，收在 `RunningJob.attempt`。

### 2.4 runner.rs — CompileRunner 接口（core）

```rust
pub struct CompileRequest {
    pub root_file: PathBuf,        // 根文件绝对路径
    pub project_root: PathBuf,     // 工作目录
    pub engine: Engine,            // 请求构造时从设置快照拷贝（之后设置变化不影响运行中任务）
    pub timeout: Duration,         // 同上
}

pub enum CompileOutcome {
    Success { pdf_path: PathBuf },
    Timeout,                       // 超时强制终止（runner 已树杀）
    ContentError { errors: Vec<ErrorEntry> },   // 进程非零退出，.log 已解析
    Aborted,                       // 收到取消信号（runner 已树杀）
    IoError { message: String },   // 拷贝/读日志等 IO 失败
}

#[async_trait]
pub trait CompileRunner: Send + Sync {
    /// cancel 是取消令牌：scheduler 调用 cancel 后 runner 必须尽快树杀并返回 Aborted。
    async fn compile(&self, req: CompileRequest, cancel: CancellationToken) -> CompileOutcome;
}
```

**设计决策 D2（超时归属 runner）**：超时检测、进程树杀、PDF 拷贝全部在 runner 内，scheduler 无时钟、无进程概念。备选"调度器注入时钟管超时"被否决：scheduler 被迫依赖 tokio 时钟，单测要注入时间源，复杂度不成比例。收益：scheduler 单测只需喂 `CompileOutcome` 假结果，超时路径由 runner 的集成测试覆盖。

### 2.5 scheduler.rs — actor 主循环（core）

```rust
pub enum SchedulerCommand {
    Compile(CompileRequest),   // 组合层已把文件事件翻译成请求（D3）
    Abort,                     // 手动终止：取消运行中 + 清空队列
}

pub struct Scheduler {
    rx: mpsc::UnboundedReceiver<SchedulerCommand>,
    runner: Arc<dyn CompileRunner>,
    queue: Queue,
    running: Option<RunningJob>,
    cancel: Option<CancellationToken>,
    emit: EmitFn,              // 注入的事件发射闭包：fn(CompileStatusDto) + fn(Vec<ErrorEntry>)
}

impl Scheduler {
    pub fn spawn(runner: Arc<dyn CompileRunner>, emit: EmitFn) -> UnboundedSender<SchedulerCommand>;
    async fn handle(&mut self, cmd: SchedulerCommand);
    async fn on_finished(&mut self, outcome: CompileOutcome);
}
```

**主循环算法**：

```
handle(Compile(req)):
  Idle        → start(req)                    // 发 Running 事件
  Running     → queue.push(req); emit(Queued) // 合并，最多一个等待
handle(Abort):
  cancel 当前任务（若有）→ queue.clear()       // 手动终止语义：停运行 + 清队列
on_finished(outcome):
  d = decide(running, outcome, !queue.is_empty())
  match d:
    Start(req)   → emit(Running); 启动新任务
    Retry(req)   → emit(Running); attempt+1 启动
    ShowError(k) → emit(Failed{kind:k}); 带 errors 时 emit(errors)
    Stay         → 无事
```

**信息局部性**：`queue`、`running`、`cancel` 全部是 task 私有字段——**调度状态没有任何一份在 task 之外**。外部只有 `UnboundedSender`，连读都读不到。emit 闭包由 src-tauri 注入（发 tauri 事件），scheduler 不知道 tauri 存在。

**事件输出契约**（emit 的两个载荷，即前端 compile-status / errors-updated）：

```rust
pub struct CompileStatusDto { pub phase: CompilePhase, pub kind: Option<FailureKind> }
pub enum CompilePhase { Queued, Running, Success, Failed }
pub enum FailureKind { Timeout, ContentError, Aborted }
// 时序：Queued(入队时) → Running(启动时) → Success / Failed
// 重试不重发 Queued，只重发 Running（attempt 对外不可见，前端不感知）
```

### 2.6 LatexmkRunner 实现（src-tauri）

```rust
pub struct LatexmkRunner { fs: Arc<dyn FileSystem> }

impl CompileRunner for LatexmkRunner {
    async fn compile(&self, req: CompileRequest, cancel: CancellationToken) -> CompileOutcome {
        // 1. 构造命令（算法见下）
        // 2. tokio::process::Command::new("latexmk").current_dir(&req.project_root)
        //    .args([...]).stdout(piped).stderr(piped).spawn()
        // 3. tokio::select! {
        //       _ = tokio::time::sleep(req.timeout)   => { kill_tree(pid); return Timeout }
        //       _ = cancel.cancelled()                => { kill_tree(pid); return Aborted }
        //       status = child.wait()                 => {
        //           if status.success() {
        //               let src = tmp/<root>.pdf; let dst = project_root/<root>.pdf;
        //               copy(src, dst) 成功 → Success{ pdf_path: dst }
        //               失败 → IoError
        //           } else {
        //               let log = fs.read_to_string(tmp/<root>.log)?;
        //               ContentError { errors: log_parser::parse_log(&log).errors }
        //           }
        //       }
        // 4. kill_tree(pid)：Windows → taskkill /T /F /PID <pid>（树杀，ADR 已定）；
        //    Unix → 进程组 kill（v1 后置，Windows 首发）
    }
}
```

**命令构造算法**（engine → 参数映射）：

```
latexmk -xelatex -outdir=tmp -synctex=1 -interaction=nonstopmode <root_file 文件名>
XeLaTeX → -xelatex；PdfLaTeX → -pdf；LuaLaTeX → -lualatex
cwd = project_root（相对 input/include 才能解析）
产物：tmp/<root>.pdf（拷贝到项目根）；tmp/<root>.synctex.gz（SyncTeX CLI 用）；tmp/<root>.log（解析用）
```

**信息局部性**：runner 无内部状态（`&self` 不可变），一次调用完全独立；每次调用所需信息全部在 `CompileRequest` 里。

## 3. 项目子系统（project 大模块）

### 3.1 拆分

```
project（core）
├── model.rs       —— Project / RootCandidate / RootResolution 类型
├── scan.rs        —— 文件收集（IO 经 FileSystem trait）
└── root_detect.rs —— 根文件探测（纯逻辑）
```

### 3.2 FileSystem trait（core 定义，src-tauri 实现）

```rust
/// core 唯一的 IO 抽象。面最小：只两个方法。
#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;   // 非递归，返回子项路径
    async fn read_to_string(&self, path: &Path) -> io::Result<String>;
}
```

**设计决策 D4**：探测、日志解析、设置读取全部经此 trait，core 因此无任何文件依赖。否决"宽松回调注入"（每个函数手写闭包签名，接口面发散）。

### 3.3 scan.rs — 文件收集

```rust
/// 递归收集项目内全部 .tex（排除 tmp/ 与隐藏目录），不跟随符号链接（防环）。
pub async fn collect_tex_files(fs: &dyn FileSystem, root: &Path) -> io::Result<Vec<PathBuf>>;
/// 忽略规则（与 watch 共用同一函数，单一事实来源）
pub fn is_ignored(path: &Path) -> bool;   // tmp/ 前缀、.git 等隐藏目录、非 .tex
```

**算法**：DFS 递归；每个目录先 `read_dir`，逐项判定 `is_ignored`，目录递归、.tex 收集。**函数内**持有递归栈与缓冲；**模块内**无缓存（每次调用全量扫描——文件树重建与探测都调它，目录量大时再优化增量）。

### 3.4 root_detect.rs — 根文件探测

```rust
pub struct RootCandidate { pub path: PathBuf }

pub enum RootResolution {
    Unique(PathBuf),
    Multiple(Vec<PathBuf>),   // 前端弹窗选择
    None,                     // 前端提示手动指定
}

/// 提取 \input{...} / \include{...} 引用（去扩展名、去花括号）
pub fn extract_includes(content: &str) -> Vec<String>;
/// 提取 \documentclass 声明（含可选参数形式 \documentclass[..]{..}）
pub fn extract_documentclass(content: &str) -> Option<String>;
/// 候选 = 含 \documentclass 且不被任何文件引用
pub fn find_candidates(files: &[PathBuf], read: impl Fn(&Path) -> Option<String>) -> Vec<RootCandidate>;
pub fn resolve(candidates: Vec<RootCandidate>) -> RootResolution;
```

**算法（设计决策 D5：正则启发式）**：

```
extract_includes:  正则 \\(input|include)\{([^}]+)\}，规范化：去 .tex 后缀 → 相对路径
extract_documentclass: 正则 \\documentclass(\[[^\]]*\])?\{([^}]+)\}
find_candidates:   读每个文件内容 → 收集引用集合（相对路径规范化）→
                   候选 = 含 documentclass 的文件 − 被引用的文件
resolve:           1 个 → Unique；>1 → Multiple（按路径排序，稳定顺序）；0 → None
```

否决"完整 TeX 词法解析"：v1 无此成本收益；**已知局限**（写进注释与测试）：注释里的 `\input` 会误报、`\includeonly` 未处理、编码（UTF-8 优先）。手动覆盖（settings.root_file）永远是逃生门。

**信息局部性**：`read` 闭包按需读内容、用完即弃——**任何文件内容不跨函数存活**；候选集合只在 find_candidates 内。探测是纯函数组合：`resolve(find_candidates(files, read))`，无状态。

## 4. 日志解析（log_parser 大模块，core）

```
log_parser
├── model.rs   —— LogMessage / MessageKind
└── scan.rs    —— 逐行扫描状态机
```

```rust
pub struct LogMessage {
    pub kind: MessageKind,          // Error | Warning
    pub message: String,            // 聚合后的可读消息
    pub file: Option<String>,       // 栈顶文件（若可确定）
    pub line: Option<u32>,          // "l.<n>" 行
    pub raw: String,                // 原始行（快照测试与调试用）
}

/// 输入只有 .log 全文，输出只有结构化消息。无状态、无 IO。
pub fn parse_log(text: &str) -> Vec<LogMessage>;
```

**算法（行级状态机）**：

```
逐行扫描，维护函数内的两个局部变量：
  file_stack: Vec<PathBuf>   —— "(" 开括号行 push（TeX 文件切换标记），")" 行 pop
  current: Option<LogMessage>—— 正在聚合的消息（跨 2-4 行）

分类规则（按行首匹配，顺序即优先级）：
  1. 以 "!" 开头                → 新 Error：file = stack 顶，后续缩进行聚合进 message
  2. 匹配 /^l\.(\d+)/           → 给 current 补 line（仅 Error 有效）
  3. /^\(([^)]*\.tex)/          → file_stack.push
  4. /^\)/                      → file_stack.pop（含 ")(" 连续切换的边界处理）
  5. /Warning|警告/             → 新 Warning（file/line 同规则尽力而为）
  6. latexmk 的 "Run #" / 目录回显行 → 丢弃
  7. 其他                       → 若 current 正在聚合且行非空 → 追加；否则丢弃
```

**信息局部性**：`file_stack` 与 `current` 是 **parse_log 的函数内局部变量**，一次调用即生即灭——这是"函数内信息"的范例。模块无缓存、无全局。

**测试**：insta 快照——真实 latexmk 日志（成功/内容错误/超时/多文件）固化为 4 个用例；含注释误报、`\include` 嵌套的探测用例另立。

## 5. SyncTeX（synctex 大模块）

```
synctex（core）         synctex_cli（src-tauri）
├── model.rs            └── cli.rs —— SyncTexProvider 实现
└── provider.rs
```

```rust
// core
pub struct SourcePosition { pub file: PathBuf, pub line: u32, pub column: u32 }
pub struct SyncTexPosition { pub page: u32, pub x: f32, pub y: f32 }

#[async_trait]
pub trait SyncTexProvider: Send + Sync {
    /// 源码 → PDF：synctex view -i "line:col:file" -o "<pdf>" -x
    async fn forward(&self, src: &SourcePosition, pdf: &Path) -> Result<SyncTexPosition>;
    /// PDF → 源码：synctex edit -o "page:x:y:<pdf>"
    async fn inverse(&self, pos: &SyncTexPosition, pdf: &Path) -> Result<SourcePosition>;
}

// src-tauri cli.rs：spawn synctex 二进制（tokio::process），解析 stdout
pub fn parse_forward_output(text: &str) -> Result<SyncTexPosition>;  // 纯函数，可单测
pub fn parse_inverse_output(text: &str) -> Result<SourcePosition>;   // 纯函数，可单测
```

**算法**：CLI 输出契约（`Page:N x:.. y:..` / `File:.. Line:.. Column:..`）需 Windows 实测定稿（ADR-0008 已记风险）；两个 `parse_*` 是纯函数，实测后以快照固化。pdf 路径指向 `tmp/<root>.synctex.gz` 对应 PDF 的**项目根副本**（synctex 按文件名关联）。

**信息局部性**：provider 无状态；一次调用 = 一次 spawn + 一次解析。前端高亮 overlay 与滚动恢复是预览模块自己的事，不流入此模块。

## 6. 设置（settings 大模块）

```
settings（core）              src-tauri
├── model.rs                  ├── storage.rs —— 读写盘、原子写
├── merge.rs                  └── hot_reload.rs —— 文件监视 → 重载 → 广播
└── validate.rs
```

```rust
// core：纯逻辑
pub struct Settings {
    pub compile: CompileSettings,
    pub root_file: Option<PathBuf>,   // 项目级手动覆盖（探测结果的逃生门）
}
pub struct CompileSettings { pub mode: CompileMode, pub debounce_ms: u64, pub timeout_secs: u64, pub engine: Engine }

pub fn default_settings() -> Settings;
/// 项目覆盖全局，逐键合并（项目缺失字段继承全局）
pub fn merge(global: Settings, project: Settings) -> Settings;
/// 范围校验：timeout 5..=600、debounce 100..=2000、engine ∈ {xelatex,pdflatex,lualatex}
pub fn validate(s: &Settings) -> Result<(), Vec<String>>;
/// 局部更新：只改 patch 里的键，其余不动
pub fn apply_patch(base: &mut Settings, patch: SettingsPatch) -> Result<()>;

// src-tauri storage.rs
pub async fn load_global(fs: &dyn FileSystem, dir: &Path) -> Settings;   // 缺失 → default + 落盘
pub async fn load_project(fs: &dyn FileSystem, project_root: &Path) -> Settings; // 缺失 → 空（全继承全局）
pub async fn save_global(..., s: &Settings);   // 原子写：临时文件 + rename
pub async fn save_project(..., s: &Settings);
```

**算法（merge）**：字段级 Option 语义——全局 `settings.json` 与项目 `.texpresso/settings.json` 同 schema（含 `schema_version`）；项目文件只写它覆盖的键，其余继承。

**热更新（设计决策 D6）**：watch 识别 `.texpresso/settings.json` 变化 → 重载 → 广播 `settings-changed`。**自写盘过滤**：`update_settings` 写盘时记录 `(path, content_hash)`；watch 事件到达时比对 hash，相同则跳过（防"自己写 → 自己重载 → 重复广播"）。hash 存在 hot_reload 小模块内，不跨模块。

**信息局部性**：core 的合并/校验是纯函数；全局设置快照是 §1 清单里唯一的 `RwLock` 共享态——写者只有 storage 模块，读方（组合层构造 CompileRequest 时）只取一次性快照拷贝，不持有引用。

## 7. 监视与触发组合（src-tauri）

```
watch.rs          —— notify 8.2 接线：事件规范化 + 过滤
compose.rs        —— 组合层：文件事件 → 编译请求（D3 的关键翻译）
```

```rust
// watch.rs：notify 事件流 → 项目内路径
// 过滤规则（与 project::is_ignored 共用）：
//   .tex（排除 tmp/）     → compose.on_tex_changed(path)
//   settings.json（全局或项目）→ hot_reload 重载 + 广播 settings-changed
//   其余                   → 丢弃
// 每个被接受的事件同时旁路广播 files-changed{paths}（前端文件树防抖重建）

// compose.rs：持有 project state + settings 快照，构造请求
pub fn on_tex_changed(&self, path: PathBuf) {
    let root = self.project.root_file();        // 读项目状态快照
    if root.is_none() { return; }               // 无根文件不编译（探测中/未设置）
    let req = CompileRequest {
        root_file: root,
        project_root: project.root,
        engine: settings.compile.engine,        // 一次性拷贝
        timeout: settings.compile.timeout,
    };
    self.scheduler.send(Compile(req));          // 唯一入口
}
```

**设计决策 D3（翻译层）**：scheduler 只认识 `CompileRequest`，不认识文件、项目、设置。文件事件 → 请求的翻译在组合层完成。否决"watch 直连 scheduler 传路径"：scheduler 被迫依赖项目状态与设置，违背最小外部依赖。手动编译 `compile_now` 走同一函数——**所有触发源收敛到同一个入口**。

**信息局部性**：组合层每次构造请求都取**快照拷贝**，不持有任何引用；请求发出后与触发源完全解耦。

## 8. 命令面实现（src-tauri commands.rs）

全部命令：DTO 进出、无业务逻辑；路径类参数一律**项目根内校验**（canonicalize + 前缀检查，防任意路径读写——自建命令没有 Tauri 权限模型兜底，这是安全底线）。

| 命令 | 实现要点（算法） |
|---|---|
| open_project(folder) | 校验目录 → 加载项目设置 → 探测根文件（有 root_file 覆盖则跳过探测）→ 更新项目状态 → 返回 ProjectInfo；探测为 Multiple → 前端弹窗后 update_settings 补 root_file |
| list_dir(path) | 递归 `collect_tex_files` 变体（返回全树 DirEntryInfo，含目录；前端防抖重建用） |
| read_file / write_file | tokio::fs 读写 + 路径校验 |
| save_all / save_file | 写盘（同 write_file）——不触发任何编译逻辑，watch 自然驱动 |
| compile_now | compose.on_tex_changed(当前活动文件) 同路径：构造请求入队 |
| abort_compile | scheduler.send(Abort) |
| synctex_forward / inverse | 调 provider，错误 → { code, message } |
| get_settings / update_settings | 读快照 / apply_patch → 校验 → 写盘（记录 hash）→ 广播 settings-changed |

## 9. 前端模块（函数级）

### 9.1 services（唯一碰 IPC 的层）

```ts
// ipc.ts —— specta 生成 + 薄封装（类型全自动，本文件不手写任何 DTO）
export const ipc = { openProject, listDir, readFile, writeFile, saveAll, saveFile,
                     compileNow, abortCompile, synctexForward, synctexInverse,
                     getSettings, updateSettings }  // 均为 Promise<T>

// events.ts —— 订阅一次，分发到各 store；返回取消函数
export function subscribeEvents(stores: { project, editor, compile, preview, settings }): () => void
// 映射表（单向：事件 → store 动作）：
//   compile-status → compileStore.setStatus
//   errors-updated → compileStore.setErrors
//   pdf-updated    → previewStore.reload
//   files-changed  → editorStore.onFilesChanged(paths)（过滤+重载判定）
//                     projectStore.refreshTreeDebounced()（300ms 防抖）
//   settings-changed → settingsStore.setSettings
```

### 9.2 stores 与依赖方向

```
依赖单向，禁止反向：
settingsStore ← projectStore（读设置）← editorStore（读设置/项目）
settingsStore ← compileStore
events.ts 是唯一"写"多个 store 的地方（订阅分发）
useAutoSave 依赖 editorStore.dirtyPaths + settingsStore（读）
```

| store | 状态（模块内） | 动作 |
|---|---|---|
| projectStore | project、rootFile、fileTree、treeVersion | openProject、refreshTree |
| editorStore | openTabs[]、activePath、dirtyPaths:Set、lastSaved:Map<path,time> | openFile、closeTab、markDirty、saveFile、saveAll、onFilesChanged |
| compileStore | phase、kind、errors[] | setStatus、setErrors |
| previewStore | pdfPath、page、scrollPos、highlight | reload、setHighlight |
| settingsStore | settings | setSettings、updateSettings |

**前端自保存过滤算法（editorStore.onFilesChanged）**：入参 paths 中，`lastSaved` 里存在且时间近（< 2s）的路径判定为"自己刚保存"→ 忽略；其余 → 已打开且不脏 → 重载内容；已打开且脏 → 保留 + 状态栏提示；未打开 → 忽略（文件树自会刷新）。`lastSaved` 是 editorStore 模块内状态，不进任何函数参数。

### 9.3 composables

```ts
// useAutoSave：防抖保存算法
// 输入事件：Monaco onDidChangeModelContent（仅当前活动文件）
// 状态：timer（函数内/组件内）——信息局部性：计时器不出 composable
// 算法：每次变更 → 重置 500ms 计时器 → 到点 → saveAll(dirtyPaths) → 成功才清 dirty
// 取消：组件卸载时 clearTimeout（防泄漏）
```

### 9.4 组件数据流

| 组件 | 输入（props/事件） | 输出（emit） | 模块内信息 |
|---|---|---|---|
| EditorPane | model(路径)、内容、语言 | 变更事件 → useAutoSave | Monaco 实例、worker、IME 组合状态 |
| PreviewPane | pdfPath、highlight | 点击坐标 → useSyncTex | pdf.js 文档句柄、滚动位置缓存 |
| FileTree | 树数据、激活路径 | 打开文件/目录展开 | 展开状态（只在前端本地） |
| ErrorList | errors[] | 点击条目 → openFile+定位 | 无 |
| StatusBar | compileStore/editorStore 只读投影 | 无 | 无 |

## 10. 通信契约总表（冻结）

**事件（tauri emit → TS 类型，specta 生成）**：

```ts
compile-status: { phase: 'queued'|'running'|'success'|'failed', kind?: 'timeout'|'content_error'|'aborted' }
errors-updated: ErrorEntry[]                       // 失败时携带；编译启动时清空由前端 setStatus('running') 触发
pdf-updated:    { path: string }
files-changed:  { paths: string[] }
settings-changed: Settings
```

**命令（invoke）**：见 §8 表，输入输出全部是 DTO。

**跨模块禁止**：共享可变引用、模块间互调私有函数、事件载荷携带非 DTO 对象、scheduler 感知项目/设置。

## 11. 设计决策记录（本轮分叉点）

| # | 决策 | 否决的备选 | 理由 |
|---|---|---|---|
| D1 | 调度器 = actor（单 task + mpsc），状态收容 task 内 | 共享锁状态机 | 零全局可变状态；单写者免锁；合并队列天然适合串行处理 |
| D2 | 超时/取消在 runner 内（CancellationToken） | 调度器注入时钟管超时 | scheduler 无时钟无进程概念；单测喂假 outcome 即可 |
| D3 | 组合层翻译"文件事件→CompileRequest"，scheduler 不感知项目 | watch 直连 scheduler 传路径 | 最小外部依赖；所有触发源收敛单一入口 |
| D4 | core IO 经 FileSystem trait（read_dir/read_to_string 两方法） | 每函数手写回调注入 | 接口面最小、单一事实来源 |
| D5 | 根文件探测正则启发式 | 完整 TeX 词法解析 | 成本收益不成比例；局限已记录，root_file 覆盖是逃生门 |
| D6 | 设置热更新 + 自写盘 content_hash 过滤 | 重启生效 / 不设防 | 防"自己写→自己重载"循环，过滤状态收在 hot_reload 模块内 |
| D7 | 前端 store 单向依赖 + events.ts 统一分发 | store 互引 / 事件总线库 | 依赖图可推理；避免引入总线抽象 |
| D8 | 自建命令路径校验（项目根内 canonicalize 前缀） | 信任前端传入路径 | 自建命令无 Tauri 权限模型兜底，必须自守 |

## 12. 与上层文档的关系

- architecture.md §2/§3/§5 的模块表在本文件展开为函数级；**本文件冻结后，architecture.md 的模块表不再单独细化**
- 新增后置项：`parse_forward_output`/`parse_inverse_output` 输出契约待 Windows 实测（ADR-0008 风险落地）；文件树增量刷新（当前全量重建）；**错误列表去重/截断**（环境性错误如缺字体触发 xelatex 错误雪崩时，单次编译可产生数十条同源错误，2026-08 实测暴露）
