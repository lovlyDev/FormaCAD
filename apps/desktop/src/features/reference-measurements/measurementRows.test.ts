import { expect,it } from "vitest";
import { setLocale } from "../../i18n";
import { formatMeasurement,measurementRows } from "./measurementRows";
it("converts canonical length, area and volume with each unit's power",()=>{
  setLocale("en");
  expect(formatMeasurement(100,1,"cm")).toBe("10 cm");
  expect(formatMeasurement(100,2,"cm")).toBe("1 cm²");
  expect(formatMeasurement(1000,3,"cm")).toBe("1 cm³");
  expect(formatMeasurement(25.4,1,"inch")).toBe("1 in");
  expect(formatMeasurement(25.4**2,2,"inch")).toBe("1 in²");
  expect(formatMeasurement(25.4**3,3,"inch")).toBe("1 in³");
  expect(formatMeasurement(1e-6,2,"mm")).toBe("1E-6 mm²");
  setLocale("ru");expect(formatMeasurement(1000,3,"cm")).toBe("1 см³");
});
it("keeps face normals unitless while converting only canonical centroid coordinates",()=>{
  setLocale("en");const rows=measurementRows({kind:"planarFace",areaMm2:100,originMm:[10,-20,30],normal:[0,0,-1]},"cm");
  expect(rows[0].value).toBe("1 cm²");expect(rows[1].value).toBe("1 cm · -2 cm · 3 cm");expect(rows[2].value).toBe("0 · 0 · -1");
});
it("formats native circular radius and diameter as independent lengths in both languages",()=>{
  setLocale("en");expect(measurementRows({kind:"edgeRadius",radiusMm:25.4},"inch")).toEqual([{label:"Exact selected edge radius",value:"1 in"}]);
  expect(measurementRows({kind:"edgeDiameter",diameterMm:50.8},"inch")).toEqual([{label:"Exact selected edge diameter",value:"2 in"}]);
  setLocale("ru");expect(measurementRows({kind:"edgeDiameter",diameterMm:20},"cm")).toEqual([{label:"Точный диаметр выбранного ребра",value:"2 см"}]);
});
