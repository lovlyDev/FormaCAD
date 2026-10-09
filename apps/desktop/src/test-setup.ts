import "@testing-library/jest-dom/vitest";
import { beforeEach } from "vitest";
import { setLocale } from "./i18n";
beforeEach(() => setLocale("en"));

Object.defineProperty(HTMLElement.prototype,"scrollIntoView",{configurable:true,value:()=>{}});
