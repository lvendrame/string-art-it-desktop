# 06 — Symmetry

## Purpose

Define symmetrical pin generation. Symmetry currently applies **only to pins** (not threads).

## Functional Requirements

Supported modes:

- None
- Vertical
- Horizontal
- Vertical + Horizontal
- Radial

Symmetry happens **while drawing** — copies appear live as the source geometry is drawn/edited, not as a separate manual "apply mirror" step.

Generated symmetrical copies remain **logically linked** to the source geometry: changing the source recalculates the symmetrical copies. They are derived data, not independent Pin Paths the user can edit directly.

## Horizontal / Vertical Symmetry

The editor generates reflected Pin Paths across the configured symmetry axis/axes.

```text
        │
    A   │   A'
────────┼────────
    A'' │   A'''
        │
```

- Vertical only → 1 source + 1 reflected copy (across vertical axis).
- Horizontal only → 1 source + 1 reflected copy (across horizontal axis).
- Vertical + Horizontal → 1 source + 3 reflected copies (both axes plus the diagonal combination).

## Radial Symmetry

Configurable angular interval, e.g.:

```text
15°
30°
45°
60°
90°
```

Custom angles must also be accepted (not limited to the preset list).

Radial symmetry has a centre point. Initially positioned at the board centre, but the user must be able to manually move it.

```text
          copy
            \
             \
copy -------- ◎ -------- copy
             /
            /
          copy
```

`◎` = movable radial symmetry centre.

Number of copies (including the source) = `360 / interval` (e.g. 45° interval → 8 total instances around the centre).

## Symmetry Domain Model

```text
PinPath
│
├── sourceGeometry
└── symmetry
    ├── type       // "none" | "horizontal" | "vertical" | "both" | "radial"
    ├── centre     // used by radial; movable, defaults to board centre
    ├── axis       // used by horizontal/vertical
    └── interval   // used by radial, degrees, accepts custom values
```

Recalculation flow:

```text
Edit source
    ↓
Regenerate source pins
    ↓
Regenerate mirrored instances
```

## Interaction Rules

- Symmetry mode/parameters are set before or during drawing; they apply to the Pin Path being created.
- Editing an existing Pin Path that has symmetry attached re-triggers the same regeneration flow.
- Mirrored instances are not independently selectable/editable as separate Pin Paths — selecting a mirrored copy selects (or navigates to) the source, per [09-selection-and-editing.md](./09-selection-and-editing.md).
- Moving the radial symmetry centre recalculates all radial copies immediately (live).
- Deleting the source Pin Path deletes its mirrored copies as part of the same undoable operation.

## Edge Cases

- Radial interval of 0° or a value that doesn't evenly divide 360° must be handled deterministically (e.g. non-dividing custom angles still place copies at `n × interval` until exceeding 360°, without wrapping duplicates onto the source angle).
- Moving the radial centre onto a pin/grid position should still go through normal snapping (see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md)).
- Symmetry + closed-path spacing algorithm ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md)) must be applied to the source only, then the resulting exact pin pattern is reflected/rotated — spacing is never recalculated independently per mirrored copy (that could produce visually inconsistent spacing between source and copies).

## Test Cases

```gherkin
Feature: Horizontal and vertical symmetry

  Scenario: Vertical symmetry generates one reflected copy
    Given symmetry type is "vertical" with a vertical axis through the board centre
    When the user draws a Pin Path entirely on the left side of the axis
    Then exactly one mirrored copy appears on the right side
    And the mirrored copy's pins are reflections of the source pins across the axis

  Scenario: Combined horizontal + vertical symmetry generates three copies
    Given symmetry type is "both"
    When the user draws a Pin Path in one quadrant
    Then three additional mirrored copies appear, one per remaining quadrant
    And all four instances (source + 3 copies) are positioned symmetrically about both axes

  Scenario: Editing the source updates mirrored copies
    Given a Pin Path with vertical symmetry and one mirrored copy already generated
    When the user resizes the source geometry
    Then the mirrored copy's geometry and pins are regenerated to match the new source
    And the mirrored copy remains a correct reflection of the updated source

Feature: Radial symmetry

  Scenario: Preset angular interval generates correct copy count
    Given symmetry type is "radial" with interval 45°
    When the user draws a source Pin Path
    Then 7 additional copies are generated (8 total instances around the centre)
    And each copy is rotated by a multiple of 45° from the source

  Scenario: Custom angular interval is accepted
    Given symmetry type is "radial"
    When the user enters a custom interval of 37°
    Then copies are generated at 37°, 74°, 111°, ... up to but not exceeding 360°
    And no copy duplicates the source angle

  Scenario: Radial centre defaults to board centre
    Given a new Pin Path is created with radial symmetry and no centre has been set yet
    Then the radial symmetry centre equals the board's geometric centre

  Scenario: Moving the radial centre recalculates all copies live
    Given a radial Pin Path with interval 60° and 5 generated copies
    When the user drags the radial centre ◎ to a new position
    Then all 5 copies' positions are recalculated around the new centre in real time
    And the source geometry itself is unchanged (only its rotated placement around the new centre)

  Scenario: Radial centre snaps like any other point
    Given Snap to grid = ON
    When the user drags the radial centre near a grid intersection
    Then the centre snaps to that intersection

Feature: Mirrored copies are derived, not independent

  Scenario: Selecting a mirrored copy does not allow independent editing
    Given a Pin Path with horizontal symmetry and one mirrored copy
    When the user attempts to select the mirrored copy directly
    Then the source Pin Path is selected (or the copy is shown as non-editable/linked), not an independently editable geometry

  Scenario: Deleting the source removes all mirrored copies in one action
    Given a Pin Path with radial symmetry and 3 generated copies
    When the user deletes the source Pin Path
    Then the source and all 3 copies are removed
    And a single Undo restores the source and all 3 copies together

Feature: Symmetry does not distort closed-path spacing

  Scenario: Spacing algorithm runs once, then mirrors the result
    Given a closed octagon Pin Path with requested spacing 2 cm and vertical symmetry
    When pins are distributed
    Then the source octagon's actual spacing is computed once via the closed-path algorithm
    And the mirrored copy's pins are exact reflections of the source pins (not independently recalculated)
    And source and mirrored copy show identical actual spacing values
```
