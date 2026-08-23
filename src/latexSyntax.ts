// LaTeX Monarch 语法（关键词色彩高亮的关键字化 token）。
// 自研语法替代内置 latex grammar，输出细粒度 token 类别，配合主题规则上色：
//   keyword.command / keyword.env / keyword.math / string.env / math /
//   comment / number / operator / delimiter / string.url / text
import type { languages } from "monaco-editor";

export const latexLanguage: languages.IMonarchLanguage = {
  defaultToken: "",
  ignoreCase: false,

  tokenizer: {
    root: [
      { include: "@comment" },

      // 环境：\begin / \end + 环境名（进入 envName 状态着色）
      [/\\begin|\\end/, { token: "keyword.env", next: "@envName" }],

      // 一般命令（含星号形式如 \section*）
      [/\\[a-zA-Z]+\*?/, "keyword.command"],

      // 行内/展示数学进入 math 状态：$...$ 、\(...\) 、\[...\]
      [/\$/, { token: "math", next: "@math" }],
      [/\\\(|\\\[/, { token: "math", next: "@math" }],

      // 转义特殊字符（\% \& \_ \\ 等）
      [/\\[^a-zA-Z]/, "operator"],

      // 链接
      [/https?:\/\/[^\s)]+/, "string.url"],

      // 数字
      [/\d[\d.]*/, "number"],

      // 运算符
      [/[+\-*/=<>^_&~:]/, "operator"],

      // 定界符
      [/[{}()[\]]/, "delimiter"],

      // 普通文本（拉丁 + 中文）
      [/[a-zA-Z\u4e00-\u9fff]+/, "text"],
    ],

    math: [
      { include: "@comment" },
      [/\\\(|\\\[/, "math"],
      [/\$/, { token: "math", next: "@pop" }],
      [/\\\)|\\\]/, { token: "math", next: "@pop" }],
      // 数学命令（\frac \sum \alpha ...）
      [/\\[a-zA-Z]+\*?/, "keyword.math"],
      [/\\[^a-zA-Z]/, "operator"],
      [/\d[\d.]*/, "number"],
      [/[+\-*/=<>^_&~:]/, "operator"],
      [/[{}()[\]]/, "delimiter"],
      [/[,.;!?]/i, "delimiter"],
      [/[a-zA-Z]+/, "math"],
      [/\s+/, "math"],
    ],

    envName: [
      [/\{[^}]*\}/, "string.env", "@pop"],
      [/./, "text", "@pop"],
    ],

    comment: [[/%[^\n]*/, "comment"]],
  },
};

/** LaTeX 语言配置：注释符、括号配对（在注册语法后调用）。 */
export const latexConfiguration: languages.LanguageConfiguration = {
  comments: { lineComment: "%" },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: "$", close: "$" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
  ],
};
