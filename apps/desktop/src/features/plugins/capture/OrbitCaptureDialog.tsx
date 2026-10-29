import { useEffect, useRef, useState } from "react";
import type * as THREE from "three";
import { Button, Modal } from "../../../components/ui";
import { errorText, number, t } from "../../../i18n";
import { useTheme } from "../../../lib/theme";
import { finishCapture, pickCaptureDestination, writeCaptureChunk, type CaptureTicket } from "./captureApi";
import { frameCount } from "./orbitMath";
import { CaptureError, recordOrbit } from "./recordOrbit";

export interface CaptureRequest {
  pluginName: string;
  fileName: string;
  ticket: CaptureTicket;
}

/**
 * Asks for the destination, then records the turntable video a plugin
 * requested, showing progress. Closing the dialog cancels the recording and
 * removes the partial file.
 */
export function OrbitCaptureDialog({
  request,
  scene,
  onDone,
  notify,
  fail,
}: {
  request: CaptureRequest;
  scene: () => THREE.Object3D | null;
  onDone: () => void;
  notify: (message: string) => void;
  fail: (message: string) => void;
}) {
  const theme = useTheme() === "light" ? "light" : "dark";
  const themeRef = useRef<"light" | "dark">(theme);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const abort = useRef<AbortController | null>(null);
  const { ticket, pluginName, fileName } = request;
  const spec = ticket.spec;

  useEffect(() => {
    // StrictMode re-runs effects; the ticket is single-use, so start once.
    if (abort.current) return;
    const controller = new AbortController();
    abort.current = controller;
    let destination: string | null = null;
    const run = async () => {
      try {
        destination = await pickCaptureDestination(ticket.token, fileName);
        if (!destination) {
          notify(t("pluginCapture.cancelled"));
          return;
        }
        const source = scene();
        if (!source) throw new CaptureError("PLUGIN_CAPTURE_EMPTY");
        setProgress({ done: 0, total: frameCount(spec) });
        await recordOrbit({
          source,
          spec,
          theme: themeRef.current,
          signal: controller.signal,
          write: (data, position) => writeCaptureChunk(ticket.token, data, position),
          onProgress: (done, total) => setProgress({ done, total }),
        });
        const saved = await finishCapture(ticket.token, true);
        notify(t("pluginCapture.saved", { plugin: pluginName, path: saved ?? destination }));
      } catch (error) {
        if (destination) await finishCapture(ticket.token, false).catch(() => undefined);
        if (error instanceof CaptureError && error.code === "PLUGIN_CAPTURE_CANCELLED") notify(t("pluginCapture.cancelled"));
        else fail(error instanceof CaptureError ? t(error.code) : errorText(error));
      } finally {
        onDone();
      }
    };
    void run();
    // One recording per ticket; callbacks are stable for its lifetime.
  }, [ticket.token]);

  if (!progress) return null;
  const percent = progress.total ? (progress.done / progress.total) * 100 : 0;
  return (
    <Modal
      open
      onClose={() => abort.current?.abort()}
      title={t("pluginCapture.title")}
      description={t("pluginCapture.description", {
        plugin: pluginName,
        width: spec.width,
        height: spec.height,
        fps: spec.fps,
        seconds: number(spec.durationMs / 1000, 1),
      })}
    >
      <div className="update-progress">
        <div className="update-progress-heading">
          <span>{t("pluginCapture.frames", { done: progress.done, total: progress.total })}</span>
          <strong>{`${number(percent, 0)}%`}</strong>
        </div>
        <div
          className="update-progress-track"
          role="progressbar"
          aria-label={t("pluginCapture.title")}
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={Math.round(percent)}
          aria-valuetext={t("pluginCapture.frames", { done: progress.done, total: progress.total })}
        >
          <div className="update-progress-fill" style={{ width: `${percent}%` }} />
        </div>
      </div>
      <div className="modal-actions">
        <Button onClick={() => abort.current?.abort()}>{t("pluginCapture.stop")}</Button>
      </div>
    </Modal>
  );
}
