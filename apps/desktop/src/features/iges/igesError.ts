import { rawError, t } from "../../i18n";

/** Technical diagnostics stay in the dialog's separate details disclosure. */
export function igesErrorMessage(error: unknown): string {
  const raw = rawError(error);
  let code = raw;
  try {
    const value: unknown = JSON.parse(raw);
    if (value && typeof value === "object" && "code" in value && typeof value.code === "string") code = value.code;
  } catch { /* Plain client-side IGES codes are also translated. */ }
  const translated = /^IGES_[A-Z_]+$/.test(code) ? t(code) : code;
  return translated !== code ? translated : t("IGES_IO_FAILED");
}
