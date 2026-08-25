// projectStore 单测：normalizePath / resolvePath 路径归一化（modules.md §9.2）。
// 覆盖 2026-08-25 修复：反向定位返回正斜杠+`./` 的绝对路径 → 须归一为已开标签路径。
import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { normalizePath, useProjectStore } from "../project";

describe("normalizePath", () => {
  it("剥离磁盘绝对路径中的 `./`", () => {
    expect(normalizePath("E:/Works/tex-presso/000test/./main.tex")).toBe(
      "E:/Works/tex-presso/000test/main.tex"
    );
  });

  it("无 `./` 的绝对路径原样保留", () => {
    expect(normalizePath("E:/Works/tex-presso/000test/main.tex")).toBe(
      "E:/Works/tex-presso/000test/main.tex"
    );
  });

  it("合并 `..` 与折叠连续斜杠", () => {
    expect(normalizePath("/home/u/proj/a/../chapters/./b.tex")).toBe("/home/u/proj/chapters/b.tex");
  });

  it("相对路径的 `./` 剥离（不变绝对）", () => {
    expect(normalizePath("./main.tex")).toBe("main.tex");
  });

  it("根绝对路径保留前导 `/`", () => {
    expect(normalizePath("/home/u/proj/main.tex")).toBe("/home/u/proj/main.tex");
  });
});

describe("resolvePath", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  function withProject(): ReturnType<typeof useProjectStore> {
    const store = useProjectStore();
    store.project = { root: "E:/Works/tex-presso/000test" } as never;
    return store;
  }

  it("绝对路径（含 `./`）归一为项目内正斜杠路径", () => {
    const store = withProject();
    expect(store.resolvePath("E:/Works/tex-presso/000test/./main.tex")).toBe(
      "E:/Works/tex-presso/000test/main.tex"
    );
  });

  it("相对 `./main.tex` 拼接项目根", () => {
    const store = withProject();
    expect(store.resolvePath("./main.tex")).toBe("E:/Works/tex-presso/000test/main.tex");
  });

  it("`\\\\?\\` verbatim 防御直通", () => {
    const store = withProject();
    expect(store.resolvePath("\\\\?\\C:\\x\\y.tex")).toBe("\\\\?\\C:\\x\\y.tex");
  });
});
