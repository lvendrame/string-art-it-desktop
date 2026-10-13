# 33 — Pin Path Tool ("Path")

## Purpose

Add a **Path** tool to Pin mode's toolbar: click to drop vertices one at a time, each click extending a live preview segment to the cursor, until the user finishes the draft — at which point it becomes a real, closed Pin Path connecting the last vertex back to the first. Unlike Freehand ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)), which continuously samples raw cursor movement into an open, unstructured point cloud, Path's vertices are individually placed, snap-resolved clicks — real corners the user deliberately chose, always closing into a polygon.

This is additive: it introduces one new `PinTool` (`"polygon"` internally, labelled "Path" in the UI) and one new `PinPathGeometry` variant (`{ type: "polygon"; points: Point[] }`), alongside the existing shape tools. It changes nothing about Line/Arc/Freehand/Text/the Polygon-Star dropdown/Eraser/Path Eraser.

## Starting and extending the draft

1. With the Path tool active, the user's first left click on the canvas places the first vertex (through the normal snap pipeline — grid/pin snap apply exactly as they do for every other Pin tool, per [05-canvas-and-viewport.md](./05-canvas-and-viewport.md)'s snapping-priority pipeline) and starts the draft.
2. Each further left click adds another vertex at the clicked (snap-resolved) point.
3. Between clicks, a dashed preview shows every confirmed vertex connected in order, plus one more live segment from the last vertex to the current cursor position — the same accent-coloured dashed styling every other Pin tool's live preview uses, including symmetry's live mirrored/radial copies ([06-symmetry.md](./06-symmetry.md)).
4. This preview segment chain is deliberately drawn **open** (not closing back to the first vertex) — the closing edge doesn't exist yet and isn't implied until the draft actually finishes.

## Closing the draft

Three ways to finish, all equivalent to a single "finish" action:

- **Esc** — finishes the draft immediately.
- **"Cut"** in the right-click radial context menu ([25-radial-context-menu.md](./25-radial-context-menu.md)) — finishes the draft immediately, same as Esc.
- **Clicking back on the first vertex** — once the draft has at least 3 vertices, a click landing within the same snap radius used for pin/grid snapping (`SnapSettings.radiusPx`, converted to document units) of the first vertex finishes the draft instead of adding a new one. While the cursor is within that radius (and the draft already has 3+ vertices), the first vertex is highlighted as a "click here to close" target, the same visual language as Thread mode's candidate-pin highlight ([12-thread-editor.md](./12-thread-editor.md) §Pin Highlight States).

Finishing always attempts to commit the draft as a real Pin Path:

```text
points.length >= 3  → commits: a closed Pin Path is created, connecting the
                       last vertex back to the first (docs/specs/02-document-
                       model.md), and the draft is cleared.
points.length < 3   → discards: the draft is cleared, nothing is added to the
                       document. A 1- or 2-point "polygon" isn't a real shape.
```

This is the same "discard below a minimum, commit at or above it" shape as Thread's draft ([12-thread-editor.md](./12-thread-editor.md) §Ending/Cutting Thread), just with a different minimum (3 vertices for a closed polygon vs. Thread's 2 pins for a segment) and no Esc/Cut asymmetry — Thread's Esc and right-click Cut behave slightly differently at very short drafts; Path's Esc and Cut are identical at every vertex count.

A newly committed Path Pin Path leaves the editor in Pin mode with the Path tool still active, same as every other shape tool ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) — `EditorStore.addPinPath` never changes mode).

## Removing vertices mid-draft

- **ArrowLeft**, or **"Back"** in the radial context menu, removes the most recently added vertex.
- If the vertex removed was the only one left (the draft had exactly 1 vertex), the entire draft is cancelled outright — there's nothing left to draw. This is the same rule as Thread's `retractThreadDraft` ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md)).

## Cancelling the draft

**"Cancel"** in the radial context menu discards the draft immediately, at any vertex count, with no attempt to commit — even a 3+-vertex draft that could otherwise become a real Pin Path is thrown away. This has no keyboard shortcut of its own (only Cut/Esc/Back do); it exists purely as an explicit "start over" menu action. Unlike Thread's draft slice set (Cut/Back/Next), Path's has no pattern-follow analogue to "Next" — Thread's `ArrowRight` pattern extrapolation ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md)) requires existing numbered pins to extrapolate from, which a free-form vertex draft has no equivalent of.

## Right-click menu while drafting

Right-clicking the canvas while a Path draft is in progress swaps the Pin mode radial menu's normal tool slices for exactly three: **Cut**, **Back**, **Cancel** — the same draft-mode slice-swap shape Thread mode already uses ([25-radial-context-menu.md](./25-radial-context-menu.md)). There is no "switch Pin tool" action available while mid-draft, matching how the toolbar itself behaves once the geometry becomes non-trivial.

```text
Left click (< 3 vertices, or not near the first vertex)
→ Add vertex

Left click (>= 3 vertices, within snap radius of the first vertex)
→ Close: commit the draft as a closed Pin Path

Right click → "Cut" (radial menu)
→ Finish now (commit if >= 3 vertices, else discard)

Esc
→ Finish now (commit if >= 3 vertices, else discard) — same as Cut

ArrowLeft / "Back" (radial menu)
→ Remove the last vertex (cancels the draft entirely if it was the only one)

"Cancel" (radial menu only)
→ Discard the draft outright, no commit attempt
```

## Geometry and pin distribution

```text
PinPathGeometry
│
type: "polygon"
points: Point[]   // in click order; closes last → first
```

The guide path is built the same way Text's closed contours are (`closedPolylineShape` — [29-text-pin-path.md](./29-text-pin-path.md)): a chain of line segments through `points` in order, with an implicit closing segment from the last point back to the first.

