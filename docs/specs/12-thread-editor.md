# 12 — Thread Editor

## Purpose

Define Thread mode: connecting pins into Thread Paths, multi-colour twist rendering, the drawing workflow, highlight states, live preview, and termination behavior.

## Functional Requirements

Threads connect **existing pins only**. A thread cannot terminate at an arbitrary coordinate.

Thread properties:

- One, two, or three colours
- Thread width
- Draw
- Segment Eraser and Path Eraser (see [11-erasers.md](./11-erasers.md))

## Thread Colour Rendering

- **One colour** — standard single strand.
- **Two colours** — two colours visually twist around each other in a spiral.
- **Three colours** — three colours visually twist around one another.

The colours represent **one logical thread path**. The underlying geometry is unaffected by strand count:

```text
centre path
A ───────────────────── B

rendered appearance
red / blue / red / blue spiral
```

A future optional parameter may control twist density / spiral pitch (Phase 2, see [00-overview-and-scope.md](./00-overview-and-scope.md)).

## Thread Rendering Model

Thread geometry is independent from thread styling:

```text
Geometry:
A ───────────────────── B
```

...can render as single colour, two-colour spiral, or three-colour spiral. The underlying geometry always remains `A → B`. Changing colour count/values never changes the stored pin sequence.

## Thread Drawing Workflow

1. Activate Thread Draw.
2. Find the nearest pin within snap radius.
3. Highlight that pin.
4. Left-click starts a Thread Path.
5. Moving the pointer displays a live preview.
6. The preview snaps to the nearest valid pin.
7. Left-click creates the segment.
8. The selected destination becomes the next origin.
9. Continue until the thread is explicitly terminated.

```text
Pin 4 → Pin 12 → Pin 27 → Pin 8
```

Stored as one Thread Path.

## Pin Layer Scope for Thread Insertion

Nearest-pin detection (candidate highlight, click-to-extend, double-click-to-finish) only considers pins belonging to **visible** Pin Layers — a hidden layer's pins are not valid Thread insertion targets, even if geometrically closest to the cursor. This applies per [13-layers.md](./13-layers.md)'s existing "hidden layers retain their contents" rule: hiding a layer removes it from Thread insertion, but does not delete its pins or detach any Thread Path that already references them.

Among visible Pin Layers, the **active Pin Layer** (the one new pins/threads are added to, per [13-layers.md](./13-layers.md)) is checked first: if it has a pin within snap radius, that pin wins the nearest-pin search even when another visible layer has a geometrically closer pin. Only when the active layer has no pin within radius does the search fall back to the other visible layers' nearest pin. This mirrors the pin-over-grid snapping priority pipeline in [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) — active-layer pins are a priority stage, not just a tie-break.

## Pin Highlight States

At least three states required:

- **Normal** — `●`
- **Nearest Candidate** — visually distinguished (e.g. `◯` ring around `●`), shown when a pin is within snap radius of the cursor and would be selected on click.
- **Active Thread Origin** — visually distinct from both Normal and Candidate; marks the last confirmed pin in the current in-progress Thread Path.

## Thread Preview

After the first pin is selected:

```text
Last selected pin ───────── Candidate pin
```

is rendered temporarily. The preview:

- Follows pointer movement
- Snaps to valid pins (per [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) snapping priority — nearest-pin snap is the first stage)
- Uses current thread style (colour count, width)
- Is not persisted until clicked

## Ending / Cutting Thread

```text
Left click
→ Add segment

Double-click
→ Add final segment and finish thread

Right click
→ Finish/cut at previous confirmed pin

Esc
→ Cancel/finish current insertion
```

Recommended `Esc` handling:

