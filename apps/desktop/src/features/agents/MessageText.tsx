import { t, errorText } from "../../i18n";
export function MessageText({ text, error }: { text: string; error: boolean }) {
  const quota = error && /usage limit|rate limit|quota/i.test(text);
  const retry = text.match(/try again at ([^.\n]+)/i)?.[1];
  if (quota)
    return (
      <div className="quota-message">
        <strong>{t("Лимит AI исчерпан")}</strong>
        <p>
          {retry
            ? t("Codex предлагает повторить запрос в {{value0}}.", {
                value0: retry,
              })
            : t("Провайдер временно не принимает новые запросы.")}
        </p>
        <p>{t("Модель и история сохранены.")}</p>
        <a
          href="https://chatgpt.com/codex/settings/usage"
          target="_blank"
          rel="noreferrer"
        >
          {t("Лимиты и кредиты ↗")}
        </a>
        <details>
          <summary>{t("Сообщение CLI")}</summary>
          <p>{text}</p>
        </details>
      </div>
    );
  if (error && text.trimStart().startsWith("{"))
    return (
      <div>
        <p>{errorText(text)}</p>
        <details>
          <summary>{t("Technical details")}</summary>
          <pre>{text}</pre>
        </details>
      </div>
    );
  return <p>{error ? errorText(text) : text}</p>;
}
