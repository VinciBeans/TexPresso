// 事件订阅（modules.md §9.1）：订阅一次，分发到各 store；单向：事件 → store 动作。
// 返回取消函数（组件卸载时调用）。

import { events } from "../bindings";
import { useCompileStore } from "../stores/compile";
import { useEditorStore } from "../stores/editor";
import { usePreviewStore } from "../stores/preview";
import { useProjectStore } from "../stores/project";
import { useSettingsStore } from "../stores/settings";

export function subscribeEvents(): () => void {
  const unlisteners: Promise<() => void>[] = [];

  unlisteners.push(
    events.compileStatus.listen((e) => {
      const dto = e.payload as any;
      useCompileStore().setStatus(dto.phase, dto.kind);
    }),
    events.errorsUpdated.listen((e) => {
      useCompileStore().setErrors(e.payload as any);
    }),
    events.pdfUpdated.listen((e) => {
      const p = e.payload as any;
      usePreviewStore().onPdfUpdated(p.path as string);
    }),
    events.filesChanged.listen((e) => {
      const p = e.payload as any;
      const paths: string[] = p.paths ?? [];
      useEditorStore().onFilesChanged(paths);
      useProjectStore().refreshTreeDebounced();
    }),
    events.settingsChanged.listen((e) => {
      useSettingsStore().setSettings(e.payload as any);
    }),
  );

  return () => {
    for (const u of unlisteners) {
      u.then((fn) => fn()).catch(() => {});
    }
  };
}
