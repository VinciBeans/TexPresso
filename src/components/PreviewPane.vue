<!-- PreviewPane（modules.md §9.4）：pdf.js 封装。
   连续分页展示：所有页面纵向排列，视口感知按需渲染（近处渲染、远处释放）。
   滚动保持：重载前记录滚动，加载后恢复（modules.md §5.2）。
   SyncTeX：高亮 overlay 绘制（跟随对应页）；点击反向定位。
   缩放：工具条 − / % / + / 适应宽度 + Ctrl+滚轮；高亮与反向定位跟随当前缩放。 -->
<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import * as pdfjsLib from "pdfjs-dist";
import { usePreviewStore } from "../stores/preview";
import { useSyncTex } from "../composables/useSyncTex";

const preview = usePreviewStore();
const { inverse } = useSyncTex();

pdfjsLib.GlobalWorkerOptions.workerSrc = "/pdfjs/pdf.worker.min.js";

const container = ref<HTMLElement | null>(null);

const scale = ref(1.5);
const SCALE_MIN = 0.5;
const SCALE_MAX = 4;
const RENDER_MARGIN = 800; // 视口外预渲染/释放的边距（px）

let doc: pdfjsLib.PDFDocumentProxy | null = null;
let loadingTask: pdfjsLib.PDFDocumentLoadingTask | null = null;
/** 总页数与当前页（响应式：页码指示器展示用）。 */
const numPages = ref(0);
const currentPageIdx = ref(1);
let scrollTop = 0;
/** 加载序号：并发 pdf-updated 时旧加载被取代（destroy 引发的 aborted 忽略）。 */
let loadSeq = 0;
/** 是否已做过首次“适应宽度”：只有首次加载自动 fit，之后保留用户手动缩放。 */
let fittedOnce = false;

// ---------- 分页 DOM 引用（模板 ref 回调） ----------
const pageEls: (HTMLElement | null)[] = [];
const pageCanvases: (HTMLCanvasElement | null)[] = [];
const pageHighlights: (HTMLElement | null)[] = [];
/** 已渲染页：页码 → 渲染时的 scale（换缩放后需重绘）。 */
const renderedScale = new Map<number, number>();
/** 在途渲染任务：页码 → RenderTask。pdf.js 不允许同一 canvas 并发渲染——
 *  编译重载/缩放/IO+预渲染双触发时，必须先取消在途任务再启动新的，
 *  否则两个 RenderTask 交错写同一个 2D context 会把页面画黑/花屏。 */
const renderTaskByPage = new Map<number, pdfjsLib.RenderTask>();
/** 每页串行渲染链：同页渲染严格 FIFO，杜绝同 canvas 交叉。 */
const renderChainByPage = new Map<number, Promise<void>>();
/** canvas 代次：文档/缩放切换时 +1，强制 v-for 重建 canvas DOM（全新 2D context，
 *  物理上排除被取消渲染残留 transform 状态导致的“黑底/文字反转”问题）。 */
const canvasEpoch = ref(0);

/** 取消某页在途渲染并等其完全结束（pdf.js 要求：cancel 后必须等 promise settle
 *  才能开始新的 render，否则两个任务仍会交错写同一个 2D context —— 页面画黑）。 */
async function cancelRender(pageNum: number) {
  const t = renderTaskByPage.get(pageNum);
  if (!t) return;
  renderTaskByPage.delete(pageNum);
  try {
    t.cancel();
  } catch {
    /* 已完成/已取消 */
  }
  try {
    await t.promise;
  } catch {
    /* 被取消：正常路径 */
  }
}

/** 取消全部在途渲染并等待结束（文档重载时调用，避免旧 doc 的渲染继续写画布）。 */
async function cancelAllRenders() {
  for (const pageNum of [...renderTaskByPage.keys()]) {
    await cancelRender(pageNum);
  }
}

/** synctex 坐标（顶部起算）→ pdf.js PDF 坐标（底部起算）。 */
async function toPdfPoint(page: pdfjsLib.PDFPageProxy, x: number, yTop: number): Promise<[number, number]> {
  const vp1 = page.getViewport({ scale: 1 });
  return [x, vp1.height - yTop];
}

/** 渲染指定页：入队到该页串行链（严格 FIFO），避免同 canvas 并发渲染。 */
function renderPage(pageNum: number) {
  if (!doc || !pageCanvases[pageNum]) return;
  if (renderedScale.get(pageNum) === scale.value) return; // 已是最新
  const prev = renderChainByPage.get(pageNum) ?? Promise.resolve();
  const next = prev
    .then(() => doRenderPage(pageNum))
    .catch(() => {
      /* 上一环失败不影响后续 */
    });
  renderChainByPage.set(pageNum, next);
}

