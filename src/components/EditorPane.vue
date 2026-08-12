<!-- EditorPane（modules.md §9.4）：Monaco 封装。
   模块内信息：Monaco 实例、model 表；对外只发内容变更与定位事件。 -->
<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import * as monaco from "monaco-editor";
import { useEditorStore } from "../stores/editor";
import { useSyncTex } from "../composables/useSyncTex";

const emit = defineEmits<{
  change: [path: string];
  cursor: [line: number, column: number];
}>();

const editor = useEditorStore();
const { forward } = useSyncTex();

const host = ref<HTMLElement | null>(null);
let monacoEditor: monaco.editor.IStandaloneCodeEditor | null = null;

function uriOf(path: string) {
  return monaco.Uri.parse("file://" + path);
}

function reportCursor() {
  const pos = monacoEditor?.getPosition();
  if (pos) emit("cursor", pos.lineNumber, pos.column);
}

onMounted(() => {
  monacoEditor = monaco.editor.create(host.value!, {
    theme: "vs-dark",
    automaticLayout: true,
    fontSize: 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    language: "latex",
  });

  // 内容变更 → 脏标记 + 缓冲（自动保存的数据源）
  monacoEditor.onDidChangeModelContent(() => {
    const model = monacoEditor!.getModel();
    if (!model) return;
    const path = model.uri.path;
    editor.markDirty(path, model.getValue());
    emit("change", path);
  });

  monacoEditor.onDidChangeCursorPosition(reportCursor);

  // Ctrl+点击 → SyncTeX 正向（modules.md §5.3）
  monacoEditor.onMouseDown((e) => {
    if (e.event.ctrlKey && e.target.position) {
      const pos = e.target.position;
      const model = monacoEditor!.getModel();
      if (model) forward(model.uri.path, pos.lineNumber, pos.column);
    }
  });
});

// 打开文件切换：get-or-create model
watch(
  () => editor.activePath,
  (path) => {
    if (!path || !monacoEditor) return;
    const uri = uriOf(path);
    let model = monaco.editor.getModel(uri);
    if (!model) {
      model = monaco.editor.createModel(editor.buffers.get(path) ?? "", "latex", uri);
    }
    monacoEditor.setModel(model);
    reportCursor();
  },
  { immediate: true }
);

// 外部重载（modules.md §5.5）：buffer 变化 → model 同步；值相同跳过（防循环）
watch(
  () => editor.buffers.get(editor.activePath ?? ""),
  (content) => {
    if (content === undefined || !monacoEditor) return;
    const model = monacoEditor.getModel();
    if (model && model.getValue() !== content) {
      model.setValue(content);
    }
  }
);

// 定位请求（错误跳转 / SyncTeX 反向）
watch(
  () => editor.pendingReveal,
  (r) => {
    if (!r || !monacoEditor) return;
    const model = monaco.editor.getModel(uriOf(r.path));
    if (model) {
      monacoEditor.setModel(model);
      monacoEditor.revealLineInCenter(r.line);
      monacoEditor.setPosition({ lineNumber: r.line, column: 1 });
      monacoEditor.focus();
    }
    editor.consumeReveal();
  }
);

onBeforeUnmount(() => {
  monacoEditor?.dispose();
  monacoEditor = null;
});
</script>

<template>
  <div ref="host" class="editor-pane" />
</template>

<style scoped>
.editor-pane { width: 100%; height: 100%; }
</style>
