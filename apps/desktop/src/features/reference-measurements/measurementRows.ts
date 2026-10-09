import { number,t,getLocale } from "../../i18n";
import type { MeasurementValue } from "./measurementSchema";
export type DisplayUnit="mm"|"cm"|"inch";
const factor=(unit:DisplayUnit)=>unit==="inch"?25.4:unit==="cm"?10:1;
export function formatMeasurement(value:number,power:1|2|3,unit:DisplayUnit):string {
  const symbol=unit==="inch"?t("in"):t(unit);
  const converted=value/factor(unit)**power;
  const formatted=converted!==0&&Math.abs(converted)<.001
    ? new Intl.NumberFormat(getLocale()==="ru"?"ru-RU":"en-US",{notation:"scientific",maximumSignificantDigits:6}).format(converted)
    : number(converted,3);
  return `${formatted} ${symbol}${power===2?"²":power===3?"³":""}`;
}
export interface MeasurementRow {label:string;value:string}
export function measurementRows(result:MeasurementValue,unit:DisplayUnit):MeasurementRow[]{
  switch(result.kind){
    case "edgeLength":return[{label:t("Exact selected edge length"),value:formatMeasurement(result.lengthMm,1,unit)}];
    case "edgeRadius":return[{label:t("Exact selected edge radius"),value:formatMeasurement(result.radiusMm,1,unit)}];
    case "edgeDiameter":return[{label:t("Exact selected edge diameter"),value:formatMeasurement(result.diameterMm,1,unit)}];
    case "faceArea":return[{label:t("Exact selected face area"),value:formatMeasurement(result.areaMm2,2,unit)}];
    case "planarFace":return[
      {label:t("Exact selected face area"),value:formatMeasurement(result.areaMm2,2,unit)},
      {label:t("Face area centroid (CAD X, Y, Z)"),value:result.originMm.map(value=>formatMeasurement(value,1,unit)).join(" · ")},
      {label:t("Outward face normal (unitless)"),value:result.normal.map(value=>number(value,6)).join(" · ")},
    ];
    case "bodyMetrics":return[
      {label:t("Exact selected body volume"),value:formatMeasurement(result.volumeMm3,3,unit)},
      {label:t("Exact selected body surface area"),value:formatMeasurement(result.areaMm2,2,unit)},
      {label:t("Selected body extents (CAD X, Y, Z)"),value:result.extentsMm.map(value=>formatMeasurement(value,1,unit)).join(" × ")},
      {label:t("Faces"),value:number(result.faceCount,0)}, {label:t("Edges"),value:number(result.edgeCount,0)},
    ];
  }
}
