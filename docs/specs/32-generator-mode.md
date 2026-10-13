# 32 — Generator Mode

## Purpose

Add a sixth Editor mode, **Generate**, that auto-creates pins and threads from a chosen mathematical pattern and its parameters — instead of drawing shapes and threading pins by hand. The user picks a pattern from a dropdown, sets its parameters, and clicks **Generate** to preview a pin+thread pattern on the board; **Re-generate** replaces that preview; **Confirm** turns the preview into two brand-new, permanent Pin/Thread Layers.

This spec was researched against a read-only analysis of a competitor app's 19 built-in generator patterns (`~/projects/pocs/research_string_art`, not part of this repository). The finding that shaped this spec's scope: **all 19 researched patterns are pure parametric math** — pins built from circle/line/polygon primitives (single or composite: e.g. a hexagram from two overlapping triangles) or, for 2 of the 19, sampled along a closed-form parametric curve. None require a hand-drawn/freehand fallback in the "couldn't figure out the shape" sense, and none require image/photo input or greedy optimization. This spec ships the **engine** (pattern registry + Generate/Re-generate/Confirm workflow), a first **curated set of 5 patterns** (M25), and the remaining **14 patterns** (M26) — all 19 researched patterns are now built (see Scope below).

## Pin-count vs. pin-spacing

Every other pin-creating path in this app ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md)) is **spacing-driven**: the user requests a physical gap (e.g. "1cm"), and the engine derives however many pins that implies. Generator patterns are **count-driven** instead (Mandala's `n`, Star's `circleNails`) — a pattern's thread-index math assumes an *exact* pin count, so an off-by-one would silently misroute threads. (Star of David is the one pattern that sidesteps this entirely — its nested-polygon tiles are built directly from computed vertex points, the same "pins ARE the computed positions" technique Spirals uses, since a fixed vertex count per nested level matters more than matching a physical spacing.)

`spacingForPinCount(length, count)` (`src/domain/paths/distribution.ts`) bridges the two: it derives a `requestedSpacing` that makes `distributeClosedPath`/`distributePathPerVertex`'s existing closest-interval-count algorithm land on *exactly* `count`, not just close to it, by biasing the spacing a hair smaller than the naive `length / count` (a floating-point safety margin — re-dividing the naive value can land a hair *below* the integer count and round down). Every generator pattern in this spec goes through this helper before calling the existing `createPinPath`, so pin distribution stays entirely inside the established engine — no new distribution algorithm was written.

## The 5 curated patterns

