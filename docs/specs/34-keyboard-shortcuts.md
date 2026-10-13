# 34 — Keyboard Shortcuts

## Purpose

Add a full set of keyboard shortcuts covering the New Board screen, general app-wide actions (save/open/new/help/zoom/fit/grid/snap), the six Editor mode tabs, and every tool inside the Edit, Pin, Thread, and Play tabs. Before this milestone, the only keyboard handling in the app was a handful of narrow, single-purpose listeners (Ctrl/Cmd+Z undo/redo in `EditorShell.tsx`, arrow-key transform nudging, Escape/ArrowLeft/ArrowRight during Thread and Pin Path drafts) — this milestone is the first app-wide shortcut system.

Several combos that would have been the most obvious choice collide with hard browser/OS reservations and were replaced (checked against Chrome/Firefox/Edge/Safari on Windows/Mac/Linux):

- `Ctrl+N` (new browser window) and `Ctrl+ +`/`Ctrl+ -` (page zoom) are intercepted before page JS ever sees them on most browsers.
- `Ctrl+H` is Windows History; `Ctrl+F` is Find; `Ctrl+A` is Select All — and this app has no "select all objects" feature yet, so spending that combo on Snap would block the obvious future use.
- An `Alt/Cmd+letter` tab-switch scheme breaks on Mac for 5 of 6 tabs: `Cmd+P`=Print, `Cmd+T`=New Tab, `Cmd+M`=Minimize, `Cmd+G`=Find Next, `Cmd+V`=Paste are all OS/browser-reserved and non-overridable.

The result is a 3-tier key namespace that stays collision-free without any central registry: a bare letter or digit is a primary tool/tab selector, `Shift+key` is a secondary action, and `Ctrl(+Shift/Alt)+key` is an app-level command. Because per-mode tool letters are only "live" while that mode is active, the same letter safely means different things in different tabs (e.g. `P` = Path in the Pin tab, Previous Frame in the Play tab).

## Architecture

One central dispatcher, `useKeyboardShortcuts` (`src/ui/useKeyboardShortcuts.ts`), mounted once in `EditorShell.tsx`. It registers a single `window` `keydown` listener and re-reads `store.getState()` on every keystroke rather than closing over stale render-time state — the same pattern `useKeyboardTransform.ts` already uses. The first check in every keystroke is `isTextEntryTarget(e.target)` (`src/ui/keyboard.ts`) — an input, textarea, or `contentEditable` element in focus suppresses every shortcut in this spec, so normal typing (a dimension field, a layer rename, a symmetry interval) is never hijacked.

Dispatch order is **mode-specific branch first, global branch last**: the current `state.mode`'s branch (Play → Thread → Pin → Edit) is checked before the global branch, and a branch that recognizes the key stops there. This never binds `Escape` or `Ctrl/Cmd+Z`, so it cannot collide with `EditorShell`'s own undo/redo effect. It does bind the four arrow keys, but only as a **fallback pan** — see "Arrow-key pan" below — by re-deriving each of `useKeyboardTransform`'s, `useThreadDrawing`'s, and `usePolygonDrawing`'s own "would I act on this key" condition, so it never fires when one of those three independent listeners already would.

Every binding below maps to an **existing** `EditorStore` method (`setMode`, `setGrid`, `setViewport`, `setPinTool`, `setSelectTool`, `setSelectGranularity`, `commitSelectionMerge`, `setSymmetryConfig`, `setThreadTool`, `setThreadProperty`, `loadProject`) or an existing UI action (Save/Open/New from `FileMenu.tsx`, transport/video-export from Play mode) — no new store surface was needed. `SymmetryPanel.tsx`'s type-change buttons and `ThreadPropertiesPanel.tsx`'s colour-count buttons were refactored to share their exact logic with the keyboard shortcut (`symmetryConfigForType`, `coloursForCount`), so the mouse and keyboard paths can never drift apart.

## New Board screen

Scoped to `BoardSetup.tsx` only, via its own local `keydown` listener (not the central dispatcher — this screen has a separate lifecycle from `EditorShell`):

| Key | Action |
|---|---|
| `Enter` | Continue to Editor (same as clicking the existing "Continue" button). Ignored while a text/number field has focus — the user must blur the field first, consistent with every other shortcut in this app suppressing itself during text entry. |
| `L` | Opens the language dropdown if closed. Pressing `L` again while it's open cycles to the next language (wrapping around) and applies it immediately. |

