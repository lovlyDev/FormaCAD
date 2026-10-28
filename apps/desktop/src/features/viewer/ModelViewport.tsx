import {
  Component,
  lazy,
  Suspense,
  type ErrorInfo,
  type ReactNode,
} from "react";
import type * as THREE from "three";
import { AlertCircle, LoaderCircle } from "lucide-react";
import { Button } from "../../components/ui";
import { t } from "../../i18n";
const Viewer = lazy(() => import("./Viewer"));
class ViewerBoundary extends Component<
  { children: ReactNode },
  { error: boolean }
> {
  state = { error: false };
  static getDerivedStateFromError() {
    return { error: true };
  }
  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("Viewer failed", error.message, info.componentStack);
  }
  render() {
    return this.state.error ? (
      <div className="empty-state">
        <AlertCircle />
        <h3>{t("3D view is unavailable")}</h3>
        <p>{t("Check that hardware acceleration and WebGL 2 are enabled.")}</p>
      </div>
    ) : (
      this.props.children
    );
  }
}
export function ModelViewport({
  projectId,
  object,
  before,
  interactive,
  targetRevision,
  displayedRevision,
  phase,
  loading,
  failed,
  failure,
  retainedLabel,
  onRetry,
  onScreenshot,
}: {
  projectId: string;
  object: THREE.Group;
  before: THREE.Group | null;
  interactive: boolean;
  targetRevision: string;
  displayedRevision: string;
  phase: string;
  loading: boolean;
  failed: boolean;
  failure: string;
  retainedLabel: string | null;
  onRetry: () => void;
  onScreenshot: (data: string) => void;
}) {
  return (
    <section className="center-panel">
      <div
        className={`viewer-area ${before ? "comparison" : ""}`}
        data-displayed-revision={displayedRevision}
        data-target-revision={targetRevision}
        data-scene-phase={phase}
      >
        <ViewerBoundary key={projectId}>
          <Suspense
            fallback={
              <div className="empty-state">
                <LoaderCircle className="spin" />
                {t("Loading 3D workspace…")}
              </div>
            }
          >
            {before && (
              <Viewer
                key="comparison"
                object={before}
                comparison
                onScreenshot={onScreenshot}
              />
            )}
            <Viewer
              key="current"
              object={object}
              interactive={interactive}
              onScreenshot={onScreenshot}
            />
          </Suspense>
        </ViewerBoundary>
        {loading && (
          <div className="model-loading">
            <LoaderCircle className="spin" />
            {t("Reading geometry…")}
            {retainedLabel && <small>{retainedLabel}</small>}
          </div>
        )}
        {failed && (
          <div className="model-loading" role="alert">
            <AlertCircle size={16} />
            <span>
              {t(
                "The committed preview could not be loaded. The previous valid model is shown when available.",
              )}
            </span>
            {retainedLabel && <small>{retainedLabel}</small>}
            <small>{failure}</small>
            <Button onClick={onRetry}>{t("Retry preview loading")}</Button>
          </div>
        )}
      </div>
    </section>
  );
}
