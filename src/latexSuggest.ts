// LaTeX 语言扩展（modules.md §6 / design.md §编辑器 v1.1）：
//   - 代码片段：CompletionItemProvider（InsertAsSnippet），覆盖文档/环境/章节/数学/格式等常用命令；
//   - 折叠：FoldingRangeProvider，按 \begin{env} ... \end{env} 环境块折叠。
// 挂接：main.ts 注册语言后调用 registerLatexProvider()。
import * as monaco from "monaco-editor";

const Snippet = monaco.languages.CompletionItemKind.Snippet;
const InsertAsSnippet = monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet;

interface SnippetDef {
  label: string;
  /** 插入文本（含 `${1:...}` 占位；用 \\ 转义 LaTeX 反斜杠）。 */
  body: string;
  detail?: string;
}

const snippets: SnippetDef[] = [
  // ---- 文档骨架 ----
  { label: "documentclass", body: "\\documentclass[${1|article,report,book,ctexart,ctexbook,beamer|}]{${2:article}}\n${3}\n\\begin{document}\n\t${4}\n\\end{document}", detail: "文档类 + document 环境" },
  { label: "document", body: "\\begin{document}\n\t${1}\n\\end{document}", detail: "document 环境" },
  { label: "usepackage", body: "\\usepackage{${1:package}}", detail: "加载宏包" },
  { label: "title", body: "\\title{${1}}", detail: "标题" },
  { label: "author", body: "\\author{${1}}", detail: "作者" },
  { label: "date", body: "\\date{${1:\\today}}", detail: "日期" },
  { label: "maketitle", body: "\\maketitle", detail: "生成标题" },
  { label: "tableofcontents", body: "\\tableofcontents", detail: "目录" },
  { label: "setlength", body: "\\setlength{\\${1:parindent}}{${2:0pt}}", detail: "设置长度" },

  // ---- 通用环境（begin/end 名字同步） ----
  { label: "env", body: "\\begin{${1:env}}\n\t${2}\n\\end{${1}}", detail: "自定义环境（名字同步）" },
  { label: "equation", body: "\\begin{equation}\n\t${1}\n\\end{equation}", detail: "编号公式" },
  { label: "align", body: "\\begin{align}\n\t${1}\n\\end{align}", detail: "对齐公式" },
  { label: "gather", body: "\\begin{gather}\n\t${1}\n\\end{gather}", detail: "多行公式" },
  { label: "itemize", body: "\\begin{itemize}\n\t\\item ${1}\n\\end{itemize}", detail: "无序列表" },
  { label: "enumerate", body: "\\begin{enumerate}\n\t\\item ${1}\n\\end{enumerate}", detail: "有序列表" },
  { label: "description", body: "\\begin{description}\n\t\\item[${1:label}] ${2}\n\\end{description}", detail: "描述列表" },
  { label: "figure", body: "\\begin{figure}[${1:htbp}]\n\t\\centering\n\t\\includegraphics[width=${2:0.5\\textwidth}]{${3:image}}\n\t\\caption{${4}}\n\t\\label{${5}}\n\\end{figure}", detail: "图片环境" },
  { label: "table", body: "\\begin{table}[${1:htbp}]\n\t\\centering\n\t\\caption{${2}}\n\t\\label{${3}}\n\\end{table}", detail: "表格环境" },
  { label: "tabular", body: "\\begin{tabular}{${1:ccc}}\n\t${2} \\\\\n\\end{tabular}", detail: "表格体" },
  { label: "center", body: "\\begin{center}\n\t${1}\n\\end{center}", detail: "居中环境" },
  { label: "abstract", body: "\\begin{abstract}\n\t${1}\n\\end{abstract}", detail: "摘要环境" },
  { label: "theorem", body: "\\begin{${1:theorem}}\n\t${2}\n\\end{${1}}", detail: "定理类环境（名字同步）" },

  // ---- 章节 ----
  { label: "part", body: "\\part{${1}}", detail: "部" },
  { label: "chapter", body: "\\chapter{${1}}", detail: "章" },
  { label: "section", body: "\\section{${1}}", detail: "节" },
  { label: "subsection", body: "\\subsection{${1}}", detail: "小节" },
  { label: "subsubsection", body: "\\subsubsection{${1}}", detail: "小小节" },
  { label: "paragraph", body: "\\paragraph{${1}}", detail: "段" },
  { label: "label", body: "\\label{${1:sec:name}}", detail: "引用标签" },
  { label: "ref", body: "\\ref{${1}}", detail: "引用" },
  { label: "eqref", body: "\\eqref{${1}}", detail: "公式引用" },
  { label: "autoref", body: "\\autoref{${1}}", detail: "自动引用" },
  { label: "cite", body: "\\cite{${1}}", detail: "文献引用" },
  { label: "pageref", body: "\\pageref{${1}}", detail: "页码引用" },

  // ---- 数学 ----
  { label: "frac", body: "\\frac{${1:num}}{${2:den}}", detail: "分数" },
  { label: "sqrt", body: "\\sqrt{${1}}", detail: "根号" },
  { label: "sum", body: "\\sum_{${1:i=1}}^{${2:n}}", detail: "求和" },
  { label: "int", body: "\\int_{${1}}^{${2}}", detail: "积分" },
  { label: "alpha", body: "\\alpha", detail: "α" },
  { label: "beta", body: "\\beta", detail: "β" },
  { label: "gamma", body: "\\gamma", detail: "γ" },
  { label: "delta", body: "\\delta", detail: "δ" },
  { label: "epsilon", body: "\\epsilon", detail: "ε" },
  { label: "theta", body: "\\theta", detail: "θ" },
  { label: "lambda", body: "\\lambda", detail: "λ" },
  { label: "mu", body: "\\mu", detail: "μ" },
  { label: "pi", body: "\\pi", detail: "π" },
  { label: "sigma", body: "\\sigma", detail: "σ" },
  { label: "omega", body: "\\omega", detail: "ω" },
  { label: "infty", body: "\\infty", detail: "∞" },
  { label: "pm", body: "\\pm", detail: "±" },
  { label: "times", body: "\\times", detail: "×" },
  { label: "leq", body: "\\leq", detail: "≤" },
  { label: "geq", body: "\\geq", detail: "≥" },
  { label: "neq", body: "\\neq", detail: "≠" },
  { label: "rightarrow", body: "\\rightarrow", detail: "→" },

  // ---- 格式 / 命令 ----
  { label: "textbf", body: "\\textbf{${1}}", detail: "粗体" },
  { label: "textit", body: "\\textit{${1}}", detail: "斜体" },
  { label: "underline", body: "\\underline{${1}}", detail: "下划线" },
  { label: "emph", body: "\\emph{${1}}", detail: "强调" },
  { label: "text", body: "\\text{${1}}", detail: "数学内文本" },
  { label: "newcommand", body: "\\newcommand{\\${1:cmd}}{${2}}", detail: "定义命令" },
  { label: "renewcommand", body: "\\renewcommand{\\${1:cmd}}{${2}}", detail: "重定义命令" },
  { label: "item", body: "\\item ${1}", detail: "列表项" },

  // ---- 文件操作 ----
  { label: "include", body: "\\include{${1}}", detail: "包含子文件（独立分页）" },
  { label: "input", body: "\\input{${1}}", detail: "输入子文件" },
  { label: "includegraphics", body: "\\includegraphics[width=${1:0.5\\textwidth}]{${2:image}}", detail: "插入图片" },
  { label: "bibliography", body: "\\bibliography{${1}}", detail: "参考文献" },
  { label: "footnote", body: "\\footnote{${1}}", detail: "脚注" },
];

