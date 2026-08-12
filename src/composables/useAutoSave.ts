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
    try {
      await ipc.saveAll(files);
      editor.markSaved(files.map((f) => f.path));
    } catch (e) {
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
