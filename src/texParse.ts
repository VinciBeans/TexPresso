// texParse.ts：LaTeX 文本行解析辅助（纯函数，不依赖 Monaco/Tauri），
// 供折叠 / 大纲 / 代码片段补全共用，便于单测（modules.md §6、design.md §测试）。
//
// 背景：Monaco 片段引擎把 `\` 当转义（`\$`/`\}`/`\\`），而折叠/大纲要正确区分
// 「真环境/章节命令」与「注释 / \verb 字面量里的假匹配」，故把这类行级剥离子程序集中于此。

/**
 * 剥离 LaTeX 行内的 `\verb` 跨度与真注释，返回「代码」部分。
 * ① 移除非行内 `\verb*?<delim>…<delim>` 字面量（其内容含 `%`/`{...}` 不应被当作注释/环境/章节）；
 * ② 在「未转义」的 `%` 处截断（`\%` 是字面量百分号，不作注释——C3）。
 * 结果用于匹配 `\begin`/`\section` 等结构命令。
 */
export function stripTexComment(line: string): string {
  // \verb 的定界符是紧跟其后的任意非字母、非空白、非反斜杠字符（常为 |）。
  // (.*?)\1 非贪婪吃到下一个相同定界符。
  const noVerbatim = line.replace(/\\verb\*?([^a-zA-Z\s\\])(.*?)\1/g, "");
  // 在未被反斜杠转义的 `%`（真注释）处截断；`\%` 保留。
  return noVerbatim.split(/(?<!\\)%/)[0];
}

/**
 * 代码片段补全时是否应剥掉片段自带的前导 `\`：当「词首前一字符」为 `\` 时返回 true。
 * 依赖：Monaco `getWordUntilPosition` 的当前词**不含** `\`（如 `\sec`→word=`sec`，startColumn=2），
 * 故用 startColumn 前一字符判断即可。若某日 `wordPattern` 把 `\` 纳入词字符，
 * 此处语义会变（建议列表可能被清空）——由 texParse.spec.ts 锁定该判定，改 wordPattern 时需同步。
 */
export function shouldStripLeadBackslash(line: string, startColumn: number): boolean {
  return startColumn > 1 && line.charCodeAt(startColumn - 2) === 92; // '\\'
}
