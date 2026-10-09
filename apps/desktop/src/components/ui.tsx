import { t } from "../i18n";
import { usePersistentState } from "../lib/persistence";
import { useWorkspace } from "../stores/workspace";
import * as Tooltip from "@radix-ui/react-tooltip";
export const TooltipProvider = Tooltip.Provider;
import { Check, ChevronRight, Minus, Plus } from "lucide-react";
import {
  useId,
  type ReactNode,
  type ButtonHTMLAttributes,
  type ChangeEvent,
  type InputHTMLAttributes,
} from "react";
import * as CheckboxPrimitive from "@radix-ui/react-checkbox";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";
export function cn(...inputs: Parameters<typeof clsx>) {
  return twMerge(clsx(inputs));
}
export function Button({
  className,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button className={cn("button", className)} {...props} />;
}
export function IconButton({
  label,
  children,
  active,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  active?: boolean;
}) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          aria-label={label}
          className={cn("icon-button", active && "active")}
          {...props}
        >
          {children}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content className="tooltip" sideOffset={7}>
          {label}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  );
}
export { Modal } from "./ui/Modal";

export { Select } from "./ui/Select";
export function Checkbox({
  checked,
  onChange,
  children,
  disabled=false,
}: {
  disabled?: boolean;
  checked: boolean;
  onChange: (checked: boolean) => void;
  children: ReactNode;
}) {
  const id=useId();
  return (
    <label className="checkbox-row" htmlFor={id}>
      <CheckboxPrimitive.Root
        className="custom-checkbox"
        id={id}
        disabled={disabled}
        checked={checked}
        onCheckedChange={(value) => onChange(value === true)}
      >
        <CheckboxPrimitive.Indicator>
          <Check size={13} />
        </CheckboxPrimitive.Indicator>
      </CheckboxPrimitive.Root>
      <span>{children}</span>
    </label>
  );
}
export function NumberInput(props: InputHTMLAttributes<HTMLInputElement>) {
  const step = Number(props.step) || 1;
  const adjust = (direction: number) => {
    const value = Math.min(
      Number(props.max ?? Infinity),
      Math.max(
        Number(props.min ?? -Infinity),
        Number(props.value || 0) + direction * step,
      ),
    );
    props.onChange?.({
      target: { value: String(value), valueAsNumber: Number(value.toFixed(6)) },
    } as ChangeEvent<HTMLInputElement>);
  };
  return (
    <div className="custom-number">
      <input {...props} />
      <div>
        <button
          type="button"
          disabled={props.disabled}
          aria-label={t("Decrease {{value0}}", {
            value0: props["aria-label"] ?? "",
          })}
          onClick={() => adjust(-1)}
        >
          <Minus size={12} />
        </button>
        <button
          type="button"
          disabled={props.disabled}
          aria-label={t("Increase {{value0}}", {
            value0: props["aria-label"] ?? "",
          })}
          onClick={() => adjust(1)}
        >
          <Plus size={12} />
        </button>
      </div>
    </div>
  );
}
export function TreeFolder({
  folderId,
  title,
  count,
  children,
  initial = true,
}: {
  folderId: string;
  title: string;
  count?: number;
  children: ReactNode;
  initial?: boolean;
}) {
  const projectId = useWorkspace(s => s.project?.id ?? "home");
  const [open, setOpen] = usePersistentState(`forma.ui.project.${projectId}.folder.${folderId}`, initial);
  const id = useId();
  return (
    <div className="tree-folder">
      <button
        className="tree-row"
        aria-expanded={open}
        aria-controls={id}
        onClick={() => setOpen(!open)}
      >
        <ChevronRight
          size={12}
          className={open ? "chevron expanded" : "chevron"}
        />
        <span>{title}</span>
        {count !== undefined && <small>{count}</small>}
      </button>
      <div
        id={id}
        className={`tree-collapse ${open ? "expanded" : ""}`}
        inert={!open}
        aria-hidden={!open}
      >
        <div>{children}</div>
      </div>
    </div>
  );
}
