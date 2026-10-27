import {
  CanvasSource,
  getFirstEncodableVideoCodec,
  Mp4OutputFormat,
  Output,
  StreamTarget,
  WebMOutputFormat,
  type StreamTargetChunk,
  type VideoCodec,
} from "mediabunny";
import type * as THREE from "three";
import { bitrate, frameCount, type OrbitSpec } from "./orbitMath";
import { createOrbitStage } from "./orbitScene";

/** Writes encoded bytes at a byte offset of the destination file. */
export type ChunkWriter = (data: Uint8Array, position: number) => Promise<void>;

export class CaptureError extends Error {
  constructor(readonly code: string) {
    super(code);
  }
}

const CODECS: Record<OrbitSpec["format"], VideoCodec[]> = { mp4: ["avc", "hevc"], webm: ["vp9", "vp8", "av1"] };

/**
 * Render every turntable frame off-screen and encode it with WebCodecs.
 * Frames are timestamped exactly (not real time), so slow machines produce
 * the same smooth video, only later.
 */
export async function recordOrbit({
  source,
  spec,
  theme,
  write,
  onProgress,
  signal,
}: {
  source: THREE.Object3D;
  spec: OrbitSpec;
  theme: "light" | "dark";
  write: ChunkWriter;
  onProgress: (done: number, total: number) => void;
  signal: AbortSignal;
}): Promise<void> {
  if (typeof VideoEncoder === "undefined") throw new CaptureError("PLUGIN_CAPTURE_UNSUPPORTED");
  const bitsPerSecond = bitrate(spec);
  const codec = await getFirstEncodableVideoCodec(CODECS[spec.format], {
    width: spec.width,
    height: spec.height,
    bitrate: bitsPerSecond,
  });
  if (!codec) throw new CaptureError("PLUGIN_CAPTURE_CODEC");
  const stage = createOrbitStage(source, spec, theme);
  const writable = new WritableStream<StreamTargetChunk>({
    write: (chunk) => write(chunk.data, chunk.position),
  });
  const output = new Output({
    format: spec.format === "mp4" ? new Mp4OutputFormat({ fastStart: false }) : new WebMOutputFormat(),
    target: new StreamTarget(writable, { chunked: true }),
  });
  try {
    const video = new CanvasSource(stage.canvas, {
      codec,
      bitrate: bitsPerSecond,
      keyFrameInterval: 2,
      latencyMode: "quality",
    });
    output.addVideoTrack(video, { frameRate: spec.fps });
    await output.start();
    const total = frameCount(spec);
    for (let index = 0; index < total; index++) {
      if (signal.aborted) throw new CaptureError("PLUGIN_CAPTURE_CANCELLED");
      stage.render(index);
      await video.add(index / spec.fps, 1 / spec.fps);
      onProgress(index + 1, total);
    }
    await output.finalize();
  } catch (error) {
    if (output.state === "started" || output.state === "pending") await output.cancel().catch(() => undefined);
    throw error;
  } finally {
    stage.dispose();
  }
}
