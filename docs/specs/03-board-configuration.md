# 03 — Board Configuration

## Purpose

Define board shape selection and real-world dimension input, configured before the user enters the drawing editor.

## Functional Requirements

### Supported Board Shapes

- Circle
- Oval
- Rectangle
- Square
- Triangle (Equilateral or Right-angled — sub-selection required)

### Dimensions per Shape

| Shape | Dimensions |
|---|---|
| Circle | Diameter |
| Oval | Width + Height |
| Rectangle | Width + Height |
| Square | Side |
| Equilateral Triangle | Side |
| Right-Angled Triangle | Base + Height |

### Triangle Sub-selection

When **Triangle** is selected, the user chooses:

```text
Triangle Type

○ Equilateral
○ Right-angled
```

**Equilateral Triangle**: single `Side` input (e.g. `Side: 50 cm`); all three sides equal.

**Right-Angled Triangle**: `Base` and `Height` inputs (e.g. `Base: 40 cm`, `Height: 30 cm`); the hypotenuse is calculated automatically as `√(base² + height²)`.

## Data Model

```text
board.shape: "circle" | "oval" | "rectangle" | "square" | "triangle"
board.triangleType: "equilateral" | "right-angled"   // only when shape === "triangle"
board.dimensions: {
  diameter?, width?, height?, side?, base?
}
```

All dimension values are stored in physical units (see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) for the physical coordinate system), never pixels.

## Interaction Rules

- Changing `board.shape` resets/hides irrelevant dimension fields.
- Changing any dimension recalculates the board's geometric path immediately (used downstream by pin distribution, printing, etc.) and updates any derived value shown to the user (e.g. hypotenuse).
- Board shape/dimension changes are undoable (see [10-undo-redo.md](./10-undo-redo.md)).

## Edge Cases

- Zero or negative dimension input must be rejected/clamped before being applied to the board geometry.
- Switching from Right-angled to Equilateral (or vice versa) must not silently carry over an invalid dimension (e.g. a stale `base` value must not leak into equilateral's `side`).

## Test Cases

```gherkin
Feature: Board shape and dimensions

  Scenario Outline: Selecting a shape exposes only its relevant dimension fields
    Given the user is on the Board Configuration screen
    When the user selects shape "<shape>"
    Then only the fields "<fields>" are shown

    Examples:
      | shape     | fields          |
      | circle    | diameter        |
      | oval      | width, height   |
      | rectangle | width, height   |
      | square    | side            |

  Scenario: Selecting Triangle requires a sub-type choice
    Given the user is on the Board Configuration screen
    When the user selects shape "triangle"
    Then the user must choose triangle type "equilateral" or "right-angled"
    And no dimension fields are shown until a triangle type is chosen

  Scenario: Equilateral triangle exposes a single side field
    Given the user selected shape "triangle" and type "equilateral"
    When the user enters Side = 50 cm
    Then all three sides of the board geometry equal 50 cm

  Scenario: Right-angled triangle computes hypotenuse automatically
    Given the user selected shape "triangle" and type "right-angled"
    When the user enters Base = 40 cm and Height = 30 cm
    Then the displayed hypotenuse equals 50 cm

  Scenario: Switching triangle type clears the other type's fields
    Given the user configured a right-angled triangle with Base = 40 cm, Height = 30 cm
    When the user switches triangle type to "equilateral"
    Then the Base and Height fields are no longer shown
    And the Side field starts empty or at a sane default, not at a stale Base/Height-derived value

  Scenario: Non-positive dimension input is rejected
    Given the user is configuring a Circle
    When the user enters Diameter = 0
    Then the input is rejected or clamped to the minimum valid value
    And the board geometry is not updated to an invalid diameter

  Scenario: Changing a dimension recalculates the board path
    Given a Rectangle board with Width = 60 cm, Height = 40 cm
    When the user changes Width to 80 cm
    Then the board's geometric path perimeter and bounding box update immediately
    And any Pin Paths anchored relative to the board (if applicable) reflect the new board size

  Scenario: Board dimension change is undoable
    Given a Circle board with Diameter = 30 cm
    When the user changes Diameter to 50 cm
    And then invokes Undo
    Then the board Diameter reverts to 30 cm
```
