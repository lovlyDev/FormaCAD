# Roadmap
[Documentation](index.md) · [Русский](../ru/roadmap.md)

I’m prioritizing the following CAD work:
1. Extend the bounded polygon Sketcher with arcs and a more scalable solver.
2. Broader, verifiable face and edge references across arbitrary operations. The current semantic edge keys cover supported owned box/circular constructors and tracked rigid transforms.
3. Extend existing draft/AI candidate review with broader regeneration diagnostics.
4. Feature build status and recovery controls for failed regeneration.

The current CAD IR v2 tree supports suppression and per-body rollback. Exact geometry metrics, structural revision diff and up to two bounded AI correction attempts are in place. These are foundations, not a finished sketcher or universal topology naming system.

Architectural references:
- [ToubkalCAD](https://github.com/ToubkalCAD/ToubkalCAD): separate geometry ownership and scene metadata.
- [OneCAD](https://github.com/andrejvysny/OneCAD): domain regeneration and worker boundaries.
- [AgentCAD](https://github.com/jdilla1277/agentcad): build/inspect/metrics/diff.
- [WEB-CAD](https://github.com/wordingone/WEB-CAD): explicit command capabilities and schemas.
- [ChiselCAD](https://github.com/LTKMN/ChiselCAD): linking parameters, code and geometry; serialized reevaluation.
- [replicad selectors](https://replicad.xyz/docs/api/classes/EdgeFinder/) and [build123d](https://github.com/gumyr/build123d): expressive operations and geometric selection.

These are references for independent implementations. No source-available editor is embedded in Forma. License terms must be checked before adopting implementation code.


Available now: [sketches](sketches.md), [3D draft preview](model-preview.md), [AI candidate review](ai-candidate-review.md), [exact measurements](reference-measurements.md) and [face sections](face-sections.md). Arbitrary-folder authority, all-consumer repository sessions, general topology remapping, arcs and media-loss/NAS guarantees remain incomplete. Release 1.2.6 does not establish 2.0 readiness.
