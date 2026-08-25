// projectStore（modules.md §9.2）：项目、根文件、文件树。
import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { ipc } from "../services/ipc";
import type { DirEntryInfo, ProjectInfo } from "../bindings";

export const useProjectStore = defineStore("project", () => {
  const project = ref<ProjectInfo | null>(null);
  const tree = ref<DirEntryInfo[]>([]);
  const treeVersion = ref(0);

  const root = computed(() => project.value?.root ?? "");

  async function openProject(folder: string) {
    project.value = await ipc.openProject(folder);
    await refreshTree();
    return project.value;
  }

  /** 全量重建文件树（files-changed 防抖后调用，modules.md Q1）。 */
  async function refreshTree() {
    if (!project.value) return;
    tree.value = await ipc.listDir(project.value.root);
    treeVersion.value++;
  }

  let timer: ReturnType<typeof setTimeout> | undefined;
  /** 300ms 防抖重建（modules.md §5.5）。 */
  function refreshTreeDebounced() {
    clearTimeout(timer);
    timer = setTimeout(() => refreshTree().catch(() => {}), 300);
  }

  /** 相对路径（如 ./main.tex、chapters/a.tex）解析为项目内绝对路径。
   *  统一输出正斜杠（存储键/后端请求一致）；Windows 接受盘符开头（\\?\ 防御直通）；WSL：/ 开头。
   *  归一化 `.`/`..`/连续斜杠——如 synctex 反向返回的 `E:/proj/./main.tex`，
   *  与已打开的 `E:/proj/main.tex` 必须归一为同一存储键（否则重复开标签）。 */
  function resolvePath(p: string): string {
    if (p.startsWith("\\\\?\\")) return p; // 防御：verbatim 直通（后端应已剥离）
    const n = p.replace(/\\/g, "/");
    const abs = n.startsWith("/") || /^[A-Za-z]:/.test(n);
    const base = abs ? n : root.value.replace(/\\/g, "/") + "/" + n.replace(/^\.\//, "");
    return normalizePath(base);
  }

  return { project, tree, treeVersion, root, openProject, refreshTree, refreshTreeDebounced, resolvePath };
});

/** 归一化文件路径：折叠连续斜杠、剥 `.`、合并 `..`（浏览器环境手写，不依赖 node:path）。
 *  磁盘绝对路径保留 `E:/...`；根绝对路径保留 `/...`。 */
function normalizePath(p: string): string {
  const abs = p.startsWith("/") || /^[A-Za-z]:\//.test(p);
  const out: string[] = [];
  for (const seg of p.split("/")) {
    if (seg === "" || seg === ".") continue;
    if (seg === "..") {
      const prev = out[out.length - 1];
      if (prev && prev !== ".." && !/^[A-Za-z]:$/.test(prev)) out.pop();
      else if (!abs && prev !== "..") out.push("..");
      continue; // 绝对路径下越界的 `..` 丢弃
    }
    out.push(seg);
  }
  if (abs) {
    const head = out[0] ?? "";
    return /^[A-Za-z]:$/.test(head) ? head + "/" + out.slice(1).join("/") : "/" + out.join("/");
  }
  return out.join("/");
}
