# 15 — Export

## Purpose

Define supported export formats and the vector-preservation requirement.

## Functional Requirements

Supported roadmap:

- Native project (MVP — see [16-persistence.md](./16-persistence.md))
- SVG (Phase 2)
- PDF (Phase 2)
- PNG (Phase 2)
- JPEG (Phase 2)

SVG and PDF should preserve vector geometry wherever possible — an exported circle Pin Path must remain a true vector circle/path in the output file, not a rasterized approximation, consistent with the Geometry Engine's precision requirements ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md)).

## Interaction Rules

- Export uses the same underlying Project state and Geometry Engine outputs as the Editor and Print renderers (see [01-architecture.md](./01-architecture.md) rendering architecture) — export must not silently diverge from what's shown on canvas.
- Export respects the same "print elements" selectability concept where applicable (e.g. exporting "pins + guides only" as SVG), reusing the configuration model from [14-printing.md](./14-printing.md) rather than defining a separate one.
- Raster exports (PNG/JPEG) require a resolution/DPI setting since document coordinates are physical, not pixel-based (see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md)).

## Test Cases

```gherkin
Feature: Export formats

  Scenario: Native project export produces a re-importable file
    Given a Project with pins and threads
    When the user exports as native project format
    Then the resulting file can be opened via "Open" and reproduces the identical document state

  Scenario: SVG export preserves vector geometry
    Given a Pin Path that is a true Circle
    When the project is exported to SVG
    Then the SVG contains a vector circle element (or equivalent path command), not a rasterized image of a circle

  Scenario: PDF export preserves vector geometry
    Given a Pin Path that is an Octagon
    When the project is exported to PDF
    Then the PDF's content stream contains vector path drawing operators for the octagon, not an embedded raster image

  Scenario: PNG export requires a resolution setting
    Given the user chooses PNG export
    Then the export dialog requires a DPI or pixel-dimension setting before proceeding

  Scenario: JPEG export produces a raster image at the requested resolution
    Given the user exports at 300 DPI
    Then the resulting JPEG's pixel dimensions correspond to the document's physical size at 300 DPI

Feature: Export consistency with editor/print state

  Scenario: Exported geometry matches the editor's current document state
    Given a board with a specific set of Pin Paths and Thread Paths
    When the project is exported to SVG
    Then the exported geometry's coordinates and shapes match what is currently rendered in the editor

  Scenario: Exporting with a selected element subset omits unselected elements
    Given the export configuration selects "pins + guides only"
    When the export is generated
    Then threads, board background, and grid are not present in the exported file
```

## Desktop notes

- PDF is written directly with krilla as true vectors (gradients, clipped wood grain, images, glyph-outline pin labels); there is no SVG→PDF round trip.
- Two web bugs are fixed: threads attached to mirrored pins are exported, and multi-colour threads draw every strand.
- Files are written through the native save dialog; the DPI field lives in the Export popover (default 150).
