import { invoke } from "@tauri-apps/api/core";
import { getLocale } from "../../../i18n";
import type { OrbitSpec } from "./orbitMath";

/** One-time ticket for a recording that a plugin requested. */
export interface CaptureTicket {
  token: string;
  spec: OrbitSpec;
}

/** Native save picker; `null` when the user cancels (the ticket is dropped). */
export const pickCaptureDestination = (token: string, fileName: string) =>
  invoke<string | null>("plugins_capture_pick", { token, fileName, locale: getLocale() });

export const writeCaptureChunk = (token: string, data: Uint8Array, position: number) =>
  invoke<void>("plugins_capture_write", data, {
    headers: { "x-capture-token": token, "x-capture-offset": String(position) },
  });

/** Keep the finished video or delete the partial file. */
export const finishCapture = (token: string, keep: boolean) =>
  invoke<string | null>("plugins_capture_finish", { token, keep });
