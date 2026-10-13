# 21 — Scale Tool & Pin Distance

## Purpose

Define a fifth Edit-mode tool, **Scale**, alongside Select/Move/Rotation/Merge ([09-selection-and-editing.md](./09-selection-and-editing.md)), and a **Pin distance** numeric input in the Selection panel. Both change a Pin Path's size or spacing and must therefore recompute how many pins fit on it (per [07-pin-geometry-engine.md](./07-pin-geometry-engine.md)'s existing distribution rules — no new distribution algorithm is introduced here). Recomputation mints fresh pin identities, which orphans any Thread Path segment referencing the old pins; this spec also defines the **Nearest-Pin Reattachment** rule that keeps those threads visually and structurally connected across a scale or respacing.

This fills the "Advanced transformations" placeholder named in [00-overview-and-scope.md](./00-overview-and-scope.md) Phase 2 Scope, and the "Resize a Pin Path" undoable-operation entry already listed in [10-undo-redo.md](./10-undo-redo.md) without a defined interaction.

## Scale Tool

Added to the Edit Tool Area ([09-selection-and-editing.md](./09-selection-and-editing.md) §Edit Tool Area) as a fifth tool, grouped with Move/Rotation/Merge:

- **Scale** — press-drag-release, resizes the selected Pin Path about its own centroid.

Disabled unless a Pin Path is currently selected, same as Move/Rotation.

### Interaction

