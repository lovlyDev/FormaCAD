import { expect,type Page,type Locator } from "@playwright/test";

/** Wait for this control's disclosure/modal motion, not unrelated scene animations. */
async function settleControl(control:Locator){
 await expect.poll(()=>control.evaluate(element=>{
  let node:Element|null=element;
  while(node){
   if(node.getAnimations().some(animation=>animation.playState==="running"||animation.pending))return false;
   node=node.parentElement;
  }
  return true;
 })).toBe(true);
}

export async function selectValue(page:Page,trigger:Locator,value:string){
 await settleControl(trigger);
 await trigger.scrollIntoViewIfNeeded();
 await trigger.click();
 const option=page.locator(`.select-menu[data-state="open"] .select-option[data-value="${value}"]`);
 await expect(option).toBeVisible();
 await settleControl(option);
 await option.scrollIntoViewIfNeeded();
 const selectedText=(await option.innerText()).trim();
 await option.click();
 await expect(trigger).toHaveAttribute("data-state","closed");
 await expect(trigger).toHaveText(selectedText);
}