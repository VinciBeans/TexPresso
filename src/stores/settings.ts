// settingsStore（modules.md §9.2）：最底层 store，其余 store 只读它。
import { defineStore } from "pinia";
import { ref } from "vue";
import { ipc } from "../services/ipc";
import type { Settings, SettingsPatch } from "../bindings";

export const useSettingsStore = defineStore("settings", () => {
  const settings = ref<Settings | null>(null);

  async function init() {
    settings.value = await ipc.getSettings();
  }

  function setSettings(s: Settings) {
    settings.value = s;
  }

  /** 局部更新（后端负责写盘与广播 settings-changed）。 */
  async function update(patch: SettingsPatch) {
    settings.value = await ipc.updateSettings(patch);
  }

  return { settings, init, setSettings, update };
});
