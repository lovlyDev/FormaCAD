import type { ReactNode } from "react";
import { ChevronDown, type LucideIcon } from "lucide-react";
import { AnimatedDetails } from "./AnimatedDetails";
import "./EditorDialog.css";

/** Shared source/review section: identical header, chevron and 260ms motion. */
export function EditorDisclosure({ icon: Icon, title, hint, hintClassName, open, onOpenChange, children, className = "model-editor-source" }: {
  icon: LucideIcon; title: string; hint?: string; hintClassName?: string; open: boolean; onOpenChange: (open: boolean) => void; children: ReactNode; className?: string;
}) {
  return <AnimatedDetails className={className} open={open} onOpenChange={onOpenChange}>
    <summary><Icon size={16} /><span>{title}</span>{hint && <small className={hintClassName}>{hint}</small>}<ChevronDown size={15} className="cad-section-chevron" /></summary>
    {children}
  </AnimatedDetails>;
}
