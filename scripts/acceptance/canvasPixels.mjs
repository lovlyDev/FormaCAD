import { createRequire } from "node:module";
import path from "node:path";
// Reuse the PNG decoder bundled with the installed Playwright; no new dependency.
const require=createRequire(import.meta.url);
const {PNG}=require(path.join(path.dirname(require.resolve("playwright-core")),"lib/utilsBundle.js"));
export function foregroundPixels(buffer){
 const {data,width,height}=PNG.sync.read(buffer);
 const background=[data[0],data[1],data[2]];
 let count=0;
 for(let y=4;y<height-4;y++)for(let x=4;x<width-4;x++){
  const offset=(y*width+x)*4;
  if(Math.abs(data[offset]-background[0])+Math.abs(data[offset+1]-background[1])+Math.abs(data[offset+2]-background[2])>60)count++;
 }
 return count;
}
