# 22 — Thread Draft Pattern-Follow (Right Arrow)

## Purpose

Thread mode's Draw tool already supports `ArrowLeft` to retract the last confirmed vertex of an in-progress Thread Path draft ([12-thread-editor.md](./12-thread-editor.md) §26-29). This spec adds the companion `ArrowRight` shortcut: once the draft has **4 or more vertices**, pressing it auto-appends the next vertex by extrapolating the numeric pattern formed by the pins already added — the same "Pin N" numbering already shown in Print Preview and SVG export (each pin's 1-based position within its own Pin Path), not the internal stable pin id. This lets a user who has clicked out the first few points of a repeating geometric pattern (e.g. a Nail/String art spiral) finish the rest with the keyboard instead of clicking every remaining vertex by hand.

This is additive to the existing Draw workflow: it changes nothing about click/double-click/right-click/Esc/`ArrowLeft` behaviour, and never affects a committed Thread Path — only the transient, non-undoable in-progress draft (`ThreadDraft`, [02-document-model.md](./02-document-model.md)).

## Vocabulary

"Vertex" here means a pin referenced by `ThreadDraft.pinIds`, in the order it was added to the draft — i.e. exactly the pins [12-thread-editor.md](./12-thread-editor.md) calls "confirmed pins." "Pin N" means a pin's 1-based position within its own Pin Path's `pins` array (the numbering already surfaced in Print Preview and SVG export), which is independent from — and not the same value as — the pin's internal stable id.

## Pattern algorithm

The draft's vertices are split by **position parity**: Group A holds the 1st, 3rd, 5th, … vertices added; Group B holds the 2nd, 4th, 6th, …. The group that receives the *next* vertex is whichever parity the new position number has — so the very first `ArrowRight` press, once the draft has exactly 4 vertices, always extends Group A (the 5th vertex); the next press extends Group B (the 6th); and so on, alternating.

For whichever group is receiving the next vertex, take its two most-recently-added members' Pin-N numbers, `secondLast` and `last` (in that order):

1. `step = last − secondLast`
2. `next = last + step`
3. Wrap `next` into a valid 1-based pin position, circularly, using the pin count `N` of **the Pin Path that `last` belongs to**: `wrapped = ((next − 1) mod N + N) mod N + 1`.
4. The new vertex is the pin at position `wrapped` in that same Pin Path.

The step and the owning Pin Path are recomputed fresh from the group's *current* last two members on every `ArrowRight` press — not fixed once when the feature first activates. This keeps the pattern correct if the user manually clicks additional vertices in between arrow presses, and is what lets a group's two seed members live in two different Pin Paths (worked example 1 below): each press still extrapolates from whatever that group's most recent member actually is.

### Worked examples

| Seed vertices (Pin N) | 1st press | 2nd press | 3rd press | 4th press |
|---|---|---|---|---|
| 3, 15, 4, 16 | 5 | 17 | 6 | — |
| 3, 15, 4, 14 | 5 | 13 | 6 | 12 |
| 3, 6, 9, 12 (path with ≥24 pins) | 15 | 18 | 21 | 24 |
| 3, 6, 9, 12 (16-pin path) | 15 | 2 | 5 | 8 |

The last row's wraparound: Group B's step is 6 (`12 − 6`); `12 + 6 = 18`, which wraps to `((18−1) mod 16 + 16) mod 16 + 1 = 2`. The next Group B press continues from the *wrapped* value: `18 + 6 = 24 → wraps to 8` (equivalently, continuing from 2: `2 + 6 = 8`, which is already in range) — both give the same result because wrapping is idempotent once a value is back in `[1, N]`.

### Cross-Pin-Path vertices

A draft is not confined to one Pin Path — the user can click a pin on any Pin Path to extend a thread. When a group's two most recent members live on different Pin Paths (example 1: pins 3/4 on one path, 15/16 on another), `step` is still just the plain numeric difference of their Pin-N values, and wraparound uses the pin count of whichever Pin Path `last` (the most recent of the two) belongs to. The feature does not attempt to correlate the two paths' geometries — it only extrapolates the numbering, per the user's request.

