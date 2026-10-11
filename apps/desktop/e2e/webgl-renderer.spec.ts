import { expect, test } from "@playwright/test";

test("software WebGL2 renders a real triangle through SwiftShader", async ({ page }) => {
  await page.goto("/");
  const report = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = canvas.height = 16;
    const gl = canvas.getContext("webgl2", { antialias: false });
    if (!gl) throw new Error("WebGL2 context unavailable");
    const debug = gl.getExtension("WEBGL_debug_renderer_info");
    if (!debug) throw new Error("Renderer information unavailable");
    const shader = (type: number, text: string) => {
      const value = gl.createShader(type);
      if (!value) throw new Error("Shader allocation failed");
      gl.shaderSource(value, text);
      gl.compileShader(value);
      if (!gl.getShaderParameter(value, gl.COMPILE_STATUS))
        throw new Error("Shader compile failed: " + gl.getShaderInfoLog(value));
      return value;
    };
    const vertex = shader(gl.VERTEX_SHADER, `#version 300 es
      void main() { vec2 p[3]=vec2[3](vec2(-1.,-1.),vec2(3.,-1.),vec2(-1.,3.));
        gl_Position=vec4(p[gl_VertexID],0.,1.); }`);
    const fragment = shader(gl.FRAGMENT_SHADER, `#version 300 es
      precision highp float; out vec4 color;
      void main() { color=vec4(.25,.5,.75,1.); }`);
    const program = gl.createProgram();
    const vertexArray = gl.createVertexArray();
    if (!program || !vertexArray) throw new Error("Program allocation failed");
    gl.attachShader(program, vertex); gl.attachShader(program, fragment);
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error("Program link failed");
    gl.useProgram(program); gl.bindVertexArray(vertexArray);
    gl.viewport(0, 0, 16, 16); gl.drawArrays(gl.TRIANGLES, 0, 3);
    const pixel = new Uint8Array(4);
    gl.readPixels(8, 8, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    const result = { renderer: String(gl.getParameter(debug.UNMASKED_RENDERER_WEBGL)),
      version: String(gl.getParameter(gl.VERSION)), pixel: [...pixel],
      error: gl.getError(), lost: gl.isContextLost() };
    gl.deleteVertexArray(vertexArray); gl.deleteProgram(program);
    gl.deleteShader(vertex); gl.deleteShader(fragment);
    return result;
  });
  expect(report.renderer).toMatch(/SwiftShader/i);
  expect(report.version).toMatch(/WebGL 2/i);
  expect(report.error).toBe(0); expect(report.lost).toBe(false);
  [64, 128, 191, 255].forEach((value, index) =>
    expect(Math.abs(report.pixel[index] - value)).toBeLessThanOrEqual(1));
});