| Pattern | Pin layout | Threading | Parameters |
|---|---|---|---|
| **Mandala** | One circle, `n` pins | For layer `L` (angular shift `floor(n/layers)·L`): for every pin `i`, connect it to pin `(i·base) mod n`. Each layer is its own Thread Path/colour. | `n` (3–400), `base` (2–99), `layers` (1–20) |
| **Star** | A "spoke wheel," not a pointed-polygon outline: `starPoints` straight spokes (`"line"` `PinPathGeometry`, `sideNails` pins each) radiating from the board centre to the rim, plus one outer circle whose pin count (`starPoints·(sideNails-1)`) is derived from `sideNails` rather than set independently | THREE interleaved zigzag Thread Paths per neighbouring pair: 2 spoke↔circle zigzags per point (`starSpokeCircleZigzag`, one sweeping toward each neighbouring point, both pivoting on the shared boundary pin between them) + 1 direct spoke-to-spoke zigzag per adjacent pair (`starAdjacentSpokeZigzag`, bypassing the circle entirely) — a denser, more textured elaboration of the same curve-stitch idea Star of David and the earlier single-fan design already use. `3·starPoints` Thread Paths total | `sideNails` (2–200), `starPoints` (3–20), `starOuterRatio`/`starInnerRatio` (rim/hub radius as fractions of the board's inscribed radius), `rotation` |
| **Freestyle** | Up to 3 independent circles, each positioned/sized as a fraction of the board | Round-robin across every *enabled* circle: visit pin `r mod count` of each, for `r` from 0 to the largest enabled circle's pin count | Per circle: enabled, `nails`, `radiusRatio`, `centerXRatio`, `centerYRatio` |
| **Star of David** | 7 tiles — 1 central hexagon + 6 equilateral triangles centred on its 6 edges (each triangle's centre 30° off the nearest hexagon vertex, at radius `2·R0/3`; hexagon vertex radius `R0/√3`; triangle vertex radius `R0/3`, where `R0` is the board's inscribed radius) — each tile expanded into `depth` nested, shrinking-and-twisting copies of itself (`nestedPolygonLevels`), pins at every vertex of every nested copy | Per tile, per side: an alternating-parity fan across all `depth` nested levels (`connectTwoSidesLocalIndices`) — one Thread Path per `(tile, side)`, 6 + 6·3 = 24 total | `depth` (1–40, nested levels per tile), `layerAngle` (0.02–0.15, twist/shrink rate per level), `rotation`, `mirrorTiling` (flips the triangles' twist direction) |
| **Spirals** | `arms` polar-curve arms, `nailsPerSpiral` pins each, sampled directly (not resampled) into a `"freehand"` `PinPathGeometry` — the one pattern demonstrating the curve-sampled/freehand case | Sequential: connect every sampled pin in generation order (arm-major → inner, radial-step-major → outer), one continuous Thread Path | `arms` (2–20), `nailsPerSpiral` (3–300), `totalAngleTurns`, `rotation` |

All five threading rules reduce to pure index arithmetic over already-built pins (`src/domain/generator/roundRobin.ts`, `mandala.ts`, `nestedPolygon.ts`), matching the research finding that every one of the 19 researched patterns threads this way — never by re-deriving geometry mid-traversal.

**Deliberate simplification, stated plainly:** these are original formulas *inspired by* the researched competitor's pattern names and general shapes, not byte-identical reproductions of its exact traversal (which, for e.g. Star, branches on odd/even side count and uses reflected round indices — extra complexity that changes which of two visually similar conventions is used, not whether the pattern is achievable). Mathematical techniques like "connect pin i to pin i·k mod n" are generic string-art methods documented across many hobbyist sources, not unique IP.

## The 14 M26 patterns

Built the same milestone after M25 shipped, reusing M25's engine and primitives wherever a pattern's general technique matched one already built (`buildTileFans`/`tileRingLayout` generalized from Star of David's own tile-ring construction; `twoRayZigzag` generalized from Star's own spoke↔circle zigzag), and adding small original functions only where nothing existing fit. Same approach as the 5 curated patterns above: each is an *original* implementation of the researched pattern's general documented technique, not a byte-identical reproduction — a few (Flower of Life, Crosses, Assymetry) ship deliberately simplified versions, called out below.

| Pattern | Pin layout | Threading | Parameters | Scope note |
|---|---|---|---|---|
| **Wave** | One circle, `n` pins (same primitive as Mandala) | Generalizes the classic modular-multiplication 4-point cell (`waveLayerSequences`: for every odd `k` in the layer's fill range, `k, (k·base mod n), (k·base mod n)+base, k+1`, all shifted by the layer's own rotating offset) to a *partial* pass per layer — only `layerFill` of the `n` positions, starting at a rotating `layerSpread`-position offset each layer, which is what produces the rippling "wave" look | `n` (3–400), `base` (2–99), `layers` (1–20), `layerFill` (1–400), `layerSpread` (0–200) | — |
| **Hexagon Spades** | The same 6-triangle ring as Star of David's own tiles (`tileRingLayout(6, 3, …)`, same proportions — `R0/3` triangle radius, `2·R0/3` helper radius), with no central hexagon hub | One adjacent-side fan per tile side (`connectTwoSidesLocalIndices`, shared with Star of David) — 6 tiles × 3 sides = 18 Thread Paths | `depth` (1–40), `layerAngle` (0.02–0.15), `rotation`, `mirrorTiling` | — |
| **Dance of Planets** | Two concentric rings, each independently a circle or `regular-polygon` Pin Path | `roundRobinSequence([outerCount, innerCount])`, repeated `rounds` times; `reverse` walks the inner ring backward, which is what makes the two rings appear to counter-rotate | `outerType`/`outerNails`/`outerSides`, `innerType`/`innerNails`/`innerSides`/`innerSizeRatio`, `rounds` (1–20), `reverse`, `rotation` | — |
| **Sun** | Reuses Star's own spoke-wheel construction verbatim for the base shape, plus `layers` extra shrinking concentric circles | Star's own 3-zigzags-per-point threading, unchanged, plus one simple base-2 modular self-weave per extra ring | Star's own params + `layers` (0–10), `layerSpread` (0–0.3) | Simplified: an original reinterpretation reusing this app's own spoke-wheel Star rather than the researched pattern's own separate pointed-star-plus-backdrop-fan construction; extra "rays" are decorative shrinking rings with a light self-weave |
| **Vortex** | `layers` nested/inscribed regular polygons (`nestedPolygonLevels`, the shrink-and-twist primitive Star of David's tiles use), each its own native `regular-polygon` Pin Path with `nailsPerSide` vertex-anchored pins per side | Within each level, side `s`'s pin `i` connects to side `(s+1) mod sides`'s pin `i` — chaining that same-index chord across all `sides` sides into one closed loop per `i` reproduces the full chord set with no extra segments (`layers·nailsPerSide` Thread Paths total, one colour per level) | `sides` (3–10), `nailsPerSide` (3–200), `layers` (1–40), `layerAngle` (0.01–0.2), `rotation` | — |
| **Polygon** | One `regular-polygon` Pin Path, `sides·nailsPerSide` pins (vertex-anchored: side `s`'s pins are local indices `s·nailsPerSide … s·nailsPerSide+nailsPerSide-1`) | `sameIndexZigzag` between side `s` and side `(s+bezierStep) mod sides` — the 4-point cell `A_k, B_k, B_k+1, A_k+1` for advancing even `k` (same-index chords across the two sides, not reversed pairing — this is what a regular polygon's own side-to-side envelope curve actually uses), one Thread Path per side | `sides` (3–20), `nailsPerSide` (2–100), `bezierStep` (1–19), `rotation` | — |
| **Flower** | `layers` rotated copies of Polygon's own geometry, same radius, increasing rotation offset per copy | `sameIndexZigzag` per layer per side (reuses Polygon's own corrected weave) | `sides` (3–20), `nailsPerSide` (2–100), `layers` (1–20), `rotation` | — |
| **Assymetry** | One circle + one spoke line (the same line-geometry construction Star's spokes use), spoke nail count derived to match the circle's own physical spacing (`round(circleNails / 2π)`) | `asymmetryZigzag`: a pivot point advances by a fixed `start` offset every other step, alternating with a second pointer that counts up from 0 — each successive chord's far end creeps forward by a constant offset (not a constant ratio, which is what makes the fan read as asymmetric) | `circleNails` (3–400), `startFraction`/`endFraction` (0–1), `reverse`, `rotation` | Simplified: a single configurable pass, not the researched pattern's 3 parallel default passes |
| **Spiral** | One circle, `n` pins | `spiralDecayingWalk`: a pointer oscillates forward by the current span then backward by span-1, repeatedly, while the span slowly shrinks from `innerLength` down to 0 (one decrement every `2·repetition-1` oscillations) — the back-and-forth oscillation is what makes the accumulated chords cross back over themselves as they tighten | `n` (10–400), `repetition` (1–20), `innerLength` (1–100, the starting span/"thickness") | — |
| **Maurer Rose** | Curve-sampled points from the classic public Maurer-rose formula (`maurerRosePoints`; Peter M. Maurer, 1987 — a well-known general construction, not any app's proprietary algorithm), built directly into pins (freehand-bypass, same architecture as Spirals) | Sequential: connect every sampled point in generation order, one continuous Thread Path | `N` (1–30, petal count), `maxSteps` (10–720), `angleDegrees` (1–180), `rotation` | — |
| **Comet** | One circle, `n` pins | `cometLayerSequences`: `layers` offset-alternation passes, connecting `i` to `i+d`; the offset `d` shrinks linearly per layer (by `layerDistance`, floored at 1) — each layer's own run length is DERIVED from `d` (`n-d+1` steps), not set independently, which is what gives the tapering "comet tail" look | `n` (10–400), `layers` (1–40), `firstLayerSize` (2–200, the starting offset), `layerDistance` (1–200), `rotation` | — |
| **Flower of Life** | The same 6-triangle ring as Hexagon Spades, plus an optional outer ring circle with its own base-`ringBase` modular self-weave | Same tile fans as Hexagon Spades (18 Thread Paths), plus 1 more when the ring is enabled | `depth` (1–40), `layerAngle` (0.02–0.15), `rotation`, `ringEnabled`, `ringNails` (3–400), `ringBase` (2–99) | Simplified: the researched pattern is a `6·levels²`-tile hex-grid; this ships a fixed 6-tile ring + optional ring, stated plainly |
| **Crosses** | The real fixed 10-line arrangement: a central spine split into 4 nail-bearing segments, plus 3 rows of 2 arm lines straddling the gaps between segments (4+6=10 lines) | `crossesWeave` — per the source's own 10 (spine-segment, row) pairs, an alternating-side zigzag between one spine segment and its row's two arms, spine index counting up or down depending on the pair — one Thread Path per pair | `nailsPerLine` (2–100), `orientation` (vertical/horizontal), `gap` (0–1, doubles as the source's own default-locked length/width gap), `sidesRotation` (tilts row 0's and row 2's arms only, pivoting on each arm's inner end; centre row never rotates) | Simplified: no canvas-autofit bounding-box growth for a large `sidesRotation` (always builds directly against the board's inscribed radius, like every other pattern here); the source's colour-banding, fine-control, and asymmetric-centre options are not ported |

**Post-ship correction (Star of David):** the first implementation modelled this pattern as two flat overlapping triangles round-robin-threaded together — topologically wrong, not just visually plainer. A side-by-side comparison against a live reference render (the same competitor app this spec's research was based on) showed the real construction is 7 independently-threaded nested-polygon tiles (1 hexagon + 6 triangles), each spiralling inward through `depth` shrinking/twisting copies of itself — that's what produces the dense inward swirl, not a flat outline. Re-derived from first principles and cross-checked numerically against the reference render's actual nail coordinates (radii and angles only — no source code or artwork was copied into this codebase): the outer tip radius equals the board's inscribed radius `R0` exactly, the central hexagon's own vertex radius is exactly `R0/√3`, and each triangle tile sits exactly 30° off the nearest hexagon vertex (centred on a hexagon edge) — all three facts matched this rebuild's formulas before any code changed to fit them.

**Post-ship removal, then re-addition (Lotus, both 2026-09-18):** shipped in M26, then rebuilt multiple times during the post-ship correction pass without reaching a correct result, and removed from the registry entirely rather than ship an incorrect pattern. Later the same day, re-implemented from a more precise source-backed mathematical derivation (`~/projects/pocs/research_string_art/docs/lotus-pattern-generation.md`, cross-referencing the researched `Lotus.pattern.ts`'s `getPatchColor`/`drawPatch`/`drawStrings`/`generatePatches`/`getNails`/`getStepCount` functions) as small, independently unit-tested pure functions (`src/domain/generator/lotus.ts`) rather than one large routine, specifically so the index arithmetic could be checked for internal consistency instead of judged by eye alone — the earlier attempts' failure mode. Live-verified via Playwright across default/no-removal/no-centre/single-centre/radial-colour parameter combinations before being judged correct this time (see Lotus section below).

### Lotus

**Pin layout:** `sides` overlapping "petal" circles arranged around a helper circle (`lotusPetalCenter`), each carrying `N` nails (`lotusFit`'s fitted nail count) on a full or partial arc — removing sections (`removeSections`, an integer 0..`ceil(sides/2)-3`) shrinks each petal circle's visible arc and enlarges (`fit`s) the whole construction back up to size, exactly mirroring the source's own "remove, then re-fit" behaviour. `renderCenter` adds an optional centre anchor: off = none, on with `centerRadius=0` = a single centre pin, on with `centerRadius>0` = an `sides`-nail centre circle sized as a fraction of how far the outer patches reach inward (`lotusMaxCenterRadius`). Pins are built directly from these exact formulas (freehand-bypass, same architecture as Spirals/Comet/Maurer Rose), since a partial arc's endpoint-inclusive spacing rule doesn't match this app's own arc-length pin distribution.

**Threading:** one Thread Path per `(petal circle, section)` "patch" (`lotusGeneratePatches`/`lotusDrawPatch`) — each patch is two fans of nails converging on a single shared target nail (the previous petal circle, or the centre anchor for the innermost patch). Since a fan with more than 2 spokes has no single trail through it without revisiting the shared point, each patch's Thread Path revisits its target between every consecutive source nail — visually identical to independent segments (same straight lines), just grouped into one recolourable run per patch (this engine's established "one Thread Path per shared colour" convention), at the cost of roughly doubling the reported thread length for patches with more than 2 sources. `radialColor` changes which patches share a colour (by section instead of by petal circle) without changing the patch/thread count.

**Parameters:** `sides` (5–64), `density` (1–30), `rotation`, `removeSections` (0–1, mapped to an integer count), `renderCenter`, `centerRadius` (0–1), `radialColor`.

**Scope notes:** this app has no canvas-margin concept (unlike the reference's absolute-pixel margin), so margin is dropped entirely — the fit ratio is algebraically independent of margin/radius once margin is 0, so this only affects the exact fitted nail count, not the pattern's shape. The reference's "Render center nails" toggle is also dropped: per the source's own documentation it hides some interior petal nails from rendering without changing which strings are drawn — a pure cosmetic detail, not part of the pattern's shape, so this app always renders every computed petal nail.

## Draft / Confirm state machine

- **Generate** (`EditorStore.generatePattern(params, colours)`): builds a fresh `{ params, pinPaths, threadPaths }` draft from the current pattern/parameters/palette, replacing any existing draft outright. **Non-undoable** — same transient-interaction-state treatment as an in-progress Thread Path draft ([12-thread-editor.md](./12-thread-editor.md)): nothing here is committed to the document yet, so there is nothing meaningful to step back to mid-draft. Requires an explicit click the first time (the `Generate` button, shown only while no draft exists) — editing fields before that first click only changes local UI state, nothing is computed yet.
- **Live auto-apply (no "Re-generate" button).** Once a draft exists, `Generate` is replaced by a short "updates live" hint: any later pattern switch, parameter edit, or palette change re-runs `generatePattern` automatically, debounced ~300ms so a fast run of edits (typing a multi-digit number, several palette clicks) only recomputes once, from the final values — never on every intermediate keystroke. Fully replaces the previous draft each time; nothing from the old draft survives. (Earlier versions of this spec had an explicit `Re-generate` button doing the same thing on click; removed in favour of live auto-apply per direct user request, since every pattern here is a pure deterministic function of its parameters — there's never a reason to keep stale output on screen instead of the current settings' real result.)
- **Preview**: while a draft exists, its pins and threads render on the canvas at reduced opacity (`GeneratorPreviewOverlay`, reusing the existing `PinPathVisual`/`ThreadPathVisual` renderers) so it visibly reads as "not yet part of the document," on top of every real layer.
- **Confirm** (`EditorStore.confirmGeneratedPattern()`): the **only undoable step** in the whole flow. Always creates two **brand-new** permanent layers (named `"Generated — <Pattern Name>"`) holding the draft's pin/thread paths, appended in one bundled `SetValueCommand<{pinLayers, threadLayers}>` — a single Undo afterward removes both layers together. Confirm never merges into or otherwise touches an existing layer, so there is no locked-layer check to make. The newly created layers become the active Pin/Thread layers afterward (a non-undoable pointer switch, same precedent as `addPinLayer`/`addThreadLayer` switching the active layer right after their own undoable creation).
- **Leaving Generator mode** with an uncommitted draft discards it (`EditorStore.setMode`) — nothing was ever committed, so there is nothing to lose. Re-entering Generator mode later never resurrects a discarded draft.
- Loading a project (Open / autosave restore) also discards any in-progress draft, same as it already does for an in-progress Thread Path draft.

Pattern/parameter *selection* itself (which pattern is picked, what its fields currently say) is ordinary local UI state in `GeneratorPanel`, not document state — it resets on Re-generate anyway (a param edit is meant to change the next generated result) and is discarded along with the draft on leaving Generator mode, so there's nothing here worth persisting through `EditorStore`.

## Multicolor

`GeneratorPanel` holds an editable colour palette — one swatch per colour, plus `+`/`−` buttons to add/remove the last colour — analogous to `ThreadPropertiesPanel`'s existing 1/2/3-colour picker but variable-length rather than fixed at 3, since a generator pattern's useful colour count varies per pattern (and, for Mandala, per parameter).

- **Cap, not a fixed number.** `maxGeneratorColours(params)` (`src/application/document/generator/generatorPatterns.ts`) returns the largest number of colours a pattern's *current* parameters can actually put to use — the count of independent Thread Path "runs" it will produce:
  - **Mandala**: `layers` (each layer is already its own Thread Path).
  - **Star of David**: a fixed `24` (6 hexagon sides + 6 triangles × 3 sides), independent of `depth`/`mirrorTiling`.
  - **Star**: `3·starPoints` — 2 spoke↔circle zigzags + 1 adjacent-spoke zigzag per point (see the pattern table above).
  - **Freestyle, Spirals, Dance of Planets, Assymetry, Spiral, Maurer Rose**: `1` — each threads as one continuous Thread Path; splitting any of them into independently-coloured runs would change what they draw, not just how they're coloured, so a 2nd colour would never be used by anything.
  - **Wave, Vortex, Comet**: `layers` (each layer/level is its own Thread Path, same rule as Mandala).
  - **Hexagon Spades**: a fixed `18` (6 tiles × 3 sides). **Flower of Life**: `18 + 1` when its optional outer ring is enabled, else `18`.
  - **Sun**: `3·starPoints + layers` (Star's own run count plus one extra ring per `layers`). **Polygon**: `sides`. **Flower**: `sides·layers`. **Crosses**: a fixed `10` (one per spine-segment/row pair).
  - **Lotus**: `sides` (one colour group per petal circle) when `radialColor` is off, or the removable-section span (`ceil(sides/2) - removedSections - (renderCenter ? 1 : 2)`) when it's on — one Thread Path per `(petal circle, section)` patch, grouped into a colour by whichever of the two the current toggle picks.
  - `+` is disabled once the palette reaches this cap; `−` is disabled at 1 colour (a pattern always has at least one) — both stay rendered per [18-design-system.md](./18-design-system.md)'s standard disabled-button treatment, never unmounted.
- **Assignment rule: cycle by run index.** Run `i`'s Thread Path gets `colours[i % paletteLength]` as its *single* colour — never the whole palette handed to one Thread Path (that would render as a multi-strand twist within one run, per [12-thread-editor.md](./12-thread-editor.md)'s existing 1/2/3-colour twist rendering, a different feature). This is "each different colour is a different thread": a colour is only ever applied to a whole separate Thread Path, never blended into a shared multi-strand twist.
- The palette is `GeneratorPanel`'s own local UI state, passed explicitly into `EditorStore.generatePattern(params, colours)` — **not** read from or written to `state.threadDefaults.colours` (the global Thread-mode drawing default), so picking Generator colours never leaks into the next hand-drawn Thread Path's colour, and vice versa.
- Switching pattern, or lowering a parameter the cap depends on (e.g. Mandala's `layers`), clamps the *displayed and generated* palette down to the new cap without discarding hidden entries — raising the parameter back up restores the colours already picked rather than re-rolling from scratch.
- A new swatch (`+`) is seeded with the next colour from a small built-in preset (same auto-pick precedent as `ThreadPropertiesPanel`'s `PALETTE`), not left blank or repeating the previous swatch.

## Panel controls

- Every pattern-parameter field renders as a Slider ([18-design-system.md](./18-design-system.md) §Slider (range input)) rather than a plain number box — bounds and steps are unchanged from the original number inputs, only the control shape changed.
- The colour palette section is a Fieldset/legend group ([18-design-system.md](./18-design-system.md) §Fieldset/legend grouping): the section title and the `+`/`−` buttons share the legend's first row (buttons right-aligned), with the colour swatches in the row(s) beneath.
- A thread-width Slider (0.5–5, step 0.5) sits below the colour section, writing directly to `state.threadDefaults.width` via `store.setThreadProperty({ width })` — the same shared state `ThreadPropertiesPanel`'s own width field edits in Thread mode. This is **intentionally shared global state, not generator-scoped**: `generatePattern` already always inherits `threadDefaults.width` unchanged (only `colours` is generator-local), so exposing width here is just a second entry point onto the same default, not a new per-generation parameter.

## Scope

- **M25 shipped 5 patterns** (Mandala, Star, Freestyle, Star of David, Spirals) — chosen to exercise every engine capability: single-shape modular math, a 2-shape composite, an N-shape composite, and the curve-sampled/freehand case. **M26 shipped 13 more** (Wave, Hexagon Spades, Dance of Planets, Sun, Vortex, Polygon, Flower, Assymetry, Spiral, Maurer Rose, Comet, Flower of Life, Crosses), and **Lotus** was added afterward following an initial removal (see the Lotus section above) — all 19 researched patterns are now built.
- **No image/photo-to-string-art mode** is in scope or was requested — the competitor research confirms none of its 19 patterns are image-based either. That would be a categorically different feature (image processing + greedy chord selection), not an extension of this engine.
- Confirm always creates new layers; it never offers to merge a generated pattern into an existing layer. A user who wants that can do it manually afterward with the existing Merge action ([26-edit-mode-multi-select.md](./26-edit-mode-multi-select.md)).
- The Generator mode's pins/threads are not integrated with Symmetry ([06-symmetry.md](./06-symmetry.md)) — patterns construct their own composite/rotated geometry directly and are not eligible for the Symmetry panel's mirror/radial config (same as every other non-Pin-mode object in the app).

## Test Cases

```gherkin
Feature: Generating a pattern

  Scenario: Generate builds a draft with the correct pin count
    Given Generator mode is active with the Mandala pattern and n=180
    When the user clicks Generate
    Then a draft with exactly 180 pins on one circle is created
    And the draft is rendered as a preview on the canvas
    And the action is not recorded in the undo history

  Scenario: The Generate button disappears once a draft exists
    Given no draft currently exists
    Then a "Generate" button is shown and no draft/preview exists
    When the user clicks it
    Then the "Generate" button is replaced by a live-update hint and a Confirm button

  Scenario: Editing a field live auto-applies (debounced), fully replacing the previous draft
    Given a Mandala draft with n=180 exists
    When the user changes n to 60
    Then the draft still has 180 pins immediately (not yet applied)
    When ~300ms pass with no further edits
    Then the draft now has exactly 60 pins
    And none of the previous 180 pins remain in the draft

  Scenario: A fast run of edits only regenerates once, from the final value
    Given a draft exists
    When the user changes a field's value three times within 300ms of each other
    Then only one regeneration happens, using the last value typed

  Scenario: Composite patterns build multiple Pin Paths in one draft
    Given Generator mode is active with the Star pattern
    When the user clicks Generate
    Then the draft contains one circle Pin Path and starPoints spoke (line) Pin Paths

  Scenario: Each point is woven from three local zigzags, not one shared weave
    Given Generator mode is active with the Star pattern (starPoints=5, sideNails=8)
    When the user clicks Generate
    Then the draft has exactly 15 Thread Paths (3 per point)
    And each point's 2 spoke↔circle Thread Paths only ever touch that spoke's pins and circle pins
    And each point's adjacent-spoke Thread Path only ever touches its own two neighbouring spokes

  Scenario: Spirals places pins via formula, not distributeClosedPath resampling
    Given Generator mode is active with the Spirals pattern (arms=3, nailsPerSpiral=80)
    When the user clicks Generate
    Then the draft's freehand Pin Path has exactly (80-1)*3 pins
    And each pin sits at its computed radius/angle, unmoved by arc-length resampling

Feature: Multicolor palette

  Scenario: The palette starts at 1 colour and Add is capped by the pattern's run count
    Given Generator mode is active with the Mandala pattern and layers=1
    Then exactly 1 colour swatch is shown
    And the Add-colour button is disabled

  Scenario: Raising a cap-driving parameter enables adding more colours
    Given Generator mode is active with the Mandala pattern
    When the user sets layers to 3
    Then the Add-colour button is enabled
    And clicking it up to 3 times adds up to 3 swatches, then disables Add again

  Scenario: Remove always drops the last swatch and stops at 1
    Given the palette has 2 colours
    When the user clicks Remove
    Then the palette has 1 colour
    And the Remove button is now disabled

  Scenario: Switching to a single-run pattern clamps the visible palette to 1
    Given the palette has 3 colours under the Mandala pattern
    When the user switches to the Star pattern
    Then only 1 colour swatch is shown

  Scenario: Generate colours each run from the palette, cycling by index
    Given Generator mode is active with the Mandala pattern, layers=3, and a 2-colour palette
    When the user clicks Generate
    Then the 3 generated Thread Paths' colours are [palette[0], palette[1], palette[0]]

Feature: Confirming a generated pattern

  Scenario: Confirm creates exactly 2 new permanent layers, as one undo step
    Given a generated draft exists
    When the user clicks Confirm
    Then a new Pin Layer and a new Thread Layer are appended to the document
    And the draft is cleared
    And the live-update hint is replaced by a "Generate" button again
    And the Confirm button disappears
    And a single Undo removes both new layers together

  Scenario: Confirm never modifies an existing layer
    Given the active Pin Layer already contains hand-drawn pins
    And a generated draft exists
    When the user clicks Confirm
    Then the active Pin Layer's existing content is unchanged
    And the generated pins land in a brand-new layer instead

  Scenario: The newly confirmed layers become active
    Given a generated draft exists
    When the user clicks Confirm
    Then the new Pin Layer becomes the active Pin Layer
    And the new Thread Layer becomes the active Thread Layer

Feature: Discarding an uncommitted draft

  Scenario: Switching Editor mode away discards the draft
    Given a generated draft exists in Generator mode
    When the user switches to Edit mode
    Then the draft is cleared
    And no partial layers were created

  Scenario: Re-entering Generator mode does not resurrect a discarded draft
    Given a draft was discarded by leaving Generator mode
    When the user switches back to Generator mode
    Then no draft or preview is shown

Feature: Generating the M26 patterns

  Scenario: Wave builds a partial-coverage 4-point-cell traversal per layer
    Given Generator mode is active with the Wave pattern (n=100, layers=4, layerFill=20)
    When the user clicks Generate
    Then the draft has one circle Pin Path with 100 pins
    And the draft has 4 Thread Paths, each with one 4-point cell per odd k in [1, layerFill)

  Scenario: Hexagon Spades builds 6 triangle tiles with no hub
    Given Generator mode is active with the Hexagon Spades pattern (depth=8)
    When the user clicks Generate
    Then the draft has 6 Pin Paths (no central hexagon)
    And the draft has 18 Thread Paths (6 tiles * 3 sides)

  Scenario: Dance of Planets round-robins two concentric rings for `rounds` passes
    Given Generator mode is active with the Dance of Planets pattern (outerNails=120, innerNails=60, rounds=2)
    When the user clicks Generate
    Then the draft has 2 Pin Paths and exactly 1 Thread Path
    And reverse=true walks the inner ring's pin order backward

  Scenario: Sun layers extra shrinking rings on top of Star's own shape
    Given Generator mode is active with the Sun pattern (layers=3)
    When the user clicks Generate
    Then the draft has Star's own Pin/Thread Paths plus 3 more of each

  Scenario: Vortex weaves same-index chords across every side of each nested level
    Given Generator mode is active with the Vortex pattern (sides=5, nailsPerSide=8, layers=6)
    When the user clicks Generate
    Then the draft has 6 regular-polygon Pin Paths (one per level), each with 40 pins (sides*nailsPerSide)
    And the draft has 48 Thread Paths (layers*nailsPerSide), each a closed loop of sides+1 pin ids
    And each loop visits side s's pin i then side (s+1 mod sides)'s pin i, for a fixed i

  Scenario: Polygon curve-stitches each side against another side
    Given Generator mode is active with the Polygon pattern (sides=6, nailsPerSide=10)
    When the user clicks Generate
    Then the draft has one regular-polygon Pin Path with 60 pins
    And the draft has 6 Thread Paths, one per side

  Scenario: Flower layers rotated copies of Polygon's own weave
    Given Generator mode is active with the Flower pattern (sides=5, layers=3)
    When the user clicks Generate
    Then the draft has 3 Pin Paths
    And the draft has 15 Thread Paths (sides * layers)

  Scenario: Assymetry weaves a single pass across the circle and spoke's combined space
    Given Generator mode is active with the Assymetry pattern
    When the user clicks Generate
    Then the draft has 2 Pin Paths (1 circle + 1 spoke) and exactly 1 Thread Path

  Scenario: Spiral threads one decaying-span continuous walk
    Given Generator mode is active with the Spiral pattern (n=60, repetition=2, innerLength=3)
    When the user clicks Generate
    Then the draft has one circle Pin Path with 60 pins and exactly 1 Thread Path

  Scenario: Maurer Rose samples the public rose-curve formula into freehand pins
    Given Generator mode is active with the Maurer Rose pattern (maxSteps=90)
    When the user clicks Generate
    Then the draft's freehand Pin Path has exactly 91 pins (maxSteps+1)
    And the single Thread Path visits them in sampled order

  Scenario: Comet's layers taper via a shrinking offset (run length is derived from it)
    Given Generator mode is active with the Comet pattern (layers=5, layerDistance=3)
    When the user clicks Generate
    Then the draft has one circle Pin Path and 5 Thread Paths
    And each layer's own run length (n-d+1) grows as its offset d shrinks

  Scenario: Flower of Life's outer ring is optional
    Given Generator mode is active with the Flower of Life pattern (ringEnabled=false)
    Then the draft has 6 Pin Paths and 18 Thread Paths
    When the user enables ringEnabled
    Then the draft has 7 Pin Paths and 19 Thread Paths

  Scenario: Crosses weaves each of its 10 spine-segment/row pairs
    Given Generator mode is active with the Crosses pattern
    When the user clicks Generate
    Then the draft has 10 line Pin Paths (4 spine segments + 3 rows of 2 arms) and 10 Thread Paths
```

## Desktop notes

- All 19 builders are ported (`sa-app/src/generator.rs`, index maths in `sa-domain/src/generator.rs` and `lotus.rs`); for every pattern's defaults the pin coordinates, thread sequences and colours were verified identical to the web output.
