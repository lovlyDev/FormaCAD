import { Children,isValidElement,useId,type ReactNode,type SelectHTMLAttributes,type ChangeEvent } from "react";
import * as SelectPrimitive from "@radix-ui/react-select";
import { Check,ChevronDown } from "lucide-react";
export function Select({
  children,
  leadingIcon,
  value,
  defaultValue,
  onChange,
  disabled,
  id,
  ...props
}: SelectHTMLAttributes<HTMLSelectElement>&{leadingIcon?:ReactNode}) {
  const autoId = useId();
  const empty=`empty:${autoId}`;
  return (
    <SelectPrimitive.Root
      value={value === undefined ? undefined : String(value)===""?empty:String(value)}
      defaultValue={
        defaultValue === undefined ? undefined : String(defaultValue)===""?empty:String(defaultValue)
      }
      disabled={disabled}
      onValueChange={(value) =>
        onChange?.({
          target: { value:value===empty?"":value },
          currentTarget: { value:value===empty?"":value },
        } as ChangeEvent<HTMLSelectElement>)
      }
    >
      <SelectPrimitive.Trigger
        id={id ?? autoId}
        className={["custom-select",props.className].filter(Boolean).join(" ")}
        aria-label={props["aria-label"]}
        aria-labelledby={props["aria-labelledby"]}
        title={props.title}
      >
        {leadingIcon}
        <SelectPrimitive.Value />
        <SelectPrimitive.Icon>
          <ChevronDown size={14} />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content
          className="select-menu"
          position="popper"
          align="start"
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
                  data-value={String(child.props.value)}
                  key={child.props.value}
                  value={String(child.props.value)===""?empty:String(child.props.value)}
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
