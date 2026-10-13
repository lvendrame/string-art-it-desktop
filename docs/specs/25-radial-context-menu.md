# 25 — Radial Context Menu

## Purpose

Every tool this feature exposes already lives in the always-visible left sidebar toolbar for the current Editor mode ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md), [09-selection-and-editing.md](./09-selection-and-editing.md), [12-thread-editor.md](./12-thread-editor.md), [05-canvas-and-viewport.md](./05-canvas-and-viewport.md), [19-play-mode.md](./19-play-mode.md)). This spec adds a second, faster path to the same actions: right-clicking the canvas opens a radial ("pie") context menu, centred at the cursor, whose slices depend on the current Editor mode (and, in Thread mode, whether a thread draft is in progress). It never introduces a new action that doesn't already exist as a toolbar tool or keyboard shortcut — it is purely an alternate way to reach existing tools without moving the mouse back to the sidebar.

## Trigger

Right mouse button (`contextmenu` event) anywhere on the canvas `<svg>` opens the menu, centred at the cursor position. If a menu is already open, right-clicking again closes it and opens a new one at the new cursor position (never two menus at once).

This repurposes the canvas's existing `onContextMenu` handler. Before this spec, right-click had two special-cased immediate actions with no menu:

- Thread mode, an in-progress draft: right-click instantly finished the draft (`useThreadDrawing.ts`'s `handleContextMenu` → `EditorStore.finishThreadDraft`).
- Edit mode, Merge tool, ≥2 accumulated pins: right-click instantly committed the merge (`useMergeTool.ts`'s `handleContextMenu` → `EditorStore.commitMergeSelection`).

Both become menu picks instead ("Cut" and "Commit Merge" respectively, below) — right-click always opens the menu now; it never fires an action directly.

## Slice sets by mode

The menu is icon-only (16px, `lucide-react`, matching each icon's use in its own toolbar so an action always looks the same everywhere); every slice carries a tooltip. Which slices render is derived from `EditorState.mode`, plus `selectTool`/`pinTool`/`threadTool`/`threadDraft`/`mergeSelection` for the modes that need it:

### Edit (`mode === "select"`)

Select, Move, Rotate, Scale, Merge — clicking any of these calls `EditorStore.setSelectTool(...)`, identical to `SelectToolbar`.

A sixth slice, **Commit Merge**, appears only when `selectTool === "merge" && mergeSelection.length >= 2` — calls `EditorStore.commitMergeSelection()`, the same call the old instant right-click made. It is absent (not merely disabled) the rest of the time, since it has no meaning outside an in-progress merge.

### Pin (`mode === "pin"`)

Two mutually exclusive slice sets depending on `polygonDraft` ([33-pin-path-tool.md](./33-pin-path-tool.md)) — the same draft-mode swap shape as Thread below:

- **Normal** (`polygonDraft === null`): Line, Arc, Ellipse, Circle, Rect, Square, Freehand, Path, Eraser, Path Eraser — each calls `EditorStore.setPinTool(...)`, identical to `PinToolbar`'s basic-tools row.
- **Path tool draft in progress** (`polygonDraft !== null`): Cut, Back, Cancel — entirely replaces the normal set.
  - **Cut** → `EditorStore.finishPolygonDraft(activePinLayerId)` — finishes the draft now, discarding it silently if it has fewer than 3 vertices.
  - **Back** → `EditorStore.retractPolygonDraft()` — same as pressing `ArrowLeft`.
  - **Cancel** → `EditorStore.cancelPolygonDraft()` — hard discard at any vertex count, no commit attempt. Unlike Thread's draft set, Path has no "Next" pattern-follow slice and Thread has no "Cancel" slice — the two draft sets are shaped independently around what each tool actually needs.

The Polygon/Star family (Pentagon, Hexagon, Octagon, Star-5/6/8, Pentagram, Heptagram, Octagram — `PinToolbar`'s dropdown) is **deliberately excluded** from the normal set. Adding nine more slices would make the ring too dense to scan quickly at a glance; that family stays toolbar-only. See [Scope limits](#scope-limits).

### Thread (`mode === "thread"`)

Two mutually exclusive slice sets depending on `threadDraft`:

- **Normal** (`threadDraft === null`): Draw, Zig-zag, Parabolic, Radial, Repeat, Eraser, Segment — each calls `EditorStore.setThreadTool(...)`, identical to `ThreadToolbar`. A two-pin draft (Zig-zag/Parabolic/Radial) swaps in its own Back/Cancel set ([35-zigzag-parabolic-tools.md](./35-zigzag-parabolic-tools.md), [37-radial-thread-tool.md](./37-radial-thread-tool.md)). A Repeat draft swaps in Generate (only once 4+ pins are picked)/Back/Cancel ([38-repeat-pattern-tool.md](./38-repeat-pattern-tool.md)).
- **Draft in progress** (`threadDraft !== null`): Cut, Back, Next — entirely replaces the normal set (there is no "switch thread tool" action while mid-draft, matching how the toolbar itself behaves).
  - **Cut** → `EditorStore.finishThreadDraft(activeThreadLayerId)` — same call the old instant right-click made; finishes the draft at its last confirmed pin, no new segment added.
  - **Back** → `EditorStore.retractThreadDraft()` — same as pressing `ArrowLeft` ([22-thread-follow-pattern.md](./22-thread-follow-pattern.md), `useThreadDrawing.ts`).
  - **Next** → `EditorStore.advanceThreadDraftByPattern()` — same as pressing `ArrowRight`; a no-op if the draft has fewer than 4 vertices (existing behaviour, unchanged).

### Pan (`mode === "pan"`)

Fit, Zoom In, Zoom Out.

- **Fit** → `EditorStore.setViewport(fitViewportForBoard(board))`, identical to `CanvasToolbar`'s Fit button.
- **Zoom In** / **Zoom Out** → the same `zoomAtPoint` step `CanvasToolbar` uses (×1.25 / ÷1.25, clamped to 5%–1600%), but anchored at the **menu's open point** (where the user right-clicked) rather than `CanvasToolbar`'s fixed viewport-centre anchor — right-clicking a specific spot and zooming there should zoom toward that spot, not recentre.

### Play (`mode === "play"`)

First Frame, Previous Frame, Play/Pause, Next Frame, Last Frame — each calls the same playback-transport action `PlayToolbar` uses (`usePlaybackTransport`'s `first`/`previous`/`togglePlay`/`next`/`last`). The Play/Pause slice's icon reflects live playing state (Play vs Pause icon), same as the toolbar button.

## Dismissal

- Clicking a slice fires its action and closes the menu.
- Clicking anywhere outside the menu closes it without firing anything.
- `Escape` closes it without firing anything.

This is a deliberate departure from the `overlay` panel convention (Print/Stats/Help — see `docs/conventions/ui-patterns.md`), which has **no** Escape-to-close and no outside-click dismissal by design. A radial menu is a different interaction shape: anchored to a point, transient, and expected to close on any interaction that isn't a slice pick — the overlay convention's rationale ("introducing this for one panel would make users expect it everywhere") doesn't transfer, since nothing else in the app is a cursor-anchored transient menu. This menu does not join the `overlay` state union and does not set a new precedent for Print/Stats/Help.

## Test Cases

```gherkin
Feature: Radial context menu trigger

  Scenario: Right-click opens the menu at the cursor
    Given any Editor mode
    When the user right-clicks the canvas
    Then a radial menu opens centred at the click position
    And the default browser context menu does not appear

  Scenario: Right-clicking again while the menu is open moves it
    Given the radial menu is already open
    When the user right-clicks a different point on the canvas
    Then the menu closes and reopens centred at the new point

Feature: Edit mode slices

  Scenario: Base slice set
    Given mode is Edit and the Merge tool is not active, or fewer than 2 pins are accumulated
    When the user right-clicks the canvas
    Then the menu shows exactly Select, Move, Rotate, Scale, Merge

  Scenario: Commit Merge appears once 2+ pins are accumulated
    Given mode is Edit, the Merge tool is active, and 2 or more pins are accumulated
    When the user right-clicks the canvas
    Then the menu shows Select, Move, Rotate, Scale, Merge, and Commit Merge
    And selecting Commit Merge merges the accumulated pins, same as the previous instant right-click behaviour

  Scenario: Right-click no longer instantly commits a merge
    Given mode is Edit, the Merge tool is active, and 2 or more pins are accumulated
    When the user right-clicks the canvas
    Then no merge is committed yet — the menu opens instead, and the merge only commits if Commit Merge is selected

Feature: Pin mode slices

  Scenario: Pin tool slices
    Given mode is Pin and no Path tool draft is in progress
    When the user right-clicks the canvas
    Then the menu shows exactly Line, Arc, Ellipse, Circle, Rect, Square, Freehand, Path, Eraser, Path Eraser
    And selecting any of them sets that as the active Pin tool

  Scenario: Path draft slice set replaces the normal set
    Given mode is Pin and a Path tool draft is in progress
    When the user right-clicks the canvas
    Then the menu shows exactly Cut, Back, Cancel — the normal Pin tool slices do not appear

  Scenario: Cut finishes the Path draft
    Given a Path tool draft with 3 or more vertices is in progress
    When the user right-clicks and selects Cut
    Then the draft finishes into a real closed Pin Path, identical to pressing Escape

  Scenario: Back retracts the last vertex of the Path draft
    Given a Path tool draft is in progress
    When the user right-clicks and selects Back
    Then the draft's last vertex is removed, identical to pressing ArrowLeft

  Scenario: Cancel discards the Path draft outright
    Given a Path tool draft with any number of vertices is in progress
    When the user right-clicks and selects Cancel
    Then the draft is discarded and no Pin Path is created, even if it had 3 or more vertices

Feature: Thread mode slices

  Scenario: Normal slice set
    Given mode is Thread and no draft is in progress
    When the user right-clicks the canvas
    Then the menu shows exactly Draw, Eraser, Segment

  Scenario: Draft slice set replaces the normal set
    Given mode is Thread and a draft is in progress
    When the user right-clicks the canvas
    Then the menu shows exactly Cut, Back, Next — Draw/Eraser/Segment do not appear

  Scenario: Cut finishes the draft without adding a segment
    Given a thread draft is in progress
    When the user right-clicks and selects Cut
    Then the draft finishes at its last confirmed pin, identical to the pre-existing instant right-click-to-finish behaviour

  Scenario: Right-click no longer instantly finishes the draft
    Given a thread draft is in progress
    When the user right-clicks the canvas
    Then the draft is not finished yet — the menu opens instead, and it only finishes if Cut is selected

  Scenario: Back retracts the last confirmed vertex
    Given a thread draft is in progress
    When the user right-clicks and selects Back
    Then the draft's last confirmed vertex is removed, identical to pressing ArrowLeft

  Scenario: Next extrapolates the pattern
    Given a thread draft with 4 or more vertices is in progress
    When the user right-clicks and selects Next
    Then a new vertex is appended per the pattern-follow rule, identical to pressing ArrowRight

Feature: Pan mode slices

  Scenario: Pan tool slices
    Given mode is Pan
    When the user right-clicks the canvas
    Then the menu shows exactly Fit, Zoom In, Zoom Out

  Scenario: Zoom In/Out anchor on the click point
    Given mode is Pan
    When the user right-clicks a point away from the viewport centre and selects Zoom In
    Then the viewport zooms in anchored at that click point, not the viewport centre

Feature: Play mode slices

  Scenario: Play tool slices
    Given mode is Play
    When the user right-clicks the canvas
    Then the menu shows exactly First Frame, Previous Frame, Play/Pause, Next Frame, Last Frame
    And each performs the same transport action as its PlayToolbar equivalent

Feature: Dismissal

  Scenario: Escape closes without firing an action
    Given the radial menu is open
    When the user presses Escape
    Then the menu closes and no action fires

  Scenario: Clicking outside closes without firing an action
    Given the radial menu is open
    When the user clicks anywhere outside the menu
    Then the menu closes and no action fires

  Scenario: Selecting a slice closes the menu
    Given the radial menu is open
    When the user clicks a slice
    Then that slice's action fires exactly once and the menu closes
```

## Scope limits

- The Pin-mode Polygon/Star family (9 tools) is intentionally not on the radial menu — toolbar-only, to keep the ring scannable. A future revision could add it as a nested sub-menu (the underlying library supports `SubMenu`) if this turns out to matter in practice.
- No new action exists anywhere in this spec that didn't already exist as a toolbar button or keyboard shortcut before it — this is purely a second way to reach existing `EditorStore` methods, never new business logic.