## Symmetry-mirrored pins

A symmetry-derived mirrored pin id ([06-symmetry.md](./06-symmetry.md)) has no *stored* position of its own — mirrors are recomputed live on every render, never stored in a Pin Path's `pins` array — but each mirror copy is built by mapping over the source's `pins` in order, so a mirror copy shares its source pin's Pin-N (a mirror of "Pin 3" is Pin-N 3, on that copy). When the active group's most recently added member (`last`) is a mirror copy, the extrapolated result stays on that **same physical copy** (not the source, and not a different copy) — same Pin-N arithmetic, same wraparound, just re-expressed as that copy's derived pin id. `secondLast` can be on a different copy (or the source) without affecting this — only `last`'s instance decides which copy the new vertex lands on, consistent with the existing "owning Pin Path of `last`" rule for wraparound.

## Scope limits

- With fewer than 4 vertices in the draft, `ArrowRight` is a no-op.
- With no draft in progress (Thread mode, but no click has started an insertion yet), `ArrowRight` is a no-op.

## Test Cases

```gherkin
Feature: Thread draft pattern-follow (Right Arrow)

  Scenario: Right Arrow is a no-op before the draft has 4 vertices
    Given a Thread Path draft has 2 confirmed vertices
    When the user presses Right Arrow
    Then no vertex is added to the draft

  Scenario: Single-path arithmetic progression with no wraparound
    Given a Thread Path draft's vertices are pins 3, 6, 9, 12 on a Pin Path with at least 24 pins
    When the user presses Right Arrow
    Then pin 15 is appended
    When the user presses Right Arrow again
    Then pin 18 is appended
    When the user presses Right Arrow again
    Then pin 21 is appended
    When the user presses Right Arrow again
    Then pin 24 is appended

  Scenario: Wraparound on a closed 16-pin path
    Given a Thread Path draft's vertices are pins 3, 6, 9, 12 on a 16-pin Pin Path
    When the user presses Right Arrow
    Then pin 15 is appended
    When the user presses Right Arrow again
    Then pin 2 is appended
    When the user presses Right Arrow again
    Then pin 5 is appended
    When the user presses Right Arrow again
    Then pin 8 is appended

  Scenario: Interleaved sequences with a negative step
    Given a Thread Path draft's vertices are pins 3, 15, 4, 14
    When the user presses Right Arrow
    Then pin 5 is appended
    When the user presses Right Arrow again
    Then pin 13 is appended
    When the user presses Right Arrow again
    Then pin 6 is appended
    When the user presses Right Arrow again
    Then pin 12 is appended

  Scenario: A group's two most recent members live on different Pin Paths
    Given a Thread Path draft's vertices are pin 3 and pin 4 on one Pin Path, and pin 15 and pin 16 on a second, larger Pin Path
    When the user presses Right Arrow
    Then pin 5 is appended, on the first Pin Path
    When the user presses Right Arrow again
    Then pin 17 is appended, on the second Pin Path

  Scenario: Right Arrow extrapolates through a symmetry-mirrored pin, staying on the same copy
    Given a Thread Path draft's 4 vertices are the mirror copy of pin 3, pin 15, the mirror copy of pin 4, and pin 16
    When the user presses Right Arrow
    Then the mirror copy of pin 5 is appended (not pin 5 itself, and not a different mirror copy)

  Scenario: Right Arrow does not affect other draft controls
    Given a Thread Path draft has 5 vertices, the last 2 added via Right Arrow
    When the user presses Left Arrow
    Then the last vertex is retracted, exactly as it would be for a manually-clicked vertex
```

## Notes

- Implemented as pure extrapolation logic in `src/application/document/threadPattern.ts` (`computeNextPatternPinId`), called from a new `EditorStore.advanceThreadDraftByPattern()` method. Like `extendThreadDraft`/`retractThreadDraft`, this mutates the transient `ThreadDraft` directly and is **not** run through a `Command` — it only becomes part of an undoable step once the draft is eventually committed (double-click/right-click/Esc), same as every other in-progress draft edit.
