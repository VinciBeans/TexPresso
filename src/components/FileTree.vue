<!-- 文件树（modules.md §9.4）：从扁平 list_dir 结果构建嵌套。展开状态只在组件内。 -->
<script setup lang="ts">
import { computed } from "vue";
import { useProjectStore } from "../stores/project";
import type { DirEntryInfo } from "../bindings";
import FileTreeItem from "./FileTreeItem.vue";

const project = useProjectStore();

interface Node {
  entry: DirEntryInfo;
  children: Node[];
}

function buildTree(): Node[] {
  const norm = (p: string) => p.replace(/\\/g, "/");
  const map = new Map<string, Node>();
  const roots: Node[] = [];
  for (const e of project.tree) {
    const path = norm(e.path);
    map.set(path, { entry: { ...e, path }, children: [] });
  }
  for (const e of project.tree) {
    const path = norm(e.path);
    const node = map.get(path)!;
    // 分隔符无关：Windows 反斜杠 / WSL 正斜杠都能切
    const pos = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
    const parentPath = pos < 0 ? "" : path.slice(0, pos);
    const parent = map.get(parentPath);
    if (parent && parent.entry.is_dir) parent.children.push(node);
    else roots.push(node);
  }
  const sort = (ns: Node[]) => {
    ns.sort((a, b) =>
      a.entry.is_dir === b.entry.is_dir
        ? a.entry.name.localeCompare(b.entry.name)
        : a.entry.is_dir
          ? -1
          : 1
    );
    for (const n of ns) sort(n.children);
  };
  sort(roots);
  return roots;
}

const tree = computed(buildTree);
</script>

<template>
  <div class="file-tree">
    <div class="tree-panel">
      <div class="panel-title">
        <span class="panel-icon">🗂</span>
        <span>资源管理器</span>
      </div>
      <div class="tree-scroll">
        <div v-if="!project.project" class="empty">
          <span class="empty-card">
            <span class="empty-icon">📂</span>
            <span class="empty-title">还没打开项目</span>
            <span class="empty-hint">点左上角「打开项目」开始</span>
          </span>
        </div>
        <FileTreeItem v-for="n in tree" :key="n.entry.path" :node="n" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.file-tree {
  height: 100%;
  background: var(--card);
  user-select: none;
}
.tree-panel { display: flex; flex-direction: column; height: 100%; }
.panel-title {
  display: flex; align-items: center; gap: 7px;
  flex: 0 0 auto;
  height: 32px; padding: 0 14px;
  font-size: 11px; font-weight: 700;
  letter-spacing: 1px;
  color: var(--ink-dim);
  border-bottom: 1.5px solid var(--line-soft);
}
.panel-icon { font-size: 12px; }
.tree-scroll { flex: 1 1 auto; overflow: auto; padding: 6px 0 14px; font-size: 13px; }
.empty { display: flex; justify-content: center; padding: 36px 10px 0; }
.empty-card {
  display: flex; flex-direction: column; align-items: center; gap: 6px;
  max-width: 100%;
  padding: 20px 12px;
  border: 1.5px dashed var(--line);
  border-radius: var(--radius);
  color: var(--ink-faint);
  white-space: nowrap;
}
.empty-icon { font-size: 26px; }
.empty-title { font-size: 12.5px; font-weight: 600; color: var(--ink-dim); }
.empty-hint { font-size: 11.5px; }
</style>
