<!-- 自研 splitter（modules.md Q6）：水平/垂直拖拽分隔。信息局部性：拖拽状态在组件内。 -->
<script setup lang="ts">
import { computed, ref } from "vue";

const props = defineProps<{
  direction?: "horizontal" | "vertical";
  /** 主面板初始占比（0-1）。 */
  initial?: number;
}>();

const ratio = ref(props.initial ?? 0.25);
const primaryFlex = computed(() => `${ratio.value * 100}%`);
const dragging = ref(false);

function onPointerDown(e: PointerEvent) {
  dragging.value = true;
  (e.target as HTMLElement).setPointerCapture(e.pointerId);
}

function onPointerMove(e: PointerEvent) {
  if (!dragging.value) return;
  const host = (e.currentTarget as HTMLElement).parentElement!;
  const rect = host.getBoundingClientRect();
  if (props.direction === "vertical") {
    ratio.value = Math.min(0.8, Math.max(0.1, (e.clientX - rect.left) / rect.width));
  } else {
    ratio.value = Math.min(0.8, Math.max(0.1, (e.clientY - rect.top) / rect.height));
  }
}

function onPointerUp() {
  dragging.value = false;
}
</script>

<template>
  <div
    class="split-pane"
    :class="[direction ?? 'horizontal', { dragging }]"
  >
    <div class="pane primary"><slot name="primary" /></div>
    <div
      class="divider"
      :class="direction ?? 'horizontal'"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
    />
    <div class="pane secondary"><slot name="secondary" /></div>
  </div>
</template>

<style scoped>
.split-pane { display: flex; width: 100%; height: 100%; overflow: hidden; }
.split-pane.horizontal { flex-direction: column; }
.divider { flex: 0 0 4px; background: #2b2b2b; cursor: col-resize; }
.divider.horizontal { cursor: row-resize; }
.split-pane.dragging .divider { background: #3b7dd8; }
.pane { min-width: 0; min-height: 0; overflow: hidden; }
.primary { flex: 0 0 v-bind(primaryFlex); }
.secondary { flex: 1 1 auto; }
</style>
