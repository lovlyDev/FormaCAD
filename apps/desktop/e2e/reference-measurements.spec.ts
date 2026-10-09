import { expect,test,type Page } from "@playwright/test";
import { referenceMeasurementFixture } from "./referenceMeasurementFixture";
const fixture=referenceMeasurementFixture();
const wording={ru:{measure:"Измерить выделение",edge:"Длина выбранного ребра",face:"Площадь выбранной грани",plane:"Выбранная плоская грань",stop:"Прекратить ожидание"},en:{measure:"Measure selection",edge:"Selected edge length",face:"Selected face area",plane:"Selected planar face",stop:"Stop waiting"}};
async function mount(page:Page,locale:"ru"|"en"){
  await page.route(/\/src\/main\.tsx(?:\?.*)?$/,route=>route.fulfill({contentType:"application/javascript",body:"export {};"}));
  await page.addInitScript(locale=>localStorage.setItem("forma.locale",locale),locale);
  await page.goto("/");
  await page.evaluate(async fixture=>{const {mountReferenceMeasurementHarness}=await import("/e2e/referenceMeasurementHarness.tsx");await mountReferenceMeasurementHarness(fixture);},fixture!);
  await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");
  const panel=page.locator(".reference-measurement").first();await expect(panel).toBeVisible();
  await panel.locator("summary").first().click();await expect(panel.getByRole("button",{name:wording[locale].measure,exact:true})).toBeVisible();return panel;
}
async function kind(page:Page,label:string){const panel=page.locator(".reference-measurement").first();await panel.getByRole("combobox").click();await page.getByRole("option",{name:label,exact:true}).click();}
for(const locale of ["ru","en"] as const){
  test(`actual App ${locale}: native measurements, units, repeated motion and both themes`,async({page},info)=>{
    test.skip(!fixture,"Requires the fresh local measurement worker");const errors:string[]=[];page.on("pageerror",error=>errors.push(error.message));
    const panel=await mount(page,locale),measure=panel.getByRole("button",{name:wording[locale].measure,exact:true});
    await measure.click();await expect(panel.locator("dd").first()).toHaveText(locale==="ru"?"8 000 мм³":"8,000 mm³");
    await expect.poll(()=>page.locator(".panel-scroll").evaluate(el=>el.getBoundingClientRect().height)).toBeGreaterThan(100);
    const properties=page.locator(".selection-model-properties");expect(await properties.evaluate(el=>el.scrollHeight>el.clientHeight)).toBe(true);
    await panel.locator(".reference-measurement-provenance").scrollIntoViewIfNeeded();await expect(panel.locator(".reference-measurement-provenance")).toBeInViewport();
    for(const units of ["cm","inch","mm"]){await page.evaluate(units=>(window as any).__applyHarness.units(units),units);const divisor=units==="cm"?10:units==="inch"?25.4:1,symbol=units==="cm"?(locale==="ru"?"см":"cm"):units==="inch"?(locale==="ru"?"дюйм":"in"):locale==="ru"?"мм":"mm";const expected=new Intl.NumberFormat(locale==="ru"?"ru-RU":"en-US",{maximumFractionDigits:3}).format(8000/divisor**3);await expect(panel.locator("dd").first()).toHaveText(`${expected} ${symbol}³`);await expect(panel.locator("dd").nth(1)).toHaveText(`${new Intl.NumberFormat(locale==="ru"?"ru-RU":"en-US",{maximumFractionDigits:3}).format(2800/divisor**2)} ${symbol}²`);}
    for(const theme of ["dark","light"]){await page.evaluate(theme=>(window as any).__applyHarness.theme(theme),theme);await page.screenshot({path:info.outputPath(`measurement-${locale}-${theme}.png`)});expect(await measure.evaluate(el=>el.getBoundingClientRect().height)).toBe(32);}
    for(let repeat=0;repeat<2;repeat++){const count=Number(await panel.locator(".editor-disclosure-body").first().getAttribute("data-animation-count"));await panel.locator("summary").first().click();await expect(panel).toHaveAttribute("data-expanded","false");await panel.locator("summary").first().click();await expect(panel).toHaveAttribute("data-expanded","true");await expect.poll(async()=>Number(await panel.locator(".editor-disclosure-body").first().getAttribute("data-animation-count"))).toBeGreaterThan(count);}
    await page.evaluate(()=>(window as any).__applyHarness.select("edge"));await kind(page,wording[locale].edge);await measure.click();await expect(panel.locator("dd").first()).toHaveText(locale==="ru"?"40 мм":"40 mm");
    await expect(panel).toContainText(locale==="ru"?"прямое ребро":"straight");
    await page.evaluate(()=>(window as any).__applyHarness.select("face"));await kind(page,wording[locale].face);await measure.click();await expect(panel.locator("dd").first()).toHaveText(locale==="ru"?"800 мм²":"800 mm²");
    await kind(page,wording[locale].plane);await measure.click();await expect(panel.locator("dd").nth(2)).toHaveText("0 · 0 · 1");
    expect(await page.evaluate(()=>(window as any).__applyHarness.calls.filter((call:any)=>call.command==="apply_program"||call.command==="request_permission"))).toEqual([]);expect(errors).toEqual([]);
  });
  test(`actual App ${locale}: late selection and project responses cannot become current measurements`,async({page})=>{
    test.skip(!fixture,"Requires the fresh local measurement worker");const errors:string[]=[];page.on("pageerror",error=>errors.push(error.message));
    const panel=await mount(page,locale),measure=panel.getByRole("button",{name:wording[locale].measure,exact:true});
    await page.evaluate(()=>(window as any).__applyHarness.setMeasurementDelayed(true));await measure.click();await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.measurementPending())).toBe(true);
    await panel.getByRole("button",{name:wording[locale].stop,exact:true}).click();await expect(measure).toBeDisabled();await expect(panel.locator("dl")).toHaveCount(0);
    await page.evaluate(()=>(window as any).__applyHarness.select("edge"));await expect(measure).toBeDisabled();await page.evaluate(()=>(window as any).__applyHarness.releaseMeasurement());await expect(measure).toBeEnabled();await expect(panel.locator("dl")).toHaveCount(0);
    await page.evaluate(()=>(window as any).__applyHarness.setMeasurementDelayed(false));await kind(page,wording[locale].edge);await measure.click();await expect(panel.locator("dd").first()).toHaveText(locale==="ru"?"40 мм":"40 mm");
    await page.evaluate(()=>(window as any).__applyHarness.setMeasurementFailure(true));await measure.click();await expect(panel.getByRole("alert")).toBeVisible();await expect(panel.locator("dd").first()).toHaveText(locale==="ru"?"40 мм":"40 mm");await expect(panel).toContainText(locale==="ru"?"Прежний результат":"Previous result");
    await page.evaluate(()=>{const h=(window as any).__applyHarness;h.setMeasurementFailure(false);h.setMeasurementDelayed(true);});await measure.click();await expect.poll(()=>page.evaluate(()=>(window as any).__applyHarness.measurementPending())).toBe(true);
    await page.evaluate(()=>(window as any).__applyHarness.switchProject());await expect(page.locator(".viewer-area")).toHaveAttribute("data-scene-phase","ready");await page.evaluate(()=>(window as any).__applyHarness.select("body"));await page.evaluate(()=>(window as any).__applyHarness.releaseMeasurement());await expect(page.locator(".reference-measurement dl")).toHaveCount(0);expect(errors).toEqual([]);
  });
}
