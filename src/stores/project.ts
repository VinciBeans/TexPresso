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
   *  统一输出正斜杠（存储键/后端请求一致）；Windows 接受盘符开头（\\?\ 防御直通）；WSL：/ 开头。 */
  function resolvePath(p: string): string {
    if (p.startsWith("\\\\?\\")) return p; // 防御：verbatim 直通（后端应已剥离）
    const n = p.replace(/\\/g, "/");
    if (n.startsWith("/") || /^[A-Za-z]:/.test(n)) return n;
    return root.value.replace(/\\/g, "/") + "/" + n.replace(/^\.\//, "");
  }

  return { project, tree, treeVersion, root, openProject, refreshTree, refreshTreeDebounced, resolvePath };
});
