# 17 — Statistics

## Purpose

Define pin and thread statistics surfaced to the user, derived read-only from document state.

## Pin Statistics

Per Pin Path/layer:

```text
Outer Circle

Pins:             120
Requested gap:    1.0 cm
Actual gap:       0.99 cm
Pin diameter:     2 mm
```

Project-level aggregate:

```text
Total pins: 384
```

## Thread Statistics

Per Thread Path/layer:

```text
Red Pattern

Segments:       184
Thread length:  23.7 m
Pins visited:   185
Colours:        Red / White
```

Future thread-length calculations should be able to account for spiral/twist overhead (i.e. actual physical thread consumed by a two/three-colour spiral is longer than the straight geometric segment length — Phase 2 refinement, see [12-thread-editor.md](./12-thread-editor.md) twist configuration).

## Interaction Rules

- Statistics are derived/computed values, never stored as separate mutable state — they must always reflect the current document (recomputed on read, or invalidated/recomputed on relevant mutation).
- "Pins visited" for a Thread Path counts pin occurrences along `pinIds[]`, including repeats if a thread revisits a pin (segment count is `pinIds.length - 1`; pins-visited count is `pinIds.length`, not the count of *unique* pins).
- Thread length is the sum of geometric distances between consecutive `pinIds[]` entries (straight-line, before the future spiral-overhead adjustment).

## Test Cases

```gherkin
Feature: Pin statistics

  Scenario: Per-path pin statistics reflect actual distribution
    Given a closed Pin Path "Outer Circle" with requested spacing 1.0 cm resulting in 120 pins at actual spacing 0.99 cm, diameter 2 mm
    When the user views its statistics
    Then the panel shows "Pins: 120", "Requested gap: 1.0 cm", "Actual gap: 0.99 cm", "Pin diameter: 2 mm"

  Scenario: Project-level total pins sums across all Pin Paths
    Given Pin Path "Outer Circle" with 120 pins and Pin Path "Inner Star" with 264 pins
    When the user views project-level statistics
    Then "Total pins: 384" is displayed

  Scenario: Statistics update after a spacing change
    Given "Outer Circle" currently shows 120 pins
    When the user changes its requested spacing, resulting in 100 pins
    Then the statistics panel updates to show "Pins: 100" without requiring a manual refresh

Feature: Thread statistics

  Scenario: Per-path thread statistics compute segments, length, and pins visited
    Given Thread Path "Red Pattern" with pinIds of length 185 (so 184 segments) and colours [Red, White]
    And the sum of consecutive pin-to-pin distances is 23.7 m
    When the user views its statistics
    Then the panel shows "Segments: 184", "Thread length: 23.7 m", "Pins visited: 185", "Colours: Red / White"

  Scenario: Pins visited counts repeated visits, not unique pins
    Given Thread Path pinIds = [pin-1, pin-2, pin-1, pin-3] (pin-1 visited twice)
    Then "Pins visited" = 4
    And "Segments" = 3

  Scenario: Thread length sums consecutive geometric distances
    Given Thread Path pinIds = [pin-A, pin-B, pin-C]
    And distance(A,B) = 5 cm, distance(B,C) = 3 cm
    Then thread length = 8 cm

  Scenario: Statistics reflect current state after cascading pin deletion
    Given a Thread Path with 184 segments including one touching a soon-to-be-deleted pin
    When that pin is deleted, cascading to remove its adjacent segments
    Then the Thread Path's statistics recompute to reflect the reduced segment count and updated thread length
```
