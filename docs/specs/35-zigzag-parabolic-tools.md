# 35 — Zig-Zag and Parabolic Thread Tools

## Purpose

Add two new Thread-mode tools — **Zig-zag** (`Waypoints` icon) and **Parabolic** (`Croissant` icon) — alongside Draw/Select/Eraser/Segment. Both insert a *computed* multi-segment Thread Path between two clicked pins, rather than the pin-by-pin accumulation Draw uses: click a first pin, click a second pin, and the tool fills in every intermediate pin itself, following a fixed pattern rather than the user's click order.

This is additive: two new `ThreadTool` values (`"zigzag"`, `"parabolic"`), one new draft type (`TwoPinDraft`), and a handful of new `EditorStore` methods. It changes nothing about Draw/Select/Eraser/Segment or any Pin-mode tool.

## The shared math

Both tools reduce to **one algorithm** — interleave two ordered pin sequences, optionally reversing the second — parameterized by a single boolean (`src/application/document/twoPinSequence.ts`):

|                | Case 1 (same Pin Path) | Case 2 (different Pin Paths) |
|---|---|---|
| **zig-zag**    | reverse second half    | no reverse                    |
| **parabolic**  | no reverse             | reverse second run             |

### Case 1 — both clicked pins are on the same Pin Path

1. Walk from the first pin (A) to the second (B) to build one contiguous **range**. On an **open** path there's exactly one way to do this (the direction that actually reaches B). On a **closed** path (circle, polygon, star, …) there are two arcs between A and B — both start at A, sharing endpoint B; picking between them is the tool's one source of ambiguity in this case (see "Disambiguation" below).
2. Split the range into two halves of length `floor(N/2)` each (`N` = range length): the first `floor(N/2)` elements, and the *last* `floor(N/2)` elements. If `N` is odd, the true middle pin belongs to neither half.
3. Interleave the two halves pin-by-pin (Zig-zag reverses the second half first; Parabolic doesn't). If `N` was odd, append the leftover middle pin alone at the end.

Worked examples (pins numbered 1..N along the range):

```text
Zig-zag,   N=10: [1,10,2,9,3,8,4,7,5,6]
Zig-zag,   N=11: [1,11,2,10,3,9,4,8,5,7,6]
Parabolic, N=10: [1,6,2,7,3,8,4,9,5,10]
Parabolic, N=11: [1,7,2,8,3,9,4,10,5,11,6]
```

### Case 2 — the clicked pins are on different Pin Paths

1. Each anchor pin independently walks its own path in one of two directions (`+1`/`-1`, wrapping if that path is closed). The number of pins available in a given direction (`remainingInDirection`) is the whole path if closed, or the count from the anchor to that end if open.
2. `L`, the pins actually used per path, is `min` of whatever each side's chosen direction allows — using every pin the shorter side has before it runs out.
3. Build one run of `L` pins per path, starting at its own anchor. Interleave the two runs (Zig-zag straight; Parabolic with the second run reversed, walking it from its far end back to its own anchor).

Worked example (path 1 has 10 pins numbered 1-10, path 2 has 11 pins numbered 11-21, first pin = 1, second pin = 11):

```text
Zig-zag:   [1,11,2,12,3,13,4,14,5,15,6,16,7,17,8,18,9,19,10,20]   (21 unused — path 1 ran out first)
Parabolic: [1,20,2,19,3,18,4,17,5,16,6,15,7,14,8,13,9,12,10,11]   (21 unused, same reason)
```

Choosing directions to maximize `L` (rather than always walking, say, "increasing index") is what makes this the fullest possible fill for whichever pair of pins was clicked — see "Disambiguation" for how the direction choice is actually made.

### Degenerate cases

- Adjacent pins on the same path (`N=2`), or a Case 2 pair where the anchor sits at the true end of an open path in the chosen direction (`L=1`), both collapse cleanly to a straight two-pin segment for either tool — no special-casing needed, the formulas above already produce this.
- The two pins being identical is rejected (nothing to build a sequence from).
- A symmetry-mirrored pin resolves to its real owning pin for hit-testing (same `nearestThreadInsertionPin` Thread Draw already uses), and every generated pin id in its walked sequence is re-mirrored back into that same physical copy — the same "stay within this instance" rule `computeNextPatternPinId` ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md)) already uses for pattern-follow.

## Click workflow