Because every point is a real, deliberately placed corner (not an arbitrary sampled cursor position like Freehand), Path uses **Vertex-Anchored Pin Distribution** ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md)): every vertex always receives a pin, and the pins between two consecutive vertices are distributed independently per edge, using the same closest-integer-interval-count rule as every other vertex-anchored shape (Line, Rectangle, Square, the regular-polygon/Star/Polygram family). The closing edge (last vertex → first vertex) is a real edge too, and gets pins the same way as any other edge.

Move/Rotate/Scale/Merge treat a Path Pin Path exactly like Freehand — transform every point directly, with the shape's centroid (for Scale's pivot) computed as the arithmetic mean of all vertices. There is no bespoke Selection-panel geometry field for it (no single position/size scalar makes sense for an arbitrary polygon), same as Freehand today.

## Locked layers and other guards

Placing/removing draft vertices is unaffected by the active Pin Layer's lock state — same as every other shape tool, whose in-progress interaction is layer-lock-agnostic. Only the final commit is blocked: finishing a draft on a locked active Pin Layer discards it silently (via the same `addPinPath` no-op every other shape tool already relies on), exactly as if fewer than 3 vertices had been placed.

Switching Pin tool away from Path, or switching Editor mode away from Pin, while a draft is in progress discards it outright (no commit attempt) — the same non-undoable-transient-state cleanup already applied to Thread's and Generator's in-progress drafts on mode/tool exit ([32-generator-mode.md](./32-generator-mode.md)).

## Test Cases

```gherkin
Feature: Pin Path tool draft lifecycle

  Scenario: First click starts the draft
    Given the Path tool is active with no draft in progress
    When the user clicks the canvas
    Then a draft starts with exactly 1 vertex at the clicked (snap-resolved) point

  Scenario: Further clicks extend the draft
    Given a Path draft has 2 vertices
    When the user clicks the canvas at a point away from the first vertex
    Then the draft has 3 vertices, the new one at the clicked point

  Scenario: Cut commits a 3+ vertex draft as a closed Pin Path
    Given a Path draft has 4 vertices
    When the user right-clicks and selects Cut
    Then a new Pin Path is created with geometry type "polygon" and those 4 points, closing the last point back to the first
    And the draft is cleared
    And the new Pin Path is selected in Edit mode

  Scenario: Esc behaves identically to Cut
    Given a Path draft has 4 vertices
    When the user presses Escape
    Then the same closed Pin Path is created as if Cut had been selected

  Scenario: Finishing with fewer than 3 vertices discards silently
    Given a Path draft has 2 vertices
    When the user presses Escape
    Then the draft is cleared and no Pin Path is created

  Scenario: Clicking back on the first vertex closes the polygon
    Given a Path draft has 4 vertices
    When the user clicks within the snap radius of the first vertex
    Then the draft commits as a closed Pin Path with those 4 vertices, identical to selecting Cut
    And no 5th vertex is added at the click location

  Scenario: Clicking near the first vertex with fewer than 3 vertices does not close
    Given a Path draft has 2 vertices
    When the user clicks within the snap radius of the first vertex
    Then the draft finishes the same as Esc/Cut would — it is discarded, since 2 vertices can't form a polygon

  Scenario: Back removes the last vertex
    Given a Path draft has 3 vertices
    When the user selects Back from the radial menu
    Then the draft has 2 vertices, the last one removed

  Scenario: ArrowLeft behaves identically to Back
    Given a Path draft has 3 vertices
    When the user presses ArrowLeft
    Then the draft has 2 vertices, the last one removed

  Scenario: Removing the only vertex cancels the draft entirely
    Given a Path draft has exactly 1 vertex
    When the user presses ArrowLeft
    Then the draft is cancelled — no vertices remain, and no Pin Path is created

  Scenario: Cancel discards the draft outright, even with enough vertices to commit
    Given a Path draft has 5 vertices
    When the user selects Cancel from the radial menu
    Then the draft is cleared and no Pin Path is created

  Scenario: Right-click menu shows Cut/Back/Cancel while drafting
    Given a Path draft is in progress
    When the user right-clicks the canvas
    Then the menu shows exactly Cut, Back, Cancel instead of the normal Pin tool slices

  Scenario: Finishing on a locked layer discards the draft
    Given a Path draft has 4 vertices and the active Pin Layer is locked
    When the user selects Cut
    Then no Pin Path is created and the draft is cleared, same as if fewer than 3 vertices had been placed

  Scenario: Switching Pin tool away from Path discards an in-progress draft
    Given a Path draft has 3 vertices
    When the user picks a different Pin tool
    Then the draft is discarded with no commit attempt

  Scenario: A committed Path Pin Path is vertex-anchored
    Given a Path draft with 5 vertices is committed
    Then every one of the 5 vertices receives a pin, and pins between consecutive vertices (including the closing edge back to the first) are distributed independently per edge, per the Vertex-Anchored Pin Distribution rule
```

## Scope limits

- No "Next"/pattern-follow slice, unlike Thread's draft ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md)) — there's no existing numbered-pin pattern to extrapolate from while placing brand-new free-form vertices.
- No dedicated Selection-panel geometry fields for a committed Path Pin Path — it's treated exactly like Freehand (generic Spacing/Colour/Diameter/Guide fields only, no shape-specific numeric fields), since an arbitrary polygon has no single meaningful position/size scalar.
- Symmetry mirrors/radial copies of the *live draft* preview follow the existing "preview shows mirrors too" rule ([06-symmetry.md](./06-symmetry.md)), but only the source draft accumulates vertices from clicks — mirrored copies are still derived/rendered only, never independently editable, same as every other shape tool.