## General (app-wide, in the Editor)

| Key | Action |
|---|---|
| `Ctrl/Cmd+S` | Save — downloads the current project as `.json`. |
| `Ctrl/Cmd+O` | Open — opens the file picker. |
| `Ctrl/Cmd+Alt+N` | New — resets to an empty project and returns to the New Board screen. |
| `?` (Shift+`/`) | Opens the Help panel. |
| bare `+` (or `=`) | Zoom In, centred on the viewport centre. |
| bare `-` | Zoom Out, centred on the viewport centre. |
| bare `0` | Fit — resets the viewport to fit the whole board. |
| `Ctrl/Cmd+Shift+G` | Toggles grid visibility. |
| `Ctrl/Cmd+Shift+A` | Toggles snap-to-grid. |
| `ArrowUp` / `ArrowDown` / `ArrowLeft` / `ArrowRight` | **Fallback pan** — moves the viewport, only when the arrow key isn't already claimed by something else (see "Arrow-key pan" below). `Shift+Arrow` pans by a bigger step, same "Shift = bigger step" convention as the existing Edit-mode nudge. |
| `Delete` / `Backspace` | Deletes the current selection — Pin Paths, individual pins, or a Thread Path — as one undo step, cascading deleted pins into referencing thread segments (same as the erasers). No-op with nothing selected, or if any involved layer is locked. |
| `Ctrl/Cmd+C` | Copies the selected Pin Paths (with attached threads) or Thread Path (with its Pin Paths) to the system clipboard — see [39-copy-paste.md](./39-copy-paste.md). Handled via the native `copy` event, not `keydown`. |
| `Ctrl/Cmd+V` | Pastes copied content (same board or another board/tab) into the active Pin and Thread layers as one undo step — see [39-copy-paste.md](./39-copy-paste.md). Handled via the native `paste` event, since `Cmd+V` is not overridable as a keystroke. |
| bare `1`–`6` | Switches to the Pin / Edit / Thread / Generate / Pan / Play tab, respectively (same order as `ModeSwitcher.tsx`). |

## Edit tab (`mode === "select"`)

| Key | Action |
|---|---|
| `S` | Select tool |
| `M` | Move tool |
| `R` | Rotation tool |
| `C` | Scale tool |
| `J` | Merge the current selection (no-op if fewer than 2 members are selected, same as the disabled Merge button) |
| `Shift+P` | Toggles selection granularity between Pin Path and Pins |

## Pin tab (`mode === "pin"`)

| Key | Action |
|---|---|
| `L` `A` `E` `C` `R` `S` `F` `P` `T` `D` `Q` | Line / Arc / Ellipse / Circle / Rect / Square / Freehand / Path / Text / Eraser / Path Eraser |
| `Shift+S` | Cycles symmetry: None → Horizontal → Vertical → Both → Radial → None, resetting axis/centre/interval to the same out-of-the-box defaults `SymmetryPanel`'s own buttons use (not preserving a previous instance of that type) |

## Thread tab (`mode === "thread"`)

| Key | Action |
|---|---|
| `D` `Z` `P` `R` `T` `S` `E` `C` | Draw / Zig-zag / Parabolic / Radial / Repeat / Select / Eraser / Segment |
| `Shift+1` / `Shift+2` / `Shift+3` | Sets colour count to 1, 2, or 3 |
| `Shift++` / `Shift+-` | Increases/decreases thread width by 0.5, clamped to 0.5–5 |

## Play tab (`mode === "play"`; icon-only toolbar, so the key hint lives in each button's tooltip, not a visible label suffix)

| Key | Action |
|---|---|
| `F` | First Frame |
| `P` | Previous Frame |
| `Space` | Play / Pause |
| `N` | Next Frame |
| `L` | Last Frame |
| `Shift+E` | Export to Video |

All six are no-ops while the transport is disabled (video export in progress, or zero frames — the same condition `PlayToolbar.tsx` uses to disable its buttons); `Shift+E` additionally no-ops if video export isn't supported in the current browser.

## Arrow-key pan

Arrow keys pan the viewport by a fixed screen-pixel step (bigger with Shift held), **except** when the key is already claimed by one of three pre-existing, independent listeners:

