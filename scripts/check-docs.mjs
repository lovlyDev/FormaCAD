import fs from "node:fs";
import path from "node:path";
const files = fs.readdirSync('.').filter(f=>f.endsWith('.md'));
files.push(...fs.readdirSync('docs',{recursive:true}).filter(f=>f.endsWith('.md')&&!f.startsWith('verification')).map(f=>path.join('docs',f)));
const errors=[];
for(const file of files) {
  const text=fs.readFileSync(file,'utf8');
  for(const match of text.matchAll(/\]\(([^)]+)\)/g)) {
    const link=match[1];
    if(/^(https?:|mailto:|#)/.test(link))continue;
    const target=decodeURIComponent(link.split('#')[0]);
    if(!fs.existsSync(path.resolve(path.dirname(file),target)))errors.push(`${file}: missing ${target}`);
  }
}
for(const f of fs.readdirSync('docs/en'))if(!fs.existsSync(path.join('docs/ru',f)))errors.push(`Missing Russian guide: ${f}`);
if(errors.length)throw new Error(errors.join('\n'));
console.log(`Checked local links in ${files.length} Markdown documents`);
