import { Html } from "@react-three/drei";
import { useFrame, useThree } from "@react-three/fiber";
import { ArrowRight } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Vector2, Vector3, type Group } from "three";
import { Button } from "../../components/ui";
import { number, t } from "../../i18n";
import type { Project } from "../../types";
import { measureReference } from "../reference-measurements/measurementApi";
import { captureMeasurement } from "../reference-measurements/measurementCapture";
import { cadPointToRender, cadWorldFrame } from "../transform/viewport/renderBasis";
import type { FaceSelection } from "../viewer/faceSelection";

type Frame = { origin: Vector3; axis: Vector3 };
type Drag = { pointerId: number; start: Vector2; pixelsPerMm: Vector2 };

export function FaceExtrusionHandle({ project, selection, object, open }: {
  project: Project;
  selection: FaceSelection;
  object: Group;
  open: (distanceMm?: number) => void;
}) {
  const { camera, size } = useThree();
  const [frame, setFrame] = useState<Frame | null>(null);
  const [distance, setDistance] = useState(0);
  const drag = useRef<Drag | null>(null);
  const icon = useRef<SVGSVGElement>(null);
  const position = useRef(new Vector3());
  const axisEnd = useRef(new Vector3());
  const projection = useRef(new Vector2());

  useEffect(() => {
    if (!selection.topologyRef) return;
    const captured = captureMeasurement({
      project, bodyId: selection.bodyId,
      query: { kind: "planarFace", reference: selection.topologyRef },
      selectionToken: object.uuid, interactive: true,
    });
    if (!captured) return;
    let current = true;
    void measureReference(captured).then(({ result }) => {
      if (!current || result.kind !== "planarFace") return;
      const basis = cadWorldFrame(object);
      const origin = cadPointToRender(result.originMm, basis);
      const axis = new Vector3(...result.normal).transformDirection(basis);
      setFrame({ origin, axis });
    }).catch(() => {
      // The exact numeric route remains available if the measurement cannot be loaded.
    });
    return () => { current = false; };
  }, [project, selection, object]);

  useFrame(() => {
    if (!frame || !icon.current) return;
    position.current.copy(frame.origin).project(camera);
    axisEnd.current.copy(frame.origin).add(frame.axis).project(camera);
    const dx = (axisEnd.current.x - position.current.x) * size.width / 2;
    const dy = (position.current.y - axisEnd.current.y) * size.height / 2;
    projection.current.set(dx, dy);
    icon.current.style.transform = `rotate(${Math.atan2(dy, dx)}rad)`;
  });

  if (!frame) return null;
  const reset = () => { drag.current = null; setDistance(0); };
  const offset = (x: number, y: number) => {
    if (!drag.current) return 0;
    const { start, pixelsPerMm } = drag.current;
    const mm = new Vector2(x, y).sub(start).dot(pixelsPerMm) / pixelsPerMm.lengthSq();
    return Math.max(-10000, Math.min(10000, mm));
  };
  return (
    <Html center position={frame.origin.clone().addScaledVector(frame.axis, distance)}>
      <Button aria-label={t("Drag to extrude face")} title={t("Drag to extrude face")}
        style={{ touchAction: "none" }}
        onPointerDown={(event) => {
          event.stopPropagation();
          if (event.button !== 0 || projection.current.lengthSq() < 0.01) return;
          event.currentTarget.setPointerCapture(event.pointerId);
          drag.current = {
            pointerId: event.pointerId, start: new Vector2(event.clientX, event.clientY),
            pixelsPerMm: projection.current.clone(),
          };
        }}
        onPointerMove={(event) => {
          if (drag.current?.pointerId === event.pointerId)
            setDistance(offset(event.clientX, event.clientY));
        }}
        onPointerUp={(event) => {
          if (drag.current?.pointerId !== event.pointerId) return;
          const mm = offset(event.clientX, event.clientY);
          reset();
          if (Math.abs(mm) > 1e-7) open(mm);
        }}
        onLostPointerCapture={reset}
        onKeyDown={(event) => { if (event.key === "Escape") reset(); }}
        onClick={(event) => { if (event.detail === 0) open(); }}>
        <ArrowRight ref={icon} size={20} aria-hidden />
        {distance !== 0 && <span>{t("{{distance}} mm", { distance: number(distance) })}</span>}
      </Button>
    </Html>
  );
}
