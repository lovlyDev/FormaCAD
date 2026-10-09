import { z } from "zod";
const metricsSchema=z.object({volumeMm3:z.number().finite().nonnegative(),areaMm2:z.number().finite().nonnegative(),faceCount:z.number().int().positive(),edgeCount:z.number().int().nonnegative(),boundsMm:z.tuple([z.number().finite().nonnegative(),z.number().finite().nonnegative(),z.number().finite().nonnegative()])});
const headerSchema=z.object({protocolVersion:z.literal(1),sourceSha256:z.string().regex(/^[a-f0-9]{64}$/),metrics:metricsSchema});
export type PreviewMetrics=z.infer<typeof metricsSchema>;
export function decodePreviewPacket(packet:ArrayBuffer,expectedHash:string):{metrics:PreviewMetrics;glb:ArrayBuffer}{
 if(packet.byteLength<24||packet.byteLength>40*1024*1024+65540)throw new Error("Model preview response is invalid");
 const view=new DataView(packet),length=view.getUint32(0,true);
 if(length>65536||length<2||length+24>packet.byteLength)throw new Error("Model preview response is invalid");
 const header=headerSchema.parse(JSON.parse(new TextDecoder().decode(new Uint8Array(packet,4,length))));
 const glb=packet.slice(4+length),glbView=new DataView(glb);
 if(header.sourceSha256!==expectedHash||glbView.getUint32(0,true)!==0x46546c67||glbView.getUint32(4,true)!==2||glbView.getUint32(8,true)!==glb.byteLength)throw new Error("Model preview response is invalid");
 return {metrics:header.metrics,glb};
}
