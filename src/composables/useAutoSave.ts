// useAutoSave（modules.md §9.3）：防抖保存算法。
// 信息局部性：计时器只存在于 composable 内，不出组件。
import { onBeforeUnmount } from "vue";
import { ipc } from "../services/ipc";
import { useEditorStore } from "../stores/editor";
import { useSettingsStore } from "../stores/settings";

export function useAutoSave() {
  const editor = useEditorStore();
  const settings = useSettingsStore();
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** 编辑器内容变化时调用：重置防抖计时器。 */
  function schedule() {
    clearTimeout(timer);
    const debounce = settings.settings?.compile.debounce_ms ?? 500;
    timer = setTimeout(run, debounce);
  }

  /** 到点：保存全部脏文件（modules.md §5.1 触发链），成功才清脏。 */
  async function run() {
    const paths = [...editor.dirty];
    if (paths.length === 0) return;
    const files = paths
      .map((p) => ({ path: p, content: editor.buffers.get(p) ?? "" }))
      .filter((f) => editor.buffers.has(f.path));
    if (files.length === 0) return;
    // 乐观记录（竞态修复：files-changed 可能先于 saveAll 返回到达）
    editor.markSaving(files.map((f) => f.path));
    try {
      await ipc.saveAll(files);
      // 陈旧保存竞态：保存期间用户可能继续输入，buffer 已变为新内容。
      // 只有「缓冲区仍等于已保存内容」的路径才清脏；否则保持 dirty（markDirty 已重新加入），
      // 并重排下次保存，避免关闭时 flush() 因 dirty 为空而丢失最新输入。
      // lastSaved 已由 markSaving 设置，自保存过滤（files-changed 自我写判定）不受影响。
      const clean = files.filter((f) => (editor.buffers.get(f.path) ?? "") === f.content);
      editor.markSaved(clean.map((f) => f.path));
      if (clean.length < files.length) schedule();
    } catch (e) {
      editor.rollbackSaving(files.map((f) => f.path));
      console.error("自动保存失败：", e);
    }
  }

  /** 立即保存（切标签/关窗口前调用）。 */
  async function flush() {
    clearTimeout(timer);
    await run();
  }

  onBeforeUnmount(() => clearTimeout(timer));

  return { schedule, flush };
}