- If no segment has been created yet: cancel the Thread Path entirely (nothing added to the document).
- If at least one segment exists: finish at the last confirmed pin (keep what's already confirmed).

## Thread Path Model

```text
ThreadPath
│
├── colours[]
├── width
├── twist configuration
└── pinIds[]
```

For `10 → 14 → 32 → 7`, stored as:

```text
["pin-10", "pin-14", "pin-32", "pin-7"]
```

Threads reference stable Pin IDs instead of copied screen coordinates (see [02-document-model.md](./02-document-model.md)).

## Test Cases

```gherkin
Feature: Thread colour rendering independent of geometry

  Scenario: One-colour thread renders as a single strand
    Given a Thread Path with colours = ["red"]
    Then it renders as a single solid strand along its geometry

  Scenario: Two-colour thread renders as a spiral, geometry unchanged
    Given a Thread Path from pin-A to pin-B with colours = ["red"]
    When the user changes colours to ["red", "blue"]
    Then the rendering shows a two-colour twisting spiral between pin-A and pin-B
    And the stored pinIds remain [pin-A, pin-B] unchanged

  Scenario: Three-colour thread renders as a three-strand spiral
    Given a Thread Path with colours = ["red", "white", "blue"]
    Then it renders as three colours twisting around one another along the same geometric path

Feature: Thread drawing workflow

  Scenario: Nearest pin is highlighted before drawing starts
    Given the editor is in THREAD mode with no active Thread Path
    When the cursor moves within snap radius of pin-4
    Then pin-4 is shown in the Nearest Candidate highlight state

  Scenario: First click starts a Thread Path at the highlighted pin
    Given pin-4 is the current Nearest Candidate
    When the user left-clicks
    Then a new Thread Path begins with pinIds = [pin-4]
    And pin-4 is now shown in the Active Thread Origin state

  Scenario: Subsequent clicks extend the Thread Path
    Given an in-progress Thread Path with pinIds = [pin-4]
    When the user moves the cursor near pin-12 and left-clicks
    Then the Thread Path's pinIds become [pin-4, pin-12]
    And pin-12 becomes the new Active Thread Origin

  Scenario: Multi-segment thread is stored as one path
    Given the user connects Pin 4 → Pin 12 → Pin 27 → Pin 8 via sequential left-clicks
    When the thread is finished
    Then exactly one Thread Path exists with pinIds = [pin-4, pin-12, pin-27, pin-8]

  Scenario: A thread cannot terminate at an arbitrary coordinate
    Given the user is drawing a Thread Path
    When the user left-clicks at a canvas position with no pin within snap radius
    Then no segment is added
    And the Thread Path remains at its last confirmed pin

Feature: Pin Layer scope for Thread insertion

  Scenario: Hidden layer's pins are not valid Thread insertion targets
    Given Pin Layer "Detail" is hidden, and pin-9 (on "Detail") is the closest pin to the cursor
    And Pin Layer "Base" is visible with pin-14 also within snap radius but farther away
    When the cursor moves near pin-9
    Then pin-9 is not shown as the Nearest Candidate
    And pin-14 is shown as the Nearest Candidate instead

  Scenario: Active Pin Layer's pin is prioritized over a closer pin on another visible layer
    Given Pin Layer "Base" is the active Pin Layer and both "Base" and "Overlay" are visible
    And pin-3 (on "Base") and pin-7 (on "Overlay") are both within snap radius, with pin-7 geometrically closer
    When the cursor moves near both pins
    Then pin-3 is shown as the Nearest Candidate, not pin-7

  Scenario: Falls back to another visible layer when the active layer has no candidate
    Given Pin Layer "Base" is the active Pin Layer but has no pin within snap radius of the cursor
    And Pin Layer "Overlay" is visible with pin-7 within snap radius
    When the cursor moves near pin-7
    Then pin-7 is shown as the Nearest Candidate

Feature: Pin highlight states

  Scenario: Normal state for pins not near the cursor and not part of an active thread
    Given a pin is far from the cursor and no Thread Path is in progress at it
    Then it renders in the Normal state

  Scenario: Candidate state for the nearest snap-eligible pin
    Given the cursor is within snap radius of exactly one pin
    Then that pin renders in the Nearest Candidate state
    And all other pins render Normal (or Active Origin if applicable)

  Scenario: Active Origin state persists on the last confirmed pin during drawing
    Given an in-progress Thread Path whose last confirmed pin is pin-12
    Then pin-12 renders in the Active Thread Origin state, visually distinct from Normal and Candidate

Feature: Thread preview

  Scenario: Preview follows pointer and snaps to nearest pin
    Given an in-progress Thread Path with last pin pin-12
    When the cursor moves near pin-27 (within snap radius)
    Then a temporary preview segment is drawn from pin-12 to pin-27
    And the preview is not yet part of the persisted Thread Path

  Scenario: Preview uses current thread style
    Given the active thread style is two-colour (red/blue)
    When the preview segment is rendered
    Then it renders using the same two-colour spiral style as confirmed segments

Feature: Ending and cutting a thread

  Scenario: Double-click adds a final segment and finishes
    Given an in-progress Thread Path with last pin pin-12
    When the user double-clicks on pin-27
    Then the segment pin-12→pin-27 is added
    And the Thread Path is finished (no longer accepting further segments)

  Scenario: Right-click finishes without adding the pending segment
    Given an in-progress Thread Path with last confirmed pin pin-12
    And the preview is currently showing a candidate segment toward pin-27
    When the user right-clicks
    Then the Thread Path finishes at pin-12
    And no segment to pin-27 is added

  Scenario: Esc cancels a thread with zero confirmed segments
    Given the user just started a Thread Path with only the origin pin selected (no segment yet)
    When the user presses Esc
    Then the Thread Path is discarded entirely
    And no Thread Path is added to the document

  Scenario: Esc finishes a thread with at least one confirmed segment
    Given an in-progress Thread Path with segments pin-4→pin-12 already confirmed
    When the user presses Esc
    Then the Thread Path finishes with pinIds = [pin-4, pin-12]
    And no further segment is added

Feature: Thread Path data model

  Scenario: Thread Path stores pin IDs, not coordinates
    Given a completed Thread Path connecting pin-10, pin-14, pin-32, pin-7
    Then its pinIds array equals ["pin-10", "pin-14", "pin-32", "pin-7"]
    And no raw screen or document coordinates are stored as the connection data

  Scenario: Continuation uses the exact destination pin
    Given the user just clicked to connect to pin-27
    When the preview for the next segment begins
    Then the new segment's origin is exactly pin-27 (by ID), not an approximate nearby coordinate
```
