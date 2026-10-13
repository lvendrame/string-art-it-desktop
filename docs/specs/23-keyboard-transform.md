# 23 — Keyboard Move / Rotate / Scale

## Purpose

Edit mode already has three mouse press-drag-release tools for transforming the selected Pin Path — Move, Rotation, Scale ([09-selection-and-editing.md](./09-selection-and-editing.md), [21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md)). `docs/conventions/ui-patterns.md` lists "arrow-key nudging" as a spec-only, unimplemented gesture. This spec fills that gap: with a Pin Path selected and one of Move/Rotation/Scale active as the current Edit tool, the arrow keys drive that same tool's transform directly, at a fixed step per key press, as an alternative to dragging.

## Trigger

`ArrowUp` / `ArrowDown` / `ArrowLeft` / `ArrowRight`, only while:

- Edit mode (`mode === "select"`) has a Pin Path selected (`selection.type === "pinPath"`).
- The active Edit tool (`selectTool`) is `"move"`, `"rotate"`, or `"scale"` — which tool is active determines which transform the arrows perform. When `selectTool` is `"select"` or `"merge"`, the arrow keys have no effect here.
- Focus is not inside a text/number input (the property panel's own numeric fields already use native `ArrowUp`/`ArrowDown` to step their value — this must not also move the shape).

Holding Shift multiplies the step by 10 (`e.shiftKey`). Alt/Cmd were considered but rejected — they collide with OS/browser-level shortcuts (e.g. Option+Arrow word-navigation on macOS, window/space-switching bindings), which fired instead of or alongside the app's own handler.

Each key press (and each auto-repeat tick, below) commits its transform as **one** undoable operation immediately; there is no drag/preview phase (a single discrete key press has nothing to preview).

### Hold-to-repeat

Holding an arrow key down repeats the transform automatically, matching the standard "hold to repeat" behaviour of native OS/UI controls (e.g. WPF/UWP's `RepeatButton`, a de facto standard across UI toolkits): **500ms** initial delay before the first repeat, then a repeat every **100ms** until the key is released.

This is a purpose-built repeat loop (`setTimeout`/`setInterval`), not the browser's native `event.repeat` auto-fire — native OS key-repeat delay/rate is a user-configurable OS setting and varies widely machine to machine, which would make the nudge feel inconsistent for different users. Native repeat `keydown` events (`event.repeat === true`) are therefore ignored entirely; the timers above are the only thing driving repeated presses. Releasing the key (`keyup`), switching to a different arrow key mid-hold, or the window losing focus (e.g. Alt-Tab) all stop the repeat. Each repeat tick re-checks the same gating as the initial press (selection, tool, lock) against live state, so a mid-hold change (Undo, switching tools, locking the layer) stops the repeat cleanly rather than replaying stale state.

## Move

Active only when the Move tool is the current Edit tool. Translates the selected Pin Path's existing pins in place (ids stay stable, same as the mouse Move tool), in the direction of the arrow pressed:

| Key | Direction |
|---|---|
| `ArrowUp` | −y |
| `ArrowDown` | +y |
| `ArrowLeft` | −x |
| `ArrowRight` | +x |

The step is measured in **screen pixels**, converted to document units via the same zoom-independent conversion already used for the pin/grid snap radius (`screenDistanceToDocument`) — so the visual size of a nudge stays constant regardless of zoom level. Plain press = 1px worth of document distance; Shift = 10px worth.

## Rotate

Active only when the Rotation tool is the current Edit tool. Rotates the selected Pin Path's pins and geometry in place (ids stay stable) about the shape's **own centroid** — unlike the mouse Rotation tool, which pivots on the external press point. Centroid is computed the same way as the Scale tool's pivot (line/arc midpoint, shape's own centre field, rectangle/square position+half-size, freehand mean).

| Key | Rotation |
|---|---|
| `ArrowUp`, `ArrowLeft` | Anticlockwise |
| `ArrowDown`, `ArrowRight` | Clockwise |

The step is measured in **degrees**: plain press = 1°, Shift = 10°. Rotation does not change pin count (no redistribution), same as dragging the mouse Rotation tool.

## Scale

Active only when the Scale tool is the current Edit tool. Resizes the selected Pin Path about its own centroid (centre invariant, same pivot rule as the mouse Scale tool), recomputing pins through the existing distribution pipeline and reattaching any connected Thread Path segments to the nearest new pin ([21-scale-and-pin-distance.md](./21-scale-and-pin-distance.md) §Nearest-Pin Reattachment) — identical mechanism to the mouse Scale tool's commit, just driven by a keyboard-computed factor instead of a drag distance.

| Key | Effect |
|---|---|
| `ArrowUp`, `ArrowRight` | Increase size |
| `ArrowDown`, `ArrowLeft` | Decrease size |

The step is measured in **percent** of current size, applied multiplicatively (`factor = 1 ± step/100`): plain press = 1%, Shift = 10%. The factor is clamped to the same small positive minimum the mouse Scale tool already enforces, so repeated decreases can never invert or collapse the shape.

## Shared rules

- Blocked entirely on a locked Pin Layer — no mutation, same as every other locked-layer edit rule ([13-layers.md](./13-layers.md)). Falls out for free: Move/Rotate reuse `EditorStore.commitPinPathTransform`, Scale reuses `EditorStore.commitPinPathScale`, both of which already check the live lock state.
- Each committed transform is one `HistoryStack` entry; Undo reverts it in a single step.

## Test Cases

```gherkin
Feature: Keyboard Move

  Scenario: Arrow key nudges the selected Pin Path by one screen-pixel of document distance
    Given a Pin Path is selected and the Move tool is active
    When the user presses ArrowRight
    Then the path's geometry and pins translate by the document-space equivalent of 1 screen pixel at the current zoom
    And pin ids stay stable

  Scenario: Shift gives a 10x step
    Given a Pin Path is selected and the Move tool is active
    When the user holds Shift and presses ArrowLeft
    Then the path translates by 10 screen-pixels' worth of document distance in the negative x direction

  Scenario: No effect without a Move-tool selection
    Given no Pin Path is selected, or the active Edit tool is not Move
    When the user presses an arrow key
    Then nothing changes

  Scenario: No effect while typing in a numeric field
    Given focus is inside the Selection panel's numeric input
    When the user presses ArrowRight
    Then the Pin Path does not move and the input's own native stepping behaves normally

  Scenario: Native OS key-repeat events are ignored in favour of the app's own repeat timer
    Given the user holds ArrowRight down, generating OS auto-repeat keydown events
    When those repeated keydown events all have event.repeat === true
    Then they are ignored — the app's own 500ms-delay/100ms-interval timer drives repeats instead

  Scenario: Holding the key repeats the nudge after the initial delay
    Given a Pin Path is selected and the Move tool is active
    When the user holds ArrowRight down
    Then one nudge commits immediately on press
    And no further nudge commits until 500ms have elapsed
    And a nudge commits every 100ms after that, for as long as the key stays held

  Scenario: Releasing the key stops the repeat
    Given the user is holding ArrowRight past the initial 500ms delay
    When the user releases the key
    Then no further nudges commit, even as more time passes

  Scenario: A mid-hold change stops the repeat cleanly
    Given the user is holding an arrow key past the initial 500ms delay, mid-repeat
    When the Pin Path's layer becomes locked (or the selection/tool/mode changes) before the next tick
    Then that tick and all subsequent ticks are silently skipped
    And no error occurs

Feature: Keyboard Rotate

  Scenario: Up/Left rotate anticlockwise about the shape's own centroid
    Given a Pin Path is selected and the Rotation tool is active
    When the user presses ArrowUp
    Then the path rotates 1° anticlockwise about its own centroid, not an external pivot
    And the centroid position is unchanged

  Scenario: Down/Right rotate clockwise
    Given a Pin Path is selected and the Rotation tool is active
    When the user presses ArrowRight
    Then the path rotates 1° clockwise about its own centroid

  Scenario: Rotation does not change pin count
    Given a Pin Path is selected and the Rotation tool is active
    When the user presses an arrow key
    Then the pin count is unchanged

Feature: Keyboard Scale

  Scenario: Up/Right increase size by 1%, Down/Left decrease it
    Given a Pin Path is selected and the Scale tool is active
    When the user presses ArrowUp
    Then the path's size fields scale up by a factor of 1.01 about its own centroid
    And the pin count recalculates per the existing distribution rules

  Scenario: A connected thread reattaches to the nearest new pin
    Given a Thread Path has an endpoint on a pin of the selected Pin Path
    When the user scales it via the keyboard
    Then the Thread Path's endpoint now references whichever new pin is nearest the old pin's position
    And the pin and thread changes commit as one undo step

  Scenario: Scaling a locked layer's Pin Path via keyboard is blocked
    Given the selected Pin Path's layer is locked
    When the user presses an arrow key with the Scale tool active
    Then no geometry, pin, or thread change occurs

Feature: Undo

  Scenario: Undo reverts one keyboard transform as a single step
    Given the user has just nudged, rotated, or scaled a Pin Path via the keyboard
    When they press Undo once
    Then the Pin Path (and any reattached threads, for Scale) is restored exactly to its pre-press state
```

## Notes

- Implemented as a new `useKeyboardTransform(store)` hook (`src/ui/canvas/useKeyboardTransform.ts`), following the same per-feature `window`-level keydown pattern as `useMoveTool.ts`/`useRotateTool.ts`/`useScaleTool.ts`/`useThreadDrawing.ts` — there is no central keyboard dispatcher in this codebase. It calls the same `EditorStore.commitPinPathTransform`/`commitPinPathScale` methods the mouse tools already use, so locked-layer blocking, undo/redo, and thread reattachment are unmodified shared code, not reimplemented.
- `geometryCenter` (previously private to `pinPath.ts`, used internally by `scaleGeometry`) is exported for reuse as the Rotate pivot, since keyboard Rotate — unlike the mouse Rotation tool — always pivots on the shape's own centroid.
- `isTextEntryTarget` was extracted from `EditorShell.tsx` into `src/ui/keyboard.ts` so both the Undo/Redo shortcut and this feature share one definition.
- Each repeat tick commits its own `HistoryStack` entry, same as a real individual key press — holding a key for a while therefore produces many undo steps (one per press-equivalent), not one merged step for the whole hold. This matches how this app already treats a single discrete keyboard action, and avoids introducing a new "coalesce these into one undo step" mechanism that no other feature here has.
- The Shift (step-size) and which-key-was-pressed state is captured once when the repeat starts and held fixed for that hold cycle — changing modifiers or pressing a second arrow key without releasing the first is not tracked per-tick. Pressing a *different* arrow key while one is already auto-repeating cancels the old repeat and starts a new one for the new key (last key wins), matching the most common "which key is currently held" interpretation.
