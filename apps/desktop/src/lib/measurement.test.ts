import { expect, it } from "vitest";
import { Vector3 } from "three";
import { measurement } from "./measurement";
it("measures a 3D angle and circle and rejects collinear circle points", () => {
  expect(
    measurement(
      [new Vector3(1, 0, 0), new Vector3(), new Vector3(0, 0, 1)],
      "angle",
    ),
  ).toBe("90.00°");
  expect(
    measurement(
      [new Vector3(5, 0, 0), new Vector3(0, 5, 0), new Vector3(-5, 0, 0)],
      "circle",
    ),
  ).toBe("R 5.00 mm · Ø 10.00 mm");
  expect(
    measurement(
      [new Vector3(), new Vector3(1, 0, 0), new Vector3(2, 0, 0)],
      "circle",
    ),
  ).toContain("collinear");
});
