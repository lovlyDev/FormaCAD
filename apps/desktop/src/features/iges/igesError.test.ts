import { expect, it } from "vitest";
import { setLocale, t } from "../../i18n";
import { igesErrorMessage } from "./igesError";

it("separates translated IGES failures from raw native diagnostics in both languages", () => {
  for (const locale of ["ru", "en"] as const) {
    setLocale(locale);
    expect(igesErrorMessage(JSON.stringify({ code: "IGES_PARTIAL_IMPORT", message: "native debug" }))).toBe(t("IGES_PARTIAL_IMPORT"));
    expect(igesErrorMessage(new Error("IGES_REPORT_INVALID"))).toBe(t("IGES_REPORT_INVALID"));
    expect(igesErrorMessage("unknown native details")).toBe(t("IGES_IO_FAILED"));
  }
  setLocale("ru");
});
