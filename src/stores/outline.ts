// outlineStore（modules.md §9.2）：LaTeX 文档结构树（大纲）。
// 数据来源：跟随根文件（root_file）的 \include / \input 图，解析各 .tex 的
//   \part/\chapter/\section/... 结构命令，按文档顺序生成嵌套标题树。
// 点击大纲项 → editor.openFile(file, line) 揭示源码 + SyncTeX 正向 → 高亮/居中 PDF 对应页。
// 刷新时机：项目打开、编译成功、结构变化（files-changed structural）——由 App/events 触发。
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { useEditorStore } from "./editor";
import { useProjectStore, normalizePath } from "./project";
import { useSyncTex } from "../composables/useSyncTex";
import { ipc } from "../services/ipc";

export interface OutlineNode {
  title: string;
  shortTitle?: string;
  /** 结构层级：0=part,1=chapter,2=section,3=subsection,4=subsubsection,5=paragraph,6=subparagraph。 */
  level: number;
  file: string;
  line: number;
  fileBase: string;
  children: OutlineNode[];
}

interface FlatItem {
  level: number;
  title: string;
  short?: string;
  file: string;
  line: number;
}

/** 结构命令 → 层级。 */
const LEVEL: Record<string, number> = {
  part: 0,
  chapter: 1,
  section: 2,
  subsection: 3,
  subsubsection: 4,
  paragraph: 5,
  subparagraph: 6,
};

/** 结构命令（含 *、可选 [short]、{title}）。需匹配到紧邻的 `{`，避免误命中 \sectionmark 等。 */
const SECTION_RE =
  /\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)(\*)?\s*(?:\[([^\]]*)\])?\s*\{([^}]*)\}/;
/** 文件引用（\include / \input）：取第一个 {...} 参数。 */
const INCLUDE_RE = /\\(include|input)(\*)?\s*\{([^}]*)\}/;

export const useOutlineStore = defineStore("outline", () => {
  const items = ref<OutlineNode[]>([]);
  const root = ref<string | null>(null);

  const isEmpty = computed(() => items.value.length === 0);

  /** 取文件内容：打开标签用实时缓冲（未落盘也反映），否则读盘。 */
  async function readContent(path: string): Promise<string | null> {
    const buf = useEditorStore().buffers.get(path);
    if (buf !== undefined) return buf;
    try {
      return await ipc.readFile(path);
    } catch {
      return null; // 文件不存在/不可读 → 跳过
    }
  }

  function dirOf(p: string): string {
    const i = p.lastIndexOf("/");
    return i >= 0 ? p.slice(0, i) : p;
  }
  function joinPath(a: string, b: string): string {
    return normalizePath(`${a.replace(/\\/g, "/")}/${b.replace(/^\.\//, "")}`);
  }

  /** \include/\input 参数 → 候选绝对 .tex 路径（先当前文件目录，再项目根）。 */
  function resolveInclude(raw: string, fromFile: string, projectRoot: string): string[] {
    let rel = raw.replace(/\\/g, "/");
    if (rel.startsWith("./")) rel = rel.slice(2);
    if (!/\.[A-Za-z0-9]+$/.test(rel)) rel += ".tex";
    const root = projectRoot.replace(/\\/g, "/");
    const cands: string[] = [];
    const fileRel = joinPath(dirOf(fromFile), rel);
    const rootRel = joinPath(root, rel);
    if (fileRel) cands.push(fileRel);
    if (rootRel && !cands.includes(rootRel)) cands.push(rootRel);
    return cands;
  }

  /** 递归解析单文件：按行扫描，结构命令入列，文件引用递归（文档顺序）。 */
  async function parseFile(path: string, ctx: { flat: FlatItem[]; visited: Set<string>; root: string }) {
    const key = normalizePath(path);
    if (ctx.visited.has(key)) return; // 防环 / 重复包含
    ctx.visited.add(key);
    const content = await readContent(key);
    if (content == null) return;
    const lines = content.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const trimmed = line.trimStart();
      if (trimmed.startsWith("%")) continue; // 注释行
      const inc = trimmed.match(INCLUDE_RE);
      if (inc) {
        for (const cand of resolveInclude(inc[3], key, ctx.root)) {
          await parseFile(cand, ctx);
        }
        continue;
      }
      const sec = trimmed.match(SECTION_RE);
      if (sec) {
        ctx.flat.push({
          level: LEVEL[sec[1]] ?? 2,
          title: sec[4].trim(),
          short: sec[3] ? sec[3].trim() : undefined,
          file: key,
          line: i + 1,
        });
      }
    }
  }

  /** 扁平列表 → 按层级嵌套的树。 */
  function buildTree(flat: FlatItem[]): OutlineNode[] {
    const roots: OutlineNode[] = [];
    const stack: { node: OutlineNode; level: number }[] = [];
    for (const f of flat) {
      if (!f.title) continue;
      const node: OutlineNode = {
        title: f.title,
        shortTitle: f.short,
        level: f.level,
        file: f.file,
        line: f.line,
        fileBase: f.file.split("/").pop() ?? f.file,
        children: [],
      };
      while (stack.length && stack[stack.length - 1].level >= f.level) stack.pop();
      const parent = stack.length ? stack[stack.length - 1].node : null;
      if (parent) parent.children.push(node);
      else roots.push(node);
      stack.push({ node, level: f.level });
    }
    return roots;
  }

  /** 重建大纲（异步，读文件）。根文件存在则按 include 图；否则解析全部 .tex（排序）。 */
  async function refresh() {
    const project = useProjectStore();
    if (!project.project) {
      items.value = [];
      root.value = null;
      return;
    }
    const flat: FlatItem[] = [];
    const ctx = { flat, visited: new Set<string>(), root: project.project.root };
    const rootFile = project.project.root_file;
    if (rootFile) {
      await parseFile(rootFile, ctx);
    } else {
      const texFiles = project.tree
        .filter((e) => !e.is_dir && /\.tex$/i.test(e.name))
        .map((e) => e.path)
        .sort();
      for (const p of texFiles) await parseFile(p, ctx);
    }
    items.value = buildTree(flat);
    root.value = rootFile;
  }

  /** 点击大纲项：揭示源码 + SyncTeX 正向高亮/居中 PDF 对应页（尽力而为）。 */
  async function goTo(node: OutlineNode) {
    void useEditorStore().openFile(node.file, node.line);
    try {
      await useSyncTex().forward(node.file, node.line, 0);
    } catch (e) {
      console.warn("大纲点击：SyncTeX 正向定位失败（仅跳源码）：", e);
    }
  }

  return { items, root, isEmpty, refresh, goTo };
});
