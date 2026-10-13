# 09 — Selection & Editing

## Purpose

Define what the Selection tool can select and how selecting an existing Pin Path exposes its editable geometry, enabling modification rather than requiring redraw. Also defines the Edit-mode tool area (Select / Move / Rotation / Merge) — the mode tab itself is labeled **"Edit"**, since it now covers more than plain selection.

## Selection Scope

Selection supports:

- Pin Paths
- Individual pins where appropriate
- Thread Paths
- Symmetry centre (radial)
- Editable board controls

## Pin Path Editing by Shape

Selecting an existing Pin Path exposes its geometry parameters for direct editing. Any relevant edit **automatically recalculates generated pins** (per [07-pin-geometry-engine.md](./07-pin-geometry-engine.md)) and, if the path has symmetry attached, regenerates mirrored/radial copies (per [06-symmetry.md](./06-symmetry.md)).

| Shape | Editable properties |
|---|---|
| Line | Start, End |
| Arc | Start, End, Curvature |
| Circle | Centre, Radius |
| Ellipse | Centre, Width, Height, Rotation |
| Rectangle | Position, Width, Height, Rotation |
| Square | Position, Side, Rotation |
| Polygon / Star | Centre, Size, Rotation, shape-specific parameters (e.g. point count for stars, inner/outer radius ratio) |

Also editable regardless of shape (see [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)):

- Spacing
- Colour
- Diameter
- Geometry size
- Position
- Rotation
- Guide visibility
- Shape-specific properties

## Interaction Rules

- Selecting an object switches focus to its property panel; the object's control handles (e.g. Line's start/end grips, Arc's curvature grip) become interactive on canvas.
- Dragging a handle updates the geometry live, with the pin-count/spacing preview updating in real time (same live-preview behavior as initial drawing).
- Selecting a Thread Path exposes thread-level properties (colours, width — see [12-thread-editor.md](./12-thread-editor.md)); it does not expose pin-geometry controls.
- Selecting the radial symmetry centre allows it to be dragged, per [06-symmetry.md](./06-symmetry.md).
- Selecting an object on a locked layer must not allow edits (see [13-layers.md](./13-layers.md)).
- A Pin Path/Pins selection only exists in Edit mode: switching to Pin or Thread mode clears it. A Thread Path selection only exists in Thread mode: switching to Pin or Edit mode clears it. (Switching to Pan or Play leaves the current selection untouched — those modes don't have their own selection concept, so there's nothing to reconcile it with.) Within Pin mode, picking any pin tool also clears a Pin Path/Pins selection (see [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)); within Thread mode, picking any thread tool also clears a Thread Path selection (see [12-thread-editor.md](./12-thread-editor.md)).

## Edit Tool Area

Edit mode has four tools:

- **Select** — click a pin to select its Pin Path (the existing behaviour above).
- **Move** — translates the selected Pin Path.
- **Rotation** — rotates the selected Pin Path.
- **Merge** — combines several pins (from any Pin Path, any layer) into one.

Move and Rotation are disabled unless a Pin Path is currently selected. Merge is always enabled — it builds its own multi-pin selection independent of the single-object Selection.

### Move

