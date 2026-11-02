import { describe, expect, it } from "vitest";
import { Vector3 } from "three";
import { previewDelta } from "../transformPreview";
import {
  cadDeltaToRender,
  cadPointToRender,
  renderDirectionToCad,
  renderPointToCad,
} from "./renderBasis";

const close = (actual: Vector3, expected: [number, number, number]) =>
  expected.forEach((value, index) =>
    expect(actual.getComponent(index)).toBeCloseTo(value, 9),
  );

describe("CAD to render basis", () => {
  it("maps CAD Z-up millimetres to the Y-up preview scene and back", () => {
    close(cadPointToRender([1, 2, 3]), [1, 3, -2]);
    expect(renderPointToCad(new Vector3(1, 3, -2))).toEqual([1, 2, 3]);
    renderDirectionToCad(new Vector3(0, 1, 0)).forEach((value, index) =>
      expect(value).toBeCloseTo([0, 0, 1][index], 12),
    );
  });
  it("moves along CAD Y and Z without swapping signs", () => {
    const y = cadDeltaToRender(previewDelta({ kind: "move", offsetMm: [0, 10, 0] }));
    close(new Vector3().applyMatrix4(y), [0, 0, -10]);
    const z = cadDeltaToRender(previewDelta({ kind: "move", offsetMm: [0, 0, 7] }));
    close(new Vector3().applyMatrix4(z), [0, 7, 0]);
  });
  it("rotates about an offset CAD pivot consistently in both spaces", () => {
    const intent = {
      kind: "rotate" as const,
      axis: [0, 0, 1] as [number, number, number],
      angleDeg: 90,
      pivotMm: [10, 0, 0] as [number, number, number],
    };
    const cad = new Vector3(20, 0, 5).applyMatrix4(previewDelta(intent));
    close(cad, [10, 10, 5]);
    const render = cadPointToRender([20, 0, 5]).applyMatrix4(
      cadDeltaToRender(previewDelta(intent)),
    );
    close(render, cadPointToRender([10, 10, 5]).toArray() as [number, number, number]);
  });
});
