# 08 — Pin Tools & Properties

## Purpose

Define the drawing tools available in Pin mode, pin/guide properties, the shape constraint modifier, the Arc drawing interaction, and pin numbering. Distribution math lives in [07-pin-geometry-engine.md](./07-pin-geometry-engine.md); editing existing Pin Paths lives in [09-selection-and-editing.md](./09-selection-and-editing.md).

## Pin Drawing Tools

**Basic tools:** Line, Arc, Ellipse, Circle, Rectangle, Square, Freehand, Text, Path (click-per-vertex free-form polygon — see [33-pin-path-tool.md](./33-pin-path-tool.md))

**Polygon / Star dropdown** — one toolbar control (`Polygon / Star ▼`) instead of many separate icons:

- Regular Polygons: Pentagon, Hexagon, Octagon
- Conventional Stars: 5-point star, 6-point star, 8-point star
- Polygrams: Pentagram, Heptagram, Octagram

**Other pin tools:** Eraser and Path Eraser (see [11-erasers.md](./11-erasers.md)), Snap to nearest pin.

## Pin Path Composition

Every pin object contains:

```text
Pin Path
│
├── Guide Geometry
└── Generated Pins[]
```

The guide is the mathematical path used to generate the pins.

## Guide Lines

Default appearance:

```text
Colour:  Black
Opacity: 80%
```

Guide visibility is **independent** from pin visibility:

```text
Pins:  visible      Pins:  visible
Guide: hidden       Guide: visible
```

Both combinations are valid. Print visibility of guides is independently configurable from editor visibility (see [14-printing.md](./14-printing.md)).

## Pin Properties

- Requested distance between pins
- Pin colour
- Pin diameter
- Guide visibility

```text
Spacing:  1 cm
Diameter: 2 mm
Colour:   Black
```

### Two Editing Contexts

- **No Pin Path selected** — changes affect the defaults applied to newly created objects.
- **Pin Path selected** — changes affect only the selected object (see [09-selection-and-editing.md](./09-selection-and-editing.md) for the full editable-property list per shape).

### Tool Switching

Picking any Pin tool (a shape/Freehand/Path/Text draw tool, Eraser, or Path Eraser) clears a live Pin Path/Pins selection — the tool is about to act on the canvas, not continue editing the selected object (see [09-selection-and-editing.md](./09-selection-and-editing.md)). Picking a shape/Freehand/Path/Text draw tool additionally resets the pin properties above (Spacing, Colour, Diameter, Guide visibility) and symmetry ([06-symmetry.md](./06-symmetry.md)) back to their defaults — Eraser and Path Eraser leave the defaults untouched, since they aren't shape tools.

Switching into Pin mode itself — e.g. clicking the Pin tab — also resets the pin properties and symmetry back to their defaults, unconditionally (regardless of which Pin tool was last active). This keeps "land on the Pin tab" behaviour consistent whether the user gets there by clicking the tab or by picking a draw tool while already in it.

Committing a new Pin Path keeps the editor in Pin mode with the same tool and pin properties active, so the user can keep drawing. The one exception is the Text tool, which hands off to Edit mode with the new path selected so its Text field is ready to type into ([29-text-pin-path.md](./29-text-pin-path.md)).

## Shape Constraint Modifier

`Alt` is the constraint modifier:

```text
Alt + Ellipse   → Circle
Alt + Rectangle → Square
```

Applied after snapping, per the snapping priority pipeline in [05-canvas-and-viewport.md](./05-canvas-and-viewport.md).

Note: this constraint still drags corner-to-corner (equal width/height forced after the fact) — it is independent of the standalone Circle tool below, which drags centre-to-radius (see Drag Gesture Conventions).

## Drag Gesture Conventions

Every Pin drawing tool except Arc (its own 3-stage interaction, below) uses one press-drag-release gesture: mousedown starts the shape, mousemove previews it live, mouseup commits it. Which point mousedown represents differs by tool:

- **Bounding-box drag** (mousedown/mouseup are opposite corners): Ellipse, Rectangle, Square (and their Alt-constrained forms).
- **Centre/start-radius drag** (mousedown is the centre or start point, mouseup determines size/direction): Line (mousedown = start, mouseup = end), Circle, and the whole Polygon/Star dropdown family (Pentagon/Hexagon/Octagon, Star-5/6/8, Pentagram/Heptagram/Octagram).

## Arc Drawing Interaction

The Arc tool uses **Start point + End point + Curvature**:

1. User clicks the arc start point.
2. User clicks the arc end point.
3. Moving the pointer adjusts the curvature.
4. A third click confirms the arc.

```text
       cursor / curvature
              ●

        ╭──────────╮
Start ●              ● End
```

