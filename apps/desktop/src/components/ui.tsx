import { t } from "../i18n";
import { usePersistentState } from "../lib/persistence";
import { useWorkspace } from "../stores/workspace";
import * as Dialog from "@radix-ui/react-dialog";
import * as Tooltip from "@radix-ui/react-tooltip";
import { X, Check, ChevronDown, ChevronRight, Minus, Plus } from "lucide-react";
import {
  Children,
  isValidElement,
  useId,
  type ReactNode,
  type ButtonHTMLAttributes,
  type SelectHTMLAttributes,
  type ChangeEvent,
  type InputHTMLAttributes,
} from "react";
import * as SelectPrimitive from "@radix-ui/react-select";
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
export function Modal({
  open,
  onClose,
  title,
  description,
  children,
  wide = false,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description: string;
  children: ReactNode;
  wide?: boolean;
}) {
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(v) => {
        if (!v) onClose();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="modal-overlay" />
        <Dialog.Content className={cn("modal", wide && "wide")}>
          <Dialog.Title>{title}</Dialog.Title>
          <Dialog.Description>{description}</Dialog.Description>
          <Dialog.Close className="modal-close" aria-label={t("Close dialog")}>
            <X size={18} />
          </Dialog.Close>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export function Select({
  children,
  value,
  defaultValue,
  onChange,
  disabled,
  id,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>) {
  const autoId = useId();
  return (
    <SelectPrimitive.Root
      value={value === undefined ? undefined : String(value)}
      defaultValue={
        defaultValue === undefined ? undefined : String(defaultValue)
      }
      disabled={disabled}
      onValueChange={(value) =>
        onChange?.({
          target: { value },
          currentTarget: { value },
        } as ChangeEvent<HTMLSelectElement>)
      }
    >
      <SelectPrimitive.Trigger
        id={id ?? autoId}
        className="custom-select"
        aria-label={props["aria-label"]}
      >
        <SelectPrimitive.Value />
        <SelectPrimitive.Icon>
          <ChevronDown size={14} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          className="select-menu"
          position="popper"
          sideOffset={5}
          collisionPadding={10}
        >
          <SelectPrimitive.Viewport>
            {Children.toArray(children).map((child) => {
              if (
                !isValidElement<{
                  value: string;
                  children: ReactNode;
                  disabled?: boolean;
                }>(child)
              )
                return null;
              return (
                <SelectPrimitive.Item
                  className="select-option"
                  key={child.props.value}
                  value={String(child.props.value)}
                  disabled={child.props.disabled}
                >
                  <SelectPrimitive.ItemText>
                    {child.props.children}
                  </SelectPrimitive.ItemText>
                  <SelectPrimitive.ItemIndicator>
                    <Check size={14} />
                  </SelectPrimitive.ItemIndicator>
                </SelectPrimitive.Item>
              );
            })}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
}
export function Checkbox({
  checked,
  onChange,
  children,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  children: ReactNode;
}) {
  return (
    <label className="checkbox-row">
      <CheckboxPrimitive.Root
        className="custom-checkbox"
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
