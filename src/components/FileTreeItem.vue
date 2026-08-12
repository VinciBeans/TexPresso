<!-- 文件树递归项（modules.md §9.4）：展开状态只在组件内（信息局部性）。 -->
<script setup lang="ts">
import { computed, ref } from "vue";
import { useEditorStore } from "../stores/editor";
import type { DirEntryInfo } from "../bindings";

interface Node {
  entry: DirEntryInfo;
  children: Node[];
}

const props = defineProps<{ node: Node }>();

const editor = useEditorStore();
const expanded = ref(false);

const isDir = computed(() => props.node.entry.is_dir);
const isActive = computed(() => editor.activePath === props.node.entry.path);

function click() {
  if (isDir.value) {
    expanded.value = !expanded.value;
  } else if (props.node.entry.path.endsWith(".tex")) {
    editor.openFile(props.node.entry.path);
  }
}
</script>

<template>
  <div class="node">
    <div class="row" :class="{ active: isActive }" @click="click">
      <span class="twist">{{ isDir ? (expanded ? "▾" : "▸") : "" }}</span>
      <span class="name">{{ node.entry.name }}</span>
    </div>
    <div v-if="isDir && expanded" class="children">
      <FileTreeItem v-for="c in node.children" :key="c.entry.path" :node="c" />
    </div>
  </div>
</template>

<script lang="ts">
// script setup 里递归引用自身：通过 defineOptions 命名
export default { name: "FileTreeItem" };
</script>

<style scoped>
.node { }
.row { display: flex; align-items: center; padding: 2px 8px; cursor: pointer; white-space: nowrap; }
.row:hover { background: #2a2d2e; }
.row.active { background: #37373d; }
.twist { width: 14px; color: #888; }
.name { overflow: hidden; text-overflow: ellipsis; }
.children { padding-left: 12px; }
</style>
