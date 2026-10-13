# 37 — Radial Thread Tool

## Purpose

Add a Thread-mode tool, **Radial** (`LoaderPinwheel` icon), that fans spokes from one anchor pin to every pin of a Pin Path in two clicks. Like Zig-zag/Parabolic ([35-zigzag-parabolic-tools.md](./35-zigzag-parabolic-tools.md)) it inserts a *computed* Thread Path rather than accumulating pins click-by-click, and it reuses that spec's two-pin draft flow (`EditorState.twoPinDraft`, `useTwoPinSequenceDrawing.ts`, `TwoPinDraftLayer.tsx`). Only the sequence math is new (`src/application/document/radialSequence.ts`).

## Behaviour

1. **Click 1** — the nearest pin becomes the **anchor** (A).
2. **Click 2** — the nearest pin picks the **target Pin Path** (the path that pin belongs to). The clicked pin itself only selects the path; it does not affect the order.
3. The tool commits immediately — there is never a 3rd click, no candidates to disambiguate.

The result is **one continuous Thread Path**: `[A, p1, A, p2, A, p3, …]`, where `p1..pN` are the target path's pins in path order (index 0 → N-1). The thread returns to the anchor between every spoke, as a real radial string-art build does. When the anchor lies on the target path, it is skipped (no zero-length `[A, A]` spoke).

Worked examples:

```text
Paths (1..6) and (7..12); click 3, then 11
→ spokes [3,7] [3,8] [3,9] [3,10] [3,11] [3,12]
→ pinIds [3,7,3,8,3,9,3,10,3,11,3,12]

One path (1..7); click 4, then 1
→ spokes [4,1] [4,2] [4,3] [4,5] [4,6] [4,7]   (no [4,4])
→ pinIds [4,1,4,2,4,3,4,5,4,6,4,7]
```

Open and closed paths behave the same — always path order.

### Symmetry

The anchor keeps its own id (real or mirrored). Target pins follow the clicked pin's physical copy: clicking a mirrored pin walks that mirror copy's pins (`mirroredPinId(pin.id, groupIndex)`), same "stay within this instance" rule as spec 35.

## Shared draft behaviour

Everything else is inherited unchanged from spec 35's two-pin draft:

- `Escape` cancels; `ArrowLeft` steps back (before click 2 that cancels the draft).
- Right-click while drafting shows Back/Cancel.
- Committing into a locked active Thread Layer discards the draft silently.
- Switching Thread tool (including to another two-pin tool) or leaving Thread mode discards an in-progress draft.
- One commit = one undo step.
- Two clicks on the same pin do nothing.

## Configuration

A **Radial settings** box in the Thread Properties panel (shown only while Radial is active) has one control:

- **Step** (slider 1–10, default 1) — 1-based stride over the target path's pins. 1 uses every pin, 2 every other pin, 3 skips two, and so on. The stride always starts at path index 0 and is applied *before* the anchor is dropped, so spoke positions stay regular wherever the anchor sits. Next-draw setting (`EditorState.radialSettings`), not undoable, same treatment as Zig-zag/Parabolic settings. Unlike their 0-based "pins skipped" sliders, Radial's Step counts the stride itself.

```text
Paths (1..6) and (7..12); Step 2; click 3, then 11
→ pinIds [3,7,3,9,3,11]

Closed path (1..8); Step 2; click 3, then 1
→ stride keeps 1,3,5,7; anchor 3 dropped → pinIds [3,1,3,5,3,7]
```

No full-fill/cycles configuration.

## UI

- Thread toolbar button after Parabolic: `Radial [R]`.
- Thread-mode radial context menu gains a `threadRadial` slice.
- Keyboard: bare `R` in the Thread tab ([34-keyboard-shortcuts.md](./34-keyboard-shortcuts.md)).
- Help: Thread tools list + Keyboard & Mouse Thread shortcuts.

## Test Cases

```gherkin
Feature: Radial thread tool

  Scenario: Anchor on one path, target a different path
    Given Pin Paths with pins 1..6 and 7..12, and the Radial tool active
    When the user clicks pin 3, then pin 11
    Then one Thread Path is created with pin order 3,7,3,8,3,9,3,10,3,11,3,12

  Scenario: Anchor and target on the same path skips the anchor
    Given one Pin Path with pins 1..7, and the Radial tool active
    When the user clicks pin 4, then pin 1
    Then one Thread Path is created with pin order 4,1,4,2,4,3,4,5,4,6,4,7

  Scenario: Order follows the path, not the clicked pin
    Given a closed Pin Path with pins 1..5, and the Radial tool active
    When the user clicks pin 1, then pin 4
    Then the Thread Path's pin order is 1,2,1,3,1,4,1,5

  Scenario: Commits on the second click as one undo step
    Given the Radial tool active
    When the user clicks an anchor pin, then a target pin
    Then the Thread Path is created without a 3rd click
    And a single Undo removes it

  Scenario: Locked layer discards
    Given the active Thread Layer is locked
    When the user completes a Radial draft
    Then no Thread Path is created and the draft is cleared

  Scenario: Step 2 skips every other target pin
    Given Pin Paths with pins 1..6 and 7..12, the Radial tool active and Step 2
    When the user clicks pin 3, then pin 11
    Then one Thread Path is created with pin order 3,7,3,9,3,11

  Scenario: Step strides before dropping the anchor
    Given a closed Pin Path with pins 1..8, the Radial tool active and Step 2
    When the user clicks pin 3, then pin 1
    Then the Thread Path's pin order is 3,1,3,5,3,7

  Scenario: R shortcut
    Given the Thread tab is active
    When the user presses R
    Then the Radial tool is active
```

## Scope limits

- No reverse-order or start-at-clicked-pin options.
- No live spoke preview before click 2 — only the anchor and hovered pin highlight, as with spec 35's pre-candidate state.
