# 13 — Layers

## Purpose

Define the two independent layer systems (Pin Layers, Thread Layers) and the operations available on each.

## Functional Requirements

Two independent layer systems are required:

```text
PIN LAYERS

👁 🔒 Outer Circle
👁 🔓 Inner Star
○  🔓 Decoration


THREAD LAYERS

👁 🔒 Red Pattern
👁 🔓 Blue Pattern
○  🔓 Detail
```

Pin layers and Thread layers remain fully separate lists — a Pin Layer never contains Thread Paths and vice versa.

## Layer Operations

Each layer supports:

- Create
- Rename
- Delete
- Duplicate
- Reorder
- Show
- Hide
- Lock
- Unlock
- Select all objects (on that layer)
- Merge with the layer above (see [36-layer-merge.md](./36-layer-merge.md) for the full rules — disabled on the topmost layer or when either layer is locked)

## Guarantees

- **Hidden layers retain their contents.** Hiding is purely visual; hidden objects are not deleted and still participate in document logic where relevant — a Thread Path already referencing a pin keeps working after that pin's Pin Layer is hidden. New Thread insertion, however, only targets pins on visible Pin Layers, and prioritizes the active Pin Layer's pins over other visible layers' (see [12-thread-editor.md](./12-thread-editor.md) §Pin Layer Scope for Thread Insertion).
- **Locked layers cannot be modified.** No create/move/resize/rotate/erase/property-change operation succeeds against an object on a locked layer (see [09-selection-and-editing.md](./09-selection-and-editing.md), [11-erasers.md](./11-erasers.md)).
- Deleting a layer deletes all objects it contains, as a single undoable operation (see [10-undo-redo.md](./10-undo-redo.md)).
- Duplicating a layer duplicates its objects with new stable IDs; any Thread Path referencing a duplicated pin still references the original pin's ID, not the duplicate's — duplication does not rewire thread connections.
- **Switching Editor mode keeps the Layers panel in sync.** Entering Pin mode switches the Layers panel to the Pin Layers tab (if it isn't already showing it); entering Thread mode switches it to the Thread Layers tab. Entering any other mode (Edit, Pan, Play) leaves the panel's current tab untouched — there's no matching layer kind to switch to.

## Test Cases

```gherkin
Feature: Independent layer systems

  Scenario: Pin layers and Thread layers are separate lists
    Given a project with 2 Pin Layers and 1 Thread Layer
    Then the Pin Layers list shows exactly 2 entries
    And the Thread Layers list shows exactly 1 entry
    And no Pin Path appears in the Thread Layers list

Feature: Layer CRUD operations

  Scenario: Creating a layer
    Given the Pin Layers panel
    When the user creates a new layer
    Then a new empty Pin Layer appears in the list, visible and unlocked by default

  Scenario: Renaming a layer
    Given a Pin Layer named "Layer 1"
    When the user renames it to "Outer Circle"
    Then the layer is displayed as "Outer Circle"

  Scenario: Deleting a layer removes all its contents in one undo step
    Given a Pin Layer "Decoration" containing 3 Pin Paths
    When the user deletes the layer
    Then the layer and all 3 Pin Paths are removed
    When the user invokes Undo once
    Then the layer and all 3 Pin Paths are restored

  Scenario: Duplicating a layer creates independent copies with new IDs
    Given a Pin Layer "Inner Star" containing 1 Pin Path with 20 pins
    When the user duplicates the layer
    Then a new layer "Inner Star copy" is created with a Pin Path containing 20 pins with new stable IDs distinct from the originals

  Scenario: Duplicated pins do not inherit existing thread connections
    Given a Pin Layer with pin-1 referenced by a Thread Path
    When the layer is duplicated, producing pin-1-copy
    Then the existing Thread Path still references pin-1 only
    And no Thread Path automatically references pin-1-copy

  Scenario: Reordering layers changes render/stacking order
    Given Pin Layers in order [A, B, C]
    When the user moves layer C above layer A
    Then the layer order becomes [C, A, B]

Feature: Layers panel tab follows Editor mode

  Scenario: Entering Pin mode switches the Layers panel to Pin Layers
    Given the Layers panel is showing Thread Layers
    When the user switches to Pin mode
    Then the Layers panel switches to Pin Layers

  Scenario: Entering Thread mode switches the Layers panel to Thread Layers
    Given the Layers panel is showing Pin Layers
    When the user switches to Thread mode
    Then the Layers panel switches to Thread Layers

  Scenario: Entering Edit, Pan or Play mode leaves the Layers panel tab as-is
    Given the Layers panel is showing Thread Layers
    When the user switches to Pan mode
    Then the Layers panel still shows Thread Layers

Feature: Visibility and locking guarantees

  Scenario: Hidden layer retains its contents
    Given a Pin Layer "Detail" with 5 Pin Paths, currently visible
    When the user hides the layer
    Then the layer is not rendered in the editor
    And all 5 Pin Paths still exist in the document data

  Scenario: Unhiding restores the exact same content
    Given the "Detail" layer is hidden per the prior scenario
    When the user shows the layer again
    Then the same 5 Pin Paths render, unchanged

  Scenario: Locked layer blocks modification
    Given a Pin Layer "Outer Circle" is locked
    When the user attempts to move, resize, or delete a Pin Path on that layer
    Then the operation is rejected and no change occurs

  Scenario: Unlocking restores editability
    Given the "Outer Circle" layer is locked
    When the user unlocks it
    Then Pin Paths on that layer can again be moved, resized, or deleted

  Scenario: Locked layer still allows visibility toggling
    Given a Pin Layer is locked
    When the user toggles its visibility
    Then the visibility changes successfully (lock only blocks content edits, not show/hide)

Feature: Select all objects on a layer

  Scenario: Select-all selects every object on that layer only
    Given a Pin Layer "Outer Circle" with 3 Pin Paths, and another layer "Inner Star" with 2 Pin Paths
    When the user invokes "Select all" on "Outer Circle"
    Then all 3 Pin Paths on "Outer Circle" become selected
    And the 2 Pin Paths on "Inner Star" remain unselected
```
