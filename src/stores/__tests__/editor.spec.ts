// editorStore 单测（modules.md §9.2 / §5.5）：openFile 去重、onFilesChanged 自保存过滤/冲突。
// mock services/ipc，避免加载真实 Tauri IPC。
import { describe, it, expect, beforeEach, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useEditorStore } from "../editor";
import { useProjectStore } from "../project";

vi.mock("../../services/ipc", () => ({
  ipc: {
    readFile: vi.fn(async (p: string) => `content-of:${p}`),
    saveFile: vi.fn(async () => undefined),
    saveAll: vi.fn(async () => []),
  },
}));

const ROOT = "E:/Works/tex-presso/test_file/projects/multifile";
const MAIN = `${ROOT}/main.tex`;

describe("editorStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const project = useProjectStore();
    project.project = { root: ROOT } as never;
  });

  it("openFile：打开一次生成单标签", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    expect(editor.tabs.map((t) => t.path)).toEqual([MAIN]);
    expect(editor.activePath).toBe(MAIN);
  });

  it("openFile：`./` 绝对路径归一为已开标签路径，不重复开（回归：resolvePath 修复）", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    await editor.openFile(`${ROOT}/./main.tex`); // synctex 反向返回的格式
    expect(editor.tabs.map((t) => t.path)).toEqual([MAIN]); // 仍 1 个
    expect(editor.activePath).toBe(MAIN);
  });

  it("openFile：并发调用不重复开标签（去重竞态回归：await 前 some 判断不够）", async () => {
    const editor = useEditorStore();
    // 两个 openFile 在 await 前都通过了初始 some 判断；靠 await 后复检去重
    const p1 = editor.openFile(MAIN);
    const p2 = editor.openFile(MAIN);
    await Promise.all([p1, p2]);
    expect(editor.tabs.map((t) => t.path)).toEqual([MAIN]);
  });

  it("onFilesChanged：已打开且不脏 → 静默重载 buffer", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    editor.buffers.set(MAIN, "old");
    await editor.onFilesChanged([MAIN]);
    expect(editor.buffers.get(MAIN)).toBe(`content-of:${MAIN}`);
    expect(editor.externalConflict.has(MAIN)).toBe(false);
  });

  it("onFilesChanged：已打开且脏 → 保留本地 + 冲突标记（components 冲突对话框数据源）", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    editor.markDirty(MAIN, "local-edit");
    await editor.onFilesChanged([MAIN]);
    expect(editor.buffers.get(MAIN)).toBe("local-edit"); // 不覆盖本地
    expect(editor.externalConflict.has(MAIN)).toBe(true);
  });

  it("onFilesChanged：最近自保存（<2s）→ 忽略，不重载", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    editor.buffers.set(MAIN, "saved-content");
    editor.markSaved([MAIN]); // lastSaved = now
    await editor.onFilesChanged([MAIN]);
    expect(editor.buffers.get(MAIN)).toBe("saved-content"); // 未重载
  });

  it("onFilesChanged：未打开 → 忽略", async () => {
    const editor = useEditorStore();
    await editor.onFilesChanged([`${ROOT}/other.tex`]);
    expect(editor.tabs.length).toBe(0);
  });

  it("closeTab：移除标签并清理脏/缓冲", async () => {
    const editor = useEditorStore();
    await editor.openFile(MAIN);
    editor.markDirty(MAIN, "x");
    editor.closeTab(MAIN);
    expect(editor.tabs.length).toBe(0);
    expect(editor.dirty.has(MAIN)).toBe(false);
    expect(editor.buffers.has(MAIN)).toBe(false);
  });
});
