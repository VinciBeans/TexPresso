<!-- 标签页条（modules.md §9.4）：打开文件 + 脏标记 + 关闭。 -->
<script setup lang="ts">
import { useEditorStore } from "../stores/editor";

const editor = useEditorStore();
</script>

<template>
  <div class="tab-bar">
    <div
      v-for="t in editor.tabs"
      :key="t.path"
      class="tab"
      :class="{ active: editor.activePath === t.path }"
      @click="editor.activePath = t.path"
      @auxclick="editor.closeTab(t.path)"
    >
      <span class="dot" v-if="editor.dirty.has(t.path)">●</span>
      <span class="name">{{ t.name }}</span>
      <span class="close" @click.stop="editor.closeTab(t.path)">×</span>
    </div>
  </div>
</template>

<style scoped>
.tab-bar { display: flex; height: 32px; background: #1e1e1e; border-bottom: 1px solid #333; overflow-x: auto; }
.tab { display: flex; align-items: center; gap: 4px; padding: 0 10px; font-size: 12px; color: #aaa; cursor: pointer; border-right: 1px solid #2d2d2d; white-space: nowrap; }
.tab.active { background: #2d2d2d; color: #fff; }
.dot { color: #e2c08d; font-size: 9px; }
.close { color: #777; border-radius: 3px; padding: 0 3px; }
.close:hover { background: #444; color: #fff; }
</style>