1. **Click 1** — the nearest pin (or mirror, resolving to its source) starts the draft (`startTwoPinDraft`). Nothing is computed yet.
2. **Click 2** — the nearest pin to the second click computes every valid candidate sequence (`chooseSecondPin`): `computeSamePathCandidates` if both pins share a Pin Path, else `computeCrossPathCandidates`.
   - **Exactly one candidate** (an open path in Case 1; or, in Case 2, a direction combination with nothing else that produces a distinct result) commits immediately — there's nothing to disambiguate.
   - **2+ candidates** populate the draft and wait for a 3rd click.
3. **Disambiguation (3rd click, only when needed)** — a closed path (Case 1) always offers both arcs; Case 2 offers up to 4 direction combinations whenever an anchor isn't pinned to a single forced direction. While ambiguous, moving the cursor live-previews whichever candidate's far endpoint is nearest the cursor (the same continuous preview-then-confirm shape as the Arc tool's bulge-side click), and a **free-position click anywhere** — it doesn't need to land on a pin — confirms the currently-previewed candidate (`resolveTwoPinDraft`).

```text
Click 1 (near a pin)
→ Start the draft with that pin as the anchor

Click 2 (near a pin, different from the anchor)
→ Compute every candidate sequence
  → 1 candidate: commit immediately
  → 2+ candidates: populate the draft, await the 3rd click

Click 3 (free position, only reached when 2+ candidates exist)
→ Commit whichever candidate the cursor was nearest

Esc / "Cancel" (radial menu)
→ Discard the draft at any stage

ArrowLeft / "Back" (radial menu)
→ Before candidates exist: cancel the whole draft (only the anchor was picked)
→ Once candidates exist: drop them, back to awaiting the second pin
```

Committing builds one `ThreadPath` from the chosen sequence via the same `createThreadPath` + `addThreadPathToLayers` + single `SetValueCommand<ThreadLayer[]>` shape `commitThreadDraft` ([12-thread-editor.md](./12-thread-editor.md)) already uses — one atomic undo step regardless of how many segments the sequence contains. Committing into a locked active Thread Layer discards the draft silently, same as Draw.

## Right-click menu while drafting

Right-clicking while a Zig-zag/Parabolic draft is in progress swaps Thread mode's normal tool slices (now `Draw`/`Zig-zag`/`Parabolic`/`Eraser`/`Segment`) for a draft-mode set, same shape as Thread Draw's own Cut/Back/Next swap ([25-radial-context-menu.md](./25-radial-context-menu.md)):

- Before a second pin is picked: **Back**, **Cancel**.
- Once 2+ candidates exist: **Cut** (commits the previewed candidate — same action as the 3rd click), **Back**, **Cancel**.

## Keyboard shortcuts

Per [34-keyboard-shortcuts.md](./34-keyboard-shortcuts.md), Thread tab bare-letter bindings gain two entries (both free in that namespace):

| Key | Tool |
|---|---|
| `Z` | Zig-zag |
| `P` | Parabolic |

`Escape` cancels and `ArrowLeft` steps back exactly like Thread Draw's own draft, registered inside `useTwoPinSequenceDrawing.ts`'s own `keydown` listener (not the central dispatcher) — the central dispatcher's arrow-key-pan fallback re-derives this hook's "would I act" condition the same way it already does for Thread Draw and the Path tool.

## Locked layers and other guards

Placing the anchor pin and picking a second pin are unaffected by layer lock state, same as Draw. Only the final commit is blocked by a locked active Thread Layer, discarding the draft silently. Switching Thread tool away from Zig-zag/Parabolic, or switching Editor mode away from Thread, while a draft is in progress discards it outright — the same non-undoable-transient-state cleanup as every other in-progress draft in this app (Thread Draw, the Path tool, Generator mode).

## Configuration

Both tools expose a small "next-draw settings" panel in `ThreadPropertiesPanel.tsx`, shown only while the matching tool is active — same treatment as `threadDefaults` (plain state, read at click-2 time, never routed through `HistoryStack`, since nothing about a committed `ThreadPath` needs to remember how it was configured).

| Field | Tool | Range | Default | Effect |
|---|---|---|---|---|
| Step A | both | 0–9 (integer) | 0 | Stride applied to side A (Case 1: firstHalf; Case 2: the first-clicked pin's own run) |
| Step B | both | 0–9 (integer) | 0 | Same, for side B |
| Full-fill | both | on/off | off | Use both of an anchor's directions/arcs instead of requiring a 3rd click to pick one |
| Cycles | Parabolic only | 1–20 (integer) | 1 | Enabled only while Full-fill is checked. Only has an effect on a same-CLOSED-path pair (see below) |

### step-by

Each side's ordered pin list is sub-sampled by its own **stride**: `stride = step + 1`. `step=0` (the default) keeps every pin — today's original shipped behaviour. `step=2` keeps every 3rd pin. A stride that doesn't land exactly on the far end of a side simply leaves the remainder unconnected — no snapping the last hop to force an endpoint.

Worked example (the case that defined this formula): two Pin Paths, 10 and 20 pins, `stepA=stepB=2`, Zig-zag, no full-fill:

```text
subA = [1,4,7,10]            (path A, stride 3)
subB = [11,14,17,20]         (path B, stride 3)
result: [1,11,4,14,7,17,10,20]
```

### full-fill

Full-fill doesn't introduce a new algorithm — it applies the **same** direction/arc extraction the tools already use (the one a 3rd click normally disambiguates between) to *both* options at once, instead of requiring the user to pick one:

- **Case 2 (different Pin Paths)**: without full-fill, an anchor pin sitting mid-path (not at a true endpoint) has two viable directions along its own path — today's existing 3rd-click disambiguation, unchanged. With full-fill, that anchor instead combines *both* directions into one run (every pin walking outward one way, then every pin walking outward the other way, anchor de-duplicated) — eliminating that anchor's own direction choice, so no 3rd click is needed for it. An anchor already at a true endpoint is unaffected either way (it only ever had one direction with real extent). The two anchors' resulting runs are still paired by simple interleave, truncated to whichever ends up shorter — full-fill's completeness comes entirely from each anchor using every one of its own reachable pins, not from extending the pairing itself past the shorter run.
- **Case 1 (same Pin Path), open path**: no effect — the second clicked pin already forces a single direction, so there's no direction/arc choice for full-fill to combine away.
- **Case 1 (same Pin Path), closed path, Zig-zag**: without full-fill, the two arcs between the clicked pins are separate 3rd-click candidates (existing behaviour). With full-fill, **both arcs are used** — the same unchanged firstHalf/secondHalf pairing the bounded (non-full-fill) case already runs is run once per arc, and each arc commits as its own **separate Thread Path** (not concatenated into one). Each arc's own pairing still starts with `(A, B)` as its first pair (exactly like today's single-arc result), then zigzags inward from there — full-fill just means neither arc is thrown away for the other, so no 3rd click is needed to pick one. Both Thread Paths are created together as one bundled undo step. They stay separate deliberately: a `ThreadPath` always renders as one continuous connect-the-dots line, so concatenating two independently-built arcs into a single `pinIds` array would draw a spurious straight segment wherever one arc's zigzag happens to end and the next arc's own `(A, B)` pair restarts — a real bug caught live (clicking two pins on a closed ring produced an unwanted long chord at exactly that boundary). One pass over both arcs is one **circle**; the identical pair of strands repeats `cycles` times — for Zig-zag this stays fixed at 1 (Zig-zag has no Cycles field at all).

- **Case 1 (same Pin Path), closed path, Parabolic**: a genuinely different shape from Zig-zag's — not the firstHalf/secondHalf split at all. Full-fill keeps the **same constant offset** between the clicked pins and just keeps walking: pin A and pin B both advance one step at a time around the whole ring in lockstep, producing one continuous Thread Path — the first pair is `(A, B)` exactly as clicked. The walk stops the moment EITHER anchor reaches or passes its own starting pin — that's one **circle**. A side doesn't have to land exactly back on its own start to count: if its stride skips past it, the pin it overshoots onto is the stopping point.

  For a side whose stride divides `pinCount` exactly, it always lands exactly on its own start — that hop is excluded (no redundant repeat of the pair). For a side whose stride does NOT divide `pinCount`, it never lands exactly on its own start — the hop where it first overshoots past it is included instead, since that pin was never otherwise visited. The walk runs for whichever side reaches this point first. After the last pair, the sequence ends on A's next pin (on an exact lap, A's own start), so the final B→A connecting segment is drawn and the lap closes.

  Confirmed against real examples: pins 125/22 on a 141-pin ring (Step A = Step B = 0, stride 1 for both — always divides exactly) gives `[125,22, 126,23, ..., 123,20, 124,21, 125]`, 141 pairs plus the closing pin; a 119-pin ring with Step A and Step B both set to skip 1 (stride 2, and `gcd(119,2)=1` — stride 2 still divides 119's own return length exactly, since both sides use the same stride) gives a full 119-pair walk. Pins 99/19 on that same 119-pin ring but with only Step B set to skip 1 (Step A=0/stride 1, Step B=1/stride 2 — 119 is odd, so stride 2 does NOT divide it exactly) stops at 61 pairs, not 119: B overshoots its own start on every lap and never lands on it exactly, so its own stopping hop is `ceil(119/2) + 1 = 61`, shorter than A's exact 119. Pins 124/15 on a 151-pin ring (Step A=0/stride 1, Step B=2/stride 3 — 151 is prime, so stride 3 doesn't divide it exactly either) stops at 52 pairs for the same reason (`ceil(151/3) + 1 = 52`). On a 10-pin ring with Step A=0/stride 1 and Step B=2/stride 3 (10 isn't a multiple of 3), B's walk is `2,5,8,1,4` — its last term, 4, is the pin it overshoots onto rather than landing back on its own start, 2.

  Reuses the existing `interleave` primitive (one modular-cycle walk extracted per anchor, independently strided, each run out to the SHARED longer cycle length rather than each to its own).

  Reaching/passing a side's own starting pin once is one **circle**; `cycles` extends the SAME walk until a side has reached/passed its own starting pin that many times — ONE continuous Thread Path, not `cycles` separate repeats (unlike Zig-zag's two-arc case above, which does use separate strands). `cycles=1`'s sequence is always an exact prefix of `cycles=2`'s, which is an exact prefix of `cycles=3`'s, and so on.