| Claimed by | Condition | Keys claimed |
|---|---|---|
| `useKeyboardTransform.ts` (Edit-mode Move/Rotation/Scale nudge) | `mode === "select"` and `selectTool` is `move`, `rotate`, or `scale` — checked on the **tool**, not on whether a selection exists, so arrows stay silent (not pan) if e.g. Move is picked with nothing selected | all 4 arrows |
| `useThreadDrawing.ts` (Thread draft retract / pattern-follow) | `mode === "thread"` and a Thread draft is in progress | `ArrowLeft`, `ArrowRight` only |
| `usePolygonDrawing.ts` (Path tool draft retract) | `mode === "pin"`, `pinTool === "polygon"`, and a Path draft is in progress | `ArrowLeft` only |
| `useRepeatDrawing.ts` (Repeat draft retract; also owns `Enter` generate / `Escape` cancel) | `mode === "thread"` and a Repeat draft is in progress | `ArrowLeft` only |

Outside those conditions — including Thread/Pin mode with no draft in progress, Generate/Pan/Play mode, or Edit mode with the Select tool — arrows pan. Direction matches `usePanInteraction.ts`'s existing mouse drag-to-pan exactly: the board moves in the direction of the arrow, same as it follows a mouse drag (pressing Right moves the board right, i.e. the viewport's `panOrigin.x` decreases — the same sign relationship that hook's `origin.x - dx/zoom` has for a positive drag `dx`). `Ctrl/Cmd+Arrow` and `Alt+Arrow` are left alone entirely (never pan, never claimed), since `Alt+Left/Right` is the browser's own back/forward navigation shortcut on most platforms.

## Toolbar labels

Edit/Pin/Thread tool buttons show their shortcut as a `[X]` suffix, composed at render time (e.g. `` `${t(...)} [S]` ``) rather than a new translated string — the bracketed key isn't natural-language content, so no i18n key exists for it and `keyParity.test.ts` is unaffected. The Play tab's icon-only buttons get the hint appended to their existing tooltip text instead, since there's no visible label to suffix. The tab-switcher pills (`ModeSwitcher.tsx`) show no visual shortcut hint — the 1-6 bindings work but aren't surfaced on the tab bar itself, only in the Help panel's Keyboard & Mouse tab.

## Design decisions

**The `+`/`-` overlap between global Zoom and Thread's width shortcut.** On a standard keyboard layout, typing `+` already requires holding Shift (the unshifted key produces `=`), so "bare `+`" and "`Shift++`" are frequently the exact same physical keystroke. This is resolved by dispatch order, not by trying to independently detect Shift: while in Thread mode, `Shift+"+"`/`Shift+"-"` (checked in the Thread-mode branch) adjusts width; pressing the *unshifted* `=` key in Thread mode still falls through to the global branch and zooms, since Thread's branch only intercepts when `e.shiftKey` is true. Outside Thread mode, `+`/`-` always zoom. The practical result matches the intended per-context behaviour even though "Shift" isn't independently distinguishable from "the character produced" for the `+` side of the pair.

**Merge (`J`) is a no-op below 2 selected members**, mirroring the toolbar's disabled Merge button rather than showing any separate feedback (this app has no toast/notification system to add one).

**Symmetry cycle (`Shift+S`) always resets to defaults for the new type**, never preserving a previous radial interval/centre or mirror axis — this matches `SymmetryPanel.tsx`'s own type-change buttons exactly (verified: they don't preserve prior per-type state either), so the keyboard shortcut can't produce a config the mouse UI couldn't already produce.

**Colour-count digits (`Shift+1`/`Shift+2`/`Shift+3`) are detected by `e.code`, not `e.key`.** On a real keyboard, holding Shift while pressing a digit key produces the shifted symbol as `e.key` (Shift+2 → `"@"`, Shift+3 → `"#"`, etc.), never the digit character itself — `e.key` is defined by the DOM spec as the actual character produced under the current modifiers, not the physical key. Detecting these bindings therefore reads `e.code` (`"Digit1"`/`"Digit2"`/`"Digit3"`, the physical key location, unaffected by Shift or keyboard layout) instead. This was caught only by live testing with a real keyboard — a Playwright-synthesized `press('Shift+2')` does not reproduce the real shift-remapping and gave a false pass in earlier verification, and a naive unit test that manually sets `key: "2", shiftKey: true` doesn't exercise the bug either; the regression test for this binding sets `key: "@", code: "Digit2", shiftKey: true` to match what a browser actually dispatches.

