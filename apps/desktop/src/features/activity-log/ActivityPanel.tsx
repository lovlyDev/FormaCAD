import { t, getLocale, systemText, errorText } from "../../i18n";
import type { Message } from "../../types";
export function ActivityPanel({
  messages,
  errorsOnly = false,
}: {
  messages: Message[];
  errorsOnly?: boolean;
}) {
  const entries = messages.filter((message) =>
    errorsOnly
      ? message.role === "error"
      : message.role === "event" || message.role === "error",
  );
  return (
    <div className="workspace-content-list">
      {entries.map((message) => (
        <article key={message.id}>
          <time>{new Date(message.createdAt).toLocaleString(getLocale())}</time>
          <p>
            {message.role === "event"
              ? systemText(message.text)
              : errorText(message.text)}
          </p>
        </article>
      ))}
      {!entries.length && (
        <p className="field-hint">
          {errorsOnly ? t("No recorded errors") : t("No recorded activity")}
        </p>
      )}
      <p className="field-hint">
        {t("Project events only. Detailed host logs remain local.")}
      </p>
    </div>
  );
}
