import { useEffect, useRef, type RefObject } from "react";
import {
  ArrowUp,
  ArrowUpRight,
  Box,
  CircleCheck,
  Command,
  Copy,
  LoaderCircle,
  Paperclip,
  Plus,
  Sparkles,
  Square,
  X,
} from "lucide-react";
import { t, getLocale, systemText, errorText } from "../../i18n";
import { IconButton } from "../../components/ui";
import { MessageText } from "../agents/MessageText";
import type { Project, ProjectFile } from "../../types";
export interface AgentEvent {
  kind: string;
  text: string;
  createdAt: string;
}
export function AssistantPanel({
  project,
  messages,
  events,
  busy,
  blocked,
  stage,
  prompt,
  attachments,
  selectedLabel,
  inputRef,
  status,
  onPrompt,
  onSend,
  onAttach,
  onRemoveAttachment,
  onClearSelection,
  onFresh,
  onSettings,
  onCancel,
  onNotice,
  onError,
}: {
  project: Project;
  messages: Project["messages"];
  events: AgentEvent[];
  busy: boolean;
  blocked: boolean;
  stage: string;
  prompt: string;
  attachments: ProjectFile[];
  selectedLabel: string | null;
  inputRef: RefObject<HTMLTextAreaElement | null>;
  status: string;
  onPrompt: (text: string) => void;
  onSend: () => void;
  onAttach: () => void;
  onRemoveAttachment: (index: number) => void;
  onClearSelection: () => void;
  onFresh: () => void;
  onSettings: () => void;
  onCancel: () => void;
  onNotice: (text: string) => void;
  onError: (text: string) => void;
}) {
  const end = useRef<HTMLDivElement>(null);
  useEffect(() => {
    end.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages.length, events.length, busy]);
  return (
    <aside className="chat-panel">
      <div className="chat-header">
        <span>
          <Sparkles size={16} />
          {t("Assistant")}
        </span>
        <IconButton
          label={t("Start a fresh conversation")}
          disabled={blocked || !messages.length}
          onClick={onFresh}
        >
          <Plus size={16} />
        </IconButton>
      </div>
      <button className="agent-picker" onClick={onSettings}>
        <Command size={15} />
        {project.agent === "codex"
          ? "OpenAI Codex"
          : project.agent === "claude"
            ? "Claude Code"
            : t("Custom agent")}
        <span className="agent-local">{status}</span>
      </button>
      <div className="chat-scroll">
        {!messages.length && (
          <>
            <div className="assistant-welcome">
              <div className="welcome-icon">
                <Sparkles size={24} />
              </div>
              <h2>{t("What will we make?")}</h2>
              <p>
                {t("Describe a part, explore an idea,")}
                <br />
                {t("or refine what’s already here.")}
              </p>
            </div>
            <div className="suggestions">
              <span>{t("A FEW PLACES TO START")}</span>
              {[
                "Создай шестерёнку с 20 зубьями",
                "Change the mounting holes to 6 mm",
                "Создай фигурку кота высотой 60 мм",
              ].map((suggestion) => (
                <button
                  key={suggestion}
                  onClick={() => {
                    onPrompt(t(suggestion));
                    inputRef.current?.focus();
                  }}
                >
                  <Box size={14} />
                  {t(suggestion)}
                  <ArrowUpRight size={13} />
                </button>
              ))}
            </div>
          </>
        )}
        {messages.map((message) => (
          <div className={`message ${message.role}`} key={message.id}>
            {message.role === "event" ? (
              <CircleCheck size={15} />
            ) : (
              <div className="message-label">
                {message.role === "user"
                  ? t("You")
                  : project.agent === "codex"
                    ? "Codex"
                    : "Claude"}
                <time>
                  {new Date(message.createdAt).toLocaleTimeString(getLocale(), {
                    hour: "2-digit",
                    minute: "2-digit",
                  })}
                </time>
              </div>
            )}
            <MessageText
              text={
                message.role === "event"
                  ? systemText(message.text)
                  : message.text
              }
              error={message.role === "error"}
            />
            {message.role === "assistant" && (
              <button
                className="message-copy"
                aria-label={t("Copy assistant message")}
                onClick={() =>
                  void navigator.clipboard
                    .writeText(message.text)
                    .then(() => onNotice(t("Message copied")))
                    .catch((error) => onError(errorText(error)))
                }
              >
                <Copy size={12} />
              </button>
            )}
            {message.role === "error" &&
              !/usage limit|rate limit|quota/i.test(message.text) && (
                <button
                  onClick={() => {
                    const last = [...messages]
                      .reverse()
                      .find((item) => item.role === "user");
                    if (last) onPrompt(last.text);
                  }}
                >
                  {t("Edit and retry")}
                  <ArrowUpRight size={12} />
                </button>
              )}
          </div>
        ))}
        {busy && events.length > 0 && (
          <div className="live-feed" aria-live="polite">
            {events.map((event, index) => (
              <div
                className={`live-event ${event.kind}`}
                key={`${event.createdAt}:${index}`}
              >
                <span className="live-dot" />
                <p>
                  {event.kind === "message"
                    ? event.text
                    : systemText(event.text)}
                </p>
              </div>
            ))}
          </div>
        )}
        {busy && (
          <div className="task-timeline">
            <LoaderCircle size={16} className="spin" />
            <div>
              <strong>{systemText(stage)}</strong>
              <p>
                {systemText(
                  events.at(-1)?.text ?? t("Starting a local session…"),
                )}
              </p>
            </div>
            <button onClick={onCancel}>
              <Square size={12} />
              {t("Stop")}
            </button>
          </div>
        )}
        <div ref={end} />
      </div>
      <div className="composer-wrap">
        {selectedLabel && (
          <div className="selection-context">
            <Box size={12} />
            {selectedLabel}
            <button
              aria-label={t("Clear selection")}
              onClick={onClearSelection}
            >
              <X size={12} />
            </button>
          </div>
        )}
        {!!attachments.length && (
          <div className="attachment-chips">
            {attachments.map((file, index) => (
              <span key={`${file.name}:${index}`}>
                {file.kind === "image" && file.data && (
                  <img src={file.data} alt="" />
                )}
                {file.name}
                <button
                  aria-label={t("Remove {{value0}}", { value0: file.name })}
                  onClick={() => onRemoveAttachment(index)}
                >
                  <X size={11} />
                </button>
              </span>
            ))}
          </div>
        )}
        <div className="composer">
          <textarea
            ref={inputRef}
            aria-label={t("Message your CAD assistant")}
            placeholder={t("Describe your next change…")}
            value={prompt}
            onChange={(event) => onPrompt(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
                event.preventDefault();
                onSend();
              }
            }}
          />
          <div className="composer-actions">
            <IconButton
              label={t("Attach a file")}
              disabled={blocked}
              onClick={onAttach}
            >
              <Paperclip size={16} />
            </IconButton>
            <span>{t("CAD context included")}</span>
            <button
              className="send-button"
              aria-label={t("Send prompt")}
              disabled={!prompt.trim() || blocked}
              onClick={onSend}
            >
              {busy ? (
                <LoaderCircle size={16} className="spin" />
              ) : (
                <ArrowUp size={17} />
              )}
            </button>
          </div>
        </div>
        <div className="composer-hint">
          <span>{t("Ctrl ↵ to send")}</span>
        </div>
      </div>
    </aside>
  );
}
