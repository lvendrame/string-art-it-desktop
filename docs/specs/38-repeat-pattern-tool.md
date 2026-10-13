# 38 — Repeat Pattern Thread Tool

## Purpose

Add a Thread-mode tool, **Repeat** (`Galaxy` icon, key `T`), that extends a pattern the user starts by hand. The user clicks any number of pins; the tool splits them into two halves, pairs them up, and repeats each pair's step for more cycles. Unlike Zig-zag/Parabolic/Radial ([35](./35-zigzag-parabolic-tools.md), [37](./37-radial-thread-tool.md)) it is an N-click draft, so it has its own state (`EditorState.repeatDraft`) rather than the two-pin draft. Math lives in `src/application/document/repeatSequence.ts`.

## Workflow

1. Pick the Repeat tool. Each click on a pin appends it to the draft (clicking the last pin again is ignored).
2. While picking, the clicked pins preview as a thread in the first Repeat colour and the current width, with a live line to the cursor — same visuals as a Thread Draw draft.
3. **Enter** or the radial menu's **Generate** commits. Generate needs at least 4 usable pins; with fewer, nothing happens and the draft stays.
4. **Escape** / radial **Cancel** discards the draft. **ArrowLeft** / radial **Back** removes the last pin.

Enter is ignored while focus is in a text field or on a button (a focused button already handles Enter itself).

## The math

Pin "number" = the pin's 1-based position within its own Pin Path — the same number Print Preview shows and Right-arrow pattern-follow ([22](./22-thread-follow-pattern.md)) uses.

1. If the click count is odd, the last click is ignored.
2. Split the clicks into two halves, `first` and `second`, and pair them by position: `(first[i], second[i])`.
3. Each pair's step is `number(second[i]) − number(first[i])`.
4. Cycle `k` (1-based) adds one group: for every pair, `number(second[i]) + k·step`, wrapped around the **second pin's** Pin Path. A mirrored pin stays in its mirror copy.

```text
Clicks [1,2,3,4,5,11,12,13,14,15], Cycles 2, 40-pin ring
pairs  [1,11] [2,12] [3,13] [4,14] [5,15]   (step 10 each)
groups [1..5] [11..15] [21..25] [31..35]
thread [1,2,3,4,5,11,12,13,14,15,21,22,23,24,25,31,32,33,34,35]
```

### Full-fill

When Full-fill is checked, Cycles is read-only and ignored. The cycle count comes from the first pair: with `d = |step₀|` and `n` = the pin count of its second pin's path, cycle `k` moves the first pair `(k+1)·d` from the first clicked pin. Generation continues while that is below `n`. Landing exactly on `n` (back on the first clicked pin) is excluded; the first overshoot past it is included, then generation stops — the same rule as Parabolic full-fill. A step of 0 generates no extra cycles.

```text
40-pin ring, pair [1,11]: cycles 21, 31 → next would land on 1 → stop (2 cycles)
35-pin ring, pair [1,11]: cycles 21, 31, 6 (overshoot kept) → stop (3 cycles)
```

## Colours

The Repeat tool has its own colour list (add/remove buttons, same control as Generator mode), up to `Cycles + 2` colours (the two clicked halves count as groups). With full-fill the cap still follows the Cycles value. Group `g` (0 = first half, 1 = second half, 2+ = generated cycles) gets `colours[g % count]`.

- **One colour:** one continuous Thread Path.
- **Two or more:** one Thread Path per group. Each group after the first starts at the previous group's last pin, so the connecting segment takes the new group's colour.

Every path is single-strand (`colours: [c]`) and uses the thread width default. The Thread Properties panel hides the strand-colour count, swatches and twist pitch while Repeat is active; width stays.

All paths from one Generate are one undo step. A locked active Thread Layer discards the draft silently. Switching Thread tool or leaving Thread mode discards an in-progress draft.

## Configuration

| Field | Range | Default | Effect |
|---|---|---|---|
| Cycles | 1–20 | 1 | Generated groups after the two clicked halves. Read-only while Full-fill is on |
| Full-fill | on/off | off | Derive the cycle count from the first pair (see above) |
| Colours | 1 to Cycles + 2 | 1 | Alternating colour per group |

Plain next-draw state (`EditorState.repeatSettings`), not undoable.

## Test Cases

```gherkin
Feature: Repeat pattern tool

  Scenario: Cycles extend each pair by its step
    Given a 40-pin closed Pin Path and the Repeat tool with Cycles 2
    When the user clicks pins 1,2,3,4,5,11,12,13,14,15 and presses Enter
    Then one Thread Path is created with pins 1..5, 11..15, 21..25, 31..35

  Scenario: Odd clicks drop the last pin
    Given the Repeat tool with Cycles 1
    When the user clicks pins 1,2,11,12,30 and presses Enter
    Then the Thread Path is 1,2,11,12,21,22

  Scenario: Fewer than 4 pins does nothing
    When the user clicks 3 pins and presses Enter
    Then no Thread Path is created and the draft stays

  Scenario: Full-fill stops before landing on the first pin
    Given a 40-pin ring, Full-fill on
    When the user clicks 1,2,11,12 and generates
    Then the groups are [1,2] [11,12] [21,22] [31,32]

  Scenario: Full-fill keeps the overshoot cycle
    Given a 35-pin ring, Full-fill on
    When the user clicks 1,2,11,12 and generates
    Then the groups are [1,2] [11,12] [21,22] [31,32] [6,7]

  Scenario: Two colours alternate per group
    Given colours A and B, Cycles 1
    When the user clicks 1,2,11,12 and generates
    Then three Thread Paths are created: [1,2] in A, [2,11,12] in B, [12,21,22] in A
    And a single Undo removes all three

  Scenario: Locked layer discards
    Given the active Thread Layer is locked
    When the user generates a Repeat draft
    Then no Thread Path is created and the draft is cleared

  Scenario: T shortcut
    Given the Thread tab is active
    When the user presses T
    Then the Repeat tool is active
```

## Scope limits

- No preview of the generated cycles before Generate — only the clicked pins preview.
- Pairs across different Pin Paths use the plain number difference, applied on the second pin's path; no geometric mapping between paths.
