# 01 — Architecture

## Purpose

Define the technical stack and architectural principles for StringArtIt, and the layer boundaries that keep geometry, document state, rendering, and I/O independent of each other. See [00-overview-and-scope.md](./00-overview-and-scope.md) for the product principle this architecture exists to protect: *never reduce meaningful geometry to pixels.*

## Technology Stack

| Concern | Choice |
|---|---|
| UI framework | React 19 |
| Build tool / dev server | Vite |
| Runtime | Node 22 LTS (build/tooling only — the shipped app is a static browser bundle) |
| Architecture style | Clean Architecture |
| Design principle | Single Responsibility Principle (SRP), applied at module, class/function, and component level |

## Clean Architecture Layering

Dependencies point inward only. Outer layers may import inner layers; inner layers must never import outer layers (no geometry code importing React, no domain code importing a renderer).

```text
┌─────────────────────────────────────────────────────────┐
│ Infrastructure                                           │
│  - Persistence adapters (localStorage/file/IndexedDB)    │
│  - Print/export adapters (SVG/PDF/PNG serializers)       │
│  - Canvas/SVG rendering adapters                         │
├─────────────────────────────────────────────────────────┤
│ Interface / UI (React)                                   │
│  - Components, hooks, toolbars, panels, canvas view      │
│  - Translates user input into Application-layer commands │
├─────────────────────────────────────────────────────────┤
│ Application                                              │
│  - Document Engine: layers, paths, selection, commands   │
│  - Command/Undo-Redo orchestration                       │
│  - Use-case-level services (e.g. "create pin path",      │
│    "connect thread", "distribute pins")                  │
├─────────────────────────────────────────────────────────┤
│ Domain / Geometry Engine (innermost, framework-agnostic) │
│  - Shapes, paths, transforms, snapping, symmetry math    │
│  - Pin distribution algorithms                           │
│  - Pure functions/classes, no DOM, no React, no I/O      │
└─────────────────────────────────────────────────────────┘
```

This mirrors the split already implied by the product spec:

```text
                 Project
                    │
         ┌──────────┼──────────┐
         ↓          ↓          ↓
      Editor      Print      Export
      Renderer    Renderer    Renderer
```

Rendering must not define document geometry. Three renderers (editor, print, export) consume the same Document Engine state and Geometry Engine outputs; none of them owns geometry truth.

## Geometry Engine Responsibilities

Framework-agnostic, unit-tested in isolation from React/DOM:

- Shape creation
- Path length
- Point-at-distance
- Pin generation (open and closed distribution — see [07-pin-geometry-engine.md](./07-pin-geometry-engine.md))
- Closed-path spacing optimization
- Bounding boxes
- Rotation / transformation
- Grid snapping, pin snapping, snap radius
- Mirror transformation, radial transformation
- Hit testing
- Nearest-pin search

## Document Engine Responsibilities

Owns document state and mutation history, independent of rendering:

- Board
- Layers (pin layers, thread layers)
- Pin Paths, Pins
- Thread Paths
- Selection
- Pin/thread relationships (cascading delete rules)
- Commands (undo/redo)
- Save/load
- Version migration

## Command Architecture (Undo/Redo)

All mutations to the document must go through a command object rather than direct state mutation, so that every user-visible change is undoable/redoable and cascading effects (e.g. deleting a pin that removes connected thread segments) are captured atomically. See [10-undo-redo.md](./10-undo-redo.md) and [11-erasers.md](./11-erasers.md).

```text
interface Command {
  execute(): void
  undo(): void
  redo(): void   // typically re-invokes execute(), but kept explicit
}
```

Commands are the only path for: create/delete/move/resize/rotate pin paths, spacing/colour/diameter/guide changes, thread add/delete, cascading thread deletions, layer operations, symmetry changes.

## SRP Guidance

- A geometry function computes geometry; it does not also decide how it's drawn.
- A React component renders and handles input; it does not implement distribution math or persistence logic inline.
- A persistence adapter serializes/deserializes; it does not validate business rules (that's the Document Engine's job).
- A command mutates one coherent unit of document state and knows how to reverse itself; it does not also drive UI feedback (that belongs to the layer above, reacting to state change).

## Suggested Folder Structure

```text
src/
  domain/                 # Geometry Engine — pure, no framework deps
    shapes/
    paths/
    transforms/
    snapping/
    symmetry/
  application/            # Document Engine + use cases
    document/
    commands/
    selection/
  infrastructure/
    persistence/
    export/
    print/
    rendering/            # canvas/SVG draw adapters consumed by UI
  ui/                      # React 19 components, hooks, toolbars, panels
    canvas/
    panels/
    toolbars/
```

## Test Cases

```gherkin
Feature: Layer boundary enforcement

  Scenario: Domain layer has no framework dependency
    Given a module under `src/domain/`
    When its imports are inspected
    Then it must not import from `react`, `react-dom`, or anything under `src/ui/`

  Scenario: Application layer does not import UI
    Given a module under `src/application/`
    When its imports are inspected
    Then it must not import from `src/ui/`

  Scenario: Rendering adapters do not mutate document state
    Given the Editor Renderer, Print Renderer, and Export Renderer
    When each renders the same Project snapshot
    Then none of them produce a different Project state as a side effect
    And all three renderers draw geometrically identical shapes for identical input

Feature: Command-based mutation

  Scenario: A mutation executed through a command is undoable
    Given a Command that changes pin spacing on a Pin Path
    When the command is executed
    Then the Pin Path's actualSpacing and pins[] reflect the new spacing
    When undo is invoked
    Then the Pin Path returns to its exact prior spacing and pins[]

  Scenario: Cascading effects are captured in one command
    Given a Pin referenced by two Thread segments
    When a "delete pin" command executes
    Then both the pin and the two thread segments are removed
    And a single undo reverses all three removals together
```