const items = snippets.map((s) => ({
  label: s.label,
  kind: Snippet,
  insertText: s.body,
  insertTextRules: InsertAsSnippet,
  detail: s.detail,
  sortText: s.label,
}));

/** LaTeX 代码片段补全（含数学/命令），trigger `\`。
 *  B1 核对：Monaco `SnippetParser._parseEscaped` 只把 `\$`/`\}`/`\\` 当转义，
 *  其余 `\x` 一律保留字面 `\`（如 `\begin`→`\`+`begin`）。故片段体用单反斜杠
 *  （TS 里 `\\begin`→JS `\begin`）插入即得真实 `\begin{...}`，无需写 `\\\\`。
 *  B2：trigger 只留 `\`（去掉 `{`，避免在带参处列出全部片段造成噪音）。
/**
 *  B7（修复 \\section）：`\sec` 时 Monaco 当前词为 `sec`（不含 `\`）。
 *  key1：range 必须等于当前词范围 `getWordUntilPosition`，若把 range 前延到含 `\`（如 [1,5]），
 *  Monaco 会因 range 与当前词不一致而**过滤掉该建议**（建议列表空白）。故 range 保持 `word.startColumn`。
 *  key2：词首前一字符是 `\` 时，该 `\` 保留在替换区间之外；若片段自带前导 `\`，拼成 `\\section`。
 *  故此时剥掉片段自带的前导 `\`（`\section`→`section`），替换后 `\`+`section{}`=`\section{}`（单反斜杠）。 */
