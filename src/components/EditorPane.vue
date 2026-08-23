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

/**
 * model uri.toString() → 存储路径（反查表）。
 * Monaco 的 `uri.path` 对 `file:///E:/...` 会带前导 `/`；对 Windows 反斜杠路径
 * 会把整段解析进 authority（uri.path 变空）。因此所有“model → 存储路径”的映射
 * 一律经此表，绝不直接读 uri.path 作为存储键。
 */
const modelPaths = new Map<string, string>();

function uriOf(path: string) {
  // 统一正斜杠：file:///E:/...（Windows 盘符）或 file:///home/...（UNIX）
  const p = path.replace(/\\/g, "/");
  return monaco.Uri.parse("file://" + (p.startsWith("/") ? p : "/" + p));
}

function storePathOf(model: monaco.editor.ITextModel | null): string | undefined {
  return model ? modelPaths.get(model.uri.toString()) : undefined;
}

function reportCursor() {
  const pos = monacoEditor?.getPosition();
  if (pos) emit("cursor", pos.lineNumber, pos.column);
}

onMounted(() => {
  monacoEditor = monaco.editor.create(host.value!, {
    automaticLayout: true,
    fontSize: 14,
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    language: "latex",
  });

  // 内容变更 → 脏标记 + 缓冲（自动保存的数据源）
  monacoEditor.onDidChangeModelContent(() => {
    const model = monacoEditor!.getModel();
    const path = storePathOf(model);
    if (!path) return;
    editor.markDirty(path, model.getValue());
    emit("change", path);
  });

  monacoEditor.onDidChangeCursorPosition(reportCursor);

  // Ctrl+点击 → SyncTeX 正向（modules.md §5.3）
  monacoEditor.onMouseDown((e) => {
    if (e.event.ctrlKey && e.target.position) {
      const pos = e.target.position;
      const model = monacoEditor!.getModel();
      const path = storePathOf(model);
      if (model && path) forward(path, pos.lineNumber, pos.column);
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
    modelPaths.set(model.uri.toString(), path);
    monacoEditor.setModel(model);
    reportCursor();
  }
);

// 最后一个标签被关闭（activePath 变 null）→ 清空编辑器：否则旧 model 残留，
// 仍可编辑/自动保存/触发编译（BUG 修复）。
watch(
  () => editor.activePath,
  (path) => {
    if (path || !monacoEditor) return;
    monacoEditor.setModel(null);
  }
);

// 关闭标签 → 释放对应 model（防残留编辑；重开时按 buffer/磁盘内容重建，不读旧 model）
watch(
  () => editor.tabs.map((t) => t.path),
  (paths) => {
    for (const m of monaco.editor.getModels()) {
      const storePath = modelPaths.get(m.uri.toString());
      if (storePath !== undefined && !paths.includes(storePath)) {
        modelPaths.delete(m.uri.toString());
        m.dispose();
      }
    }
  }
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
