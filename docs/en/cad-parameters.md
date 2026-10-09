# Named parameters and dimensions

[Documentation](index.md) · [Русский](../ru/cad-parameters.md) · [Constraints](sketch-constraints.md)

Expand **Named CAD parameters** in the typed feature tree. Enter a name and millimeter value and select **Add parameter**. Names remain user text; a unique parameter_N ID stays stable as values change. Existing parameters appear immediately. Editing a value drives every referencing operation on regeneration.

Add a length constraint to a profile or construction edge, then select the parameter in **Dimension source**. The constraint stores `{kind: "parameter", parameterId: ...}`. Its numeric input is replaced by the current parameter value. Returning to **Numeric dimension** copies the current value with a 0.01 mm lower bound. Fixed coordinates do not support parameter binding yet.

Each parameter row lists referencing feature IDs. Deletion is disabled while references exist: unbind the dimensions first. Reference scanning includes all operation fields and suppressed features. Unused parameters can be removed. Limits are 10,000 parameters, finite values with absolute magnitude at most 10,000 mm, and nonempty names of at most 120 characters without control characters. Negative values suit some operations, but sketch lengths must be positive; the solver reports invalid dimensions.

The [editor](../../apps/desktop/src/components/CadParameterEditor.tsx) changes `document.parameters` in the same draft as the feature tree. [Reference scanning](../../apps/desktop/src/lib/cadParameters.ts) leaves operations unchanged. Successful regeneration saves a new revision; editing and diagnostics do not overwrite the saved model. Older documents and SQL remain compatible. [Tests](../../apps/desktop/src/lib/cadParameters.test.ts) cover IDs, limits and references. Preview currently covers the 2D sketch; there is no automatic 3D preview of the entire model.


In 1.2.6 parameters can drive coordinates and origins across multiple sketches. Open Edit 2D sketch → Coordinate parameter links; [the sketch guide](sketches.md#explicit-coordinate-links) explains storage, limits and the linked-profile example. Pre-build review requires an explicit choice when geometric constraints change the drawn shape.
