/** Explicit web URLs and dashboard links take precedence over session preferences. */
export function startupProjectRoute(hash:string,saved:string|null,native:boolean):string|null {
 return native&&!hash&&saved&&/^#\/project\/[a-zA-Z0-9-]+$/.test(saved)?saved:null;
}
