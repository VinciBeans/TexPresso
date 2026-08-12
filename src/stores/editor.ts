// editorStore（modules.md §9.2）：打开文件、脏标志、活动标签、外部修改处理。
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { ipc } from "../services/ipc";
import { useProjectStore } from "./project";

export interface OpenFile {
  path: string; // 项目内绝对路径
  name: string;
}

export const useEditorStore = defineStore("editor", () => {
  const tabs = ref<OpenFile[]>([]);
  const activePath = ref<string | null>(null);
  /** 脏缓冲（内容已在内存，未落盘）。 */
  const dirty = ref<Set<string>>(new Set());
  /** 最新内容（EditorPane 变更时写入；自动保存时读出）。 */
  const buffers = ref<Map<string, string>>(new Map());
  /** 自保存过滤（modules.md §5.5）：保存时刻记录，files-changed 里近期的路径视为自己写的。 */
  const lastSaved = ref<Map<string, number>>(new Map());
  /** 外部修改冲突提示（打开且脏 → 保留本地）。 */
  const externalConflict = ref<Set<string>>(new Set());

  const project = useProjectStore();
  const activeTab = computed(() => tabs.value.find((t) => t.path === activePath.value) ?? null);

  async function openFile(rawPath: string, revealLine?: number) {
    const path = project.resolvePath(rawPath);
    if (!tabs.value.some((t) => t.path === path)) {
      const content = await ipc.readFile(path);
      tabs.value.push({ path, name: path.split("/").pop() ?? path });
      buffers.value.set(path, content);
    }
    activePath.value = path;
    if (revealLine) {
      // 让 EditorPane 感知定位请求
      pendingReveal.value = { path, line: revealLine };
    }
  }

  function closeTab(path: string) {
    const i = tabs.value.findIndex((t) => t.path === path);
    if (i < 0) return;
    tabs.value.splice(i, 1);
    dirty.value.delete(path);
    buffers.value.delete(path);
    if (activePath.value === path) {
      activePath.value = tabs.value[i]?.path ?? tabs.value[i - 1]?.path ?? null;
    }
  }

  function markDirty(path: string, content: string) {
    buffers.value.set(path, content);
    dirty.value.add(path);
    externalConflict.value.delete(path);
  }

  /** 自动保存成功后调用：清脏 + 记录时间（自保存过滤）。 */
  function markSaved(paths: string[]) {
    const now = Date.now();
    for (const p of paths) {
      dirty.value.delete(p);
      lastSaved.value.set(p, now);
    }
  }

  /** 乐观记录自保存（修竞态：后端写盘 → notify → files-changed 可能先于 saveAll 返回到达，
   *  此时若 lastSaved 未记录会误判外部修改）。失败时回滚。 */
  function markSaving(paths: string[]) {
    const now = Date.now();
    for (const p of paths) lastSaved.value.set(p, now);
  }

  function rollbackSaving(paths: string[]) {
    for (const p of paths) lastSaved.value.delete(p);
  }

  /** files-changed 处理（modules.md §5.5 算法）：
   * 1. 自己刚保存的（<2s）→ 忽略；
   * 2. 打开且不脏 → 静默重载；
   * 3. 打开且脏 → 保留本地 + 冲突标记；
   * 4. 未打开 → 忽略（文件树自己刷新）。 */
  async function onFilesChanged(paths: string[]) {
    const now = Date.now();
    for (const raw of paths) {
      const path = project.resolvePath(raw);
      if (!tabs.value.some((t) => t.path === path)) continue;
      const savedAt = lastSaved.value.get(path);
      if (savedAt !== undefined && now - savedAt < 2000) {
        lastSaved.value.delete(path);
        continue;
      }
      if (dirty.value.has(path)) {
        externalConflict.value.add(path);
        continue;
      }
      try {
        const content = await ipc.readFile(path);
        buffers.value.set(path, content);
      } catch {
        // 文件可能被删除
      }
    }
  }

  /** 外部修改冲突确认：放弃本地、采用磁盘（v1 状态栏提供按钮）。 */
  async function acceptExternal(path: string) {
    const content = await ipc.readFile(path);
    buffers.value.set(path, content);
    dirty.value.delete(path);
    externalConflict.value.delete(path);
  }

  const pendingReveal = ref<{ path: string; line: number } | null>(null);
  function consumeReveal() {
    const r = pendingReveal.value;
    pendingReveal.value = null;
    return r;
  }

  return {
    tabs, activePath, dirty, buffers, lastSaved, externalConflict, activeTab,
    openFile, closeTab, markDirty, markSaved, markSaving, rollbackSaving,
    onFilesChanged, acceptExternal,
    pendingReveal, consumeReveal,
  };
});
