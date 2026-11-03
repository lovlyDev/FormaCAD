import { beforeEach, expect, it, vi } from "vitest";

const invoke = vi.fn();
const check = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args) }));
vi.mock("@tauri-apps/plugin-updater", () => ({
  check: (...args: unknown[]) => check(...args),
  Update: class { constructor(readonly metadata: { version: string }) {} },
}));
const { checkChannel, effectiveChannel } = await import("./updateChannel");

beforeEach(() => { invoke.mockReset(); check.mockReset(); });

it("uses beta only with the supporter right", () => {
  expect(effectiveChannel("beta", ["account.basic", "beta.access"])).toBe("beta");
  expect(effectiveChannel("beta", ["account.basic"])).toBe("stable");
  expect(effectiveChannel("stable", ["beta.access"])).toBe("stable");
});

it("checks stable through the configured manifest and beta through the Rust command", async () => {
  check.mockResolvedValue(null);
  expect(await checkChannel("stable")).toBeNull();
  expect(check).toHaveBeenCalledWith({ timeout: 15000 });
  expect(invoke).not.toHaveBeenCalled();

  invoke.mockResolvedValue({ rid: 7, version: "1.7.0-beta.1", currentVersion: "1.6.1", rawJson: {} });
  const update = await checkChannel("beta") as unknown as { metadata: { version: string } };
  expect(invoke).toHaveBeenCalledWith("beta_update_check");
  expect(update.metadata.version).toBe("1.7.0-beta.1");

  invoke.mockResolvedValue(null);
  expect(await checkChannel("beta")).toBeNull();
});
