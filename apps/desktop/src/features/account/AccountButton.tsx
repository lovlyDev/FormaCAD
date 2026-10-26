import { useCallback, useEffect, useRef, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { AlertCircle, CheckCircle2, ExternalLink, Globe, HardDrive, LoaderCircle, LogOut, RefreshCw, ShieldCheck, UserRound, WifiOff } from "lucide-react";
import { Button, IconButton, Modal } from "../../components/ui";
import { errorText, t, useLocale } from "../../i18n";
import {
  accountAvailable,
  accountErrorCode,
  accountStatus,
  cancelSignIn,
  needsRefresh,
  refreshAccount,
  signIn,
  signOut,
  type AccountStatus,
  type DocumentState,
} from "./api";
import "./account.css";

/** Friendly sentence for host errors: account codes are translated without the technical prefix. */
function accountMessage(cause: unknown): string {
  const code = accountErrorCode(cause);
  if (code) {
    const text = t(code);
    if (text !== code) return text;
  }
  return errorText(cause);
}

const STATE_TONE: Record<DocumentState, "ok" | "warn" | "bad"> = {
  valid: "ok",
  stale: "warn",
  expired: "bad",
  clockRollback: "bad",
  invalid: "bad",
};

/**
 * Topbar entry for the optional FormaCAD account. Nothing here blocks the
 * CAD workspace: failures only show a message inside this dialog.
 */
export function AccountButton() {
  const locale = useLocale();
  const [open, setOpen] = useState(false);
  const [status, setStatus] = useState<AccountStatus | null>(null);
  const [busy, setBusy] = useState<"signin" | "refresh" | "signout" | null>(null);
  const [error, setError] = useState<{ text: string; offline: boolean } | null>(null);
  const autoChecked = useRef(false);

  const load = useCallback(async () => {
    try {
      setStatus(await accountStatus());
    } catch (cause) {
      setError({ text: accountMessage(cause), offline: false });
    }
  }, []);

  const refresh = useCallback(async (quiet = false) => {
    setBusy("refresh");
    if (!quiet) setError(null);
    try {
      setStatus(await refreshAccount());
    } catch (cause) {
      const code = accountErrorCode(cause);
      // Offline: keep the cached, bounded rights; only explain when asked.
      if (!quiet || code === "ACCOUNT_SESSION_ENDED") setError({ text: accountMessage(cause), offline: code === "ACCOUNT_OFFLINE" });
      await load();
    } finally {
      setBusy(null);
    }
  }, [load]);

  useEffect(() => {
    if (!accountAvailable()) return;
    void load();
  }, [load]);

  useEffect(() => {
    if (status && !autoChecked.current && needsRefresh(status)) {
      autoChecked.current = true;
      void refresh(true);
    }
  }, [status, refresh]);

  if (!accountAvailable()) return null;

  const start = async () => {
    setBusy("signin");
    setError(null);
    try {
      setStatus(await signIn());
    } catch (cause) {
      if (accountErrorCode(cause) !== "ACCOUNT_SIGN_IN_CANCELLED") setError({ text: accountMessage(cause), offline: accountErrorCode(cause) === "ACCOUNT_OFFLINE" });
    } finally {
      setBusy(null);
    }
  };

  const leave = async () => {
    setBusy("signout");
    setError(null);
    try {
      setStatus(await signOut());
    } catch (cause) {
      setError({ text: accountMessage(cause), offline: false });
    } finally {
      setBusy(null);
    }
  };

  const date = (seconds: number | null) =>
    seconds ? new Intl.DateTimeFormat(locale === "ru" ? "ru-RU" : "en-US", { dateStyle: "medium", timeStyle: "short" }).format(seconds * 1000) : "—";
  const profile = status?.signedIn ? status.profile : null;
  const label = profile ? profile.displayName || t("account.title") : t("account.signIn");
  const initial = (profile?.displayName || "?").slice(0, 1).toUpperCase();
  const tone = status?.documentState ? STATE_TONE[status.documentState] : "warn";
  const openPortal = () => void openUrl(`${status?.server ?? "https://cad.kartochk1n.ru"}/account`).catch((cause) => setError({ text: accountMessage(cause), offline: false }));

  return (
    <>
      <IconButton label={label} active={open} onClick={() => setOpen(true)}>
        {profile?.avatarUrl
          ? <img className="account-avatar" src={profile.avatarUrl} alt="" />
          : profile ? <span className="account-initial">{initial}</span> : <UserRound size={17} />}
      </IconButton>
      <Modal open={open} onClose={() => { if (busy === "signin") void cancelSignIn(); setOpen(false); }} title={t("account.title")} description={t("account.description")}>
        <div className="account-panel">
          {profile ? (
            <>
              <div className="account-profile">
                <div className="account-profile-avatar" aria-hidden>
                  {profile.avatarUrl ? <img src={profile.avatarUrl} alt="" /> : initial}
                </div>
                <div className="account-profile-text">
                  <strong>{profile.displayName || t("account.title")}</strong>
                  <span>{profile.email ?? t("account.noEmail")}</span>
                </div>
                <span className="account-plan">{status?.plan ? status.plan.toUpperCase() : t("account.freePlan")}</span>
              </div>
              <div className="account-facts">
                <div className="account-fact">
                  <span>{t("account.rights")}</span>
                  <strong className={`account-state ${tone}`}>
                    {tone === "ok" ? <ShieldCheck size={14} /> : <AlertCircle size={14} />}
                    {status?.documentState ? t(`account.state.${status.documentState}`) : t("account.state.none")}
                  </strong>
                </div>
                <div className="account-fact">
                  <span>{t("account.offlineUntil")}</span>
                  <strong>{date(status?.offlineUntil ?? null)}</strong>
                </div>
                <div className="account-fact">
                  <span>{t("account.lastSync")}</span>
                  <strong>{date(status?.syncedAt ?? null)}</strong>
                </div>
              </div>
            </>
          ) : busy === "signin" ? (
            <div className="account-waiting" role="status">
              <LoaderCircle size={22} className="spin" aria-hidden />
              <div>
                <strong>{t("account.waitingTitle")}</strong>
                <span>{t("account.waitingBrowser")}</span>
              </div>
            </div>
          ) : (
            <div className="account-intro">
              <div className="account-intro-icon" aria-hidden><UserRound size={22} /></div>
              <div className="account-intro-text">
                <strong>{t("account.introTitle")}</strong>
                <ul>
                  <li><Globe size={14} aria-hidden /> {t("account.introBrowser")}</li>
                  <li><CheckCircle2 size={14} aria-hidden /> {t("account.introNoPassword")}</li>
                  <li><HardDrive size={14} aria-hidden /> {t("account.introLocal")}</li>
                </ul>
              </div>
            </div>
          )}

          {error && (
            <div className={`account-notice ${error.offline ? "warn" : "bad"}`} role="alert">
              {error.offline ? <WifiOff size={16} aria-hidden /> : <AlertCircle size={16} aria-hidden />}
              <span>{error.text}</span>
            </div>
          )}

          <div className="account-actions">
            {!status?.signedIn ? (
              busy === "signin"
                ? <Button onClick={() => void cancelSignIn()}>{t("account.cancel")}</Button>
                : <Button className="primary account-main" disabled={!!busy} onClick={() => void start()}><Globe size={14} />{t("account.signInBrowser")}</Button>
            ) : (
              <>
                <Button disabled={!!busy} onClick={() => void refresh()}><RefreshCw size={14} className={busy === "refresh" ? "spin" : ""} />{t("account.recheck")}</Button>
                <Button disabled={!!busy} onClick={() => void leave()}><LogOut size={14} />{t("account.signOut")}</Button>
              </>
            )}
            <Button className="account-link" onClick={openPortal}><ExternalLink size={14} />{t("account.openPortal")}</Button>
          </div>
          <p className="account-local-note"><HardDrive size={12} aria-hidden /> {t("account.localNote")}</p>
        </div>
      </Modal>
    </>
  );
}
