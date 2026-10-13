# 20 — In-App Help

## Purpose

Define an in-app Help overlay reachable from the top toolbar, giving users a tab-per-topic reference for every tool/button in the editor without leaving the app or consulting external documentation.

## Entry Point

- A "Help" button in the top toolbar, placed between Stats and Print.
- Opens a full-bleed overlay identical in structural convention to the Statistics and Print Preview overlays ([17-statistics.md](./17-statistics.md), [14-printing.md](./14-printing.md)) — same overlay chrome, same "Close" button, no backdrop, no Escape-to-close, no `aria-modal` dialog semantics.

## Tabs

Seven tabs, in this order: **Edit, Pin, Thread, Pan, Play, Layers, Keyboard & Mouse**. The first five mirror the Editor Mode Switcher's order and icons ([05-canvas-and-viewport.md](./05-canvas-and-viewport.md)); Layers and Keyboard & Mouse are appended as two cross-cutting topics that don't map to a single Editor mode.

### Default tab

Opening Help defaults to the tab matching the current Editor mode (select→Edit, pin→Pin, thread→Thread, pan→Pan, play→Play). Reopening Help after switching modes re-defaults to the new mode's tab — the panel is remounted fresh on each open, so this requires no special-cased effect.

## Content per tab

### Edit
Select (click to select a Pin Path); Move (press-drag-release, disabled unless a Pin Path is selected, one undo step); Rotation (press-drag-release about the press point, 0.3°/screen-pixel, disabled unless selected, one undo step); Merge (left-click accumulates pins, right-click commits into one averaged pin; Esc cancels an in-progress Move/Rotation/Merge). Selection panel (shown when a Pin Path is selected): live geometry fields per shape, an embedded Symmetry panel, Delete Pin Path.

### Pin
Line, Arc (click start → click end → drag curvature → click to confirm), Ellipse (Alt constrains to Circle), Circle, Rect (Alt constrains to Square), Square, Freehand, Path (click-per-vertex free-form polygon draft — see [33-pin-path-tool.md](./33-pin-path-tool.md)), Eraser (one pin), Path Eraser (whole path, cascades into referencing thread segments). Polygon/Star dropdown: Pentagon, Hexagon, Octagon, 5/6/8-point star, Pentagram, Heptagram, Octagram. Pin Properties: Spacing, Actual gap (read-only, selection only), Diameter, Colour, Guide visible. Symmetry: None/Horiz/Vert/Both/Radial, Interval° (radial only), Centre X/Y (radial only, draggable, snaps to grid).

### Thread
Draw (click extends the draft to the nearest pin; double-click finishes with a final segment; right-click finishes without one; Esc cancels; ArrowLeft retracts the last point; ArrowRight, once the draft has 4+ vertices, auto-adds the next one by following the numeric pin-position pattern of the vertices added so far — see [22-thread-follow-pattern.md](./22-thread-follow-pattern.md)), Eraser (whole path), Segment (one segment, splitting the path; fragments under 2 pins are dropped). Colour count (1/2/3 strands), colour swatches, Width, Twist pitch (2+ colours only), and a read-only pin-state legend (Normal / Nearest candidate / Active origin / Used in this thread).

### Pan
Left-click-drag pans the viewport. Grid ON/OFF, Snap ON/OFF, Gap X/Y, Grid colour, Grid opacity. Zoom out/in (×÷1.25, centred on the viewport centre), Fit (fits the whole board to view).

**Note**: a "100%" zoom shortcut is not implemented in this app (mouse-wheel/trackpad zoom anchored at the cursor, zoom in/out and Fit are) — the Help content must describe only what's actually implemented, regardless of anything listed as a future capability elsewhere in the spec set.

### Play
First/Previous/Play-Pause/Next/Last transport row (Play restarts from frame 0 if pressed again at the last frame; stops rather than loops at the end). Frame (editable, clamped), Total (read-only), Time between frames (ms). Export to Video (records to `.webm`, live progress, disabled if unsupported or there are zero frames).

