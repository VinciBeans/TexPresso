// texParse 单测：LaTeX 行级剥离子程序（modules.md §6 / design.md §测试）。
// 锁定折叠/大纲的注释+`\verb` 处理（B3/C2/C3）与补全「词首前一字符是 `\`」判定（B7/C4），
// 防日后改 wordPattern 或剥离逻辑时回归。
import { describe, it, expect } from "vitest";
import { stripTexComment, shouldStripLeadBackslash } from "../texParse";

describe("stripTexComment", () => {
  it("在真注释 `%` 处截断", () => {
    expect(stripTexComment("\\begin{eq} % 注释")).toBe("\\begin{eq} ");
    expect(stripTexComment("\\section{A}%x")).toBe("\\section{A}");
  });

  it("行首注释（%）整行剥空", () => {
    expect(stripTexComment(" % \\begin{Y}")).toBe(" ");
  });

  it("移除非行内 \\verb 跨度（其内 % / 结构命令是字面量）", () => {
    // \verb|%| 是字面量，其 % 不应被当注释；\section{X} 在其后保留
    expect(stripTexComment("\\verb|%|\\section{X}")).toBe("\\section{X}");
    // \verb|...| 内 `\begin{Y}` 是字面量，不应生成环境；把跨度剥掉
    expect(stripTexComment("\\begin{X} \\verb|\\begin{Y}| \\end{X}")).toBe(
      "\\begin{X}  \\end{X}"
    );
  });

  it("转义 \\% 是字面量百分号，不作注释（C3）", () => {
    expect(stripTexComment("\\%\\section{X}")).toBe("\\%\\section{X}");
  });

  it("偶数反斜杠 \\\\% 的 % 是真注释（C5：\\\\) 换行+% 注释）", () => {
    // "a\\\\%b" = a\\%b（% 前 2 个反斜杠=偶数 → 注释）→ 截断到 a\\
    expect(stripTexComment("a\\\\%b")).toBe("a\\\\");
  });
});

describe("shouldStripLeadBackslash", () => {
  it("词首前一字符为 \\ 时返回 true（\\sec 的 word=sec）", () => {
    expect(shouldStripLeadBackslash("\\sec", 2)).toBe(true);
  });

  it("startColumn=1（词在列首）或前一字符非 \\ 时返回 false", () => {
    expect(shouldStripLeadBackslash("\\sec", 1)).toBe(false);
    expect(shouldStripLeadBackslash("abc", 2)).toBe(false);
    expect(shouldStripLeadBackslash("sec", 1)).toBe(false);
  });
});