Press-drag-release: pressing the left mouse button anywhere starts the translation, dragging shows a live preview (both the path's pins and any Thread Path segments referencing them visibly follow), and releasing commits the move as **one** undoable operation. Unlike a numeric geometry edit in the property panel (which fully regenerates `pins[]` from `geometry`+spacing, discarding any manual Pin Eraser removals), Move translates the **existing** pins in place and keeps their ids stable — any pin already erased stays erased, and every Thread Path segment referencing this Pin Path's pins keeps resolving correctly after the move.

### Rotation

Press-drag-release: the point where the mouse was pressed becomes the rotation pivot — **not** the shape's own centre. Dragging **right increases** the angle, dragging **left decreases** it, at a rate of **0.3° per screen pixel** dragged (a tunable constant, independent of zoom level so the feel stays consistent). Releasing commits the rotation as one undoable operation, with the same live pin+thread preview and same in-place, id-stable pin rotation as Move.

Rotating about an external pivot moves the shape's centre/position to `rotatePoint(oldCentre, pivot, θ)` and adds `θ` to the shape's own `rotation` field (for shapes that have one). For Rectangle/Square this requires care: `position` is defined as the shape's corner in its own unrotated frame, so the centre must be derived, rotated about the pivot, and re-derived back into a new `position` — rotating `position` directly around an external pivot would not keep the shape's actual centre correctly placed.

### Merge

Left-click accumulates pins into a pending selection (clicking an already-selected pin again deselects it); right-click commits. Merging fewer than 2 pins is a no-op that just clears the pending selection. On commit:

- A new pin is created at the **average position** of every selected pin.
- The new pin is added to the **first-clicked pin's Pin Path** (which becomes the new Selection); every merged-away pin is removed from wherever it lived.
- Every Thread Path segment referencing a merged-away pin is repointed to the new pin. This **contracts** the Thread Path (it is never fragmented the way the Segment Eraser splits one) — any adjacent duplicate ids the merge creates are collapsed, and a Thread Path left with fewer than 2 distinct ids after collapsing is dropped entirely.
- The whole merge is one undoable operation. If any selected pin belongs to a locked Pin Layer, the entire merge is aborted with no mutation — the pending selection is left intact so the user can deselect the locked pin or press Esc.
- Only real, stored pins are selectable for Merge — a mirrored/radial copy is not, since it has no entry in any Pin Path's `pins[]` to remove.

Esc cancels an in-progress Move, Rotation, or Merge without committing anything.

## Test Cases

```gherkin
Feature: Selecting objects

  Scenario: Selecting a Pin Path exposes its geometry handles
    Given a Circle Pin Path exists on canvas
    When the user selects it with the Selection tool
    Then centre and radius handles become visible and draggable
    And the property panel shows Centre, Radius, Spacing, Colour, Diameter, Guide visibility

  Scenario: Selecting a Thread Path exposes thread properties, not pin geometry
    Given a Thread Path exists connecting several pins
    When the user selects it
    Then the property panel shows colours, width, and twist configuration
    And no pin-geometry controls (radius, side length, etc.) are shown

  Scenario: Selecting the radial symmetry centre allows repositioning
    Given a Pin Path with radial symmetry and a visible centre indicator ◎
    When the user selects and drags the centre
    Then all radial copies recompute around the new centre position

Feature: Selection clears across mode and tool switches

  Scenario: Entering Pin mode clears a Pin Path/Pins selection
    Given a Pin Path is selected in Edit mode
    When the user switches to Pin mode
    Then no Pin Path or pin is selected

  Scenario: Entering Thread mode clears a Pin Path/Pins selection
    Given a Pin Path is selected in Edit mode
    When the user switches to Thread mode
    Then no Pin Path or pin is selected

  Scenario: Entering Edit mode clears a Thread Path selection
    Given a Thread Path is selected in Thread mode
    When the user switches to Edit mode
    Then no Thread Path is selected

  Scenario: Entering Pin mode clears a Thread Path selection
    Given a Thread Path is selected in Thread mode
    When the user switches to Pin mode
    Then no Thread Path is selected

Feature: Editing geometry recalculates pins

  Scenario: Changing Circle radius recalculates pin count
    Given a selected Circle Pin Path with radius 5 cm and requested spacing 1 cm
    When the user drags the radius handle to 8 cm
    Then the perimeter, actual spacing, and pin positions are recalculated for the new radius
    And the pin count updates to match the closed-path algorithm's result for the new perimeter

  Scenario: Changing Line endpoints recalculates open-path pins
    Given a selected Line Pin Path from (0,0) to (8,0) cm with spacing 1 cm (9 pins)
    When the user drags the End handle to (7.5, 0) cm
    Then the pin count updates to 8 per the open-path algorithm

  Scenario: Changing Arc curvature recalculates pins live
    Given a selected Arc Pin Path
    When the user drags the curvature handle to increase the arc's radius
    Then the arc length, and thus the pin count/positions, update in real time

  Scenario: Editing a shape with symmetry regenerates mirrored copies
    Given a selected Pin Path with vertical symmetry and one mirrored copy
    When the user changes the Rectangle's Width
    Then the source Rectangle's pins are recalculated
    And the mirrored copy is regenerated to reflect the updated source

  Scenario: Rotating a Polygon updates pin positions without changing pin count
    Given a selected regular Hexagon Pin Path with 30 pins
    When the user rotates it by 15°
    Then all 30 pins rotate correspondingly
    And the pin count remains 30 (rotation does not trigger redistribution)

Feature: Editing respects layer locks

  Scenario: Cannot edit a Pin Path on a locked layer
    Given a Pin Path belongs to a locked Pin Layer
    When the user attempts to drag one of its geometry handles
    Then the geometry does not change
    And the property panel controls for that object are disabled or the edit is rejected

Feature: Move tool

  Scenario: Moving a Pin Path shows threads following live during the drag
    Given a selected Pin Path with a Thread Path connecting two of its pins
    When the user presses and drags with the Move tool
    Then the pins and the connecting thread segment visibly follow the cursor before release
    And nothing is committed to the document until the mouse is released

  Scenario: Move commits once, as a single undo step
    Given a selected Pin Path
    When the user drags it with the Move tool and releases
    Then the Pin Path's geometry and pins reflect the translation
    When the user invokes Undo once
    Then the Pin Path is restored to its exact pre-move geometry and pins

Feature: Rotation tool

  Scenario: Rotating a Rectangle about an external pivot keeps its shape and size
    Given a selected Rectangle Pin Path
    When the user presses at a point OUTSIDE the rectangle and drags right by an amount producing a 90° rotation
    Then the rectangle's centre moves to exactly where rotating its original centre by 90° about that pivot would place it
    And the rectangle's width and height are unchanged

  Scenario: Dragging left decreases the angle
    Given a selected Pin Path and the Rotation tool
    When the user presses and drags left
    Then the rotation angle is negative relative to the press point

Feature: Merge tool

  Scenario: Merging pins from two different Pin Paths lands the result in the first-clicked path
    Given pin-1 in Pin Path A and pin-2 in Pin Path B
    When the user left-clicks pin-1, then pin-2, then right-clicks to commit
    Then a new pin appears in Pin Path A at the midpoint of pin-1 and pin-2
    And pin-1 and pin-2 no longer exist

  Scenario: Merge collapses adjacent duplicate thread pin ids it creates
    Given a Thread Path visiting pin-1, pin-2, pin-3 in order
    When the user merges pin-2 and pin-3 into a new pin
    Then the Thread Path now visits pin-1 and the new pin only, with no repeated consecutive id

  Scenario: Merge aborts entirely when one selected pin's layer is locked
    Given pin-1 on an unlocked Pin Layer and pin-2 on a locked Pin Layer
    When the user selects both and right-clicks to commit
    Then no pin is merged and no Thread Path changes
    And the pending merge selection is left intact

  Scenario: Clicking an already-selected pin again removes it from the pending merge
    Given the user has left-clicked pin-1 with the Merge tool
    When the user left-clicks pin-1 again
    Then pin-1 is no longer part of the pending merge selection
```
