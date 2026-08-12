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
  const map = new Map<string, Node>();
  const roots: Node[] = [];
  for (const e of project.tree) {
    map.set(e.path, { entry: e, children: [] });
  }
  for (const e of project.tree) {
    const node = map.get(e.path)!;
    const parentPath = e.path.slice(0, e.path.lastIndexOf("/"));
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
    <div v-if="!project.project" class="empty">打开一个文件夹开始</div>
    <FileTreeItem v-for="n in tree" :key="n.entry.path" :node="n" />
  </div>
</template>

<style scoped>
.file-tree { height: 100%; overflow: auto; padding: 4px 0; font-size: 13px; user-select: none; }
.empty { color: #888; padding: 12px; }
</style>
