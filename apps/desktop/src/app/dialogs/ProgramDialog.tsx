import { EditorDisclosure } from "../../features/model-editor/EditorDisclosure";
import { createLinkedProfiles } from "../../lib/templates/linkedProfiles";
import { useSketchBuildReview } from "../../features/model-editor/sketch-review/useSketchBuildReview";
import { SketchBuildReview } from "../../features/model-editor/sketch-review/SketchBuildReview";
import { EditorScroll } from "../../features/model-editor/EditorScroll";
import { EdgeFilletEditor } from "../../features/model-editor/EdgeFilletEditor";
import { useEditorPreference } from "../../features/model-editor/editorPreferences";
import { flushPreferences } from "../../lib/persistence";
import { useState } from "react";
import { Box,Link2,Code2,Braces,Check,Plus,Layers } from "lucide-react";
import { t,number } from "../../i18n";
import { CadFeatureEditor } from "../../components/CadFeatureEditor";
import { TypedFeatureTree } from "../../components/TypedFeatureTree";
import { Button,Modal } from "../../components/ui";
import { native } from "../../lib/api";
import { usePersistentState } from "../../lib/persistence";
import { useWorkspace } from "../../stores/workspace";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { importStepHistory, hasImportedSource } from "../../features/import/importStepHistory";
import { createStarterSketch } from "../../lib/sketchDocument";
import { useModelPreview } from "../../features/model-preview/useModelPreview";
import { ModelPreviewPanel } from "../../features/model-preview/ModelPreviewPanel";
import { previewError } from "../../features/model-preview/previewError";
import "./ProgramDialog.css";
export function ProgramDialog({open,close,source,disabled,nativeCadAvailable,nativeCadChecked,importedSource,onApply}:{open:boolean;close:()=>void;source:string;disabled:boolean;nativeCadAvailable:boolean;nativeCadChecked:boolean;importedSource?:string;onApply:(source:string)=>void}){
 const project=useWorkspace(state=>state.project);
 const draftKey=`forma.ui.project.${project?.id??"home"}.source.${project?.currentRevision??"new"}`;
 const [draft,setDraft]=usePersistentState(draftKey,source),document=readTypedCadDocument(draft),typed=!!document;
 const [sourceOpen,setSourceOpen]=useEditorPreference("source.open",!readTypedCadDocument(source)),[formatError,setFormatError]=useState(false);
 const [autoPreview,setAutoPreview]=useEditorPreference("preview.auto",false),[refresh,setRefresh]=useState(0);
 const review=useSketchBuildReview(draft,open&&typed&&nativeCadAvailable&&!disabled);
 const preview=useModelPreview(project?.id??"",draft,project?.currentRevision??null,open&&autoPreview&&typed&&nativeCadAvailable&&!disabled,refresh);
 const importFile=project?.files.find(file=>file.name===importedSource);
 const importHistory=importStepHistory(importFile,project?.currentRevision??"draft");
 const importedOnly=!!importedSource&&!source.trim()&&!hasImportedSource(document,importFile);
 const blocked=disabled||importedOnly||review.pending||review.blocked||preview.pending||preview.waiting;
 const finding=preview.error?previewError(preview.error):null;
 return <Modal open={open} onClose={()=>{void flushPreferences().catch(console.error);close();}} title={t("Model editor")} description={t("Edit parameters, review geometry and build a new revision.")} wide className="model-editor-modal">
  <div className="model-editor-toolbar"><span><Box size={15}/>{typed?t("CAD IR v2"):t("CAD source")}</span><span className="model-editor-draft">{draft!==source?t("Unsaved draft"):t("Saved source")}</span>
   {document&&<span className="model-editor-counts"><Layers size={13}/>{t("{{value0}} operations · {{value1}} bodies",{value0:number(document.features.length,0),value1:number(document.bodies.length,0)})}</span>}
  </div>
  <EditorScroll><div className={typed?"model-editor-layout":"model-editor-layout legacy"}>
   <div className="model-editor-main">
    <SketchBuildReview {...review} source={draft} disabled={disabled} onChange={setDraft}/>
    <EdgeFilletEditor source={draft} savedSource={source} disabled={disabled} onChange={setDraft}/>
    {typed?<TypedFeatureTree source={draft} disabled={disabled} onChange={setDraft} failedFeatureId={finding?.targetId}/>:<CadFeatureEditor source={draft} disabled={disabled} onChange={setDraft}/>}
    {importedOnly&&<div className="model-editor-empty"><Box size={30}/><p>{importHistory?t("Keep the original STEP as the first feature and add native operations above it. The original construction history cannot be recovered."):t("Imported geometry has no editable operation history. Create a separate project for a new sketch; this model will stay unchanged.")}</p><small>{importedSource}</small>{importHistory&&<Button disabled={disabled||!nativeCadAvailable} onClick={()=>{setDraft(importHistory);setSourceOpen(false);}}><Plus size={15}/>{t("Enable native operations for this STEP")}</Button>}</div>}
    {!importedOnly&&!draft.trim()&&<div className="model-editor-empty"><Box size={30}/><p>{t("Start with a 2D sketch or paste a CAD document.")}</p><Button disabled={disabled||!nativeCadAvailable} onClick={()=>{setDraft(createStarterSketch());setSourceOpen(false);}}><Plus size={15}/>{t("Create 2D sketch")}</Button><Button disabled={disabled||!nativeCadAvailable} onClick={()=>{setDraft(createLinkedProfiles());setSourceOpen(false);}}><Link2 size={15}/>{t("Create linked base and triangle")}</Button></div>}
    {!draft.trim()&&native&&nativeCadChecked&&!nativeCadAvailable&&<p className="field-hint">{t("2D sketches require the native OpenCascade build.")}</p>}
    {!importedOnly&&<EditorDisclosure icon={Code2} title={t("CAD source")} hint={t("Advanced")} open={sourceOpen} onOpenChange={setSourceOpen}>
     <div className="model-editor-code-toolbar"><span>{typed?t("JSON · CAD IR v2"):t("CAD source")}</span><button type="button" disabled={disabled||!draft.trimStart().startsWith("{")} onClick={()=>{try{setDraft(JSON.stringify(JSON.parse(draft),null,2));setFormatError(false);}catch{setFormatError(true);}}}><Braces size={13}/>{t("Format JSON")}</button></div>
     {formatError&&<p role="alert">{t("Invalid JSON; source was not changed.")}</p>}
     <textarea className="program-editor" aria-label={t("CAD source")} value={draft} spellCheck={false} onChange={event=>{setDraft(event.target.value);setFormatError(false);}} disabled={disabled}/>
    </EditorDisclosure>}
   </div>
   {typed&&<ModelPreviewPanel {...preview} available={preview.available&&nativeCadAvailable} enabled={autoPreview} disabled={disabled} onEnable={setAutoPreview} onRefresh={()=>{setAutoPreview(true);setRefresh(value=>value+1);}}/>}
  </div></EditorScroll>
  <div className="model-editor-footer"><small><Check size={13}/>{t("Building saves a new revision after geometry validation.")}</small><div className="modal-actions"><Button onClick={()=>{void flushPreferences().catch(console.error);close();}}>{t("Закрыть")}</Button><Button className="primary" disabled={blocked||!draft.trim()||!native} onClick={()=>{void flushPreferences().catch(console.error);onApply(draft);}}><Check size={15}/>{t("Построить")}</Button></div></div>
 </Modal>;
}
