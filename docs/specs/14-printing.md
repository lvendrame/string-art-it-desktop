# 14 — Printing

## Purpose

Define print element selection, scale, paper configuration, tiled printing, and print calibration. Print visibility is always independent from editor visibility.

## Print Elements

Independently selectable checklist:

```text
PRINT ELEMENTS

[x] Board outline
[x] Background
[x] Pins
[x] Pin guide lines
[ ] Pin numbers
[ ] Threads
[ ] Grid
```

Supported combinations include: pins only; pins + guides; pins + guides + numbers; pins + guides + grid; threads only; complete project — any combination of the checklist is valid.

**Print visibility does not change editor visibility**, and vice versa: hiding guides in the editor does not uncheck "Pin guide lines" for print, and unchecking it for print does not hide guides in the editor.

## Print Scale

```text
○ Actual size / 1:1
○ Fit to page
○ Custom scale
```

- **1:1** — one centimetre in the design prints as one physical centimetre, subject to printer calibration (see below).
- **Custom** — e.g. `1:2`, `1:5`, `50%`, `125%`.

## Paper Configuration

- Paper: A4, A3, Letter, Custom
- Orientation: Portrait, Landscape

## Tiled Printing

Large designs may span multiple pages:

```text
┌─────┬─────┬─────┐
│ A1  │ A2  │ A3  │
├─────┼─────┼─────┤
│ B1  │ B2  │ B3  │
├─────┼─────┼─────┤
│ C1  │ C2  │ C3  │
└─────┴─────┴─────┘
```

Options: overlap, trim marks, alignment marks, page numbers, page coordinates.

## Print Calibration

Print a known measurement:

```text
|────────────── 10 cm ──────────────|
```

The user measures the actual printed result; the application derives and stores a correction factor applied to future 1:1 prints from that printer/paper combination.

## Data Model

```text
printSettings: {
  elements: { boardOutline, background, pins, pinGuides, pinNumbers, threads, grid }: boolean each,
  scale: { mode: "1:1" | "fit" | "custom", customRatio? },
  paper: { size: "A4" | "A3" | "Letter" | "custom", customDimensions?, orientation: "portrait" | "landscape" },
  tiling: { enabled, overlap, trimMarks, alignmentMarks, pageNumbers, pageCoordinates },
  calibration: { correctionFactor }
}
```

## Test Cases

```gherkin
Feature: Print element selection

  Scenario Outline: Valid print element combination
    Given the print elements checklist is set to "<combination>"
    When the print preview is generated
    Then only the checked elements appear in the preview

    Examples:
      | combination                              |
      | pins only                                |
      | pins + guides                            |
      | pins + guides + numbers                  |
      | pins + guides + grid                     |
      | threads only                              |
      | board outline + background + pins + guides + numbers + threads + grid |

  Scenario: Print visibility is independent from editor visibility
    Given guides are hidden in the editor
    And "Pin guide lines" is checked in print settings
    When the print preview is generated
    Then guide lines appear in the print preview
    And guides remain hidden in the editor canvas

  Scenario: Unchecking a print element does not affect the editor
    Given "Threads" is unchecked in print settings
    And threads are visible in the editor
    When the print preview is generated
    Then threads do not appear in the print preview
    And threads remain visible in the editor canvas

Feature: Print scale

  Scenario: 1:1 scale prints actual physical size
    Given print scale mode is "1:1"
    When a design element measuring 10 cm is printed
    Then it is rendered at 10 cm on the printed page, subject to printer calibration correction if configured

  Scenario: Fit to page scales the whole design into the paper size
    Given a board larger than the selected paper size
    And print scale mode is "fit"
    Then the entire board is scaled down to fit within one page's printable area

  Scenario: Custom scale applies the specified ratio
    Given print scale mode is "custom" with ratio "1:2"
    When a design element measuring 10 cm is printed
    Then it is rendered at 5 cm on the printed page

Feature: Paper configuration

  Scenario: Selecting paper size and orientation affects page layout
    Given paper size = "A4" and orientation = "landscape"
    Then the print preview page dimensions match A4 landscape orientation

  Scenario: Custom paper size accepts user dimensions
    Given paper size = "custom" with width 50 cm and height 70 cm
    Then the print preview page dimensions equal 50 cm × 70 cm

Feature: Tiled printing

  Scenario: Large design splits into multiple pages
    Given a board too large to fit one page at 1:1 scale
    And tiling is enabled
    Then the print output spans multiple pages arranged in a grid (e.g. A1, A2, A3 / B1, B2, B3 / ...)

  Scenario: Overlap is applied between adjacent tiles
    Given tiling is enabled with overlap = 1 cm
    Then adjacent page tiles share a 1 cm overlapping strip of content

  Scenario: Alignment marks and page coordinates appear on tiled pages
    Given tiling is enabled with alignment marks and page coordinates turned on
    Then each tile shows alignment marks and its row/column coordinate label

Feature: Print calibration

  Scenario: Calibration print shows a known measurement
    Given the user requests a calibration print
    Then the printed page shows a marked reference length labeled "10 cm"

  Scenario: Entering measured length derives a correction factor
    Given the user printed the calibration page and measured the reference as 9.8 cm instead of 10 cm
    When the user enters 9.8 cm as the measured value
    Then the application stores a correction factor of 10/9.8 (~1.0204)

  Scenario: Correction factor is applied to future 1:1 prints
    Given a stored correction factor of 1.02
    When the user prints a design element measuring 10 cm at 1:1 scale
    Then the print output is scaled by the correction factor so the physical printed result is closer to 10 cm on that printer
```

## Desktop notes

- Output is a vector multi-page PDF built with krilla (`sa-infra/src/print.rs`) instead of `window.print()` + CSS `@page`. "Print…" sends it to a chosen system printer via the `printers` crate (CUPS / winspool); with no printer, or via "Open PDF", it opens in the OS viewer, whose own dialog prints.
- Tiles start at the content's top-left (the web started them at the page origin, wasting the first row/column).
- The calibration factor is persisted in the desktop prefs file instead of `localStorage`.
