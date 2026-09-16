import fs from "node:fs";
import path from "node:path";
import ts from "typescript";

const root = "apps/desktop/src";
const en = JSON.parse(fs.readFileSync(`${root}/i18n/en.json`, "utf8"));
const ru = JSON.parse(fs.readFileSync(`${root}/i18n/ru.json`, "utf8"));
const issues = [];
const placeholders = (text) => [...text.matchAll(/\{\{(\w+)\}\}/g)].map((m) => m[1]).sort().join(",");
for (const key of new Set([...Object.keys(en), ...Object.keys(ru)])) {
  if (!en[key] || !ru[key]) issues.push(`Missing translation: ${key}`);
  else if (placeholders(en[key]) !== placeholders(ru[key])) issues.push(`Placeholder mismatch: ${key}`);
}
// Product names, keyboard notation, language self-names and actual source filenames.
const literals = new Set(["forma", "FORMA /", "F", "v", "Ctrl K", "ESC", "OpenAI Codex", "Claude Code", "Codex", "Claude", "Русский", "English", "model.cad.json", "model.py", "model.parameters.json"]);
const attrs = new Set(["title", "label", "description", "placeholder", "aria-label"]);
for (const file of fs.readdirSync(root, { recursive: true }).filter((file) => /\.tsx?$/.test(file) && !file.includes(".test.") && !file.includes("test-setup"))) {
  const source = fs.readFileSync(path.join(root, file), "utf8");
  const sf = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true);
  const report = (node, text) => issues.push(`${file}:${sf.getLineAndCharacterOfPosition(node.getStart(sf)).line + 1}: ${text}`);
  function visit(node) {
    if (ts.isJsxText(node)) {
      const text = node.text.replace(/\s+/g, " ").trim();
      if (/[A-Za-zА-Яа-яЁё]/.test(text) && !literals.has(text)) report(node, `Unlocalized JSX: ${text}`);
    }
    if (ts.isJsxAttribute(node) && attrs.has(node.name.getText(sf)) && node.initializer && ts.isStringLiteral(node.initializer) && !literals.has(node.initializer.text)) report(node, `Unlocalized attribute: ${node.initializer.text}`);
    if (ts.isCallExpression(node) && node.expression.getText(sf) === "t" && node.arguments[0] && ts.isStringLiteral(node.arguments[0]) && !en[node.arguments[0].text]) report(node, `Unknown translation: ${node.arguments[0].text}`);
    if (ts.isStringLiteral(node) && (ts.isConditionalExpression(node.parent) || (ts.isBinaryExpression(node.parent) && node.parent.right === node && [ts.SyntaxKind.QuestionQuestionToken, ts.SyntaxKind.BarBarToken].includes(node.parent.operatorToken.kind)))) {
      let owner = node.parent;
      while (owner.parent && !ts.isJsxExpression(owner) && !ts.isStatement(owner)) owner = owner.parent;
      if (ts.isJsxExpression(owner) && (!ts.isJsxAttribute(owner.parent) || attrs.has(owner.parent.name.getText(sf))) && /[A-Za-zА-Яа-яЁё]/.test(node.text) && !literals.has(node.text)) report(node, `Unlocalized conditional label: ${node.text}`);
    }
    ts.forEachChild(node, visit);
  }
  visit(sf);
}
if (issues.length) { console.error(issues.join("\n")); process.exitCode = 1; }
else console.log(`i18n checked: ${Object.keys(en).length} EN/RU messages, matching placeholders, no raw JSX labels.`);
