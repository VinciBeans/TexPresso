// pdf.js worker 拷贝：`?worker&inline` 在 Tauri 下有 MIME/生命周期坑（modules.md §6），
// 生产验证过的做法是拷到 public/ 用 .js 扩展名（Graphium PR #548）。
// 运行：npm run copy:pdf-worker（predev/prebuild 自动执行）
import { cpSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const src = join(root, "node_modules/pdfjs-dist/build/pdf.worker.min.mjs");
const dstDir = join(root, "public/pdfjs");
const dst = join(dstDir, "pdf.worker.min.js");

mkdirSync(dstDir, { recursive: true });
cpSync(src, dst);
console.log(`[copy-pdf-worker] ${dst}`);