## Test Cases

```gherkin
Feature: Zig-zag / Parabolic tool sequence math

  Scenario: Zig-zag, same open path, mirrors outer-in
    Given a Pin Path with 10 pins and the Zig-zag tool active
    When the user clicks pin 1, then pin 10
    Then a Thread Path is created with pin order 1,10,2,9,3,8,4,7,5,6

  Scenario: Parabolic, same open path, ascending halves
    Given a Pin Path with 10 pins and the Parabolic tool active
    When the user clicks pin 1, then pin 10
    Then a Thread Path is created with pin order 1,6,2,7,3,8,4,9,5,10

  Scenario: Zig-zag, different paths, stops when the shorter path is exhausted
    Given Pin Path 1 has 10 pins and Pin Path 2 has 11 pins, Zig-zag tool active
    When the user clicks Path 1's pin 1, then Path 2's pin 1
    Then a Thread Path is created alternating both paths for 10 pins each (20 pins total), and Path 2's 11th pin is unused

  Scenario: Parabolic, different paths, second path walked in reverse
    Given the same two Pin Paths as above, Parabolic tool active
    When the user clicks Path 1's pin 1, then Path 2's pin 1
    Then the resulting Thread Path's Path-2 pins run from its far end back to the clicked anchor

  Scenario: Closed path requires a 3rd click to pick an arc
    Given a closed Pin Path (e.g. Circle) and the Zig-zag tool active
    When the user clicks two pins that are not adjacent
    Then the draft populates 2 candidates and waits for a 3rd, free-position click before committing

  Scenario: Open-path Case 1 commits without a 3rd click
    Given an open Pin Path (e.g. Line) and either tool active
    When the user clicks two pins
    Then the Thread Path commits immediately — there is only one valid direction

  Scenario: Adjacent pins degenerate to a straight segment
    Given any Pin Path and either tool active
    When the user clicks two adjacent pins
    Then the resulting Thread Path is just those two pins, identical for both tools

  Scenario: Escape cancels an in-progress draft
    Given a Zig-zag/Parabolic draft with an anchor picked but no second pin yet
    When the user presses Escape
    Then the draft is discarded and no Thread Path is created

  Scenario: ArrowLeft steps back from disambiguation to awaiting the second pin
    Given a draft with 2+ candidates populated
    When the user presses ArrowLeft
    Then the candidates are cleared and the draft returns to awaiting a second pin, keeping the same anchor

  Scenario: Committing on a locked Thread Layer discards the draft
    Given a draft has resolved to a single committable sequence and the active Thread Layer is locked
    When the commit is attempted
    Then no Thread Path is created and the draft is cleared

  Scenario: Switching Thread tool away mid-draft discards it
    Given a Zig-zag draft is in progress
    When the user picks a different Thread tool
    Then the draft is discarded with no commit attempt

  Scenario: step-by reproduces the worked example
    Given two Pin Paths of 10 and 20 pins, Zig-zag tool active, Step A and Step B both set to 2, Full-fill off
    When the user clicks Path 1's pin 1, then Path 2's pin 1
    Then the resulting Thread Path is [1,11,4,14,7,17,10,20]

  Scenario: Full-fill combines a mid-path anchor's two directions into one run
    Given Pin Path 1 has 10 pins and the user clicks its 5th pin (mid-path, not an endpoint), Pin Path 2's endpoint pin is the second click, Zig-zag tool active, Full-fill on
    When the draft is committed
    Then no 3rd click was needed, and Path 1's anchor pin's run walks outward one direction fully, then the other direction fully (anchor not duplicated)

  Scenario: Without full-fill, the same mid-path anchor still needs the 3rd click
    Given the same setup as above but Full-fill off
    When the user clicks the second pin
    Then multiple direction candidates are populated and a 3rd, free-position click is required before committing

  Scenario: Full-fill's outer pairing still truncates to the shorter run — no pins are appended past it
    Given Pin Path 1 has 10 pins (mid-path anchor) and Pin Path 2 has 20 pins (endpoint anchor), Zig-zag tool active, Full-fill on
    When the draft is committed
    Then the resulting Thread Path pairs up to Path 1's 10-pin combined run, and Path 2's pins 11-20 never appear anywhere in it

  Scenario: Without full-fill, a closed-path pair still needs the 3rd click to pick one arc
    Given a closed Pin Path and either tool active, Full-fill off
    When the user clicks two non-adjacent pins
    Then 2 candidates are populated (one per arc) and a 3rd click is required — unchanged from before full-fill existed

  Scenario: Zig-zag full-fill on a closed-path pair uses both arcs, committed as two separate Thread Paths
    Given a closed Pin Path and the Zig-zag tool active, Full-fill on
    When the user completes a draft between two of its pins
    Then no 3rd click is needed, and TWO Thread Paths are created together as one undo step — the short arc's own bounded-arc pairing, and the long arc's own bounded-arc pairing — each starting with (A,B) as its first pair, with no segment connecting the two

  Scenario: Zig-zag Cycles repeats the pair of strands
    Given the same closed-path Zig-zag setup as above but Cycles set to 3
    When the draft is committed
    Then 6 Thread Paths are created (3 repetitions of the short-arc/long-arc pair), all as one undo step

  Scenario: Parabolic full-fill on a closed-path pair is a single continuous constant-offset walk, not the two-arc shape
    Given a closed Pin Path and the Parabolic tool active, Full-fill on, Cycles set to 1
    When the user clicks two of its pins, A then B
    Then no 3rd click is needed, and ONE Thread Path is created whose first pair is (A,B), continuing with both pins advancing together one step at a time around the whole ring, stopping once either pin reaches or passes its own starting position

  Scenario: Parabolic Cycles extends the same walk instead of repeating it as separate strands
    Given the same closed-path Parabolic setup as above but Cycles set to 3
    When the draft is committed
    Then still ONE Thread Path is created, whose sequence starts with exactly the Cycles=1 sequence and continues until a side has reached/passed its own starting pin 3 times total

  Scenario: Equal strides coprime with the pin count visit every pin before repeating, not a truncated half
    Given a closed Pin Path with an odd pin count and the Parabolic tool active, Full-fill on, Step A and Step B both set to skip 1 pin (stride 2)
    When the user completes a draft between two of its pins
    Then the resulting walk has as many pairs as the pin count, not roughly half of it

  Scenario: The walk stops as soon as either anchor reaches or passes its own starting pin
    Given a closed Pin Path and the Parabolic tool active, Full-fill on, Step A and Step B set to different strides
    When the user clicks pin A then pin B
    Then ONE Thread Path is created stopping at whichever anchor reaches or passes its own starting pin first — exactly, if its stride divides pinCount, or by overshooting onto a not-yet-visited pin otherwise

  Scenario: Cycles has no effect outside a same-closed-path pair
    Given two different Pin Paths, Parabolic tool active, Full-fill on, Cycles set to 5
    When the user completes a draft between them
    Then the resulting Thread Path is identical to the same draft with Cycles left at 1
```

## Scope limits

- No pattern-follow ("Next") analogue — unlike Thread Draw's `ArrowRight` extrapolation ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md)), the whole point of these tools is computing the sequence from a fixed rule, not extrapolating from click history.
- The 3rd-click disambiguation always offers every geometrically valid candidate (up to 4 in Case 2); it does not attempt to auto-pick or hide "less useful" short candidates (e.g. an anchor pinned near an open path's own end) — the live preview and hover-nearest selection make a bad choice visually obvious before it's confirmed.
- No dedicated Selection-panel fields for the resulting Thread Path — it's a plain `ThreadPath` like any other, edited afterward through the existing Thread Select tool ([27-thread-select-tool.md](./27-thread-select-tool.md)).
