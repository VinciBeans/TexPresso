# TexPresso `texpresso-core` 代码审查报告

Reviewed crate: `crates/texpresso-core` (pure Rust, no Tauri/IO, unit-testable).
All 25 listed files read in full; panics/unsafe cross-checked with grep.

## Overview

The core crate is well-structured and disciplined: it is genuinely IO-free and Tauri-free (all IO goes through injected `FileSystem`/`CompileRunner`/`SyncTexProvider` traits, ADR-0006/D2/D4), and the scheduler is a single-owner actor with no production locks, no `unsafe`, and no production `unwrap`/`expect` that a real input could trip. The command/event surface is coherent and the policy table is a pure, thoroughly-tested function. The main findings are not crashes but a few designed-but-fragile contracts (abort outcome enforcement, symlink-cycle guarantee, root-detection regex gaps) and some doc-contract/naming drift.

## Findings

**Medium**

- `[severity: Medium] scheduler/actor.rs:190-198, 206-211` — In the error-with-pending path the errors are broadcast and then immediately erased by the `Running` event that `StartPending` emits, because the comment on line 190 states the frontend *clears errors on Running*. In the no-pending path the errors survive (Failed never clears). So a content error that occurs while there is a queued/merged request is effectively never shown — inconsistent with the standalone-error path. **Suggestion:** key the error emission to a compile attempt id, or emit a `Failed`/errors state that the frontend keys per attempt instead of clearing on `Running`.

- `[severity: Medium] scheduler/actor.rs:155-161, 121-135` — `Abort` only cancels the `CancellationToken` and clears the queue; it does not record "this run was aborted", so the scheduler relies entirely on the runner honoring cancel (`runner.rs:13`). Two consequences: (a) if a runner returns `Success`/`Timeout` instead of `Aborted`, the scheduler emits `Success` after a user abort; (b) if a completion is processed *before* a queued `Abort`, that `Abort` will cancel a job that was just started from pending. Both are races the actor has no defense against. **Suggestion:** mark the `RunningJob` as cancelled on Abort and force `Decide::Fail(Aborted)` for it; ignore stale Aborts whose token is not the current one.

- `[severity: Medium] project/scan.rs:35, 41-52` — The doc comment "不跟随符号链接（防环）" is not enforced: `DirEntry` carries no symlink metadata, so if the injected `FileSystem::read_dir` returns a symlink-to-directory with `is_dir == true`, `collect_tex_files` recurses into it and can loop forever (a hang in `open_project`). The guarantee is delegated to the FS implementation that is not constrained here. **Suggestion:** add a `is_symlink`/`file_type` to `DirEntry` or track visited canonical dirs to break cycles.

- `[severity: Medium] project/root_detect.rs:27` — `\documentclass` regex requires the class name to immediately follow the optional `[options]` with no whitespace: `\documentclass {article}` or `\documentclass[12pt] {ctexart}` (both legal TeX, a space is ignored) are not matched, so a real document using a space triggers `RootResolution::None`. **Suggestion:** allow `\s*` before `{` (`\\documentclass(\\[[^\\]]*\\])?\\s*\\{([^}]+)\\}`).

- `[severity: Medium] synctex/provider.rs:30-48 vs 22-23` — The doc claims "取第一个完整块", but the code captures the *first occurrence of each* of `Page:`/`x:`/`y:` independently. If the first block is partial (e.g. missing `y`) and a later block completes it, fields are mixed across blocks rather than picking a full block. **Suggestion:** reset the collected fields at each `Output:`/`SyncTeX result begin` boundary and emit the first fully-populated block.

- `[severity: Medium] project/root_detect.rs:43` — `read(f) else continue` silently skips any `.tex` that fails to read (e.g. non-UTF-8): such a file is neither a candidate nor contributes its `\input`/`\include` refs, which can silently mis-detect the root. The UTF-8 assumption is documented (line 6) but the *skip* amplifies it beyond a convention. **Suggestion:** surface a warning/error for unreadable candidate files rather than dropping them silently.

- `[severity: Medium] settings/merge.rs:8-28 + settings/validate.rs:9-32` — `merge` does not validate and can produce a `Settings` whose `timeout_secs`/`debounce_ms` are out of range; validation is split into `validate`/`validate_overrides` that the caller must remember, while `apply_patch` does validate. Also `validate` returns `Result<(), Vec<String>>` (a non-idiomatic error type, no `thiserror`). **Suggestion:** have `merge` validate (or return a validated result), or document that merge must always be followed by `validate`; consider a proper error enum.

**Low**

- `[severity: Low] types.rs:15-24` — `Engine` has both `#[serde(rename_all = "snake_case")]` and per-variant `#[serde(rename = "xelatex")]` etc.; rename_all is redundant/dead for these variants. Keep only one form.

