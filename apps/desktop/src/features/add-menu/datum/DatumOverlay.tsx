import { Line } from "@react-three/drei";
import { useMemo } from "react";
import { DoubleSide, Quaternion, Vector3, type Object3D } from "three";
import { useWorkspace } from "../../../stores/workspace";
import { cadPointToRender, cadWorldFrame } from "../../transform/viewport/renderBasis";
import { projectDatums } from "./datumPlacement";

const localNormal = new Vector3(0, 0, 1);
const noPick = () => {};

export function DatumOverlay({ object, light }: { object: Object3D; light: boolean }) {
  const program = useWorkspace((state) => state.project?.revisions.find(
    (revision) => revision.id === state.project?.currentRevision,
  )?.program);
  const nativeFrames = object.userData.formaNativeCadPreview ? object.userData.formaDatums : undefined;
  const datums = useMemo(() => projectDatums(program, nativeFrames), [program, nativeFrames]);
  if (datums.length === 0) return null;
  const frame = cadWorldFrame(object.userData.formaNativeCadPreview ? object : null);
  const scale = new Vector3().setFromMatrixScale(frame);
  const color = light ? "#52616e" : "#bdcad5";
  return (
    <group name="forma.datums">
      {datums.map(({ id, datum }) => {
        const centre = cadPointToRender(datum.originMm, frame);
        if (datum.type === "datumPoint") {
          return (
            <mesh key={id} name={id} position={centre} raycast={noPick}>
              <sphereGeometry args={[0.85 * scale.x, 12, 8]} />
              <meshBasicMaterial color={color} />
            </mesh>
          );
        }
        if (datum.type === "datumAxis") {
          const half = new Vector3(...datum.direction).normalize().multiplyScalar(datum.lengthMm / 2);
          const origin = new Vector3(...datum.originMm);
          const start = origin.clone().sub(half).applyMatrix4(frame);
          const end = origin.add(half).applyMatrix4(frame);
          return <Line key={id} name={id} points={[start, end]} color={color} lineWidth={1.5} raycast={noPick} />;
        }
        const normal = new Vector3(...datum.normal).transformDirection(frame);
        const rotation = new Quaternion().setFromUnitVectors(localNormal, normal);
        return (
          <mesh key={id} name={id} position={centre} quaternion={rotation} raycast={noPick}>
            <planeGeometry args={[datum.sideMm * scale.x, datum.sideMm * scale.x]} />
            <meshBasicMaterial color={color} transparent opacity={0.25} depthWrite={false} side={DoubleSide} />
          </mesh>
        );
      })}
    </group>
  );
}
