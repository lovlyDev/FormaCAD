import {it,expect} from "vitest";
import {startupProjectRoute} from "./startupRoute";
it("restores a desktop session only when startup has no explicit route",()=>{
 expect(startupProjectRoute("","#/project/a",true)).toBe("#/project/a");
 for(const hash of ["#/","#/project/b"])expect(startupProjectRoute(hash,"#/project/a",true)).toBeNull();
});
it("honors browser dashboard navigation and rejects invalid saved routes",()=>{
 expect(startupProjectRoute("","#/project/a",false)).toBeNull();
 for(const saved of [null,"#/","#/project/../a","https://other.example"])expect(startupProjectRoute("",saved,true)).toBeNull();
});
