import { afterEach,expect,it,vi } from "vitest";
import { cleanup,fireEvent,render,screen } from "@testing-library/react";
import { createStarterSketch,readSketchOperation } from "../../lib/sketchDocument";
import { resolveSketchBindings } from "../../lib/sketchBindings";
import { SketchBindingsEditor } from "./SketchBindingsEditor";
afterEach(cleanup);
it("adding an origin link keeps coordinates and editing its factor changes the shared coordinate",()=>{
 const op=readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!,parameters=[{id:"width",name:"Width",valueMm:100}],change=vi.fn();
 const {rerender}=render(<SketchBindingsEditor operation={op} parameters={parameters} disabled={false} onChange={change}/>);
 fireEvent.click(screen.getByRole("button",{name:"Link coordinate"}));const next=change.mock.calls[0][0];expect(next.bindings[0].value).toEqual({kind:"parameter",parameterId:"width",scale:1,offsetMm:-100});expect(resolveSketchBindings(next,parameters).originMm).toEqual([0,0,0]);
 rerender(<SketchBindingsEditor operation={next} parameters={parameters} disabled={false} onChange={change}/>);expect(screen.getByRole("button",{name:"Link coordinate"})).toBeDisabled();
 fireEvent.change(screen.getByLabelText("Factor"),{target:{value:"2"}});expect(resolveSketchBindings(change.mock.calls.at(-1)![0],parameters).originMm[0]).toBe(100);
});
it("no parameter prevents adding an unresolved binding",()=>{
 const op=readSketchOperation(JSON.parse(createStarterSketch()).features[0].operation)!;render(<SketchBindingsEditor operation={op} parameters={[]} disabled={false} onChange={vi.fn()}/>);expect(screen.getByRole("button",{name:"Link coordinate"})).toBeDisabled();
});