### Layers
Pin Layers / Thread Layers tabs (two independent lists; the visible tab automatically follows the current Editor mode — see [13-layers.md](./13-layers.md)). Per-row: eye icon (show/hide, visual only), lock icon (blocks create/move/resize/rotate/erase/property-change; visibility still togglable), name (double-click to rename), click to select as active. Actions: New Layer, Duplicate (new IDs; thread references to duplicated pins keep pointing at the originals), Merge with layer above (moves the active layer's content into the layer above, appended on top, then deletes it — one undo step; disabled on the topmost layer or when either layer is locked — see [36-layer-merge.md](./36-layer-merge.md)), Move layer up/down, Delete (one undo step, disabled when only one layer remains).

### Keyboard & Mouse
Only documents interactions actually implemented in code — omits anything spec'd elsewhere as a future capability but not yet built (e.g. shift-click multi-select).

- **Keyboard**: Ctrl/Cmd+Z (undo), Ctrl/Cmd+Shift+Z (redo) — both ignored while a text field has focus; Escape (cancels or finishes the active tool's in-progress action — a thread draft, a Path tool polygon draft, a Move/Rotation drag, or a pending Merge selection, whichever applies); ArrowLeft (Thread Draw tool — retracts the draft's last point; Path tool — retracts the last vertex, cancelling the draft entirely if it was the only one — see [33-pin-path-tool.md](./33-pin-path-tool.md)); ArrowRight (Thread Draw tool only — once the draft has 4+ vertices, extends it by extrapolating the vertices' numeric pin-position pattern, wrapping around the relevant Pin Path's pin count); Alt held while dragging (Pin mode Ellipse/Rectangle only — constrains to Circle/Square); Ctrl/Cmd held (Pin mode — snaps the cursor to the board centre, see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) §Snapping Priority).
- **Mouse**: left button drives every draw/select/drag gesture, and also closes an in-progress Path tool polygon draft when it lands back on the draft's first vertex ([33-pin-path-tool.md](./33-pin-path-tool.md)); right-click opens a radial context menu whose icon slices match the current Editor mode — Select/Move/Rotate/Scale/Merge (plus Commit Merge once 2+ pins are accumulated) in Edit mode, the Pin drawing tools in Pin mode (or Cut/Back/Cancel while a Path tool draft is in progress), Draw/Eraser/Segment in Thread mode (or Cut/Back/Next while a thread draft is in progress), Fit/Zoom In/Zoom Out in Pan mode, and the playback transport in Play mode — see [25-radial-context-menu.md](./25-radial-context-menu.md); double-click (Thread Draw — finishes the draft with one final segment); left-drag (pans in Pan mode; drives Move/Rotation in Edit mode and most shape tools in Pin mode); mouse wheel / trackpad (zooms in/out anchored at the cursor, every mode except Play, same 5%–1600% clamp as the zoom buttons); middle-button drag (pans the board in every mode except Play).

## Test Cases

```gherkin
Feature: In-app Help

  Scenario: Opening Help defaults to the tab matching the current mode
    Given the Editor mode is "pin"
    When the user clicks the Help button
    Then the Help overlay opens with the "Pin" tab active

  Scenario: Switching tabs shows different content
    Given the Help overlay is open on the "Edit" tab
    When the user clicks the "Thread" tab
    Then the "Thread" tab's content is shown and the "Edit" tab's content is not

  Scenario: Closing Help
    Given the Help overlay is open
    When the user clicks "Close"
    Then the Help overlay is dismissed and the editor is shown underneath

  Scenario: Reopening Help after switching modes re-defaults
    Given the Help overlay was last showing the "Pin" tab
    And the user closed Help and switched the Editor mode to "thread"
    When the user clicks the Help button again
    Then the Help overlay opens with the "Thread" tab active
```