Press-drag-release, same family of gesture as Move/Rotation: pressing the left mouse button anywhere starts the gesture (the press location is **not** used as a pivot — unlike Rotation, which pivots on the press point). Dragging **right increases** the scale factor, dragging **left decreases** it, driven by horizontal screen-pixel delta (independent of zoom level, same convention as Rotation's `0.3°/screen-pixel`). A live preview updates pin positions/count (and any Thread Path segments already following those pins) in real time. Releasing commits the scale as **one** undoable operation. Esc cancels an in-progress Scale without committing anything, same as Move/Rotation/Merge.

The scale factor is clamped to a small positive minimum so a shape can never invert or collapse to a point.

### Pivot: the shape's own centroid

Unlike Rotation (external pivot at the press point), Scale always pivots on the shape's **own centroid** — recomputed once at gesture start and held fixed for the drag:

| Shape | Centroid |
|---|---|
| Line, Arc | Midpoint of Start and End |
| Circle, Ellipse, Regular polygon, Star, Polygram | Their existing Centre field |
| Rectangle, Square | `position + half-size` (same derivation Rotation already uses to find the shape's true centre before rotating it) |
| Freehand | Arithmetic mean of all points |

Because the pivot is always the shape's own centroid, the centroid/centre coordinate is **invariant** under scaling — only size fields change:

| Shape | Fields scaled by factor `f` | Fields left unchanged |
|---|---|---|
| Line, Arc | Start, End (each moved toward/away from the centroid) | — |
| Arc | Curvature (a physical sagitta length, scales with the shape) | — |
| Circle | Radius | Centre |
| Ellipse | Radius X, Radius Y | Centre, Rotation |
| Rectangle | Width, Height (Position re-derived from the unchanged centre) | Rotation |
| Square | Side (Position re-derived from the unchanged centre) | Rotation |
| Regular polygon | Radius | Centre, Rotation, Sides |
| Star | Outer radius, Inner radius | Centre, Rotation, Points |
| Polygram | Radius | Centre, Rotation, Points, Skip |
| Freehand | Every point (moved toward/away from the centroid) | — |

## Pin Distance Input

The Selection panel ([09-selection-and-editing.md](./09-selection-and-editing.md), [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) §Pin Properties) gains a **Pin distance** numeric field (the same "requested spacing" property already settable for new shapes before drawing) editable directly on the selected Pin Path. Changing it recomputes pins from the *existing* geometry against the new requested spacing, through the same per-shape distribution rules as any other spacing change — no new algorithm.

## Pin Quantity Recalculation

Both the Scale tool and the Pin distance input feed their result (scaled geometry, or unchanged geometry with a new requested spacing) through the existing distribution pipeline unchanged:

- Vertex-anchored shapes (Line, Rectangle, Square, regular polygons, Stars, Polygrams) → [Vertex-Anchored Pin Distribution](./07-pin-geometry-engine.md#vertex-anchored-pin-distribution).
- Closed curved shapes (Circle, Ellipse) → [Closed-Path Pin Distribution](./07-pin-geometry-engine.md#continuous-pin-spacing-through-corners).
- Open shapes (Arc, Freehand) → [Open-Path Pin Distribution](./07-pin-geometry-engine.md#open-path-pin-distribution).

`actualSpacing` and the live pin-count status bar behave exactly as they do for any other geometry/spacing edit ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md) §Live Preview Values).

## Nearest-Pin Reattachment on Scale/Respacing

Recomputing pins mints an entirely new set of pin identities. Without intervention, every Thread Path segment referencing an old pin would silently go dangling (as already happens today for a typed-in geometry edit — a known, unaddressed gap outside this spec's scope). Scale and Pin distance changes must instead reattach:

- For every old pin — including its symmetry-mirrored derived pins ([06-symmetry.md](./06-symmetry.md)), since a mirrored pin is a valid Thread endpoint too — find the **nearest new pin** (including new mirrored pins) by straight-line distance from the old pin's position.
- Remap every Thread Path pin reference from its old id to that nearest new id.
- This **contracts** a Thread Path the same way Merge's repoint does ([09-selection-and-editing.md](./09-selection-and-editing.md) §Merge): any adjacent duplicate ids created by the remap are collapsed, and a Thread Path left with fewer than 2 distinct ids after collapsing is dropped entirely.
- The pin recompute and the thread reattachment happen together as **one** undoable operation — the same bundled-command pattern already used for cascading pin deletion ([11-erasers.md](./11-erasers.md)) and Merge.
- If the Pin Path's layer is locked, the entire operation is blocked — no mutation, same as every other locked-layer edit rule ([13-layers.md](./13-layers.md)).

This rule is scoped to Scale and Pin distance only. Other numeric geometry-field edits (typing an exact new Position, Radius, etc. directly) keep today's existing behavior — pins get fresh ids, referencing threads are not reattached. That gap is not addressed here.

## Test Cases

```gherkin
Feature: Scale tool

  Scenario: Scaling up a Circle Pin Path recalculates pin count
    Given a Circle Pin Path is selected with radius 4cm and spacing 1cm
    When the user drags the Scale tool to double the radius
    Then the path's radius becomes 8cm
    And the pin count recalculates per the closed-path distribution rule for the new perimeter
    And the centre point does not move

  Scenario: Scaling preserves the shape's own centroid
    Given a Rectangle Pin Path is selected away from the origin
    When the user scales it up or down
    Then the rectangle's centre (position + half width/height) stays fixed
    And only width and height change

  Scenario: Scaling an Arc scales its curvature
    Given an Arc Pin Path is selected with a non-zero curvature
    When the user scales it down by half
    Then start and end move toward the arc's midpoint
    And curvature is also halved, keeping the arc's proportions

  Scenario: A thread endpoint reattaches to the nearest new pin after scaling
    Given a Thread Path has one endpoint on a pin of a Pin Path
    When that Pin Path is scaled, changing its pin positions and count
    Then the Thread Path's endpoint now references whichever new pin is nearest the old pin's position
    And the thread renders connected with no visible gap

  Scenario: Two old pins mapping to the same new pin collapse the thread segment
    Given a Thread Path connects two pins on a Pin Path that are close together
    When that Pin Path is scaled down enough that both old pins map to the same nearest new pin
    Then the Thread Path's duplicate adjacent reference collapses to one
    And if fewer than 2 distinct pin ids remain, the Thread Path is dropped entirely

  Scenario: Scaling a locked layer's Pin Path is blocked
    Given a Pin Path is selected on a locked Pin Layer
    When the user attempts to drag the Scale tool
    Then no geometry, pin, or thread change occurs

  Scenario: Esc cancels an in-progress Scale
    Given the user is mid-drag on the Scale tool
    When they press Esc
    Then the Pin Path's geometry, pins, and any threads are left exactly as they were before the drag started

  Scenario: Undo reverts a Scale as one step
    Given a Scale has just been committed, changing pins and reattaching a thread
    When the user presses Undo
    Then both the pin layer and thread layer revert together in a single undo step

Feature: Pin distance input

  Scenario: Changing Pin distance recalculates pin count
    Given a Pin Path is selected with requested spacing 1cm
    When the user sets Pin distance to 0.5cm in the Selection panel
    Then the pins recompute per the shape's existing distribution rule at the new spacing
    And the geometry itself is unchanged

  Scenario: Changing Pin distance reattaches existing threads
    Given a Thread Path references a pin on the selected Pin Path
    When the user changes Pin distance
    Then the Thread Path's reference is remapped to the nearest new pin, same as the Scale tool
    And the pin and thread changes commit as one undo step

  Scenario: Changing Pin distance on a locked layer's Pin Path is blocked
    Given the selected Pin Path's layer is locked
    When the user changes Pin distance
    Then no change occurs
```
