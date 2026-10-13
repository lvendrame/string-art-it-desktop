# 26 — Edit-Mode Multi-Select & Granularity

## Purpose

Extend Edit mode's Select/Move/Rotation/Scale/Merge tools ([09-selection-and-editing.md](./09-selection-and-editing.md), [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md)) to operate over a **multi-object selection** instead of a single Pin Path, gated by a new **granularity switch** that decides whether the unit of selection is a whole Pin Path or an individual Pin. Merge stops being a tool with its own click-to-accumulate-then-commit gesture and becomes an instant action fired against whatever the current selection holds.

## Granularity Switch

A two-option switch, placed **before** the Edit Tools area (above the Select/Move/Rotation/Scale tool grid), reads:

```text
[ Pin Path | Pins ]
```

- **Pin Path** (default) — the unit of selection is a whole Pin Path.
- **Pins** — the unit of selection is an individual Pin, which may belong to any Pin Path on any (unlocked or locked — selection itself is always allowed, only edits are blocked) Pin Layer.

Flipping the switch **clears the current selection** to none — a path-selection and a pin-selection are different kinds of things and are never translated across the switch.

## Selection Gestures

The Select tool builds the selection via:

- **Click** — selects exactly the clicked item, replacing any existing selection.
- **Alt/Cmd + Click** — toggles the clicked item's membership in the current selection (adds it if absent, removes it if present), leaving every other selected item untouched. Clicking empty space with Alt/Cmd held is a no-op (the existing selection is left exactly as it was).
- **Click-drag-release (rubber-band)** — pressing the left mouse button, moving the pointer, and releasing selects every item touched by the rectangle formed by the press and release points, replacing any existing selection. An empty rectangle (nothing touched) results in an empty selection.
- **Alt/Cmd + click-drag-release** — same rectangle, but the touched items are **added** to the current selection instead of replacing it (existing selected items outside the rectangle remain selected).
- **Esc** while dragging a rubber-band cancels the drag with no change to the selection (nothing is committed).

**What "touched by the rectangle" means, per granularity:**

- **Pin Path** granularity — a Pin Path is touched if any of its real, stored pins (or a symmetry-derived mirrored/radial pin belonging to it) falls inside the rectangle — the same "a mirrored pin's click resolves to its source path" rule single-object Select already uses ([09-selection-and-editing.md](./09-selection-and-editing.md), [06-symmetry.md](./06-symmetry.md)).
- **Pins** granularity — only real, stored pins are selectable (mirrored/radial copies are excluded, since a pins-mode transform writes directly into a Pin Path's `pins[]` and a mirrored pin has no entry there to write to — the same restriction Merge's pin-picking already had).

## Pin Layer Scope for Selection

Both Selection Gestures above (click and rubber-band, either granularity) only target pins on **visible** Pin Layers — a hidden layer's Pin Paths/pins are not selectable, and therefore never reach Move/Rotation/Scale/Merge either, since those tools only ever act on whatever `state.selection` already holds (see [13-layers.md](./13-layers.md)'s "hidden layers retain their contents" guarantee — hiding a layer removes it from Select targeting without deleting anything).

Among visible Pin Layers, the **active Pin Layer** ([13-layers.md](./13-layers.md)) is a priority stage, not a tie-break:

- **Click** — if the active layer has a pin/path within snap radius of the click, it wins outright, even when another visible layer has a geometrically closer one. Only when the active layer has nothing in range does the click fall back to the nearest match among the other visible layers.
- **Rubber-band** — if the rectangle touches anything on the active layer, only the active layer's touched paths/pins are selected — touched items on other visible layers are dropped, not merged in. Only when the active layer has nothing touched does the rectangle fall back to the touched set from the other visible layers.

This is the same scope/priority rule [12-thread-editor.md](./12-thread-editor.md) applies to Thread insertion, applied here to Select.

## Tools by Granularity

### Pin Path granularity

Select, Move, Rotation, Scale and Merge all act on the set of **selected Pin Paths**.

- **Move** — translates every selected Pin Path by the same drag delta, in place (pin ids stable), exactly like today's single-path Move. One undo step covers every affected path.
- **Rotation** — pivots at the **press point** (external pivot, unaffected by how many paths are selected — same 0.3°/screen-pixel convention as today), rotating every selected path's pins/geometry about that one shared point. One undo step.
- **Scale** — pivots on a **combined centroid**:
  - Exactly one path selected → the existing per-shape centroid formula from [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md) (unchanged, byte-identical to today).
  - Two or more paths selected → the arithmetic mean of every real pin across every selected path.
  This is also the pivot the keyboard-driven Rotate/Scale ([23-keyboard-transform.md](./23-keyboard-transform.md), which already pivots on its own centroid rather than a press point) uses when multiple paths are selected. Scaling recomputes pins per path exactly as today's single-path Scale does (existing distribution pipeline, existing nearest-pin thread reattachment), bundled as one undo step covering every path plus the thread layer.
