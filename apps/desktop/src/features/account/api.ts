import { invoke, isTauri } from "@tauri-apps/api/core";

/**
 * Optional FormaCAD account. Tokens stay in the Rust host (OS credential
 * store + memory); the interface only receives profile and rights.
 */
export interface AccountProfile {
  sub: string;
  displayName: string;
  avatarUrl: string | null;
  email: string | null;
  locale: string;
}

export type DocumentState = "valid" | "stale" | "expired" | "clockRollback" | "invalid";

export interface AccountStatus {
  server: string;
  signedIn: boolean;
  profile: AccountProfile | null;
  plan: string | null;
  subscriptionStatus: string | null;
  documentState: DocumentState | null;
  capabilities: string[];
  offlineUntil: number | null;
  refreshAfter: number | null;
  syncedAt: number | null;
}

export const accountAvailable = () => isTauri();
export const accountStatus = () => invoke<AccountStatus>("account_status");
export const signIn = () => invoke<AccountStatus>("account_sign_in");
export const cancelSignIn = () => invoke<void>("account_cancel_sign_in");
export const refreshAccount = () => invoke<AccountStatus>("account_refresh");
export const signOut = () => invoke<AccountStatus>("account_sign_out");

/** Stable error code from `{code, message}` errors returned by the host. */
export function accountErrorCode(error: unknown): string {
  try {
    const value = JSON.parse(String(error)) as { code?: unknown };
    return typeof value.code === "string" ? value.code : "";
  } catch {
    return "";
  }
}

export const needsRefresh = (s: AccountStatus) =>
  s.signedIn && (s.documentState !== "valid" || (s.refreshAfter !== null && s.refreshAfter * 1000 < Date.now()));