Before the third click, the arc preview updates continuously, and the pin placement preview (per [07-pin-geometry-engine.md](./07-pin-geometry-engine.md)) also updates continuously as curvature changes — the user sees the resulting pin count/spacing live, not just the arc shape.

## Pin Numbering

```text
Show pin numbers:  Yes / No
Print pin numbers: Yes / No
```

Displayed numbering is independent from stable internal Pin IDs ([02-document-model.md](./02-document-model.md)) — renumbering display order (if ever supported) must never change a pin's underlying ID or break thread references.

## Test Cases

```gherkin
Feature: Pin drawing tools

  Scenario: Polygon/Star dropdown exposes all sub-types under one control
    Given the Pin toolbar
    When the user opens the "Polygon / Star" dropdown
    Then Pentagon, Hexagon, Octagon, 5-point star, 6-point star, 8-point star, Pentagram, Heptagram, and Octagram are all available
    And no separate top-level toolbar icon exists for each of these individually

  Scenario: Circle tool drags from centre outward
    Given the editor is in PIN mode
    When the user selects the "Circle" tool, presses at a point, drags, and releases
    Then a Circle Pin Path is created whose centre is the press point and whose radius is the distance to the release point
    And the editor stays in PIN mode with the Circle tool still active

  Scenario: Line tool uses a single press-drag-release gesture
    Given the editor is in PIN mode
    When the user selects the "Line" tool, presses at a point, drags, and releases
    Then a Line Pin Path is created from the press point to the release point
    And no intermediate click is required to commit the second point

Feature: Guide and pin visibility independence

  Scenario: Pins visible, guide hidden
    Given a Pin Path with Pins = visible, Guide = hidden
    Then the pins render on canvas
    And the guide line does not render

  Scenario: Pins visible, guide visible
    Given a Pin Path with Pins = visible, Guide = visible
    Then both the pins and the guide line render

  Scenario: Guide default appearance
    Given a newly created Pin Path with no custom guide style
    Then the guide colour is black and opacity is 80%

Feature: Pin property editing contexts

  Scenario: Editing properties with no Pin Path selected changes future defaults
    Given no Pin Path is selected
    When the user sets Spacing = 1.5 cm
    And then draws a new Circle
    Then the new Circle uses 1.5 cm as its requested spacing

  Scenario: Editing properties with a Pin Path selected changes only that object
    Given Pin Path "Outer Circle" is selected with Spacing = 1 cm
    And another Pin Path "Inner Star" exists with Spacing = 1 cm
    When the user changes Spacing to 2 cm while "Outer Circle" is selected
    Then "Outer Circle" now has Spacing = 2 cm
    And "Inner Star" still has Spacing = 1 cm

Feature: Shape constraint modifier

  Scenario: Alt constrains Ellipse to Circle
    Given the Ellipse tool is active
    When the user holds Alt while dragging
    Then the resulting shape is a Circle (equal width and height)

  Scenario: Alt constrains Rectangle to Square
    Given the Rectangle tool is active
    When the user holds Alt while dragging
    Then the resulting shape is a Square (equal width and height)

  Scenario: Releasing Alt mid-drag reverts to unconstrained shape
    Given the user is dragging a Rectangle with Alt held (currently constrained to Square)
    When the user releases Alt before finishing the drag
    Then the shape becomes an unconstrained Rectangle again

Feature: Arc drawing interaction

  Scenario: Arc requires three clicks to confirm
    Given the Arc tool is active
    When the user clicks a start point
    And clicks an end point
    And moves the pointer (curvature preview updates)
    And clicks a third time
    Then an Arc Pin Path is created with the confirmed start, end, and curvature

  Scenario: Arc preview and pin preview update continuously before confirmation
    Given the user has placed the arc start and end points
    When the user moves the pointer to change curvature
    Then the arc preview shape updates in real time
    And the previewed pin count/positions update in real time to match the current curvature

  Scenario: Cancelling before third click leaves no Pin Path
    Given the user has placed the arc start and end points
    When the user switches editor mode before the third click
    Then no Arc Pin Path is added to the document

Feature: Pin numbering

  Scenario: Toggling display numbering does not affect print numbering
    Given Show pin numbers = OFF and Print pin numbers = ON
    Then pin numbers are not shown in the editor canvas
    And pin numbers will appear in the print output

  Scenario: Displayed numbers are independent of internal Pin IDs
    Given a Pin Path with pins internally IDed pin-10, pin-11, pin-12
    When pin numbers are displayed starting at 1
    Then pin-10 may display as "1" without its internal ID changing
    And any Thread Path referencing "pin-10" remains valid regardless of displayed number
```
