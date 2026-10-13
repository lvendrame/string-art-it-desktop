# 04 — Board Appearance

## Purpose

Define how the board's visual surface is configured, independent of its shape/dimensions ([03-board-configuration.md](./03-board-configuration.md)) and independent of pins, threads, guides, and grid.

## Functional Requirements

Supported board appearance types:

- Solid colour
- Linear gradient
- Radial gradient
- Wood texture
- Painted wood
- Custom uploaded image/texture

### Solid Colour
User-selectable custom colour.

### Linear Gradient
Configuration:
- Colour stops
- Direction

### Radial Gradient
Configuration:
- Colour stops
- Gradient centre

### Wood Texture
Predefined wood finishes (fixed set, no upload).

### Painted Wood
Painted-board appearances that retain some physical surface texture (i.e. not a flat solid colour, but a colour rendered over a wood-grain texture).

### Custom Texture
User uploads an image or texture file, used as the board fill.

## Independence Rule

Board appearance must remain fully independent from:

- Pins
- Threads
- Guides
- Grid

Changing appearance never affects pin geometry, thread geometry, guide visibility/style, or grid settings, and vice versa.

## Data Model

```text
board.appearance: {
  type: "solid" | "linear-gradient" | "radial-gradient" | "wood-texture" | "painted-wood" | "custom-texture",
  // type === "solid"
  colour?,
  // type === "linear-gradient"
  stops?: [{ offset, colour }],
  direction?,
  // type === "radial-gradient"
  stops?: [{ offset, colour }],
  centre?: { x, y },
  // type === "wood-texture" | "painted-wood"
  presetId?,
  // type === "custom-texture"
  imageAssetId?,
}
```

## Test Cases

```gherkin
Feature: Board appearance configuration

  Scenario: Setting a solid colour
    Given the board appearance type is "solid"
    When the user picks colour #3A2A1E
    Then the board renders filled with #3A2A1E
    And no gradient or texture is applied

  Scenario: Configuring a linear gradient
    Given the board appearance type is "linear-gradient"
    When the user adds colour stops [red@0%, blue@100%] and sets direction to 45°
    Then the board renders a linear gradient from red to blue at 45°

  Scenario: Configuring a radial gradient
    Given the board appearance type is "radial-gradient"
    When the user adds colour stops [white@0%, grey@100%] and moves the gradient centre off the board centre
    Then the board renders a radial gradient centred at the specified point

  Scenario: Selecting a wood texture preset
    Given the board appearance type is "wood-texture"
    When the user selects the "Oak" preset
    Then the board renders the Oak texture fill
    And no custom image upload is required

  Scenario: Uploading a custom texture
    Given the board appearance type is "custom-texture"
    When the user uploads an image file
    Then the board renders filled with that image
    And the image is stored/referenced so it persists on save/reload

  Scenario: Board appearance changes do not affect pins, threads, guides, or grid
    Given a board with visible pins, visible guides, a visible grid, and an existing thread
    When the user changes board appearance from "solid" to "wood-texture"
    Then pin positions, guide visibility, grid settings, and thread geometry are all unchanged

  Scenario: Pin/thread/guide/grid changes do not affect board appearance
    Given a board with appearance type "radial-gradient"
    When the user hides all guides and toggles grid visibility off
    Then the board appearance configuration (type, stops, centre) is unchanged
```
