<!-- ErrorList（modules.md §9.4）：错误条目 + 点击跳转源码。
   去重/截断（modules.md §12 后置项）：同源错误（文件 + 首行消息相同）聚合为一条并显示 ×N；
   不同源最多展示 MAX_DISPLAY 组，超出提示隐藏数量（xelatex 错误雪崩时几十条同源错误不再刷屏）。 -->
<script setup lang="ts">
import { computed } from "vue";
import { useCompileStore } from "../stores/compile";
import { useEditorStore } from "../stores/editor";
import type { ErrorEntry } from "../bindings";

const compile = useCompileStore();
const editor = useEditorStore();

/** 展示上限（去重后的组数）。 */
const MAX_DISPLAY = 30;

interface Group {
  entry: ErrorEntry;
  count: number;
}

/** 去重：file + 首行消息 作为键；保留首次出现的文件/行号。 */
const grouped = computed(() => {
  const seen = new Map<string, Group>();
  for (const e of compile.errors) {
    const key = `${e.file ?? ""}\n${e.message.split("\n")[0] ?? ""}`;
    const g = seen.get(key);
    if (g) g.count += 1;
    else seen.set(key, { entry: e, count: 1 });
  }
  const items = [...seen.values()];
  return {
    items: items.slice(0, MAX_DISPLAY),
    hidden: items.length - MAX_DISPLAY,
  };
});

function jump(file: string | null, line: number | null) {
  if (!file) return;
  editor.openFile(file, line ?? 1);
}
</script>

<template>
  <div class="error-list">
    <div class="list-head">
      <span class="head-title">问题</span>
      <span class="head-count" v-if="grouped.items.length > 0">{{ grouped.items.length }}</span>
    </div>
    <div class="list-body">
      <div v-if="grouped.items.length === 0" class="empty">
        <span class="empty-icon">✓</span>
        <span>{{ compile.phase === "running" ? "排版中…" : "没有发现错误" }}</span>
      </div>
      <div
        v-for="(g, i) in grouped.items"
        :key="i"
        class="entry"
        :class="g.entry.kind"
        @click="jump(g.entry.file, g.entry.line)"
      >
        <span class="badge">!</span>
        <span class="count" v-if="g.count > 1" :title="`同类错误共 ${g.count} 条，已折叠`">×{{ g.count }}</span>
        <span class="loc" v-if="g.entry.file">{{ g.entry.file }}<template v-if="g.entry.line">:{{ g.entry.line }}</template></span>
        <span class="msg">{{ g.entry.message.split("\n")[0] }}</span>
      </div>
      <div class="more" v-if="grouped.hidden > 0">
        另有 {{ grouped.hidden }} 组错误未显示（仅展示前 {{ MAX_DISPLAY }} 组去重结果）
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
.count {
  flex: 0 0 auto;
  display: inline-flex; align-items: center;
  padding: 0 5px; height: 15px;
  border-radius: 8px;
  background: rgba(255, 181, 74, 0.18);
  color: #d98d18;
  font-size: 10.5px; font-weight: 700;
}
.loc { color: var(--blueberry); font-family: var(--mono); flex: 0 0 auto; max-width: 45%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.msg { color: var(--ink-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.more {
  padding: 8px 16px 12px;
  color: var(--ink-faint);
  font-size: 11.5px;
}
</style>