- `[severity: Low] settings/merge.rs:58-68` — `overrides_to_patch` is dead code (`#[allow(dead_code)]`), untested, and cannot express the "clear" patch (`Some(None)`).

- `[severity: Low] types.rs:129,137 + synctex/model.rs:6-20` — Parallel duplicate shapes: `SyncTexPosition` vs `SyncTexTarget`, and `SourcePosition` vs `SourcePositionDto`; only the DTOs cross IPC. Naming is asymmetric and easy to mix up.

- `[severity: Low] project/fs.rs:7-12 vs types.rs:120-125` — Core `DirEntry { path, is_dir }` vs DTO `DirEntryInfo { name, path, is_dir }`; `name` is derivable from `path.file_name()` (and can be absent for a path with no file-name component), so exposing it in the DTO is redundant and not guaranteed non-empty.

- `[severity: Low] types.rs:84` — Doc contract cites `CompileStatus`, but the exported specta type is `CompileStatusDto`. Frontend bindings follow the code name, so docs are inconsistent. Verify the generated TS name in `src-tauri` to close the drift.

- `[severity: Low] scheduler/actor.rs:121-135` — `tokio::select!` picks randomly when both a command and the running job are ready, so event ordering near the boundary is nondeterministic (benign for merge semantics, but makes timing-sensitive tests/UI ordering racy).

- `[severity: Low] project/scan.rs:27` — `.tex` extension match is case-sensitive (`== Some("tex")`); `.TeX`/`.TEX` files are excluded from both scan and the compose trigger.

- `[severity: Low] compose.rs:28` — `changed.starts_with(&ctx.project.root)` is component-wise but not normalized: a case-variant root (`C:\proj` vs `c:\proj`) fails the match, and a path containing `..` is silently dropped because the `..` component is treated as hidden in `is_ignored` (scan.rs:14). Watcher paths should be normalized before this call.

## Test coverage

Strong overall (every module has meaningful tests; scheduler + policy + synctex + log_parser are well covered). Concrete gaps:

- **Scheduler (actor.rs tests)**: no test for a *panicking* runner (the `JoinError → IoError "编译任务异常终止"` path at actor.rs:125-129); no test for the abort race where the runner ignores cancel and returns `Success`; no actor-level test for *Timeout with pending → StartPending*; no test for `next()` when the channel closes mid-run (detached task leak); no test for "Abort cancels a job started after the abort".
- **Policy**: all decision-table rows are covered (Success/Timeout/ContentError/Aborted/IoError × pending/attempt). No gap.
- **Settings**: `overrides_to_patch` untested (dead code); no invariant test that `merge` output stays valid (it can't, since merge doesn't validate); `validate` doesn't check `root_file` values.
- **log_parser**: CRLF line endings (relies on `str::lines()`) not explicitly tested; indented file-open markers (`  (/path`) not tested; a continuation line of an error that happens to contain "Warning" (would split the aggregation) not tested.
- **project/scan**: symlink-cycle/`..`/case-variant paths, and uppercase `.tex`, not tested; the "no-symlink" guarantee is exercised nowhere.
- **project/root_detect**: whitespace-before-`{` `\documentclass`; non-UTF8/unreadable file skipped; reference with `.tex` suffix and leading `./` normalization; these are untested.
- **synctex/provider**: no test for the partial-block "mixing fields" edge; no mock of the `SyncTexProvider` trait (only the pure parse functions are tested).
- **compose**: no test for a directory change event, a case-variant path, or a mixed-separator path; no test where `changed == root`.

## Strengths

- Genuinely IO- and Tauri-free core; all effects injected via `FileSystem`/`CompileRunner`/`SyncTexProvider` traits; scheduler never spawns processes (runner contract, D2).
- Single-owner actor (D1): state is entirely inside `Scheduler`, no production locks, **no `unsafe`** anywhere, and no production `unwrap`/`expect` that real input can trip (the only production `expect`s are guarded by invariants at actor.rs:209 and root_detect.rs:65). All other `unwrap`/`expect` are in `#[cfg(test)]`.
- Merge queue is type-enforced to at most one pending entry (queue.rs:9-36), correctly absorbing event storms and emitting `Queued` only on the empty→non-empty transition (actor.rs:145-152).
- `decide` policy is a pure function with a full unit matrix; retry keeps request identity and increments `attempt` correctly (max 1 retry, no overflow).
- Excellent real-world fixtures: snapshot tests for real TeX .log output (log_parser/scan.rs:202-262) and real Windows `synctex` 1.5/1.21 output (synctex/provider.rs:190-273) — a strength for a Windows-first app.
- CRLF handling is correct by construction (`str::lines()` strips `\r\n`), and path handling is component-based (correctly rejects `proj2/...` for root `proj`).
- Doc comments map cleanly to ADRs/design and reflect information-locality discipline.