/** 实际渲染（串行链内执行）：取消并等待旧任务结束后再画。 */
async function doRenderPage(pageNum: number) {
  if (!doc || !pageCanvases[pageNum]) return;
  if (renderedScale.get(pageNum) === scale.value) return; // 已是最新
  await cancelRender(pageNum);
  const page = await doc.getPage(pageNum);
  const canvas = pageCanvases[pageNum]!;
  const viewport = page.getViewport({ scale: scale.value });
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.floor(viewport.width * dpr);
  canvas.height = Math.floor(viewport.height * dpr);
  canvas.style.width = `${viewport.width}px`;
  canvas.style.height = `${viewport.height}px`;
  // 关键：canvas 物理像素 = viewport × dpr，必须传 dpr 变换，否则内容按 1x 绘制，
  // 只占 canvas 左上角 1/dpr（高 DPI 下内容偏左上、缩水）。
  const transform = dpr !== 1 ? [dpr, 0, 0, dpr, 0, 0] : undefined;
  const task = page.render({ canvas, viewport, transform });
  renderTaskByPage.set(pageNum, task);
  try {
    await task.promise;
    // 完成时若未被更新任务替换，记录已完成
    if (renderTaskByPage.get(pageNum) === task) {
      renderTaskByPage.delete(pageNum);
      renderedScale.set(pageNum, scale.value);
    }
  } catch {
    // 被取消或被取代：不留置 renderedScale（等待下次渲染）
    if (renderTaskByPage.get(pageNum) === task) renderTaskByPage.delete(pageNum);
  }
}

/** 释放远处页面：清空 canvas 尺寸，等待滚回时重绘。 */
function releasePage(pageNum: number) {
  if (!pageCanvases[pageNum]) return;
  void cancelRender(pageNum);
  const canvas = pageCanvases[pageNum]!;
  canvas.width = 0;
  canvas.height = 0;
  renderedScale.delete(pageNum);
}

/** 渲染视口附近的所有页（含边距），远离视口的释放。 */
function renderNearViewport() {
  if (!doc || !numPages.value || !container.value) return;
  const rect = container.value.getBoundingClientRect();
  for (let n = 1; n <= numPages.value; n++) {
    const el = pageEls[n];
    if (!el) continue;
    const r = el.getBoundingClientRect();
    const near = r.bottom >= rect.top - RENDER_MARGIN && r.top <= rect.bottom + RENDER_MARGIN;
    if (near) {
      void renderPage(n);
    } else if (renderedScale.has(n)) {
      releasePage(n);
    }
  }
}

/** IntersectionObserver 版的按需渲染（滚动时触发）。 */
let io: IntersectionObserver | null = null;
function setupObserver() {
  io?.disconnect();
  io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        const n = Number((e.target as HTMLElement).dataset.page);
        if (e.isIntersecting) void renderPage(n);
        else if (n > 0 && renderedScale.has(n)) releasePage(n);
      }
    },
    { root: container.value, rootMargin: `${RENDER_MARGIN}px 0px` }
  );
  for (let n = 1; n <= numPages.value; n++) {
    const el = pageEls[n];
    if (el) io.observe(el);
  }
}

/** 更新当前页（视口中心所在页），用于 fitWidth/高亮等。 */
function updateCurrentPage() {
  if (!doc || !numPages.value || !container.value) return;
  const rect = container.value.getBoundingClientRect();
  const mid = rect.top + rect.height / 2;
  let best = 1;
  let bestDist = Infinity;
  for (let n = 1; n <= numPages.value; n++) {
    const el = pageEls[n];
    if (!el) continue;
    const r = el.getBoundingClientRect();
    const d = Math.abs((r.top + r.bottom) / 2 - mid);
    if (d < bestDist) {
      bestDist = d;
      best = n;
    }
  }
  currentPageIdx.value = best;
}

/** 设置缩放：0.5–4，重绘视口页并尽量保持滚动位置。 */
async function setScale(next: number) {
  const s = Math.min(SCALE_MAX, Math.max(SCALE_MIN, Math.round(next * 100) / 100));
  if (Math.abs(s - scale.value) < 0.005) return;
  const keep = container.value?.scrollTop ?? 0;
  scale.value = s;
  canvasEpoch.value++; // 重建 canvas：全新 2D context，排除残留状态
  renderedScale.clear();
  await cancelAllRenders();
  if (doc) {
    await nextTick();
    renderNearViewport();
    await nextTick();
    if (container.value) container.value.scrollTop = keep;
    updateCurrentPage();
  }
}

