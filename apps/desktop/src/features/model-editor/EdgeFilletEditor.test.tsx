import { afterEach,expect,it,vi } from "vitest";
import { cleanup,fireEvent,render,screen } from "@testing-library/react";
import { EdgeFilletEditor } from "./EdgeFilletEditor";
import { newProject,useWorkspace } from "../../stores/workspace";
import { createStarterSketch } from "../../lib/sketchDocument";
import { readTypedCadDocument } from "../../lib/typedCadDocument";
import { t } from "../../i18n";
afterEach(()=>{cleanup();localStorage.clear();useWorkspace.setState({project:null,selectedEdge:null});});
it("keeps the fillet panel visible without a selection",()=>{
 const source=createStarterSketch();render(<EdgeFilletEditor source={source} savedSource={source} disabled={false} onChange={()=>{}}/>);
 expect(screen.getByText(t("Edge fillet"))).toBeVisible();expect(screen.getByRole("button",{name:t("Add fillet to draft")})).toBeDisabled();
});
it("adds a selected edge to the draft and blocks applying it to changed source",()=>{
 const source=createStarterSketch(),project={...newProject("Part","blank","mm","codex"),currentRevision:"revision"};useWorkspace.setState({project,selectedEdge:{bodyId:"body_1",revisionId:"revision",edgeOrdinal:4,semanticKey:"box-edge:x:ymin:zmax"}});const change=vi.fn();const view=render(<EdgeFilletEditor source={source} savedSource={source} disabled={false} onChange={change}/>);
 fireEvent.click(screen.getByRole("button",{name:t("Add fillet to draft")}));expect(change).toHaveBeenCalledTimes(1);const draft=change.mock.calls[0][0];expect(JSON.parse(draft).features.at(-1).operation.type).toBe("filletEdge");view.rerender(<EdgeFilletEditor source={draft} savedSource={source} disabled={false} onChange={change}/>);expect(screen.getByRole("button",{name:t("Add fillet to draft")})).toBeDisabled();expect(screen.getByText(t("Build the current draft and select the edge again before adding a fillet."))).toBeVisible();
});
it("explains why measured circular edges do not enable an unverified fillet",()=>{
 const document=readTypedCadDocument(createStarterSketch())!;const pad=document.features.find(feature=>feature.operation.type==="extrude")!;const profile=document.features.find(feature=>feature.id===pad.operation.sketchId)!;profile.operation={type:"circle",radius:{kind:"literal",mm:10}};
 const source=JSON.stringify(document),project={...newProject("Part","blank","mm","codex"),currentRevision:"revision"};
 useWorkspace.setState({project,selectedEdge:{bodyId:document.bodies[0].id,revisionId:"revision",edgeOrdinal:1,topologyRef:{schemaVersion:1,kind:"edge",ownerFeatureId:pad.id,role:"cylinder-edge:top",occurrencePath:[]}}});
 render(<EdgeFilletEditor source={source} savedSource={source} disabled={false} onChange={()=>{}}/>);
 expect(screen.getByRole("button",{name:t("Add fillet to draft")})).toBeDisabled();expect(screen.getByText(t("Fillet currently supports mapped box edges. Circular-edge measurements remain available in model properties."))).toBeVisible();
});