export const latexCompletionProvider: monaco.languages.CompletionItemProvider = {
  triggerCharacters: ["\\"],
  provideCompletionItems(model, position) {
    const word = model.getWordUntilPosition(position);
    const lower = word.word.toLowerCase();
    const line = model.getLineContent(position.lineNumber);
    const backslashBefore = word.startColumn > 1 && line.charCodeAt(word.startColumn - 2) === 92 /* '\\' */;
    const suggestions = items
      .filter((it) => !lower || it.label.toLowerCase().startsWith(lower))
      .map((it) => {
        const insertText =
          backslashBefore && it.insertText.startsWith("\\") ? it.insertText.slice(1) : it.insertText;
        return {
          ...it,
          insertText,
          range: {
            startLineNumber: position.lineNumber,
            endLineNumber: position.lineNumber,
            startColumn: word.startColumn,
            endColumn: word.endColumn,
          },
        };
      });
    return { suggestions };
  },
};

/** LaTeX 环境块折叠：\begin{env} ... \end{env}（含嵌套）。
 *  注意 1：Monaco FoldingRange.start/end 为 **1-based** 行号（见 monaco.d.ts），
 *  此处 i 是 0-based 下标 → 输出需 +1，否则折叠锚点会整体上移一行（\begin 前一行、\end 露出）。
 *  注意 2（B3）：剥离行内注释（`%` 之后）且跳过含 `\verb` 的行，避免把注释/字面量里的
 *  `\begin{}`/`\end{}` 误判为真环境而生成伪折叠区。 */
export const latexFoldingProvider: monaco.languages.FoldingRangeProvider = {
  provideFoldingRanges(model) {
    const ranges: monaco.languages.FoldingRange[] = [];
    const lines = model.getValue().split(/\r?\n/);
    const beginRe = /\\(begin|end)\{([^}]*)\}/;
    const stack: { start: number; env: string }[] = [];
    for (let i = 0; i < lines.length; i++) {
      const raw = lines[i];
      if (/\\verb/.test(raw)) continue; // \verb 内是字面量，其 `{...}` 不应成环境
      const code = raw.split("%")[0];   // 剥注释：`%` 之后为注释，其 `\begin{}` 不应成环境
      const m = code.match(beginRe);
      if (!m) continue;
      if (m[1] === "begin") {
        stack.push({ start: i, env: m[2] });
      } else {
        // end：与栈内最近的同名 begin 配对（转为 1-based 输出；B4：畸形/未闭合环境按就近配对、多余项可忽略）
        for (let s = stack.length - 1; s >= 0; s--) {
          if (stack[s].env === m[2]) {
            if (i - stack[s].start >= 1) {
              ranges.push({ start: stack[s].start + 1, end: i + 1 });
            }
            stack.length = s; // 弹出该 begin 及其上的未闭合项
            break;
          }
        }
      }
    }
    return ranges;
  },
};

/** 一次注册两套语言扩展（main.ts 在注册语法后调用）。 */
export function registerLatexProvider() {
  monaco.languages.registerCompletionItemProvider("latex", latexCompletionProvider);
  monaco.languages.registerFoldingRangeProvider("latex", latexFoldingProvider);
}
