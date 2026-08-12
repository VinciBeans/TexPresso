import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";

// Monaco worker（modules.md §6：直接 ESM + Vite ?worker，不用 CDN 包装）
import * as monaco from "monaco-editor";
import editorWorker from "monaco-editor/editor/editor.worker?worker";

self.MonacoEnvironment = {
  getWorker() {
    return new editorWorker();
  },
};
monaco.editor.setTheme("vs-dark");

const app = createApp(App);
app.use(createPinia());
app.mount("#app");
