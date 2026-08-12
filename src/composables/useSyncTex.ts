// useSyncTex（modules.md §9.3）：双向定位编排（modules.md §5.3）。
import { ipc } from "../services/ipc";
import { useEditorStore } from "../stores/editor";
import { usePreviewStore } from "../stores/preview";

export function useSyncTex() {
  const preview = usePreviewStore();
  const editor = useEditorStore();

  /** 正向：源码 Ctrl+点击 → PDF 高亮。 */
  async function forward(file: string, line: number, column: number) {
    try {
      const target = await ipc.synctexForward(file, line, column);
      if (target.x !== null && target.y !== null) {
        preview.setHighlight({ page: target.page, x: target.x, y: target.y });
      }
    } catch (e) {
      console.error("SyncTeX 正向定位失败：", e);
    }
  }

  /** 反向：PDF 点击 → 源码跳转。 */
  async function inverse(page: number, x: number, y: number) {
    try {
      const src = await ipc.synctexInverse(page, x, y);
      await editor.openFile(src.file, src.line);
    } catch (e) {
      console.error("SyncTeX 反向定位失败：", e);
    }
  }

  return { forward, inverse };
}
