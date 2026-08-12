// previewStore（modules.md §9.2）：PDF 文档、滚动位置、SyncTeX 高亮。
import { defineStore } from "pinia";
import { ref } from "vue";

export interface Highlight {
  page: number;
  x: number;
  y: number;
}

export const usePreviewStore = defineStore("preview", () => {
  const pdfPath = ref<string | null>(null);
  /** 每次 pdf-updated 递增，触发 PreviewPane 重载（modules.md §5.2）。 */
  const reloadKey = ref(0);
  const highlight = ref<Highlight | null>(null);

  function onPdfUpdated(path: string) {
    pdfPath.value = path;
    reloadKey.value++;
  }

  function setHighlight(h: Highlight | null) {
    highlight.value = h;
  }

  return { pdfPath, reloadKey, highlight, onPdfUpdated, setHighlight };
});
