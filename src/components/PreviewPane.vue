<!-- PreviewPane（modules.md §9.4）：pdf.js 封装。
   滚动保持：重载前记录页码+滚动，加载后恢复（modules.md §5.2）。
   SyncTeX：高亮 overlay 绘制；点击反向定位。
   缩放：工具条 − / % / + / 适应宽度 + Ctrl+滚轮；高亮与反向定位跟随当前缩放。 -->
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

const scale = ref(1.5);
const SCALE_MIN = 0.5;
const SCALE_MAX = 4;

let doc: pdfjsLib.PDFDocumentProxy | null = null;
let loadingTask: pdfjsLib.PDFDocumentLoadingTask | null = null;
let currentPage = 1;
let scrollTop = 0;
/** 加载序号：并发 pdf-updated 时旧加载被取代（destroy 引发的 aborted 忽略）。 */
let loadSeq = 0;
/** 是否已做过首次“适应宽度”：只有首次加载自动 fit，之后保留用户手动缩放。 */
let fittedOnce = false;

/** synctex 坐标（顶部起算）→ pdf.js PDF 坐标（底部起算）。 */
async function toPdfPoint(page: pdfjsLib.PDFPageProxy, x: number, yTop: number): Promise<[number, number]> {
  const vp1 = page.getViewport({ scale: 1 });
  return [x, vp1.height - yTop];
}

/** 渲染指定页到 canvas（按当前 scale，含 devicePixelRatio 变换）。 */
async function renderPage(pageNum: number) {
  if (!doc || !canvas.value) return;
  const page = await doc.getPage(pageNum);
  const viewport = page.getViewport({ scale: scale.value });
  const dpr = window.devicePixelRatio || 1;
  canvas.value.width = Math.floor(viewport.width * dpr);
  canvas.value.height = Math.floor(viewport.height * dpr);
  canvas.value.style.width = `${viewport.width}px`;
  canvas.value.style.height = `${viewport.height}px`;
  // 关键：canvas 物理像素 = viewport × dpr，必须传 dpr 变换，否则内容按 1x 绘制，
  // 只占 canvas 左上角 1/dpr（高 DPI 下内容偏左上、缩水）。
  const transform = dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined;
  await page.render({ canvas: canvas.value, viewport, transform }).promise;
}

/** 设置缩放：0.5–4，重新渲染当前页并尽量保持滚动位置。 */
async function setScale(next: number) {
  const s = Math.min(SCALE_MAX, Math.max(SCALE_MIN, Math.round(next * 100) / 100));
  if (Math.abs(s - scale.value) < 0.005) return;
  const keep = container.value?.scrollTop ?? 0;
  scale.value = s;
  if (doc) {
    await renderPage(currentPage);
    if (container.value) container.value.scrollTop = keep;
  }
}

/** 适应宽度：按容器宽度计算缩放。 */
async function fitWidth() {
  if (!doc || !container.value) return;
  const page = await doc.getPage(currentPage);
  const vp1 = page.getViewport({ scale: 1 });
  const avail = Math.max(200, container.value.clientWidth - 40);
  await setScale(avail / vp1.width);
}

/** Ctrl+滚轮缩放（拦截 webview 页面缩放）。 */
function onWheel(e: WheelEvent) {
  if (!e.ctrlKey) return;
  e.preventDefault();
  void setScale(scale.value + (e.deltaY < 0 ? 0.25 : -0.25));
}

