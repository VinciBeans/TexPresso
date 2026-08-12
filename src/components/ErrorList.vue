<!-- ErrorList（modules.md §9.4）：错误条目 + 点击跳转源码。 -->
<script setup lang="ts">
import { useCompileStore } from "../stores/compile";
import { useEditorStore } from "../stores/editor";

const compile = useCompileStore();
const editor = useEditorStore();

function jump(file: string | null, line: number | null) {
  if (!file) return;
  editor.openFile(file, line ?? 1);
}
</script>

<template>
  <div class="error-list">
    <div v-if="compile.errors.length === 0" class="empty">
      {{ compile.phase === "running" ? "编译中…" : "无错误" }}
    </div>
    <div
      v-for="(e, i) in compile.errors"
      :key="i"
      class="entry"
      :class="e.kind"
      @click="jump(e.file, e.line)"
    >
      <span class="tag">错误</span>
      <span class="loc" v-if="e.file">{{ e.file }}<template v-if="e.line">:{{ e.line }}</template></span>
      <span class="msg">{{ e.message.split("\n")[0] }}</span>
    </div>
  </div>
</template>

<style scoped>
.error-list { height: 100%; overflow: auto; font-size: 12px; background: #1e1e1e; }
.empty { color: #777; padding: 8px 12px; }
.entry { display: flex; gap: 8px; padding: 4px 12px; cursor: pointer; border-left: 3px solid transparent; }
.entry:hover { background: #2a2d2e; }
.entry.content_error, .entry.io, .entry.timeout { border-left-color: #e06c75; }
.tag { color: #e06c75; flex: none; }
.loc { color: #61afef; flex: none; }
.msg { color: #ccc; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
