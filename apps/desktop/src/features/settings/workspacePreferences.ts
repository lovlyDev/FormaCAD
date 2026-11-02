import { usePersistentState } from "../../lib/persistence";
export function useNavigationPreferences() {
  const [orbitSelection, setOrbitSelection] = usePersistentState(
    "forma.ui.navigation.orbitSelection",
    false,
  );
  const [zoomCursor, setZoomCursor] = usePersistentState(
    "forma.ui.navigation.zoomCursor",
    false,
  );
  const [zoomSpeed, setZoomSpeed] = usePersistentState(
    "forma.ui.navigation.zoomSpeed",
    1,
  );
  const [panSpeed, setPanSpeed] = usePersistentState(
    "forma.ui.navigation.panSpeed",
    0.3,
  );
  const [emulateNumpad, setEmulateNumpad] = usePersistentState(
    "forma.ui.input.emulateNumpad",
    false,
  );
  return {
    orbitSelection,
    setOrbitSelection,
    zoomCursor,
    setZoomCursor,
    zoomSpeed:
      typeof zoomSpeed === "number" && Number.isFinite(zoomSpeed)
        ? Math.max(0.2, Math.min(3, zoomSpeed))
        : 1,
    setZoomSpeed,
    panSpeed:
      typeof panSpeed === "number" && Number.isFinite(panSpeed)
        ? Math.max(0.1, Math.min(2, panSpeed))
        : 0.3,
    setPanSpeed,
    emulateNumpad,
    setEmulateNumpad,
  };
}
