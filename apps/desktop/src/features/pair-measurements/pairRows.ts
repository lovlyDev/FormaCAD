import { number,t } from "../../i18n";
import { formatMeasurement,type DisplayUnit,type MeasurementRow } from "../reference-measurements/measurementRows";
import type { PairReport } from "./pairSchema";
export function pairRows(result:PairReport["result"],unit:DisplayUnit):MeasurementRow[]{
 if(result.kind==="minimumDistance")return[
  {label:t("Exact minimum distance"),value:formatMeasurement(result.distanceMm,1,unit)},
  {label:t("First minimum witness (CAD X, Y, Z)"),value:result.pointAMm.map(value=>formatMeasurement(value,1,unit)).join(" · ")},
  {label:t("Second minimum witness (CAD X, Y, Z)"),value:result.pointBMm.map(value=>formatMeasurement(value,1,unit)).join(" · ")},
 ];
 return[{label:t(result.kind==="faceNormalAngle"?"Exact outward face-normal angle":"Exact acute straight-edge angle"),value:`${number(result.angleDeg,6)}°`}];
}
