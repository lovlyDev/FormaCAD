import { expect, test, vi } from "vitest";
import { PerspectiveCamera, Scene, Vector2, type WebGLRenderer } from "three";
import { viewportPng } from "./viewportPng";

test("rejects a capture beyond the dimension limit before changing the renderer", () => {
  const render = vi.fn();
  const renderer = {
    getSize: (size: Vector2) => size.set(1600, 900),
    getDrawingBufferSize: (size: Vector2) => size.set(1600, 900),
    render,
  } as unknown as WebGLRenderer;
  const scene = new Scene();
  expect(() => viewportPng(renderer, scene, new PerspectiveCamera(), { scale: 3, transparent: true }))
    .toThrow("The snapshot exceeds 4096 pixels on one side.");
  expect(render).not.toHaveBeenCalled();
});