- **Merge** — combines every selected Pin Path into **one** resulting Pin Path:
  1. The first-selected path's identity (id, style, symmetry configuration) is kept; every selected path's pins are pooled together.
  2. Any pins that land within a small position tolerance of one another collapse into one pin, using the existing merge-pin averaging rule (the destination position is the average of the colliding pins' positions).
  3. **Every surviving pin** — colliding or not — is then reminted a brand-new stable id, and every original pin (from every selected path) is remapped to whichever surviving new pin is nearest it, using the same nearest-pin reattachment mechanism Scale already uses ([21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md) §Nearest-Pin Reattachment). Every Thread Path segment referencing any of the merged paths' pins is remapped accordingly (adjacent-duplicate collapse, <2-distinct-ids drop — same rules as today's Merge).
  4. The destination's **guide geometry becomes a Freehand path threading through the combined pins**, in the same order as the merged `pins[]` — it does **not** keep the first-selected path's original shape type (circle/line/etc.) unchanged. A merge almost always combines pins from more than one shape; retaining only one shape's geometry while `pins[]` holds more than that shape can produce would be a data-loss trap, since every later geometry-driven recompute (Scale, Pin distance — see [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md)) regenerates `pins[]` **strictly from `geometry`**, silently discarding whatever isn't explained by it. Freehand is the one geometry type that can honestly represent an arbitrary combined point set, so recomputes after a merge stay data-preserving instead of silently dropping whichever shape doesn't match a retained single-shape geometry.
  5. The whole combine is one undoable operation covering the pin layer and the thread layer together. Merging fewer than 2 selected paths is a no-op.

### Pins granularity

Select, Move, Rotation, Scale and Merge all act on the set of **selected Pins** (which may span several Pin Paths and layers).

- **Move / Rotation / Scale** — a **direct transform of just the selected pins' raw x/y positions**. The owning Pin Path's `geometry`/requested-spacing metadata is left untouched — no pin-count recompute, no distribution re-run (the same "geometry becomes stale/custom" precedent the Pin Eraser already established when removing individual pins, [11-erasers.md](./11-erasers.md)). Rotation pivots at the press point, same as path-mode. Scale/keyboard-driven Rotate pivot at the **mean position of every selected pin** (always — there is no single-pin special case to preserve, this is a new granularity). One undo step covers every affected pin, however many paths they span.
- **Merge** — same per-pin averaging/thread-repoint rule as today's Merge (destination position = average of every selected pin's position), but the **destination Pin Path** is chosen by:
  1. Whichever selected Pin Path contributed the most selected pins.
  2. Tie → the path whose own selected pins' mean position is closest to the new pin's position.
  3. Tie → the path with the lowest (oldest/first-created) Pin Path id.
  Merging fewer than 2 selected pins is a no-op.

## Locked Layers

If any selected/involved Pin Path or Pin belongs to a locked Pin Layer, the entire gesture (Move, Rotation, Scale, or Merge) is aborted with **zero mutation** — the same all-or-nothing rule Merge already has today ([13-layers.md](./13-layers.md)).

## Esc

Esc cancels an in-progress Move, Rotation, Scale, or rubber-band drag without committing anything, same as today.

## Test Cases

