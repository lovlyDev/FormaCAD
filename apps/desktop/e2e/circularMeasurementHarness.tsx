import { mountReferenceMeasurementHarness } from "./referenceMeasurementHarness";

/** Projects real mesh metadata through the actual App camera, without setting edge/face selection. */
export async function mountCircularMeasurementHarness(fixture:any){
 await mountReferenceMeasurementHarness(fixture);
 const Fiber=await import("@react-three/fiber"),THREE=await import("three"),{cadEdges}=await import("../src/features/viewer/edgeSelection"),{pickCadEdge}=await import("../src/features/viewer/pickCadEdge"),{faceReference,faceTriangleCounts,faceForTriangle}=await import("../src/features/viewer/faceSelection"),{useWorkspace}=await import("../src/stores/workspace");
 const globals=window as any;
 Object.assign(globals.__applyHarness,{
  selectedEdge:()=>useWorkspace.getState().selectedEdge,selectedFace:()=>useWorkspace.getState().selectedFace,
  seam:()=>{useWorkspace.getState().setSelected("body");useWorkspace.getState().setSelectedEdge({bodyId:"body",revisionId:fixture.head,edgeOrdinal:2});},
  candidates:()=>{
   const canvas=document.querySelector(".viewer-area canvas") as HTMLCanvasElement,live=Fiber._roots.get(canvas)?.store.getState(),mesh=live?.scene.getObjectByName("body");if(!live||!(mesh instanceof THREE.Mesh))return null;
   const rect=canvas.getBoundingClientRect(),width=rect.width,height=rect.height,camera=live.camera;mesh.updateWorldMatrix(true,false);camera.updateWorldMatrix(true,false);
   const project=(point:InstanceType<typeof THREE.Vector3>)=>{const p=point.clone().project(camera);return{x:(p.x+1)*width/2,y:(1-p.y)*height/2};};
   const edges:any[]=[],faces:any[]=[];
   const catalog=cadEdges(mesh)??[];
   catalog.forEach((edge,index)=>{let found=false;for(let i=1;i<edge.points.length&&!found;i++){const point=new THREE.Vector3(...edge.points[i-1]).lerp(new THREE.Vector3(...edge.points[i]),.5).applyMatrix4(mesh.matrixWorld),screen=project(point);
    // A silhouette edge needs an interior click within the production 8px edge tolerance.
    for(const [shiftX,shiftY] of [[0,0],[3,0],[-3,0],[0,3],[0,-3]]){const x=Math.round(rect.x+screen.x)+shiftX,y=Math.round(rect.y+screen.y)+shiftY;
    const stable=[[0,0],[1,0],[-1,0],[0,1],[0,-1]].every(([dx,dy])=>{
     if(x+dx<rect.left+3||x+dx>rect.right-3||y+dy<rect.top+3||y+dy>rect.bottom-3||document.elementFromPoint(x+dx,y+dy)!==canvas)return false;
     const cursor={x:x+dx-rect.left,y:y+dy-rect.top},ray=new THREE.Raycaster();ray.setFromCamera(new THREE.Vector2(cursor.x/width*2-1,1-cursor.y/height*2),camera);
     return ray.intersectObject(mesh,false).length>0&&pickCadEdge(mesh,camera,cursor,width,height)?.edgeOrdinal===index+1;
    });
    if(stable){edges.push({x,y,reference:edge.topologyRef??null});found=true;break;}}}});
   const counts=faceTriangleCounts(mesh),position=mesh.geometry.getAttribute("position"),indices=mesh.geometry.index;
   if(counts){let start=0;counts.forEach((count,index)=>{
    const points:{point:InstanceType<typeof THREE.Vector3>;score:number}[]=[],bounds=new THREE.Box3();
    for(let triangle=start;triangle<start+count;triangle++){
     const vertices=Array.from({length:3},(_,i)=>new THREE.Vector3().fromBufferAttribute(position,indices?indices.getX(triangle*3+i):triangle*3+i).applyMatrix4(mesh.matrixWorld));vertices.forEach(vertex=>bounds.expandByPoint(vertex));
     const screen=vertices.map(project),area=Math.abs((screen[1].x-screen[0].x)*(screen[2].y-screen[0].y)-(screen[2].x-screen[0].x)*(screen[1].y-screen[0].y));
     points.push({point:vertices[0].clone().add(vertices[1]).add(vertices[2]).divideScalar(3),score:area});
    }
    if(!bounds.isEmpty())points.push({point:bounds.getCenter(new THREE.Vector3()),score:Number.MAX_VALUE});
    for(const candidate of points.sort((a,b)=>b.score-a.score)){
     const screen=project(candidate.point),x=Math.round(rect.x+screen.x),y=Math.round(rect.y+screen.y);
     const stable=[[0,0],[2,0],[-2,0],[0,2],[0,-2]].every(([dx,dy])=>{
      if(x+dx<rect.left+3||x+dx>rect.right-3||y+dy<rect.top+3||y+dy>rect.bottom-3||document.elementFromPoint(x+dx,y+dy)!==canvas)return false;
      const ray=new THREE.Raycaster();ray.setFromCamera(new THREE.Vector2((x+dx-rect.left)/width*2-1,1-(y+dy-rect.top)/height*2),camera);const hit=ray.intersectObject(mesh,false)[0];return hit?.faceIndex!=null&&faceForTriangle(counts,hit.faceIndex)===index+1;
     });
     if(stable){faces.push({x,y,reference:faceReference(mesh,index+1)});break;}
    }
    start+=count;
   });}
   return{edges,faces,catalog:catalog.map(edge=>edge.topologyRef??null)};
  },
 });
}
