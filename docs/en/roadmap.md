# Roadmap
[Documentation](index.md) · [Русский](../ru/roadmap.md)

I’m prioritizing the following CAD work:
1. A full feature tree with suppression, rollback and build status.
2. Stable semantic face/edge references, fillets, chamfers and sketch planes.
3. Geometry metrics, AI change previews and bounded repair attempts.
4. Debounced parameter previews and an interactive constrained sketcher.

The current typed document and parameter editor are a foundation, not a completed sketcher or full feature-history system.

Architectural references:
- [ToubkalCAD](https://github.com/ToubkalCAD/ToubkalCAD): separate geometry ownership and scene metadata.
- [OneCAD](https://github.com/andrejvysny/OneCAD): domain regeneration and worker boundaries.
- [AgentCAD](https://github.com/jdilla1277/agentcad): build/inspect/metrics/diff.
- [WEB-CAD](https://github.com/wordingone/WEB-CAD): explicit command capabilities and schemas.
- [ChiselCAD](https://github.com/LTKMN/ChiselCAD): linking parameters, code and geometry; serialized reevaluation.
- [replicad selectors](https://replicad.xyz/docs/api/classes/EdgeFinder/) and [build123d](https://github.com/gumyr/build123d): expressive operations and geometric selection.

These are references for independent implementations. No source-available editor is embedded in Forma. License terms must be checked before adopting implementation code.
