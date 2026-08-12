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
    running: "编译中…",
    success: "就绪",
    failed: "失败",
  };
  return map[compile.phase] ?? "";
});

const kindText = computed(() => {
  switch (compile.kind) {
    case "timeout": return "（超时）";
    case "content_error": return "（内容错误）";
    case "aborted": return "（已终止）";
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
    <span class="phase" :class="compile.phase">{{ phaseText }}{{ kindText }}</span>
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
    <span class="cursor">{{ editor.activeTab?.name ?? "" }} {{ cursorLine }}:{{ cursorCol }}</span>
    <button class="mode" @click="toggleMode">{{ isContinuous ? "连续编译" : "保存触发" }}</button>
  </div>
</template>

<style scoped>
.status-bar { display: flex; align-items: center; gap: 12px; height: 24px; padding: 0 10px; background: #007acc; color: #fff; font-size: 12px; }
.phase.running { animation: pulse 1.2s infinite; }
@keyframes pulse { 50% { opacity: 0.5; } }
.conflict { cursor: pointer; background: #d9534f; padding: 0 6px; border-radius: 3px; }
.spacer { flex: 1; }
.mode { background: transparent; border: 1px solid rgba(255,255,255,0.5); color: #fff; font-size: 11px; border-radius: 3px; cursor: pointer; padding: 0 6px; }
</style>
