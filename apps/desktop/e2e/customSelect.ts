import { expect,type Page,type Locator } from "@playwright/test";
export async function selectValue(page:Page,trigger:Locator,value:string){await trigger.click();const option=page.locator(`.select-option[data-value="${value}"]`);await expect(option).toBeVisible();await option.click();}
