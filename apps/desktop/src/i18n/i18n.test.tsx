import { fireEvent, render, screen } from "@testing-library/react";
import { expect, it } from "vitest";
import {
  errorText,
  fixedNumber,
  getLocale,
  quantity,
  setLocale,
  systemText,
  t,
  useLocale,
} from ".";
import { useState } from "react";
import en from "./en.json";
import ru from "./ru.json";

it("has matching complete catalogs and substitutions", () => {
  expect(Object.keys(ru).sort()).toEqual(Object.keys(en).sort());
  setLocale("ru");
  expect(t("Remove {{value0}}", { value0: "model.step" })).toBe(
    "Удалить model.step",
  );
  expect(t("<user-supplied-name>")).toBe("<user-supplied-name>");
});
it("switches without remounting or losing a draft, and persists language", () => {
  function Editor() {
    useLocale();
    const [draft, setDraft] = useState("user text");
    return (
      <>
        <input
          aria-label="draft"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <button onClick={() => setLocale("ru")}>{t("Settings")}</button>
      </>
    );
  }
  render(<Editor />);
  fireEvent.change(screen.getByLabelText("draft"), {
    target: { value: "Keep my part" },
  });
  fireEvent.click(screen.getByRole("button", { name: "Settings" }));
  expect(screen.getByRole("button", { name: "Настройки" })).toBeVisible();
  expect(screen.getByLabelText("draft")).toHaveValue("Keep my part");
  expect(getLocale()).toBe("ru");
  expect(localStorage.getItem("forma.locale")).toBe("ru");
  expect(document.documentElement.lang).toBe("ru");
});
it("localizes historical events in either language and keeps feature diagnostics", () => {
  setLocale("ru");
  expect(systemText("Revision 3 saved")).toBe("Ревизия 3 сохранена");
  const error = JSON.stringify({
    code: "GEOMETRY_BUILD_FAILED",
    featureId: "Pocket001",
    message: "CAD: Feature must produce valid, nonempty solids",
  });
  expect(errorText(error)).toContain("Pocket001 · GEOMETRY_BUILD_FAILED");
  expect(errorText(error)).toContain("Операция должна создавать");
  setLocale("en");
  expect(systemText("Ревизия 3 сохранена")).toBe("Revision 3 saved");
});
it("formats Russian numbers and plural forms", () => {
  setLocale("ru");
  expect(fixedNumber(1.5)).toBe("1,50");
  expect(quantity("revisions", 1)).toBe("1 ревизия");
  expect(quantity("revisions", 2)).toBe("2 ревизии");
  expect(quantity("revisions", 5)).toBe("5 ревизий");
  expect(quantity("bodies", 21)).toBe("21 тело");
});
