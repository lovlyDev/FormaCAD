import { Equal, Link2, Spline } from "lucide-react";
import { IconButton } from "../../../components/ui";
import { t } from "../../../i18n";
import type { useSmartSelection } from "./useSmartSelection";

export function SmartSelectionTools({ tools }: { tools: ReturnType<typeof useSmartSelection> }) {
  if (!tools.canSelect) return null;
  return (
    <div className="view-toolbar">
      {tools.canLink && <IconButton label={t("Select linked body")} onClick={tools.linked}><Link2 size={17} /></IconButton>}
      <IconButton label={t("Select similar topology")} onClick={tools.similar}><Equal size={17} /></IconButton>
      {tools.canChain && <IconButton label={t("Select tangent chain")} onClick={tools.chain}><Spline size={17} /></IconButton>}
      {tools.status && <span role="status" className="field-hint">{tools.status}</span>}
    </div>
  );
}
