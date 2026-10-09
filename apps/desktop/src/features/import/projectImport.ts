import { t } from "../../i18n";
import { loadModel } from "../../lib/files";
import { disposeModel } from "../../lib/model";
import { defaults, type Project, type ProjectFile } from "../../types";
/** Prepare a validated file import without mutating or persisting the supplied project. */
export async function prepareProjectImport(project: Project, files: ProjectFile[]) {
 const merged=[...project.files];
 for(const file of files) {
  if(merged.some(existing=>existing.name===file.name))throw new Error(t("A file named {{value0}} already exists. Rename it before importing.",{value0:file.name}));
  merged.push(file);
 }
 const mesh=files.find(file=>/\.(stl|obj|glb|3mf)$/i.test(file.name));
 const now=new Date().toISOString();
 const next:Project={...project,files:merged,updatedAt:now};
 if(mesh) {
  const object=await loadModel(mesh);disposeModel(object);
  const revision={id:crypto.randomUUID(),parent:project.currentRevision,createdAt:now,parameters:{...defaults,kind:"blank" as const},prompt:t("Imported {{value0}}",{value0:mesh.name}),source:mesh.name};
  next.revisions=[...project.revisions,revision];next.currentRevision=revision.id;
 }
 return {project:next,hasMesh:!!mesh,step:files.find(file=>/\.st(e)?p$/i.test(file.name))};
}
