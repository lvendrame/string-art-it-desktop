# 02 — Document Model

## Purpose

Define the core document structure that every other feature spec builds on. This is the Document Engine's owned data shape (see [01-architecture.md](./01-architecture.md)).

## Core Document Model

```text
Project
│
├── Board
├── Global Settings
│
├── Pin Layers
│   └── Pin Layer
│       └── Pin Path
│           ├── Geometry
│           ├── Guide
│           └── Pins[]
│
└── Thread Layers
    └── Thread Layer
        └── Thread Path
            └── Segments[]
```

Two invariants drive this shape:

- A geometric pin drawing remains a **Pin Path** instead of decomposing into unrelated individual points.
- A continuous thread remains one **Thread Path** instead of decomposing into unrelated line segments.

## Suggested Domain Model

```text
Project
│
├── id
├── version
├── board
├── editorSettings
│
├── pinLayers[]
│   ├── id
│   ├── name
│   ├── visible
│   ├── locked
│   │
│   └── pinPaths[]
│       ├── id
│       ├── geometry
│       ├── transform
│       ├── guideStyle
│       ├── guideVisible
│       ├── pinStyle
│       ├── requestedSpacing
│       ├── actualSpacing
│       ├── symmetry
│       └── pins[]
│
└── threadLayers[]
    ├── id
    ├── name
    ├── visible
    ├── locked
    │
    └── threadPaths[]
        ├── id
        ├── colours[]
        ├── width
        ├── twist
        └── pinIds[]
```

Field-level detail for each nested type is defined in its owning spec:

| Entity | Owning spec |
|---|---|
| `board` | [03-board-configuration.md](./03-board-configuration.md), [04-board-appearance.md](./04-board-appearance.md) |
| `pinLayers[]` operations | [13-layers.md](./13-layers.md) |
| `pinPaths[].geometry`, distribution | [07-pin-geometry-engine.md](./07-pin-geometry-engine.md) |
| `pinPaths[].symmetry` | [06-symmetry.md](./06-symmetry.md) |
| `pinPaths[]` drawing tools/properties | [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) |
| `threadLayers[]` operations | [13-layers.md](./13-layers.md) |
| `threadPaths[]` | [12-thread-editor.md](./12-thread-editor.md) |

## Stable Pin IDs

Every generated pin has a stable, unique ID that persists across re-distribution (e.g. spacing change) as long as the pin still exists at a corresponding position. Threads reference pins by **Pin ID**, never by copied screen or document coordinates:

```text
ThreadPath.pinIds = ["pin-10", "pin-14", "pin-32", "pin-7"]
```

This decouples thread topology from pin geometry: moving/resizing a Pin Path updates pin coordinates without invalidating threads that reference those pin IDs, as long as the referenced pins still exist. If a pin is deleted, see cascading delete rules in [11-erasers.md](./11-erasers.md).

## Test Cases

```gherkin
Feature: Document model integrity

  Scenario: Pin Path remains one object after drawing
    Given the user draws a hexagon with the Polygon tool
    When the drawing is confirmed
    Then exactly one Pin Path is created
    And it contains one geometry, one guide, and a pins[] array
    And no separate unrelated point objects exist in the document

  Scenario: Thread Path remains one object across multiple segments
    Given the user connects Pin 10 → Pin 14 → Pin 32 → Pin 7
    When the thread is finished
    Then exactly one Thread Path is created
    And its pinIds equal ["pin-10", "pin-14", "pin-32", "pin-7"]
    And no separate unrelated segment objects exist in the document

  Scenario: Threads reference pins by stable ID, not coordinates
    Given a Thread Path referencing pin-10 and pin-14
    When the Pin Path containing pin-10 is moved 5 cm to the right
    Then pin-10's stored coordinate updates
    And the Thread Path's pinIds array is unchanged (["pin-10", "pin-14", ...])
    And the rendered thread segment follows pin-10 to its new position

  Scenario: Project structure round-trips through serialization
    Given a Project with 2 pin layers and 1 thread layer
    When the project is serialized and then deserialized
    Then the resulting Project has the same pinLayers and threadLayers structure
    And every Pin Path's pins[] and every Thread Path's pinIds[] are unchanged
```
