// pdf.js 资源拷贝：`?worker&inline` 在 Tauri 下有 MIME/生命周期坑（modules.md §6），
// 生产验证过的做法是拷到 public/ 用 .js 扩展名（Graphium PR #548）。
// 运行：npm run copy:pdf-worker（predev/prebuild 自动执行）
import { cpSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const pdfjsDist = join(root, "node_modules/pdfjs-dist");
const dstDir = join(root, "public/pdfjs");

// 1. worker：`.js` 扩展名规避 Tauri asset 协议对 .mjs 的 MIME 拒载
mkdirSync(dstDir, { recursive: true });
cpSync(join(pdfjsDist, "build/pdf.worker.min.mjs"), join(dstDir, "pdf.worker.min.js"));

// 2. cmaps：中文 PDF 的字体映射（缺了会报 cMapUrl 缺失，文字渲染失败）
mkdirSync(join(dstDir, "cmaps"), { recursive: true });
cpSync(join(pdfjsDist, "cmaps"), join(dstDir, "cmaps"), { recursive: true });

console.log(`[copy-pdf-worker] worker + cmaps → ${dstDir}`);
