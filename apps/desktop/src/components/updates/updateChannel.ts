import { invoke } from "@tauri-apps/api/core";
import { check, Update } from "@tauri-apps/plugin-updater";

export type UpdateChannel = "stable" | "beta";
export const UPDATE_CHANNEL_KEY = "forma.updates.channel";
export const BETA_CAPABILITY = "beta.access";

/** Beta needs the supporter right in the account document; without it the stable channel is used. */
export function effectiveChannel(preferred: UpdateChannel, capabilities: readonly string[]): UpdateChannel {
  return preferred === "beta" && capabilities.includes(BETA_CAPABILITY) ? "beta" : "stable";
}

type BetaMetadata = ConstructorParameters<typeof Update>[0];

/**
 * Stable: the signed GitHub manifest from the app config. Beta: the account server, asked by
 * the Rust side with the bearer token, which returns the same update handle as `check()`.
 */
export async function checkChannel(channel: UpdateChannel): Promise<Update | null> {
  if (channel === "stable") return check({ timeout: 15000 });
  const metadata = await invoke<BetaMetadata | null>("beta_update_check");
  return metadata ? new Update(metadata) : null;
}