/** 适应宽度：按容器宽度计算缩放。 */
async function fitWidth() {
  if (!doc || !container.value) return;
  const page = await doc.getPage(currentPageIdx.value);
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

/** 加载 PDF；带滚动恢复。 */
async function load() {
  const mySeq = ++loadSeq;
  const path = preview.pdfPath;
  if (!path) return;
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
    // 切换文档：取消并等待所有在途渲染结束（旧 doc 的 RenderTask 不允许继续写画布）
    await cancelAllRenders();
    canvasEpoch.value++; // 重建 canvas：全新 2D context，排除残留状态
    numPages.value = doc.numPages;
    renderedScale.clear();
    currentPageIdx.value = Math.min(currentPageIdx.value, numPages.value);
    await nextTick(); // 等 v-for 页面节点挂载
    // 首次加载自动“适应宽度”：默认 150% 固定值在窄面板里会溢出，
    // 页面只能从左上角排起（无剩余空间时 margin:auto 无法居中）。之后保留用户手动缩放。
    if (!fittedOnce) {
      fittedOnce = true;
      await fitWidth();
    }
    setupObserver();
    renderNearViewport();
    if (container.value) container.value.scrollTop = keepScroll;
    updateCurrentPage();
    // 恢复后再渲染一次（字体加载可能改变布局）
    await nextTick();
    renderNearViewport();
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

// 高亮 overlay（SyncTeX 正向）：放到对应页的 wrap 内
watch(
  () => preview.highlight,
  async (h) => {
    for (const box of pageHighlights) if (box) box.style.display = "none";
    if (!h || !doc) return;
    if (h.page < 1 || h.page > numPages.value) return;
    const box = pageHighlights[h.page];
    const canvas = pageCanvases[h.page];
    if (!box || !canvas) return;
    // 保证该页已渲染（若远离视口则临时渲染）
    if (renderedScale.get(h.page) !== scale.value) {
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
    // 滚动阅读区，使高亮点尽量居中（Ctrl+点击源码后跳转可见）
    const pageEl = pageEls[h.page];
    const c = container.value;
    if (pageEl && c) {
      const pageRect = pageEl.getBoundingClientRect();
      const cRect = c.getBoundingClientRect();
      const targetY = pageRect.top + pt[1] - 10 + 10; // 高亮中心（页面坐标系）
      const targetX = pageRect.left + pt[0]; // 高亮中心（页面坐标系）
      c.scrollTo({
        top: c.scrollTop + (targetY - cRect.top) - cRect.height / 2,
        left: c.scrollLeft + (targetX - cRect.left) - cRect.width / 2,
        behavior: "smooth",
      });
    }
  }
);

// 点击 → SyncTeX 反向（modules.md §5.3）
async function onCanvasClick(pageNum: number, e: MouseEvent) {
  if (!doc) return;
  const canvas = pageCanvases[pageNum];
  if (!canvas) return;
  const rect = canvas.getBoundingClientRect();
  const x = e.clientX - rect.left;
  const y = e.clientY - rect.top;
  const page = await doc.getPage(pageNum);
  const viewport = page.getViewport({ scale: scale.value });
  const clickPt = viewport.convertToPdfPoint(x, y);
  // pdf.js 坐标底部起算 → synctex 顶部起算（翻转）
  const vp1 = page.getViewport({ scale: 1 });
  await inverse(pageNum, clickPt[0], vp1.height - clickPt[1]);
}

function onScroll(e: Event) {
  scrollTop = (e.target as HTMLElement).scrollTop;
  updateCurrentPage();
}

function setPageEl(n: number, el: HTMLElement | null) {
  pageEls[n] = el;
}
function setCanvasEl(n: number, el: HTMLCanvasElement | null) {
  pageCanvases[n] = el;
}
function setHighlightEl(n: number, el: HTMLElement | null) {
  pageHighlights[n] = el;
}

onMounted(() => {
  if (preview.pdfPath) load();
});

onBeforeUnmount(() => {
  io?.disconnect();
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
      <span class="page-indicator" v-if="numPages > 0">{{ currentPageIdx }} / {{ numPages }}</span>
    </div>
    <div ref="container" class="preview-pane" @scroll.passive="onScroll" @wheel="onWheel">
      <div v-if="!preview.pdfPath" class="empty">
        <span class="empty-icon">📕</span>
        <span class="empty-title">PDF 在这里等你</span>
        <span class="empty-hint">写好 main.tex，点「编译」就能提前看到成品</span>
      </div>
      <div class="pages" v-if="numPages > 0">
        <div
          v-for="n in numPages"
          :key="`${n}-${canvasEpoch}`"
          class="page-wrap"
          :data-page="n"
          :ref="(el) => setPageEl(n, el as HTMLElement)"
        >
          <canvas :ref="(el) => setCanvasEl(n, el as HTMLCanvasElement)" @click="onCanvasClick(n, $event)" />
          <div class="highlight" :ref="(el) => setHighlightEl(n, el as HTMLElement)" />
        </div>
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
.page-indicator {
  margin-left: auto;
  font-family: var(--mono);
  font-size: 11.5px;
  color: var(--ink-faint);
}

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
.pages { padding: 18px 16px 40px; }
.page-wrap {
  position: relative;
  margin: 0 auto 18px;
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
