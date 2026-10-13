# 00 — Overview & Scope

## Purpose

Define what StringArtIt is, the core mental model behind the editor, and the boundary between MVP, Phase 2, and future/out-of-scope work. This document is the entry point for all other specs in `docs/specs/`.

## Product Overview

StringArtIt is a browser-based design tool for creating physical string-art/string-line boards. The user defines a board, places virtual pins/nails along geometric paths, and creates thread paths by connecting those pins.

The editor is built around three independent concepts:

1. **Board / Background**
2. **Pins / Nails**
3. **Threads / Lines**

Pins and threads are independent object types with separate layer systems (see [13-layers.md](./13-layers.md)).

StringArtIt must behave as a **structured vector/geometric editor**, not a bitmap painting application. The fundamental model:

```text
Shape
  ↓
Geometric Path
  ↓
Pins
  ↓
Thread Connections
```

Meaningful geometry must remain editable at every stage — a circle stays a circle, an octagon stays an octagon, a thread path stays a path of pin references. See the [Product Principle](#product-principle) below.

## Main Product Goals

The application must allow users to:

- Define a physical board and configure its real-world dimensions and appearance.
- Create pin arrangements geometrically, with precise control over pin spacing.
- Snap geometry to grid intersections and to existing pins.
- Create symmetrical pin arrangements (mirror and radial).
- Draw threads exclusively between pins.
- Create single-colour or multi-colour twisted threads.
- Organize pins and threads into separate layer systems, with hide/lock support.
- Select and modify existing objects rather than redrawing them.
- Undo and redo changes.
- Print physical construction templates.
- Export designs.
- Save and reload projects.

## Related Specs

| Area | Spec |
|---|---|
| Technical architecture | [01-architecture.md](./01-architecture.md) |
| Domain model | [02-document-model.md](./02-document-model.md) |
| Board setup | [03-board-configuration.md](./03-board-configuration.md), [04-board-appearance.md](./04-board-appearance.md) |
| Canvas, grid, viewport | [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) |
| Symmetry | [06-symmetry.md](./06-symmetry.md) |
| Pin geometry & distribution algorithms | [07-pin-geometry-engine.md](./07-pin-geometry-engine.md) |
| Pin drawing tools & properties | [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) |
| Selection & object editing | [09-selection-and-editing.md](./09-selection-and-editing.md) |
| Undo/redo | [10-undo-redo.md](./10-undo-redo.md) |
| Erasers & cascading deletes | [11-erasers.md](./11-erasers.md) |
| Thread editor | [12-thread-editor.md](./12-thread-editor.md) |
| Layers | [13-layers.md](./13-layers.md) |
| Printing | [14-printing.md](./14-printing.md) |
| Export | [15-export.md](./15-export.md) |
| Persistence | [16-persistence.md](./16-persistence.md) |
| Statistics | [17-statistics.md](./17-statistics.md) |
| Design system | [18-design-system.md](./18-design-system.md) |
| Play mode (thread-drawing animation replay + video export) | [19-play-mode.md](./19-play-mode.md) |
| In-app help | [20-help.md](./20-help.md) |
| Scale tool & pin distance | [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md) |
| Thread draft pattern-follow (Right Arrow) | [22-thread-follow-pattern.md](./22-thread-follow-pattern.md) |
| Keyboard Move/Rotate/Scale | [23-keyboard-transform.md](./23-keyboard-transform.md) |
| Internationalization (i18n) | [24-internationalization.md](./24-internationalization.md) |
| Radial context menu | [25-radial-context-menu.md](./25-radial-context-menu.md) |
| Edit-mode multi-select & granularity | [26-edit-mode-multi-select.md](./26-edit-mode-multi-select.md) |
| Thread select tool | [27-thread-select-tool.md](./27-thread-select-tool.md) |
| Help: About tab (app info + mailto contact form) | [28-help-about-tab.md](./28-help-about-tab.md) |
| Text pin path | [29-text-pin-path.md](./29-text-pin-path.md) |
| SEO (meta, structured data, landing content) | [30-seo.md](./30-seo.md) |
| WebMCP agent tools | [31-webmcp-agent-tools.md](./31-webmcp-agent-tools.md) |
| Generator mode (auto pin+thread pattern generation) | [32-generator-mode.md](./32-generator-mode.md) |
| Pin Path tool (click-per-vertex free-form polygon) | [33-pin-path-tool.md](./33-pin-path-tool.md) |
| Keyboard shortcuts | [34-keyboard-shortcuts.md](./34-keyboard-shortcuts.md) |
| Zig-Zag and Parabolic thread tools | [35-zigzag-parabolic-tools.md](./35-zigzag-parabolic-tools.md) |
| Layer merge (merge a layer into the layer above) | [36-layer-merge.md](./36-layer-merge.md) |
| Radial thread tool | [37-radial-thread-tool.md](./37-radial-thread-tool.md) |
| Repeat pattern thread tool | [38-repeat-pattern-tool.md](./38-repeat-pattern-tool.md) |
| Copy / paste | [39-copy-paste.md](./39-copy-paste.md) |

## MVP Scope

### Board
- Circle, Oval, Rectangle, Square, Equilateral triangle, Right triangle
- Physical dimensions
- Solid colour, gradients, basic textures

### Canvas
- Physical units, grid, grid snapping, pin snapping, snap radius, zoom, pan

### Pins
- Line, Arc, Ellipse, Circle, Rectangle, Square
- Pentagon, Hexagon, Octagon
- 5-star, 6-star, 8-star
- Pentagram, Heptagram, Octagram
- Pin spacing, closed-path uniform spacing, guides
- Selection, editing, eraser

### Symmetry
- Horizontal, Vertical, Both, Radial
- Live drawing, linked mirrored geometry, movable centre

### Threads
- Nearest-pin detection, snap radius, highlight, preview
- Continuous Thread Paths
- One-colour, two-colour spiral, three-colour spiral thread
- Eraser, thread termination

### Layers
- Separate Pin and Thread lists
- Create, rename, delete, duplicate, reorder, visibility, locking

### Editor
- Undo, redo, select, pan

### Output
- Print preview: pins, guides, threads, pin numbers, grid
- 1:1 printing, fit to page, custom scale

### Persistence
- Save, open, native project format

## Phase 2 Scope

- Rich board textures, uploaded board textures
- Print calibration
- Tiled printing
- SVG, PDF, PNG/JPEG export
- Autosave
- Statistics
- ~~Advanced transformations~~ — see [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md)
- Configurable spiral twist density

## Future Features (Out of Scope for Core)

- Automatic string-art generation from images
- Pattern generators
- Generated pin sequences
- Assembly instructions
- Thread colour optimisation
- Thread quantity estimation
- Pin quantity/material estimation
- Touch/tablet support, stylus support
- Template library
- Shared projects, community gallery

## Recommended Development Sequence

```text
Phase A — Geometry Engine
  1. Coordinate system
  2. Paths
  3. Length calculation
  4. Point-at-distance
  5. Open pin distribution
  6. Closed pin distribution
  7. Polygon/star geometry
  8. Arc geometry
  9. Transformations
  10. Snapping
  11. Snap radius
  12. Symmetry
  13. Automated tests

Phase B — Board & Viewport
  Board creation, board appearance, grid, zoom, pan, physical coordinate conversion

Phase C — Pin Editor
  Pin Paths, drawing tools, live preview, guide lines, pin properties, selection, editing, eraser

Phase D — Symmetry
  Horizontal, Vertical, Both, Radial, linked instances, movable centre

Phase E — Thread Editor
  Spatial pin lookup, candidate highlighting, preview, Thread Path creation,
  thread termination, spiral rendering, eraser

Phase F — Layers
  Pin layers, thread layers, visibility, locking, ordering, duplication

Phase G — Commands
  All mutations go through an undoable command architecture (Execute / Undo / Redo)

Phase H — Persistence
  Project schema, save, load, versioning

Phase I — Print & Export
  Print preview, physical scaling, calibration, tiling, SVG, PDF, raster export
```

This sequence should drive implementation order: the geometry engine (Phase A) has no UI dependency and should be fully unit-tested before any canvas rendering is built on top of it.

## Remaining Minor Product Decisions

These are lower-level UX/configuration decisions that do not change the architecture and can be decided during detailed UI/UX design:

- Default nearest-pin snap radius in pixels
- Default pin diameter
- Default pin spacing
- Default grid size
- Default guide-line width
- Whether snap radius is user-configurable or a fixed application default
- Default radial symmetry centre indicator appearance
- Exact UI for rotating polygons/stars during creation
- Default spiral pitch for two/three-colour threads
- Default print overlap for tiled pages

## Product Principle

> **Never reduce meaningful geometry to pixels.**

The application should understand:

```text
This is an arc.
This is an octagon.
These pins belong to that path.
This thread connects these pins.
These shapes are generated by this symmetry rule.
```

rather than storing only:

```text
pixels
coordinates
unrelated line segments
```

This principle enables: accurate geometry, object editing, uniform pin spacing, snapping, symmetry, layering, undo/redo, physical printing, material calculations, and future automated pattern generation. Every spec in this folder must be read with this principle as the tie-breaker when a design decision is ambiguous.
