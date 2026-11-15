import { number, t } from "../../i18n";
import type { IgesReport } from "./igesSchema";
import "./iges.css";
export function IgesMeasurements({ report }: { report: IgesReport }) {
  const rows = [
    [t("Volume"), `${number(report.source.volumeMm3, 6)} ${t("mm³")}`, `${number(report.roundTrip.volumeMm3, 6)} ${t("mm³")}`],
    [t("Surface area"), `${number(report.source.areaMm2, 6)} ${t("mm²")}`, `${number(report.roundTrip.areaMm2, 6)} ${t("mm²")}`],
    [t("Closed solids"), number(report.source.solidCount, 0), number(report.roundTrip.solidCount, 0)],
    [t("Faces"), number(report.source.faceCount, 0), number(report.roundTrip.faceCount, 0)],
    [t("Edges"), number(report.source.edgeCount, 0), number(report.roundTrip.edgeCount, 0)],
    [t("Bounds (mm)"), report.source.boundsMm.map(v => number(v, 4)).join(" × "), report.roundTrip.boundsMm.map(v => number(v, 4)).join(" × ")],
  ];
  return <table className="iges-exchange-table"><caption>{t("IGES round-trip measurements")}</caption>
    <thead><tr><th>{t("Measurement")}</th><th>{t("Before conversion")}</th><th>{t("After round-trip")}</th></tr></thead>
    <tbody>{rows.map(([label, source, restored]) => <tr key={label}><th scope="row">{label}</th><td>{source}</td><td>{restored}</td></tr>)}</tbody>
  </table>;
}
