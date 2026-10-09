# Point constraints and profile editing

[Documentation](index.md) · [Русский](../ru/sketch-constraints.md) · [Contours](sketch-profiles.md)

Open the source editor and expand **Edit 2D sketch**. Point coordinates use millimeters in the selected workplane; dragging changes the draft. **Fix point** captures the current X/Y in a fixed constraint. Edit its target coordinates or remove it in the constraint list. Select two distinct points and **Add coincidence** to make the solver match their coordinates. Duplicate fixed constraints and reversed duplicate coincidences are blocked.

Coinciding endpoints can collapse an edge; fixed targets can conflict with dimensions. Adding a constraint does not guarantee a valid profile. Diagnostics show the result; model regeneration checks the profile and B-Rep before saving a revision. Constraints reference stable IDs rather than screen positions.

**Split edge** inserts a midpoint and second edge. The first edge retains its ID, but its existing constraints are removed because an old full length cannot constrain a half edge. **Remove point** joins its neighbors; triangles cannot lose another vertex. Constraints on changed adjacent edges, attached guides, orphan points and their fixed/coincident references are removed. Unrelated IDs and constraints remain. Controls disable at limits and during builds.

[Profile tools](../../apps/desktop/src/lib/sketchProfileEditing.ts), [point constraints](../../apps/desktop/src/lib/sketchPointConstraints.ts) and [reference cleanup](../../apps/desktop/src/lib/sketchGeometry.ts) update a typed operation immutably. The draft CAD JSON reflects the change; the existing revision build route persists it. No SQL migration is needed. [Editing tests](../../apps/desktop/src/lib/sketchProfileEditing.test.ts) cover stable IDs, limits and constraint cleanup.
