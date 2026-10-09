import { Checkbox } from "../../components/ui";
import { PersistentDetails } from "../model-editor/editorPreferences";
import { useState,lazy,Suspense } from "react";
import { Box,Scan,RotateCw,Maximize,LoaderCircle,AlertCircle,CheckCircle2 } from "lucide-react";
import { t,number } from "../../i18n";
import type { ModelPreview } from "./previewApi";
const PreviewCanvas=lazy(()=>import("./PreviewCanvas").then(module=>({default:module.PreviewCanvas})));
import { previewError } from "./previewError";
import "./ModelPreviewPanel.css";
export function ModelPreviewPanel({available,enabled,pending,waiting,preview,error,disabled,onEnable,onRefresh}:{available:boolean;enabled:boolean;pending:boolean;waiting:boolean;preview:ModelPreview|null;error:string|null;disabled:boolean;onEnable:(value:boolean)=>void;onRefresh:()=>void}) {
 const [fit,setFit]=useState(0);const finding=error?previewError(error):null;
 return <section className="model-preview-panel" aria-label={t("3D draft preview")}>
  <header><Box size={18}/><strong>{t("3D draft preview")}</strong><span className="preview-readonly">{t("Read-only")}</span></header>
  <p className="model-preview-description">{t("Review exact geometry before saving a new revision.")}</p>
  <div className="model-preview-actions">
   <button type="button" disabled={disabled||!available||pending} onClick={onRefresh}><RotateCw size={14}/>{t("Preview model")}</button>
   <button type="button" disabled={!preview} aria-label={t("Fit preview to view")} onClick={()=>setFit(value=>value+1)}><Maximize size={15}/></button>
  </div>
  <div className="model-preview-auto"><Checkbox checked={enabled} disabled={disabled||!available} onChange={onEnable}>{t("Auto-update preview")}</Checkbox></div>
  <div className="model-preview-stage">
   {preview?<Suspense fallback={<div className="model-preview-empty">{t("Loading preview…")}</div>}><PreviewCanvas object={preview.object} fit={fit}/></Suspense>:<div className="model-preview-empty">{pending||waiting?<LoaderCircle size={30} className="preview-spinner"/>:<Scan size={36}/>}<span>{!available?t("3D preview is available in the native desktop app."):pending||waiting?t("Building draft preview…"):finding?t("Preview could not be built"):t("Preview your draft to inspect its geometry.")}</span></div>}
  </div>
  <div className="model-preview-status" role="status">{pending||waiting?<><LoaderCircle size={14} className="preview-spinner"/>{t("Building draft preview…")}</>:preview?<><CheckCircle2 size={14}/>{t("Draft geometry is valid")}</>:finding?<><AlertCircle size={14}/>{t(finding.message)}</>:<>{t("Preview does not change the saved model or history.")}</>}</div>
  {preview&&!pending&&!waiting&&<dl className="model-preview-metrics">
   <div><dt>{t("Preview volume")}</dt><dd>{number(preview.metrics.volumeMm3,3)} {t("mm³")}</dd></div>
   <div><dt>{t("Preview surface area")}</dt><dd>{number(preview.metrics.areaMm2,3)} {t("mm²")}</dd></div>
   <div><dt>{t("Preview bounds")}</dt><dd>{preview.metrics.boundsMm.map(value=>number(value,2)).join(" × ")} {t("mm")}</dd></div>
  </dl>}
  {finding&&<PersistentDetails stateId="preview.error" className="model-preview-error"><summary>{t("Technical details")}</summary><code>{finding.detail}</code></PersistentDetails>}
 </section>;
}
