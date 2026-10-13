# 39 — Copy / Paste

## Purpose

Let the user copy Pin Paths and Thread Paths with `Ctrl/Cmd+C` and paste them with `Ctrl/Cmd+V`. Paste works on the same board and on a **different board**: another browser tab, or a project opened or created after the copy. The system clipboard carries the data, so it survives switching tabs and projects.

## Clipboard Envelope

The copied content is one self-contained JSON object, written as `text/plain`:

```json
{
  "kind": "stringartit/clipboard",
  "version": 1,
  "copyId": "<random, new per copy>",
  "sourceBoardId": "<random id of the open board copied from>",
  "pinPaths": [ /* full PinPath objects: geometry, pins, symmetry, colour, diameter, spacing */ ],
  "threadPaths": [ /* full ThreadPath objects; pinIds reference pins inside pinPaths (real or ~mirror-N) */ ]
}
```

- The envelope never references anything outside itself. Every thread `pinId` resolves to a pin, real or mirrored, in the envelope's own `pinPaths`.
- `sourceBoardId` is a transient token for the open board and is never persisted. It is regenerated when a project is loaded, including New and autosave restore. Its only job is to tell "same board" apart from "another board".
- On paste, anything that is not a valid envelope is ignored and left to the browser: a wrong `kind`, a `version` newer than the app supports, bad shapes, an unresolved thread pin, or plain text.
- The implementation is in `src/application/document/clipboard.ts` (pure). `src/ui/useClipboard.ts` wires the native `copy`/`paste` DOM events. Listening for a `keydown` would not work because `Cmd+V` is OS-reserved as a keystroke (see [34-keyboard-shortcuts.md](./34-keyboard-shortcuts.md)).

## Copy Rules

- **Pin Paths selected** (Edit tab, Pin Path granularity): copies the selected paths and every Thread Path in any thread layer that touches one of their pins, mirrored pins included. When such a thread also touches pins on other Pin Paths, those paths are copied too, so the pasted threads are complete. Only one hop is followed: threads that touch only the pulled-in paths are not copied.
- **Pins selected** (Edit tab, Pins granularity): behaves as if their owning Pin Paths were selected.
- **Thread Path selected** (Thread tab): copies the thread and every Pin Path that owns one of its pins, across all pin layers. A mirrored pin resolves to its source path.
- With nothing selected, or focus in a text field, or selected page text, or in Play mode, the native copy runs untouched.

## Paste Rules

- Content goes to the **active Pin Layer** (paths) and the **active Thread Layer** (threads), in its original order, after existing content.
- Every pasted path, pin and thread gets a fresh id. Threads are rewired to the pasted pins, including `~mirror-N` endpoints.
- Paste is one undo step.
- If the active Pin Layer is locked, nothing is pasted. The same applies if the content has threads and the active Thread Layer is locked. This is the same all-or-nothing rule as Delete and Merge.
- After a paste in the Edit tab (Pin Path granularity), the pasted Pin Paths are selected. After a paste in the Thread tab of exactly one thread, that thread is selected. Otherwise nothing is selected.
- Paste is ignored in Play mode and while a text field has focus.

## Placement

The offset step is 10 mm (`PASTE_OFFSET = 1`, board units are cm). Pastes of the same `copyId` are counted, so the *n*-th paste of one copy has index *n*.

- **Same board:** the content moves by `n × 10 mm` on both axes, so a paste never lands exactly on its original.
- **Another board:** the first paste keeps the original coordinates, because nothing sits there to overlap. Later pastes cascade by `(n−1) × 10 mm`.
- **Fit check:** if the moved content's bounding box would not fit inside the target board's bounding box, the content is re-centred on the board instead, plus `(n−1) × 10 mm`. Content larger than the board is centred, not scaled.
- Paste moves the whole copy rigidly: the geometry, the pins, and the symmetry axis or centre, so mirrored copies and the threads on them keep their shape. This differs on purpose from the Move tool, which leaves the symmetry config alone.

## Future

- Cut (`Ctrl/Cmd+X`) and WebMCP copy/paste tools are not in scope.

## Test Cases

```gherkin
Feature: Copy and paste Pin Paths and Thread Paths

  Scenario: Copying a Pin Path includes its threads and the paths they reach
    Given Pin Paths A, B, C and a Thread Path between a pin of A and a pin of B
    And Pin Path A is selected
    When the user copies
    Then the clipboard holds Pin Paths A and B and that Thread Path

  Scenario: Copying pins copies their owning Pin Paths
    Given a pin of Pin Path C is selected at Pins granularity
    When the user copies
    Then the clipboard holds Pin Path C

  Scenario: Copying a Thread Path includes every Pin Path its pins belong to
    Given a Thread Path between pins on Pin Paths in two different pin layers
    And that Thread Path is selected
    When the user copies
    Then the clipboard holds that Thread Path and both Pin Paths

  Scenario: A mirrored pin endpoint pulls in its source Pin Path
    Given a Thread Path ending on a mirrored pin of Pin Path A
    When that Thread Path is copied
    Then the clipboard holds Pin Path A

  Scenario: Copy with nothing selected leaves the native copy alone
    Given nothing is selected
    When the user copies
    Then the clipboard is not written by the app

  Scenario: Paste adds the content to the active layers as one undo step
    Given the clipboard holds Pin Paths A and B and one Thread Path
    When the user pastes
    Then the active Pin Layer gains 2 Pin Paths with fresh ids
    And the active Thread Layer gains 1 Thread Path wired to the pasted pins
    When the user invokes Undo once
    Then the document is back to its pre-paste content

  Scenario: Repeated pastes on the same board cascade
    Given Pin Path C was copied on this board
    When the user pastes twice
    Then the first copy is offset by 10 mm and the second by 20 mm on both axes

  Scenario: Paste into another board keeps the original position
    Given Pin Path A was copied on another board
    When the user pastes into this board
    Then the pasted Pin Path sits at A's original coordinates
    And no pasted Thread Path references a pin id from the source board

  Scenario: Loading a project counts a previous copy as coming from another board
    Given Pin Path C was copied, then a project was loaded
    When the user pastes
    Then the pasted Pin Path sits at C's original coordinates

  Scenario: Content that does not fit the target board is re-centred
    Given the copied content lies outside the target board's bounds
    When the user pastes
    Then the content is centred on the target board

  Scenario: Paste translates symmetry with the content
    Given a copied Pin Path has radial symmetry centred at (1, 1)
    When it is pasted with a (2, 3) offset
    Then the pasted path's symmetry centre is (3, 4)

  Scenario: Thread endpoints on mirrored pins are rewired
    Given a copied Thread Path ends on a mirrored pin of a copied Pin Path
    When it is pasted
    Then the pasted Thread Path ends on the matching mirrored pin of the pasted Pin Path

  Scenario: Pasted content becomes the selection
    Given the Edit tab is active
    When the user pastes Pin Paths
    Then the pasted Pin Paths are selected
    Given the Thread tab is active
    When the user pastes content with exactly one Thread Path
    Then the pasted Thread Path is selected

  Scenario: Paste is blocked by a locked target layer
    Given the active Pin Layer is locked
    When the user pastes
    Then nothing changes
    Given the active Pin Layer is unlocked and the active Thread Layer is locked
    When the user pastes content containing Thread Paths
    Then nothing changes

  Scenario: Foreign clipboard content is ignored
    Given the clipboard holds plain text, foreign JSON, a different kind, or a newer version
    When the user pastes
    Then nothing changes and the browser's native paste runs

  Scenario: Text fields keep native copy and paste
    Given a text field has focus
    When the user copies or pastes
    Then the app does not intercept it
```
