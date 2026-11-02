/** Actual App with an IPC boundary that emulates the host Command API and rebuilds every revision with the real worker. */
type Built = {
  files: Record<string, { data: string; size: number; sha256: string }>;
  boundsMm: number[];
  program: string;
};
declare global {
  interface Window {
    __m2Build: (document: unknown, revisionId: string) => Promise<Built>;
  }
}

export async function mountViewportTransformHarness(
  base: Built & { revisionId: string },
) {
  const callbacks = new Map<number, (...args: any[]) => void>();
  const listeners = new Map<number, { event: string; handler: number }>();
  let callback = 0,
    eventId = 0;
  const calls: { command: string; args: any }[] = [];
  const assets: Record<string, string> = {};
  const projectId = crypto.randomUUID();
  let project: any;
  // Transactions emulate the host's logical Undo slots: amend replaces the last one.
  const slots: {
    revision: string;
    parent: string;
    commands: any[];
    generation: number;
  }[] = [];
  let generation = 1;
  const addFiles = (built: Built) =>
    Object.entries(built.files).map(([name, file]) => {
      assets[name] = file.data;
      return { name, kind: "model", size: file.size, sha256: file.sha256 };
    });
  const revision = (
    id: string,
    parent: string | null,
    built: Built,
    prompt: string,
  ) => ({
    id,
    parent,
    createdAt: new Date().toISOString(),
    prompt,
    parameters: {
      kind: "blank",
      width: 1,
      depth: 1,
      height: 1,
      thickness: 1,
      holeDiameter: 0,
      holes: 0,
    },
    source: `${id}.step`,
    preview: `${id}.glb`,
    program: built.program,
  });
  const commit = async (document: any, parent: string, prompt: string) => {
    const id = crypto.randomUUID();
    const built = await window.__m2Build(document, id);
    project = {
      ...project,
      currentRevision: id,
      revisions: [...project.revisions, revision(id, parent, built, prompt)],
      files: [...project.files, ...addFiles(built)],
    };
    return id;
  };
  const programOf = (id: string) =>
    JSON.parse(project.revisions.find((item: any) => item.id === id).program);
  const applyTransforms = (document: any, commands: any[]) => {
    for (const command of commands) {
      const body = document.bodies.find(
        (item: any) => item.id === command.bodyId,
      );
      if (!body)
        throw JSON.stringify({
          code: "MISSING_TARGET",
          message: command.bodyId,
        });
      document.features.push({
        id: command.featureId,
        name: command.name,
        suppressed: false,
        operation: {
          type: "transform",
          bodyFeatureId: body.sourceFeatureId,
          matrix: command.matrix,
        },
      });
      body.sourceFeatureId = command.featureId;
    }
    return document;
  };
  const invoke = async (command: string, args: any) => {
    calls.push({ command, args: structuredClone(args) });
    if (command === "plugin:event|listen") {
      listeners.set(++eventId, { event: args.event, handler: args.handler });
      return eventId;
    }
    if (command === "list_projects") return [project];
    if (
      ["interrupted_sessions", "cad_task_status", "plugins_list"].includes(
        command,
      )
    )
      return [];
    if (command === "detect_environment")
      return [
        {
          name: "OpenCascade kernel",
          available: true,
          detail: "isolated fixture",
        },
      ];
    if (command === "acquire_project_access")
      return { projectId: args.projectId, mode: "write", ownerPid: 1 };
    if (command === "get_confirmation_settings")
      return { mode: "all", overrides: {} };
    if (command === "request_permission") return "grant";
    if (command === "read_project_file") return assets[args.name];
    if (command === "history_status")
      return {
        canUndo: slots.length > 0,
        canRedo: false,
        expectedRevision: project.currentRevision,
      };
    if (command === "apply_ir_commands") {
      if (args.expectedRevision !== project.currentRevision)
        throw JSON.stringify({ code: "STALE_REVISION", message: "stale" });
      const parent = project.currentRevision;
      const document = applyTransforms(programOf(parent), args.commands);
      const id = await commit(document, parent, args.prompt);
      slots.push({
        revision: id,
        parent,
        commands: args.commands,
        generation: ++generation,
      });
      return project;
    }
    if (command === "last_transform") {
      const slot = slots.at(-1);
      if (!slot || slot.revision !== project.currentRevision) return null;
      return {
        expectedRevision: slot.revision,
        expectedGeneration: slot.generation,
        transactionId: slot.revision,
        baseRevision: slot.parent,
        commands: slot.commands.map((item: any, index: number) => ({
          // Host shape: no command tag, only the replaceable fields.
          commandIndex: index,
          bodyId: item.bodyId,
          featureId: item.featureId,
          name: item.name,
          matrix: item.matrix,
        })),
      };
    }
    if (command === "amend_last_transform") {
      const slot = slots.at(-1);
      if (
        !slot ||
        slot.revision !== args.expectedRevision ||
        slot.generation !== args.expectedGeneration ||
        slot.revision !== args.transactionId
      )
        throw JSON.stringify({ code: "AMEND_STALE", message: "stale" });
      const commands = slot.commands.map((item: any, index: number) => {
        const replacement = args.replacements.find(
          (value: any) => value.commandIndex === index,
        );
        return replacement ? { ...item, matrix: replacement.matrix } : item;
      });
      const document = applyTransforms(programOf(slot.parent), commands);
      const id = await commit(
        document,
        project.currentRevision,
        `history:amend:${slot.revision}`,
      );
      slots[slots.length - 1] = {
        revision: id,
        parent: slot.parent,
        commands,
        generation: ++generation,
      };
      return project;
    }
    if (command === "undo_model") {
      const slot = slots.pop();
      if (!slot)
        throw JSON.stringify({ code: "HISTORY_NO_UNDO", message: "none" });
      project = { ...project, currentRevision: slot.parent };
      return project;
    }
    if (command === "save_project") {
      project = args.project;
      return project;
    }
    if (command === "get_agent_settings") return {};
    return null;
  };
  Object.assign(window, {
    isTauri: true,
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: (_event: string, id: number) => listeners.delete(id),
    },
    __TAURI_INTERNALS__: {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
      invoke,
      transformCallback: (handler: (...args: any[]) => void) => {
        callbacks.set(++callback, handler);
        return callback;
      },
      unregisterCallback: (id: number) => callbacks.delete(id),
    },
  });
  const React = await import("react"),
    ReactDOM = await import("react-dom/client"),
    Router = await import("react-router-dom"),
    Query = await import("@tanstack/react-query"),
    Tooltip = await import("@radix-ui/react-tooltip");
  void React;
  const { native } = await import("../src/lib/api");
  if (!native) throw Error("App imported the API before native flags were set");
  const { useWorkspace, newProject } = await import("../src/stores/workspace");
  project = newProject("M2", "blank", "mm", "codex");
  project.id = projectId;
  project.currentRevision = base.revisionId;
  project.revisions = [revision(base.revisionId, null, base, "Baseline")];
  project.files = addFiles(base);
  useWorkspace.getState().setProject(project);
  const { default: App } = await import("../src/app/App"),
    { applyTheme } = await import("../src/lib/theme");
  applyTheme("dark");
  await import("../src/styles.css");
  const Fiber = await import("@react-three/fiber"),
    THREE = await import("three");
  const root = ReactDOM.createRoot(document.getElementById("root")!);
  root.render(
    <Query.QueryClientProvider
      client={
        new Query.QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      <Tooltip.Provider>
        <Router.MemoryRouter initialEntries={[`/project/${projectId}`]}>
          <Router.Routes>
            <Router.Route path="/project/:projectId" element={<App />} />
          </Router.Routes>
        </Router.MemoryRouter>
      </Tooltip.Provider>
    </Query.QueryClientProvider>,
  );
  const live = () => {
    const canvas = document.querySelector(
      ".viewer-area canvas",
    ) as HTMLCanvasElement | null;
    return canvas ? Fiber._roots.get(canvas)?.store.getState() : null;
  };
  Object.assign(window, {
    __m2: {
      calls,
      theme: applyTheme,
      project: () => useWorkspace.getState().project,
      selection: () => useWorkspace.getState().selectedBodies,
      visible: () => live()?.scene.getObjectByName("body")?.visible ?? null,
      viewDirection: () =>
        live()
          ?.camera.getWorldDirection(new THREE.Vector3())
          .toArray()
          .map((value: number) => Math.round(value * 1000) / 1000) ?? null,
      /** World-space render bounds of the body; render X equals CAD X. */
      bounds: () => {
        const body = live()?.scene.getObjectByName("body");
        if (!body) return null;
        const box = new THREE.Box3().setFromObject(body);
        return [...box.min.toArray(), ...box.max.toArray()];
      },
      bodyScreen: () => {
        const state = live();
        const body = state?.scene.getObjectByName("body");
        if (!state || !body) return null;
        const centre = new THREE.Box3()
          .setFromObject(body)
          .getCenter(new THREE.Vector3())
          .project(state.camera);
        const rect = state.gl.domElement.getBoundingClientRect();
        return {
          x: rect.left + ((centre.x + 1) / 2) * rect.width,
          y: rect.top + ((1 - centre.y) / 2) * rect.height,
        };
      },
      /** Frame rate while the modal preview is updated every frame. */
      previewFps: (ms: number) =>
        new Promise<number>((resolve) => {
          let frames = 0;
          const start = performance.now();
          const tick = (now: number) => {
            frames++;
            const canvas = document.querySelector(".viewer-area canvas")!;
            const rect = canvas.getBoundingClientRect();
            window.dispatchEvent(
              new PointerEvent("pointermove", {
                clientX: rect.left + rect.width / 2 + Math.sin(frames / 5) * 80,
                clientY: rect.top + rect.height / 2,
              }),
            );
            if (now - start < ms) requestAnimationFrame(tick);
            else resolve((frames * 1000) / (now - start));
          };
          requestAnimationFrame(tick);
        }),
    },
  });
}