**Arrow-key pan re-derives three other listeners' guard conditions instead of a shared flag.** `useKeyboardTransform`/`useThreadDrawing`/`usePolygonDrawing` are independent `window` `keydown` listeners with no shared "I handled this" signal (none of them call `stopPropagation`, and two of the three never call `preventDefault` even when they do act, since retracting/no-opping a draft has no browser default to suppress). Rather than modifying those three files to broadcast a claim, the pan fallback locally re-implements each one's "would I act" predicate against the same `EditorState` fields. This keeps the new dispatcher purely additive (matching the rest of this milestone's architecture) at the cost of the guard logic needing to stay in sync if any of those three hooks' own conditions ever change.

## Test Cases

```gherkin
Feature: Keyboard shortcuts

  Scenario: Shortcuts are suppressed while a text field has focus
    Given a text input has focus
    When the user presses "g" with Ctrl and Shift held
    Then grid visibility does not change

  Scenario: New Board Enter continues to the Editor
    Given the New Board screen is showing with no field focused
    When the user presses Enter
    Then the Editor opens

  Scenario: New Board Enter is ignored while a dimension field has focus
    Given the New Board screen is showing with a dimension number field focused
    When the user presses Enter
    Then the Editor does not open

  Scenario: New Board L opens the language dropdown
    Given the New Board screen is showing with the language dropdown closed
    When the user presses "L"
    Then the language dropdown opens

  Scenario: New Board L cycles languages once open
    Given the New Board screen is showing with the language dropdown open on English
    When the user presses "L"
    Then the next supported language is applied and the dropdown closes

  Scenario: Ctrl+S saves the project
    When the user presses Ctrl+S (or Cmd+S)
    Then the current project is downloaded as a .json file

  Scenario: Ctrl+O opens the file picker
    When the user presses Ctrl+O (or Cmd+O)
    Then the file picker opens

  Scenario: Ctrl+Alt+N starts a new project
    When the user presses Ctrl+Alt+N
    Then the document resets to an empty project and the New Board screen is shown

  Scenario: "?" opens Help
    When the user presses "?"
    Then the Help panel opens

  Scenario: Bare "+" zooms in outside Thread mode
    Given the current mode is Pin
    When the user presses "+"
    Then the viewport zoom increases by one step, centred on the viewport centre

  Scenario: Bare "-" zooms out
    When the user presses "-"
    Then the viewport zoom decreases by one step, centred on the viewport centre

  Scenario: Bare "0" fits the board
    When the user presses "0"
    Then the viewport resets to fit the whole board

  Scenario: Ctrl+Shift+G toggles grid visibility
    When the user presses Ctrl+Shift+G
    Then grid visibility flips

  Scenario: Ctrl+Shift+A toggles snap
    When the user presses Ctrl+Shift+A
    Then snap-to-grid flips

  Scenario Outline: Bare digits switch tabs
    When the user presses "<digit>"
    Then the active mode becomes "<mode>"

    Examples:
      | digit | mode     |
      | 1     | select   |
      | 2     | pin      |
      | 3     | thread   |
      | 4     | generate |
      | 5     | pan      |
      | 6     | play     |

  Scenario: Edit-tab letters select tools
    Given the current mode is Edit
    When the user presses "m"
    Then the Move tool becomes active

  Scenario: Edit-tab letters do nothing outside Edit mode
    Given the current mode is Pin
    When the user presses "m"
    Then the Select tool (Edit tab) does not change, and the Pin tool does not change either (m is unbound in Pin mode)

  Scenario: J merges a valid selection
    Given the current mode is Edit and 2 Pin Paths are selected
    When the user presses "j"
    Then the selected Pin Paths are merged

  Scenario: J is a no-op below 2 selected members
    Given the current mode is Edit and 1 Pin Path is selected
    When the user presses "j"
    Then no merge occurs

  Scenario: Shift+P toggles selection granularity
    Given the current mode is Edit
    When the user presses Shift+P
    Then selection granularity toggles between Pin Path and Pins

  Scenario Outline: Pin-tab letters select tools
    Given the current mode is Pin
    When the user presses "<key>"
    Then the "<tool>" tool becomes active

    Examples:
      | key | tool        |
      | l   | line        |
      | a   | arc         |
      | e   | ellipse     |
      | c   | circle      |
      | r   | rectangle   |
      | s   | square      |
      | f   | freehand    |
      | p   | polygon     |
      | t   | text        |
      | d   | eraser      |
      | q   | path-eraser |

  Scenario: Shift+S cycles symmetry
    Given the current mode is Pin and symmetry is currently "none"
    When the user presses Shift+S
    Then symmetry becomes "horizontal" with its default axis

  Scenario: Symmetry cycle wraps around
    Given the current mode is Pin and symmetry is currently "radial"
    When the user presses Shift+S
    Then symmetry becomes "none"

  Scenario Outline: Thread-tab letters select tools
    Given the current mode is Thread
    When the user presses "<key>"
    Then the "<tool>" tool becomes active

    Examples:
      | key | tool           |
      | d   | draw           |
      | s   | select         |
      | e   | eraser         |
      | c   | segment-eraser |

  Scenario: Shift+1/2/3 sets colour count
    Given the current mode is Thread
    When the user presses Shift+2
    Then the colour count becomes 2

  Scenario: Shift++ increases thread width
    Given the current mode is Thread and the current width is 1.5
    When the user presses Shift++
    Then the width becomes 2.0

  Scenario: Shift+- decreases thread width, clamped at 0.5
    Given the current mode is Thread and the current width is 0.5
    When the user presses Shift+-
    Then the width stays at 0.5

  Scenario: Unshifted "=" still zooms while in Thread mode
    Given the current mode is Thread
    When the user presses "=" without Shift held
    Then the viewport zooms in, and thread width does not change

  Scenario Outline: Play-tab keys drive the transport
    Given the current mode is Play and the transport is enabled
    When the user presses "<key>"
    Then "<action>" occurs

    Examples:
      | key   | action                    |
      | f     | jump to the first frame   |
      | p     | jump to the previous frame|
      | n     | jump to the next frame    |
      | l     | jump to the last frame    |

  Scenario: Space toggles play/pause
    Given the current mode is Play and the transport is enabled and paused
    When the user presses Space
    Then playback starts

  Scenario: Play-tab keys are no-ops while the transport is disabled
    Given the current mode is Play and there are zero frames
    When the user presses "f"
    Then nothing happens

  Scenario: Shift+E exports to video
    Given the current mode is Play, the transport is enabled, and video export is supported
    When the user presses Shift+E
    Then video export starts

  Scenario: Arrow keys pan the viewport by default
    Given the current mode is Pin with no draft in progress
    When the user presses ArrowRight
    Then the viewport pans right

  Scenario: Shift+Arrow pans by a bigger step
    Given the current mode is Pin with no draft in progress
    When the user presses Shift+ArrowRight instead of a bare ArrowRight
    Then the viewport pans further

  Scenario: Arrows do not pan while the Move/Rotation/Scale tool is active in Edit mode
    Given the current mode is Edit and the Move tool is active
    When the user presses ArrowRight
    Then the viewport does not pan

  Scenario: Arrows do not pan ArrowLeft/ArrowRight during an in-progress Thread draft
    Given the current mode is Thread with a draft in progress
    When the user presses ArrowRight
    Then the viewport does not pan, and the draft's own ArrowRight pattern-follow behaviour applies instead

  Scenario: ArrowUp/ArrowDown still pan during an in-progress Thread draft
    Given the current mode is Thread with a draft in progress
    When the user presses ArrowDown
    Then the viewport pans down, since the Thread draft only claims ArrowLeft/ArrowRight

  Scenario: Arrows do not pan ArrowLeft during an in-progress Path (polygon) draft
    Given the current mode is Pin, the Path tool is active, and a draft is in progress
    When the user presses ArrowLeft
    Then the viewport does not pan, and the draft's own ArrowLeft retract behaviour applies instead

  Scenario: Ctrl/Cmd+Arrow and Alt+Arrow never pan
    Given the current mode is Pin with no draft in progress
    When the user presses Ctrl+ArrowRight or Alt+ArrowLeft
    Then the viewport does not pan
```

## Scope limits

- No visual on-screen list of shortcuts beyond the Help panel's Keyboard & Mouse tab — no first-run tour, no cheat-sheet overlay.
- No user-customizable key rebinding — every binding above is fixed.
- The New Board screen's `Enter`/`L` shortcuts have no in-app Help surface (Help is Editor-only) — they're documented here and, if this repo's `README.md` carries a shortcuts section, there too, but not inside the app itself.
- No shortcut exists for a "select all objects" action — deliberately left unclaimed (see Purpose) for a future feature to use `Ctrl/Cmd+A` once it exists.