/** 加载 PDF；带页码/滚动恢复。 */
async function load() {
  const mySeq = ++loadSeq;
  const path = preview.pdfPath;
  if (!path) return;
  const keepPage = currentPage;
  const keepScroll = scrollTop;
  try {
    const resp = await fetch(convertFileSrc(path));
    const data = await resp.arrayBuffer();
    loadingTask?.destroy().catch(() => {});
    // 中文 PDF 的 CMap 字体映射（缺失报 cMapUrl 错误，文字渲染失败）；
    // 必须绝对 URL：worker 内相对路径会基于 worker 脚本 URL（asset 协议）解析导致 404
    loadingTask = pdfjsLib.getDocument({
      data,
      cMapUrl: new URL("/pdfjs/cmaps/", document.baseURI).href,
      cMapPacked: true,
    });
    doc = await loadingTask.promise;
    if (mySeq !== loadSeq) return; // 已被更新的加载取代
    currentPage = Math.min(keepPage, doc.numPages);
    // 首次加载自动“适应宽度”：默认 150% 固定值在窄面板里会溢出，
    // 页面只能从左上角排起（无剩余空间时 margin:auto 无法居中）。之后保留用户手动缩放。
    if (!fittedOnce) {
      fittedOnce = true;
      await fitWidth();
    } else {
      await renderPage(currentPage);
    }
    if (mySeq !== loadSeq) return;
    if (container.value) container.value.scrollTop = keepScroll;
    // 恢复后再渲染一次（字体加载可能改变布局）
    await renderPage(currentPage);
  } catch (e) {
    if (mySeq !== loadSeq) return; // 被取代的加载（destroy 引发 aborted）忽略
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
    // synctex 的 y 从页面顶部起算，pdf.js 的坐标从底部起算——翻转（2026-08 实测）
    const pdfPt = await toPdfPoint(page, h.x, h.y);
    const viewport = page.getViewport({ scale: scale.value });
    const pt = viewport.convertToViewportPoint(pdfPt[0], pdfPt[1]);
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
  const viewport = page.getViewport({ scale: scale.value });
  const clickPt = viewport.convertToPdfPoint(x, y);
  // pdf.js 坐标底部起算 → synctex 顶部起算（翻转）
  const vp1 = page.getViewport({ scale: 1 });
  await inverse(currentPage, clickPt[0], vp1.height - clickPt[1]);
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
  <div class="preview-root">
    <div class="preview-toolbar">
      <button
        class="zoom-btn"
        title="缩小（Ctrl+滚轮）"
        :disabled="scale <= SCALE_MIN"
        @click="setScale(scale - 0.25)"
      >−</button>
      <span class="zoom-pct" title="当前缩放比">{{ Math.round(scale * 100) }}%</span>
      <button
        class="zoom-btn"
        title="放大（Ctrl+滚轮）"
        :disabled="scale >= SCALE_MAX"
        @click="setScale(scale + 0.25)"
      >+</button>
      <span class="toolbar-sep" />
      <button class="zoom-btn fit" title="适应宽度" @click="fitWidth">⤢ 适应宽度</button>
    </div>
    <div ref="container" class="preview-pane" @scroll.passive="onScroll" @wheel="onWheel">
      <div v-if="!preview.pdfPath" class="empty">
        <span class="empty-icon">📕</span>
        <span class="empty-title">PDF 在这里等你</span>
        <span class="empty-hint">写好 main.tex，点「编译」就能提前看到成品</span>
      </div>
      <div class="page-wrap">
        <canvas ref="canvas" @click="onCanvasClick" />
        <div ref="highlightBox" class="highlight" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.preview-root { display: flex; flex-direction: column; height: 100%; }
.preview-toolbar {
  display: flex; align-items: center; gap: 6px;
  flex: 0 0 auto;
  height: 32px; padding: 0 10px;
  background: var(--card);
  border-bottom: 1.5px solid var(--line);
}
.zoom-btn {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 26px; height: 22px;
  padding: 0 8px;
  background: var(--card);
  border: 1.5px solid var(--line);
  border-radius: 6px;
  color: var(--ink);
  font-size: 13px; font-weight: 600;
  cursor: pointer;
  transition: background 0.12s, border-color 0.12s, color 0.12s;
}
.zoom-btn:hover:not(:disabled) { border-color: var(--blueberry); color: var(--blueberry); background: rgba(93, 95, 239, 0.06); }
.zoom-btn:disabled { opacity: 0.4; cursor: default; }
.zoom-btn.fit { font-size: 11.5px; font-weight: 550; }
.zoom-pct {
  min-width: 44px;
  text-align: center;
  font-family: var(--mono);
  font-size: 11.5px;
  color: var(--ink-dim);
}
.toolbar-sep { width: 1.5px; height: 14px; background: var(--line-soft); margin: 0 2px; }

/* 点阵网格纸：呼应“写作的方格纸” */
.preview-pane {
  flex: 1 1 auto;
  min-height: 0;
  overflow: auto;
  background:
    radial-gradient(rgba(93, 95, 239, 0.10) 1.2px, transparent 1.2px) 0 0 / 20px 20px,
    var(--paper);
}
.empty { padding: 56px 24px; color: var(--ink-faint); text-align: center; }
.empty-icon { font-size: 38px; display: block; }
.empty-title { display: block; margin-top: 12px; font-size: 14px; font-weight: 700; color: var(--ink-dim); }
.empty-hint { display: block; margin-top: 6px; font-size: 12px; }
.page-wrap {
  position: relative;
  margin: 18px auto;
  width: fit-content;
  border-radius: 3px;
  box-shadow: 0 6px 28px rgba(43, 36, 56, 0.18), 0 1px 4px rgba(43, 36, 56, 0.12);
  outline: 1px solid var(--line);
}
.highlight {
  display: none;
  position: absolute;
  background: rgba(255, 181, 74, 0.4);
  border: 1.5px solid #e8a72c;
  pointer-events: none;
}
</style>
