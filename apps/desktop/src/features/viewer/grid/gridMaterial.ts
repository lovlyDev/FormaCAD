import { Color,DoubleSide,ShaderMaterial } from "three";
/** Crossfade adjacent metric decades; suppress lines that are smaller than pixels. */
export function createGridMaterial(){return new ShaderMaterial({transparent:true,depthWrite:false,side:DoubleSide,uniforms:{fadeDistance:{value:550},cellSize:{value:10},scaleBlend:{value:0},cellColor:{value:new Color()},sectionColor:{value:new Color()}},vertexShader:`
 uniform float fadeDistance;varying vec3 worldPosition;
 void main(){vec3 local=vec3(position.x*fadeDistance,0.,position.y*fadeDistance);vec4 world=modelMatrix*vec4(local,1.);worldPosition=world.xyz;gl_Position=projectionMatrix*viewMatrix*world;}
 `,fragmentShader:`
 uniform float fadeDistance;uniform float cellSize;uniform float scaleBlend;uniform vec3 cellColor;uniform vec3 sectionColor;varying vec3 worldPosition;
 float lines(float spacing,float thickness){vec2 coordinate=worldPosition.xz/spacing;vec2 derivative=max(fwidth(coordinate),vec2(.000001));vec2 edge=abs(fract(coordinate-.5)-.5)/derivative;float coverage=1.-smoothstep(thickness*.5,thickness*.5+1.,min(edge.x,edge.y));float pixels=1./max(derivative.x,derivative.y);return coverage*smoothstep(3.,12.,pixels);}
 void main(){float fine=mix(lines(cellSize,.65),lines(cellSize*10.,.65),scaleBlend);float major=mix(lines(cellSize*5.,1.),lines(cellSize*50.,1.),scaleBlend);float fade=pow(max(0.,1.-length(worldPosition.xz)/fadeDistance),1.8);float alpha=min(.65,fine*.32+major*.55)*fade;if(alpha<.005)discard;gl_FragColor=vec4(mix(cellColor,sectionColor,major),alpha);
 #include <tonemapping_fragment>
 #include <colorspace_fragment>
 }
 `});}
