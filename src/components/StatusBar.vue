<!-- StatusBar（modules.md §9.4）：编译状态 / 行列号 / 编译模式 / 冲突提示。只读投影，无状态。 -->
<script setup lang="ts">
import { computed } from "vue";
import { useCompileStore } from "../stores/compile";
import { useEditorStore } from "../stores/editor";
import { useSettingsStore } from "../stores/settings";
import type { CompilePhase } from "../bindings";

defineProps<{ cursorLine: number; cursorCol: number }>();

const compile = useCompileStore();
const editor = useEditorStore();
const settings = useSettingsStore();

const phaseText = computed(() => {
  const map: Record<CompilePhase, string> = {
    queued: "排队中…",
    running: "排版中…",
    success: "就绪",
    failed: "失败",
  };
  return map[compile.phase] ?? "";
});

const kindText = computed(() => {
  switch (compile.kind) {
    case "timeout": return "· 超时";
    case "content_error": return "· 内容错误";
    case "aborted": return "· 已终止";
    default: return "";
  }
});

const isContinuous = computed(() => settings.settings?.compile.mode === "continuous");

async function toggleMode() {
  await settings.update({ mode: isContinuous.value ? "on_save" : "continuous" });
}
</script>

<template>
  <div class="status-bar">
    <span class="phase" :class="compile.phase">
      <span class="phase-dot" />
      {{ phaseText }}{{ kindText }}
    </span>
    <span
      v-for="p in editor.externalConflict"
      :key="p"
      class="conflict"
      title="文件已被外部修改，点击采用磁盘版本"
      @click="editor.acceptExternal(p)"
    >
      外部修改：{{ p.split("/").pop() }}
    </span>
    <span class="spacer" />
    <span class="cursor">
      <span class="cursor-file">{{ editor.activeTab?.name ?? "" }}</span>
      <span class="cursor-pos">Ln {{ cursorLine }}, Col {{ cursorCol }}</span>
    </span>
    <button class="mode" :class="{ on: isContinuous }" @click="toggleMode">
      {{ isContinuous ? "连续编译" : "保存触发" }}
    </button>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex; align-items: center; gap: 12px;
  height: 27px; padding: 0 14px;
  background: var(--card);
  border-top: 1.5px solid var(--line);
  color: var(--ink-dim);
  font-size: 11.5px;
  flex: 0 0 auto;
}
.phase {
  display: inline-flex; align-items: center; gap: 7px;
  padding: 0 11px; height: 19px;
  border-radius: 10px;
  background: rgba(93, 95, 239, 0.10);
  color: var(--blueberry);
  font-weight: 600;
}
.phase-dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; }
.phase.success { background: rgba(47, 191, 143, 0.12); color: #23a377; }
.phase.failed { background: rgba(255, 122, 110, 0.13); color: #e85f52; }
.phase.queued { background: rgba(255, 181, 74, 0.16); color: #e09a2e; }
.phase.running .phase-dot { animation: bounce 0.9s ease-in-out infinite; }
@keyframes bounce { 50% { transform: translateY(-2px); } }
.conflict {
  cursor: pointer;
  background: rgba(255, 122, 110, 0.14);
  color: #e85f52;
  padding: 2px 9px;
  border-radius: 5px;
}
.spacer { flex: 1; }
.cursor { display: inline-flex; align-items: center; gap: 9px; }
.cursor-file { color: var(--ink); font-family: var(--mono); max-width: 320px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.cursor-pos { font-family: var(--mono); color: var(--ink-faint); }
.mode {
  display: inline-flex; align-items: center;
  padding: 2px 11px;
  background: transparent;
  border: 1.5px solid var(--line);
  border-radius: 6px;
  color: var(--ink-dim);
  font-size: 11px; font-weight: 550;
  cursor: pointer;
  transition: all 0.15s;
}
.mode:hover { border-color: var(--blueberry); color: var(--ink); }
.mode.on { border-color: var(--blueberry); color: var(--blueberry); background: rgba(93, 95, 239, 0.08); }

@media (prefers-reduced-motion: reduce) {
  .phase.running .phase-dot { animation: none; }
}
</style>