```gherkin
Feature: Granularity switch

  Scenario: Switching granularity clears the selection
    Given a Pin Path is selected in Pin Path granularity
    When the user switches the granularity switch to Pins
    Then the selection becomes empty

Feature: Selection gestures

  Scenario: Click replaces the selection
    Given two Pin Paths are selected
    When the user clicks a third, unselected Pin Path
    Then only the third Pin Path is selected

  Scenario: Alt-click toggles membership without affecting the rest
    Given Pin Path A and Pin Path B are selected
    When the user Alt-clicks Pin Path A
    Then Pin Path A is deselected
    And Pin Path B remains selected

  Scenario: Alt-click on an unselected item adds it
    Given Pin Path A is selected
    When the user Alt-clicks Pin Path B
    Then both Pin Path A and Pin Path B are selected

  Scenario: Alt-click on empty space is a no-op
    Given Pin Path A is selected
    When the user Alt-clicks empty canvas
    Then Pin Path A remains the only selected item

  Scenario: Rubber-band drag replaces the selection with the touched set
    Given Pin Path A is selected
    When the user drags a rectangle touching only Pin Path B and Pin Path C
    Then the selection becomes exactly Pin Path B and Pin Path C

  Scenario: Alt-rubber-band drag adds the touched set to the existing selection
    Given Pin Path A is selected
    When the user Alt-drags a rectangle touching Pin Path B
    Then Pin Path A and Pin Path B are both selected

  Scenario: Esc cancels an in-progress rubber-band drag
    Given Pin Path A is selected
    When the user starts dragging a rubber-band rectangle and presses Esc before releasing
    Then the selection remains exactly Pin Path A, unchanged

Feature: Pin Layer scope for Selection

  Scenario: Hidden layer's pin is not a valid click target
    Given Pin Layer "Detail" is hidden, and pin-9 (on "Detail") is the closest pin to the click point
    And Pin Layer "Base" is visible with pin-14 also within snap radius but farther away
    When the user clicks near pin-9 in Pins granularity
    Then pin-14 is selected, not pin-9

  Scenario: Active Pin Layer's path is prioritized over a closer path on another visible layer
    Given Pin Layer "Base" is the active Pin Layer and both "Base" and "Overlay" are visible
    And Pin Path X (on "Base") and Pin Path Y (on "Overlay") both have a pin within snap radius of the click, with Y's pin closer
    When the user clicks in Pin Path granularity
    Then Pin Path X is selected, not Pin Path Y

  Scenario: Rubber-band selects only the active layer's touched paths when it has any
    Given Pin Layer "Base" is the active Pin Layer and both "Base" and "Overlay" are visible
    And a rubber-band rectangle touches Pin Path X (on "Base") and Pin Path Y (on "Overlay")
    When the user releases the drag
    Then only Pin Path X is selected

  Scenario: Rubber-band falls back to another visible layer when the active layer has nothing touched
    Given Pin Layer "Base" is the active Pin Layer with nothing inside the rubber-band rectangle
    And Pin Layer "Overlay" is visible with Pin Path Y inside the rectangle
    When the user releases the drag
    Then Pin Path Y is selected

  Scenario: A hidden layer's paths are never selected by rubber-band, even without an active-layer match
    Given Pin Layer "Detail" is hidden with Pin Path Z inside the rubber-band rectangle
    And no visible layer has anything inside the rectangle
    When the user releases the drag
    Then the selection is empty

Feature: Path-mode Move/Rotation/Scale over multiple selected paths

  Scenario: Moving two selected Pin Paths commits as one undo step
    Given two Pin Paths are selected in Pin Path granularity
    When the user drags the Move tool and releases
    Then both paths' pins reflect the translation
    When the user invokes Undo once
    Then both paths are restored to their exact pre-move geometry and pins

  Scenario: Scale centroid is the per-shape formula when exactly one path is selected
    Given exactly one Circle Pin Path is selected
    When the user scales it
    Then the centre point does not move, matching today's single-path Scale behaviour exactly

  Scenario: Scale centroid is the mean of all pins when multiple paths are selected
    Given two Pin Paths are selected together
    When the user scales them
    Then the pivot used equals the arithmetic mean of every real pin across both paths

Feature: Pins-mode Move/Rotation/Scale

  Scenario: Moving selected pins leaves the owning path's geometry untouched
    Given three individual pins from the same Pin Path are selected in Pins granularity
    When the user moves them
    Then the three pins' positions change
    And the Pin Path's geometry field is unchanged
    And the Pin Path's pin count is unchanged

  Scenario: Scale centroid in Pins granularity is always the mean of selected pins
    Given two pins from two different Pin Paths are selected in Pins granularity
    When the user scales them
    Then the pivot used equals the mean of those two pins' positions

Feature: Locked layers block multi-select gestures

  Scenario: Any locked layer in the selection aborts the whole gesture
    Given two Pin Paths are selected, one on a locked layer
    When the user attempts a Move
    Then no geometry, pin, or thread change occurs on either path

Feature: Path-mode Merge combines selected paths into one

  Scenario: Merging two Pin Paths combines their pins into the first-selected path
    Given Pin Path A (selected first) and Pin Path B are selected in Pin Path granularity
    When the user commits Merge
    Then a single resulting Pin Path exists with the identity of Pin Path A
    And it contains pins pooled from both A and B

  Scenario: Coincident pins collapse and every surviving pin is renumbered
    Given Pin Path A and Pin Path B share two pins at (nearly) the same position
    When the user commits Merge
    Then those coincident pins collapse into one
    And every surviving pin in the resulting path has a newly minted id
    And every Thread Path that referenced any of the original pins now references the nearest surviving new pin

  Scenario: Merging fewer than two selected paths is a no-op
    Given only one Pin Path is selected
    When the user commits Merge
    Then nothing changes

  Scenario: A later Scale does not lose either shape's pins after merging different shape types
    Given a Circle Pin Path and a Line Pin Path are merged together
    When the user then scales the resulting Pin Path
    Then the recomputed pins include contributions from both the original circle and the original line
    And no pins are silently dropped

Feature: Pins-mode Merge destination path rule

  Scenario: Destination path is whichever path contributed the most selected pins
    Given 3 selected pins belong to Pin Path A and 1 selected pin belongs to Pin Path B
    When the user commits Merge
    Then the new merged pin is added to Pin Path A

  Scenario: A count tie is broken by closeness of each tied path's own centroid
    Given 2 selected pins belong to Pin Path A and 2 selected pins belong to Pin Path B
    And Pin Path A's selected pins have a mean position closer to the new pin's position than Pin Path B's
    When the user commits Merge
    Then the new merged pin is added to Pin Path A

  Scenario: A full tie is broken by the lowest (oldest) Pin Path id
    Given a count tie and an equal-centroid-distance tie between Pin Path A and Pin Path B
    And Pin Path A was created before Pin Path B
    When the user commits Merge
    Then the new merged pin is added to Pin Path A
```
