# 10 — Undo / Redo

## Purpose

Define the undo/redo requirement and the exhaustive list of operations that must be undoable. Implementation mechanism (command objects) is defined in [01-architecture.md](./01-architecture.md).

## Functional Requirements

```text
Undo
Ctrl/Cmd + Z

Redo
Ctrl/Cmd + Shift + Z
```

## Undoable Operations

- Create Pin Path
- Delete Pin Path
- Move
- Resize (Scale tool / Pin distance change — see [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md))
- Rotate
- Change spacing
- Change colour
- Change diameter
- Change guides
- Add thread
- Add segment
- Delete thread
- Cascading thread deletions (see [11-erasers.md](./11-erasers.md))
- Layer operations (see [13-layers.md](./13-layers.md))
- Symmetry changes (see [06-symmetry.md](./06-symmetry.md))

## Interaction Rules

- Every listed operation is captured as exactly one undo step, even when it triggers cascading effects (e.g. deleting a pin that also deletes connected thread segments is one undo step, not several).
- Redo is invalidated (redo stack cleared) the moment a new undoable operation is performed after an undo — standard linear undo history, not a branching one.
- Undo/redo apply regardless of current editor mode or selection state, as long as focus is on the canvas/document (not, e.g., inside a text input where the OS/browser's native undo may apply instead).

## Test Cases

```gherkin
Feature: Undo and redo shortcuts

  Scenario: Ctrl/Cmd+Z undoes the last operation
    Given the user created a Circle Pin Path
    When the user presses Ctrl/Cmd+Z
    Then the Circle Pin Path is removed from the document

  Scenario: Ctrl/Cmd+Shift+Z redoes the last undone operation
    Given the user undid the creation of a Circle Pin Path
    When the user presses Ctrl/Cmd+Shift+Z
    Then the Circle Pin Path reappears in the document with identical geometry

Feature: Every listed operation type is undoable

  Scenario Outline: <operation> is undoable
    Given the user performs "<operation>"
    When the user invokes Undo
    Then the document state reverts to exactly what it was before "<operation>"

    Examples:
      | operation                          |
      | Create Pin Path                    |
      | Delete Pin Path                    |
      | Move a Pin Path                    |
      | Resize a Pin Path                  |
      | Rotate a Pin Path                  |
      | Change pin spacing                 |
      | Change pin colour                  |
      | Change pin diameter                |
      | Change guide visibility            |
      | Add a thread                       |
      | Add a thread segment               |
      | Delete a thread                    |
      | Rename a layer                     |
      | Reorder layers                     |
      | Toggle layer visibility            |
      | Toggle layer lock                  |
      | Change symmetry mode               |

Feature: Cascading operations are single undo steps

  Scenario: Deleting a pin referenced by two threads undoes in one step
    Given Pin B is referenced by Thread A-B and Thread B-C
    When the user deletes Pin B
    Then Pin B and both thread segments are removed
    When the user invokes Undo once
    Then Pin B and both thread segments are all restored together

Feature: Redo stack invalidation

  Scenario: Performing a new action after undo clears redo history
    Given the user created Pin Path X, then undid it (Undo stack has redo available for X)
    When the user creates a new Pin Path Y
    Then invoking Redo does not restore Pin Path X
    And the document only reflects Pin Path Y
```
