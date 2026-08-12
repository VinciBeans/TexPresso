<!-- PreviewPane（modules.md §9.4）：pdf.js 封装。
   滚动保持：重载前记录页码+滚动，加载后恢复（modules.md §5.2）。
   SyncTeX：高亮 overlay 绘制；点击反向定位。 -->
<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as pdfjsLib from "pdfjs-dist";
import { usePreviewStore } from "../stores/preview";
import { useSyncTex } from "../composables/useSyncTex";

const preview = usePreviewStore();
const { inverse } = useSyncTex();

pdfjsLib.GlobalWorkerOptions.workerSrc = "/pdfjs/pdf.worker.min.js";

const container = ref<HTMLElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);
const highlightBox = ref<HTMLElement | null>(null);

let doc: pdfjsLib.PDFDocumentProxy | null = null;
let loadingTask: pdfjsLib.PDFDocumentLoadingTask | null = null;
let currentPage = 1;
let scrollTop = 0;

/** 渲染指定页到 canvas。 */
async function renderPage(pageNum: number) {
  if (!doc || !canvas.value) return;
  const page = await doc.getPage(pageNum);
  const viewport = page.getViewport({ scale: 1.5 });
  const dpr = window.devicePixelRatio || 1;
  canvas.value.width = viewport.width * dpr;
  canvas.value.height = viewport.height * dpr;
  canvas.value.style.width = `${viewport.width}px`;
  canvas.value.style.height = `${viewport.height}px`;
  await page.render({ canvas: canvas.value, viewport }).promise;
}

/** 加载 PDF；带页码/滚动恢复。 */
async function load() {
  const path = preview.pdfPath;
  if (!path) return;
  const keepPage = currentPage;
  const keepScroll = scrollTop;
  try {
    const data = await fetch(convertFileSrc(path)).then((r) => r.arrayBuffer());
    loadingTask?.destroy().catch(() => {});
    loadingTask = pdfjsLib.getDocument({ data });
    doc = await loadingTask.promise;
    currentPage = Math.min(keepPage, doc.numPages);
    await renderPage(currentPage);
    if (container.value) container.value.scrollTop = keepScroll;
    // 恢复后再渲染一次（字体加载可能改变布局）
    await renderPage(currentPage);
  } catch (e) {
    console.error("PDF 加载失败：", e);
  }
}

// pdf-updated → 重载
watch(
  () => preview.reloadKey,
  () => {
    if (container.value) scrollTop = container.value.scrollTop;
    load();
  }
);

// 高亮 overlay（SyncTeX 正向）
watch(
  () => preview.highlight,
  async (h) => {
    const box = highlightBox.value;
    const c = canvas.value;
    if (!h || !box || !c || !doc) {
      if (box) box.style.display = "none";
      return;
    }
    if (h.page !== currentPage) {
      currentPage = h.page;
      await renderPage(h.page);
    }
    const page = await doc.getPage(h.page);
    const viewport = page.getViewport({ scale: 1.5 });
    const pt = viewport.convertToViewportPoint(h.x, h.y);
    box.style.display = "block";
    box.style.left = `${pt[0] - 25}px`;
    box.style.top = `${pt[1] - 10}px`;
    box.style.width = "50px";
    box.style.height = "20px";
  }
);

// 点击 → SyncTeX 反向（modules.md §5.3）
async function onCanvasClick(e: MouseEvent) {
  if (!doc || !canvas.value) return;
  const rect = canvas.value.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  const page = await doc.getPage(currentPage);
  const viewport = page.getViewport({ scale: 1.5 });
  const pdfPt = viewport.convertToPdfPoint(x, y);
  await inverse(currentPage, pdfPt[0], pdfPt[1]);
}

function onScroll(e: Event) {
  scrollTop = (e.target as HTMLElement).scrollTop;
}

onMounted(() => {
  if (preview.pdfPath) load();
});

onBeforeUnmount(() => {
  loadingTask?.destroy().catch(() => {});
  loadingTask = null;
  doc = null;
});
</script>

<template>
  <div ref="container" class="preview-pane" @scroll.passive="onScroll">
    <div v-if="!preview.pdfPath" class="empty">编译成功后在此预览 PDF</div>
    <div class="page-wrap">
      <canvas ref="canvas" @click="onCanvasClick" />
      <div ref="highlightBox" class="highlight" />
    </div>
  </div>
</template>

<style scoped>
.preview-pane { height: 100%; overflow: auto; background: #262626; }
.empty { color: #888; padding: 16px; text-align: center; margin-top: 40px; }
.page-wrap { position: relative; margin: 8px auto; width: fit-content; box-shadow: 0 2px 8px rgba(0,0,0,0.5); }
.highlight { display: none; position: absolute; background: rgba(255, 200, 60, 0.35); border: 1px solid #e0a800; pointer-events: none; }
</style>
