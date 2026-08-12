<!-- 应用壳（modules.md §3）：三栏布局 + 底部错误列表 + 状态栏，自研 splitter。 -->
<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import SplitPane from "./components/SplitPane.vue";
import FileTree from "./components/FileTree.vue";
import TabBar from "./components/TabBar.vue";
import EditorPane from "./components/EditorPane.vue";
import PreviewPane from "./components/PreviewPane.vue";
import ErrorList from "./components/ErrorList.vue";
import StatusBar from "./components/StatusBar.vue";
import { useProjectStore } from "./stores/project";
import { useEditorStore } from "./stores/editor";
import { useSettingsStore } from "./stores/settings";
import { useCompileStore } from "./stores/compile";
import { useAutoSave } from "./composables/useAutoSave";
import { ipc } from "./services/ipc";
import { subscribeEvents } from "./services/events";

const project = useProjectStore();
const editor = useEditorStore();
const settings = useSettingsStore();
const compile = useCompileStore();
const autoSave = useAutoSave();

const cursorLine = ref(0);
const cursorCol = ref(0);

let unsubscribe: (() => void) | null = null;

onMounted(async () => {
  unsubscribe = subscribeEvents();
  await settings.init();
});

onBeforeUnmount(() => unsubscribe?.());

/** 打开项目（dialog 选文件夹）。 */
async function chooseProject() {
  const dir = await open({ directory: true, title: "打开 TeX 项目文件夹" });
  if (!dir) return;
  try {
    const info = await project.openProject(dir);
    if (info.root_file) {
      await editor.openFile(info.root_file);
    } else {
      // 多候选/零候选：v1 提示手动选择根文件
      console.warn("未探测到唯一根文件，请在设置中手动指定 root_file");
    }
  } catch (e) {
    console.error("打开项目失败：", e);
  }
}

function onEditorChange(_path: string) {
  autoSave.schedule();
}

async function manualCompile() {
  try {
    await ipc.compileNow();
  } catch (e) {
    console.error("手动编译失败：", e);
  }
}

async function abort() {
  await ipc.abortCompile();
}

const isRunning = () => compile.phase === "running" || compile.phase === "queued";
</script>

<template>
  <div class="app">
    <div class="toolbar">
      <button class="btn" @click="chooseProject">📂 打开项目</button>
      <button class="btn primary" :disabled="isRunning()" @click="manualCompile">▶ 编译</button>
      <button class="btn" :disabled="!isRunning()" @click="abort">■ 终止</button>
      <span class="title">{{ project.project ? project.project.root : "TeXPresso" }}</span>
    </div>

    <div class="main">
      <SplitPane direction="vertical" :initial="0.2">
        <template #primary>
          <FileTree />
        </template>
        <template #secondary>
          <SplitPane direction="horizontal" :initial="0.65">
            <template #primary>
              <div class="editor-area">
                <TabBar />
                <EditorPane
                  @change="onEditorChange"
                  @cursor="(l: number, c: number) => { cursorLine = l; cursorCol = c }"
                />
              </div>
            </template>
            <template #secondary>
              <PreviewPane />
            </template>
          </SplitPane>
        </template>
      </SplitPane>
    </div>

    <div class="bottom">
      <SplitPane direction="horizontal" :initial="0.7">
        <template #primary>
          <div class="error-area"><ErrorList /></div>
        </template>
        <template #secondary>
          <div class="placeholder">大纲（后置）</div>
        </template>
      </SplitPane>
    </div>

    <StatusBar :cursor-line="cursorLine" :cursor-col="cursorCol" />
  </div>
</template>

<style>
html, body, #app { height: 100%; margin: 0; }
body { font-family: "Segoe UI", "Microsoft YaHei", sans-serif; background: #1e1e1e; color: #ccc; }
</style>

<style scoped>
.app { display: flex; flex-direction: column; height: 100%; }
.toolbar { display: flex; align-items: center; gap: 8px; padding: 6px 10px; background: #252526; border-bottom: 1px solid #333; }
.btn { background: #3a3d41; border: none; color: #ccc; padding: 4px 12px; border-radius: 4px; cursor: pointer; font-size: 13px; }
.btn:hover:not(:disabled) { background: #45484c; }
.btn.primary { background: #0e639c; color: #fff; }
.btn:disabled { opacity: 0.5; cursor: default; }
.title { margin-left: 8px; font-size: 12px; color: #888; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.main { flex: 1 1 auto; min-height: 0; }
.bottom { flex: 0 0 160px; min-height: 0; border-top: 1px solid #333; }
.editor-area { display: flex; flex-direction: column; height: 100%; }
.editor-area > :last-child { flex: 1; min-height: 0; }
.error-area { height: 100%; }
.placeholder { height: 100%; display: flex; align-items: center; justify-content: center; color: #555; font-size: 12px; }
</style>
