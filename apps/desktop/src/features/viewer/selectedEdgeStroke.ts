import { LineSegments2, LineSegmentsGeometry, LineMaterial } from "three-stdlib";
import type { CadEdge } from "./edgeSelection";
export function selectedEdgeStroke(edge: CadEdge, width: number, height: number) {
  const positions: number[] = [];
  for (let i = 1; i < edge.points.length; i++) positions.push(...edge.points[i - 1], ...edge.points[i]);
  const geometry = new LineSegmentsGeometry();
  geometry.setPositions(positions);
  const material = new LineMaterial({ color: 0xf2a43a, linewidth: 4, depthTest: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -3 });
  material.resolution.set(width, height);
  const stroke = new LineSegments2(geometry, material);
  stroke.userData.formaSelectionOverlay = true;
  stroke.raycast = () => {};
  stroke.renderOrder = 20;
  return stroke;
}
