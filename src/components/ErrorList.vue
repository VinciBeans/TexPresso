<!-- ErrorList（modules.md §9.4）：错误条目 + 点击跳转源码。 -->
<script setup lang="ts">
import { computed } from "vue";
import { useCompileStore } from "../stores/compile";
import { useEditorStore } from "../stores/editor";

const compile = useCompileStore();
const editor = useEditorStore();

const count = computed(() => compile.errors.length);

function jump(file: string | null, line: number | null) {
  if (!file) return;
  editor.openFile(file, line ?? 1);
}
</script>

<template>
  <div class="error-list">
    <div class="list-head">
      <span class="head-title">问题</span>
      <span class="head-count" v-if="count > 0">{{ count }}</span>
    </div>
    <div class="list-body">
      <div v-if="count === 0" class="empty">
        <span class="empty-icon">✓</span>
        <span>{{ compile.phase === "running" ? "排版中…" : "没有发现错误" }}</span>
      </div>
      <div
        v-for="(e, i) in compile.errors"
        :key="i"
        class="entry"
        :class="e.kind"
        @click="jump(e.file, e.line)"
      >
        <span class="badge">!</span>
        <span class="loc" v-if="e.file">{{ e.file }}<template v-if="e.line">:{{ e.line }}</template></span>
        <span class="msg">{{ e.message.split("\n")[0] }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.error-list { height: 100%; overflow: hidden; display: flex; flex-direction: column; background: var(--card); font-size: 12.5px; }
.list-head {
  display: flex; align-items: center; gap: 8px;
  flex: 0 0 auto;
  height: 32px; padding: 0 14px;
  font-size: 11px; font-weight: 700;
  letter-spacing: 1px;
  color: var(--ink-dim);
  border-bottom: 1.5px solid var(--line-soft);
}
.head-count {
  display: inline-flex; align-items: center; justify-content: center;
  min-width: 17px; height: 17px; padding: 0 5px;
  border-radius: 9px;
  background: rgba(255, 122, 110, 0.15);
  color: #e85f52;
  font-size: 10.5px; font-weight: 700;
  letter-spacing: 0;
}
.list-body { flex: 1 1 auto; overflow: auto; }
.empty {
  display: flex; align-items: center; gap: 9px;
  padding: 16px 18px;
  color: var(--ink-faint);
}
.empty-icon {
  display: inline-flex; align-items: center; justify-content: center;
  width: 17px; height: 17px;
  border-radius: 50%;
  background: rgba(47, 191, 143, 0.14);
  color: #23a377;
  font-size: 10px; font-weight: 700;
}
.entry {
  display: flex; align-items: center; gap: 9px;
  padding: 6px 14px;
  margin: 1px 6px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  border-left: 2.5px solid transparent;
  transition: background 0.12s;
}
.entry:hover { background: var(--card-2); }
.entry.content_error, .entry.io, .entry.timeout { border-left-color: var(--coral); }
.badge {
  flex: 0 0 auto;
  display: inline-flex; align-items: center; justify-content: center;
  width: 16px; height: 16px;
  border-radius: 5px;
  background: rgba(255, 122, 110, 0.15);
  color: #e85f52;
  font-size: 11px; font-weight: 800;
}
.loc { color: var(--blueberry); font-family: var(--mono); flex: 0 0 auto; max-width: 45%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.msg { color: var(--ink-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
