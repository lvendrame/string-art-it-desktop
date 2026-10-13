# 11 — Erasers & Cascading Deletion

## Purpose

Define the two eraser tools per mode (a fine-grained one and a whole-object one) and the cascading deletion rule that keeps threads consistent when a referenced pin is removed.

## Functional Requirements

Pin and Thread modes each have **two erasers**, one fine-grained and one whole-object:

- **Pin Eraser** — removes one pin from its Pin Path.
- **Path Eraser (Pin mode)** — removes an entire Pin Path and all its pins in one click, cascading into any thread segments referencing them, as one undoable operation (same guarantee as the Cascading Deletion Rule below).
- **Segment Eraser (Thread mode)** — removes one segment from a Thread Path, splitting it into up to two surviving fragments; the pins at either end of the removed segment are untouched.
- **Path Eraser (Thread mode)** — removes an entire Thread Path in one click.

### Cascading Deletion Rule

If a deleted pin is referenced by a thread, all thread segments connected to that pin are also removed.

```text
A ─ B ─ C
```

Deleting `B` removes:

```text
A ─ B
B ─ C
```

The entire cascading deletion is a **single undoable operation** (see [10-undo-redo.md](./10-undo-redo.md)).

## Interaction Rules

- All four erasers are only available/active in their own mode (Pin Eraser + Path Eraser in Pin mode, Segment Eraser + Path Eraser in Thread mode), consistent with the mode model in [05-canvas-and-viewport.md](./05-canvas-and-viewport.md).
- Erasing a pin that belongs to a Pin Path removes that pin from the path's `pins[]`; if erasing individual pins is supported, the path's guide/spacing metadata may become "custom" (no longer strictly matching the requested-spacing-derived set) — the engine must not attempt to silently re-run distribution and re-add the erased pin.
- The Path Eraser (Pin mode) removes every pin in the clicked Pin Path in one click; any thread segment referencing one of those pins is removed too, in the same undo step.
- The Segment Eraser splits a Thread Path's `pinIds[]` at the clicked segment; either resulting side is dropped if it has fewer than 2 pins (a path needs at least 2 pins to represent a segment) — so removing an end segment trims to one surviving fragment, and removing the only segment of a 2-pin path removes it entirely.
- Erasing respects layer locks: erasing is blocked on a locked layer, for all four erasers.

## Test Cases

```gherkin
Feature: Separate erasers per mode

  Scenario: Pin-mode erasers only work in Pin mode
    Given the editor is in THREAD mode
    Then the Pin Eraser and Path Eraser (Pin mode) tools are not available/active

  Scenario: Thread-mode erasers only work in Thread mode
    Given the editor is in PIN mode
    Then the Segment Eraser and Path Eraser (Thread mode) tools are not available/active

  Scenario: Pin Eraser removes a pin
    Given a Pin Path with pins including pin-5
    When the user erases pin-5 with the Pin Eraser
    Then pin-5 no longer exists in the Pin Path's pins[]

  Scenario: Segment Eraser removes thread geometry without touching pins
    Given a Thread Path connecting pin-1 → pin-2 → pin-3
    When the user erases the segment between pin-2 and pin-3 with the Segment Eraser
    Then the Thread Path no longer includes the pin-2→pin-3 segment
    And pin-2 and pin-3 still exist as pins

Feature: Path Eraser (Pin mode)

  Scenario: Path Eraser removes an entire Pin Path and its pins
    Given a Pin Path with pins pin-1, pin-2, pin-3
    When the user erases the Pin Path with the Path Eraser
    Then the Pin Path and all of pin-1, pin-2, pin-3 no longer exist

  Scenario: Path Eraser cascades into referencing threads as one undo step
    Given a Pin Path with pins pin-1, pin-2, pin-3
    And a Thread Path connecting pin-1 → pin-2 → pin-3
    When the user erases the Pin Path with the Path Eraser
    Then the Pin Path is removed
    And the Thread Path is removed too
    When the user invokes Undo once
    Then the Pin Path, its pins, and the Thread Path are all restored

  Scenario: Cannot use the Path Eraser on a locked Pin Layer
    Given a Pin Path belongs to a locked Pin Layer
    When the user attempts to erase the Pin Path with the Path Eraser
    Then nothing is removed

Feature: Segment Eraser (Thread mode)

  Scenario: Erasing a middle segment splits the Thread Path into two fragments
    Given a Thread Path A → B → C → D → E
    When the user erases the segment between B and C with the Segment Eraser
    Then a fragment A → B remains
    And a fragment C → D → E remains
    And both fragments are separate Thread Paths

  Scenario: Erasing an end segment leaves one surviving fragment
    Given a Thread Path A → B → C
    When the user erases the segment between A and B with the Segment Eraser
    Then the surviving Thread Path is B → C
    And no second fragment is created

  Scenario: Erasing the only segment of a 2-pin Thread Path removes it entirely
    Given a Thread Path with only pin-A and pin-B (A → B)
    When the user erases the A→B segment with the Segment Eraser
    Then the Thread Path is deleted entirely (no fragment has >= 2 pins)

  Scenario: Cannot use the Segment Eraser on a locked Thread Layer
    Given a Thread Path belongs to a locked Thread Layer
    When the user attempts to erase one of its segments with the Segment Eraser
    Then no segment is removed

Feature: Cascading deletion on pin removal

  Scenario: Deleting a middle pin removes both adjacent thread segments
    Given a Thread Path A → B → C (pin-A, pin-B, pin-C)
    When the user erases pin-B with the Pin Eraser
    Then the segment A→B is removed
    And the segment B→C is removed
    And pin-B no longer exists

  Scenario: Cascading deletion is one undo step
    Given the scenario above has just occurred
    When the user invokes Undo once
    Then pin-B is restored
    And both the A→B and B→C segments are restored
    And the Thread Path structure matches its pre-deletion state exactly

  Scenario: Deleting an endpoint pin only removes its one segment
    Given a Thread Path A → B → C
    When the user erases pin-A
    Then only the A→B segment is removed
    And the B→C segment remains intact
    And the resulting Thread Path is B → C

  Scenario: Deleting a pin used by multiple independent Thread Paths cascades across all of them
    Given pin-X is referenced by Thread Path 1 (W→X) and Thread Path 2 (X→Z)
    When the user erases pin-X
    Then Thread Path 1's W→X segment is removed
    And Thread Path 2's X→Z segment is removed
    And both removals plus the pin removal undo together in a single Undo

  Scenario: Thread Path reduced below 2 pins is removed entirely
    Given a Thread Path with only pin-A and pin-B (A→B)
    When the user erases pin-B
    Then the Thread Path is deleted entirely (fewer than 2 pins remain)

Feature: Erasing respects layer locks

  Scenario: Cannot erase on a locked layer
    Given a Pin Path belongs to a locked Pin Layer
    When the user attempts to erase one of its pins
    Then no pin is removed
```
