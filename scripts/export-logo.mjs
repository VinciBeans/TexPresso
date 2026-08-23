import { Resvg } from "@resvg/resvg-js";
import { mkdirSync } from "node:fs";

mkdirSync("docs/logo-export", { recursive: true });

function exportSvg(svgPath, pngPath, width) {
  const svg = readFileSyncSafe(svgPath);
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: width },
    background: "transparent",
  });
  const png = resvg.render().asPng();
  writeFileSyncSafe(pngPath, png);
  console.log("✓", pngPath, png.length, "bytes");
}

import { readFileSync, writeFileSync } from "node:fs";
const readFileSyncSafe = (p) => readFileSync(p);
const writeFileSyncSafe = (p, b) => writeFileSync(p, b);

exportSvg("public/logo.svg", "docs/logo-export/mark-1024.png", 1024);
exportSvg("public/logo.svg", "docs/logo-export/mark-512.png", 512);
exportSvg("public/app-icon.svg", "docs/logo-export/tile-1024.png", 1024);
exportSvg("public/logo-full.svg", "docs/logo-export/lockup-1600w.png", 1600);
exportSvg("public/logo-mono.svg", "docs/logo-export/mark-mono-1024.png", 1024);
