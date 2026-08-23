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
// 与全局设计系统一致的浅色 Monaco 主题（Candy Desk，App.vue :root 色板）
monaco.editor.defineTheme("texpresso", {
  base: "vs",
  inherit: true,
  rules: [],
  colors: {
    "editor.background": "#ffffff",
    "editor.foreground": "#2b2438",
    "editor.lineHighlightBackground": "#f7f5fd",
    "editorLineNumber.foreground": "#b8b2ce",
    "editorLineNumber.activeForeground": "#7a7490",
    "editorCursor.foreground": "#5d5fef",
    "editor.selectionBackground": "#dcd7f6",
    "editor.inactiveSelectionBackground": "#ece9fa",
    "editorIndentGuide.background1": "#e7e3f4",
    "editorWidget.background": "#ffffff",
    "editorWidget.border": "#ded9ee",
    "editorSuggestWidget.background": "#ffffff",
    "editorSuggestWidget.border": "#ded9ee",
    "scrollbarSlider.background": "#c9c2e480",
    "scrollbarSlider.hoverBackground": "#b3aad6a0",
    "editorGutter.background": "#fbfaff",
  },
});
monaco.editor.setTheme("texpresso");

const app = createApp(App);
app.use(createPinia());
app.mount("#app");
