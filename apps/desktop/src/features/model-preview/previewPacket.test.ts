import { expect,it } from "vitest";
import { decodePreviewPacket } from "./previewPacket";
const hash="a".repeat(64);
function packet(){
 const header=new TextEncoder().encode(JSON.stringify({protocolVersion:1,sourceSha256:hash,metrics:{volumeMm3:5520,areaMm2:2400,faceCount:14,edgeCount:36,boundsMm:[40,30,5]}}));
 const bytes=new Uint8Array(4+header.length+20),view=new DataView(bytes.buffer);view.setUint32(0,header.length,true);bytes.set(header,4);const start=4+header.length;view.setUint32(start,0x46546c67,true);view.setUint32(start+4,2,true);view.setUint32(start+8,20,true);return bytes.buffer;
}
it("decodes metrics and bounded GLB from a matching source",()=>{const result=decodePreviewPacket(packet(),hash);expect(result.metrics.volumeMm3).toBe(5520);expect(result.glb.byteLength).toBe(20);});
it("rejects stale source, malformed headers, truncated or unsupported GLB",()=>{
 expect(()=>decodePreviewPacket(packet(),"b".repeat(64))).toThrow();expect(()=>decodePreviewPacket(packet().slice(0,5),hash)).toThrow();
 const bad=packet();new DataView(bad).setUint32(0,100000,true);expect(()=>decodePreviewPacket(bad,hash)).toThrow();
 const version=packet(),view=new DataView(version);view.setUint32(4+view.getUint32(0,true)+4,1,true);expect(()=>decodePreviewPacket(version,hash)).toThrow();
});
