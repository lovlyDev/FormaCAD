import { cleanup, fireEvent, render } from "@testing-library/react";
import * as Tooltip from "@radix-ui/react-tooltip";
import { afterEach, expect, it, vi } from "vitest";
import type { Project } from "../../types";
import { ExportProjectBundleControl } from "./ExportProjectBundleControl";
afterEach(cleanup);
it("shows precisely what redaction retains and forwards the chosen mode after review", () => {
  const project = { id: "project", name: "Part" } as Project; const onExport = vi.fn();
  const view = render(<Tooltip.Provider><ExportProjectBundleControl project={project} onExport={onExport} /></Tooltip.Provider>);
  fireEvent.click(view.getByRole("button", { name: "Export project bundle" }));
  expect(onExport).not.toHaveBeenCalled(); fireEvent.click(view.getByRole("checkbox"));
  expect(view.getByRole("dialog")).toHaveTextContent("Model geometry, source code, names, parameters and required files remain included");
  fireEvent.click(view.getByRole("button", { name: "Export" })); expect(onExport).toHaveBeenCalledWith(project, true);
});
