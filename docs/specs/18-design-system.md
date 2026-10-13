# 18 — Design System

## Purpose

Define the visual design system the application UI must follow, so every screen (board setup, main editor, print preview, dialogs) reads as one coherent product. This spec is normative for styling decisions; it does not replace the functional specs (00–17) — it constrains *how* their required UI is rendered.

## Reference Artifact

The canonical visual reference is the editor mockup at [`docs/design/editor-mockup.dc.html`](../design/editor-mockup.dc.html) (published preview: https://claude.ai/code/artifact/decf318a-e2d0-43f2-822a-d32c5c4b28d8). It implements the main editor screen — top bar with mode switcher, left tool/properties panel, centre canvas with board + status bar, right layers panel — using the tokens and components defined below. When this document and the mockup disagree, treat the mockup as the concrete example and this document as the source of truth for the rule; update one to match the other rather than letting them drift.

## Design Direction

**Dark professional tool** — dense, focused chrome that recedes so the board/pins/threads (the user's actual work) stay the visual focus. Modeled on precision creative tools (vector/CAD editors), not a consumer or marketing aesthetic. This is a deliberate choice for a geometry-precision tool used for extended sessions; it is not a hard requirement to stay dark-only forever, but any future light theme must reuse the same tokens, spacing, and component anatomy defined here (see [Theming](#theming-future)).

## Design Tokens

### Colour

Defined as CSS custom properties (`oklch()`), computed once at the app shell root and consumed everywhere — no component hardcodes a colour value.

| Token | Value | Usage |
|---|---|---|
| `--bg-app` | `oklch(19% 0.012 264)` | Outermost app background (top bar, panel surrounds) |
| `--bg-panel` | `oklch(22.5% 0.014 264)` | Panel surfaces (left/right sidebars, top bar) |
| `--bg-panel-2` | `oklch(26% 0.015 264)` | Raised surfaces within a panel (inputs, tool buttons, chips) |
| `--bg-canvas` | `oklch(13.5% 0.010 264)` | Canvas/board viewport background |
| `--border` | `oklch(33% 0.014 264)` | Default hairline borders/dividers |
| `--border-strong` | `oklch(41% 0.016 264)` | Hover/focus borders, stronger separators |
| `--text-primary` | `oklch(93% 0.006 264)` | Primary text, active labels |
| `--text-secondary` | `oklch(68% 0.014 264)` | Secondary labels, inactive control text |
| `--text-tertiary` | `oklch(50% 0.014 264)` | Section headers, muted metadata |
| `--accent` | `oklch(65–68% 0.19 hue)` | Active/selected state, primary actions (as foreground), focus rings, links — single hue, user/brand-configurable |
| `--accent-strong` | `oklch(from var(--accent) 55% c h)` | Solid-fill background of primary CTA buttons with white text (e.g. Continue, Restore) — `--accent` itself is too light for white text to clear 4.5:1 AA contrast |
| `--accent-soft` | `color-mix(in oklch, var(--accent) 8%, transparent)` | Active-state fill (buttons, chips, selected rows) |
| `--accent-soft-2` | `color-mix(in oklch, var(--accent) 28%, transparent)` | Stronger active fill (toggles) |
| `--danger` | `oklch(66% 0.18 22)` | Destructive actions, lock/error indicators |
| `--danger-soft` | `color-mix(in oklch, var(--danger) 16%, transparent)` | Destructive hover/fill |
| `--shadow-float` | `0 8px 24px oklch(0% 0 0 / 0.4)` | Drop shadow for components that float above the canvas/chrome rather than sitting in panel document flow — see [Radial context menu](#radial-context-menu) and [Anchored settings popover](#anchored-settings-popover). Not used by panels/sidebars/toolbars, which read elevation from background-lightness alone per the Rules below. |

Rules:
- Only **one** accent hue is used across the entire app. It marks active tool, active mode, selected layer/object, primary buttons, focus rings, and links. It is never used for two unrelated meanings on the same screen (e.g. don't also use it for a "warning" state — that's `--danger`).
- Backgrounds step in fixed increments of lightness only (`--bg-canvas` < `--bg-app` < `--bg-panel` < `--bg-panel-2`), so elevation is always readable as "further from the canvas = lighter."
- Never introduce a new named colour outside this table for chrome/UI. Illustrative content *inside* the board (board appearance fills, thread colours, pin colours per [04-board-appearance.md](./04-board-appearance.md) and [12-thread-editor.md](./12-thread-editor.md)) is user data, not UI chrome, and is exempt from this palette — those are literal colours the user picks.

### Typography

| Role | Font | Fallback stack |
|---|---|---|
| UI text (labels, panels, buttons) | Manrope (400/500/600/700/800) | `system-ui, sans-serif` |
| Numeric/coordinate/status readouts | JetBrains Mono (400/500/600) | `ui-monospace, monospace` |

Loaded via Google Fonts `<link>`. Rules:
- Any value that is a **measurement, count, coordinate, or percentage** (pin count, spacing, zoom %, X/Y position, statistics) renders in the monospace family — this is a functional cue, not decoration: it tells the user "this number is precise/computed," consistent with the geometry-precision principle in [00-overview-and-scope.md](./00-overview-and-scope.md).
- Everything else (labels, layer names, buttons, panel headers) renders in Manrope.
- Section headers (e.g. "PIN TOOLS", "SYMMETRY", "LAYERS") are 11px, 700 weight, `0.06em` letter-spacing, uppercase, `--text-tertiary` — this exact treatment is the one and only "section label" style; do not invent a second variant.
- No more than two font families total. Never substitute a third font for "emphasis" — use weight/size/colour instead.

### Spacing & Sizing

- Base spacing unit: 4px. Component padding/gaps use multiples of it (6, 8, 9, 10, 12, 14, 16px observed in the reference) — never arbitrary odd values like 13px or 17px.
- Corner radius: `--radius-sm: 6px` for buttons/chips/small inputs, `--radius-md: 10px` for panels/cards/grouped controls. No third radius value.
- Standard chrome heights: top bar 56px, canvas toolbar/status bar 32–48px, panel row (layer list item, list row) 9px vertical padding × full width.
- Icon sizes: 16px (`--icon` default) and 14px (`--icon-sm` for compact/inline contexts). No other icon size.
- Hit targets: every clickable control (buttons, toggle chips, layer row, icon buttons) is at least 32px in its smallest dimension, consistent with the 44px mockup-content minimum guidance being relaxed for dense pro-tool chrome — see [Accessibility](#accessibility) for the floor that still applies.

## Component Vocabulary

These are the only component patterns the UI should use; new screens compose from these rather than inventing new visual patterns.

### Segmented mode switcher
A pill-shaped container (`--bg-app` background, `--border` outline, 3px padding) holding button segments with no gaps between the outer container's rounding and inner buttons (2px gap between segments). Active segment: `--accent-soft` background, `--accent` text/icon, no border. Inactive: transparent, `--text-secondary`. Used for the six editor modes (Select/Pin/Thread/Generate/Pan/Play) — see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) §Editor Modes and [32-generator-mode.md](./32-generator-mode.md). Corrected from an earlier "four modes, ~4 options" note that had already fallen behind Play mode (M11) and Generator mode (M25) without being updated — six is the current, working count; there's no hard cap, but this component still isn't meant for long option lists (that's a `<select>`, per the Pin/Star family picker or the Generator pattern picker).

### Tool grid button
Square-ish button, icon on top (16px) + 10px label below, `--radius-sm`, arranged in a 3-column grid. Inactive: `--bg-app` fill, `--border` outline, `--text-secondary`. Active: `--accent-soft` fill, `--accent` outline + icon/text colour. Used for pin drawing tools ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)) and symmetry options ([06-symmetry.md](./06-symmetry.md)).

### Toggle chip (pill)
Rounded-full (999px) button with a small solid dot + label, used for independent binary states shown inline with the canvas (Grid ON/OFF, Snap ON/OFF — see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md)). Active: `--accent-soft`/`--accent`. Inactive: `--bg-panel-2`/`--text-secondary`. Two toggle chips representing independent settings are always shown side by side, never nested or implying one depends on the other — this directly encodes the "grid visibility and snap are independent" rule.

### Labeled slide switch
A track+knob control for a mutually-exclusive 2-way choice the user wants read as a literal switch rather than the Segmented mode switcher's tab-like look. Two plain-text end labels flank a pill track (`--bg-app` fill, `--border` outline, 40×20px) holding a circular knob (`--accent` fill, 14px) that slides between a left and right resting position on 150ms ease (same timing as the zoom-transition/radial-menu precedent under [Motion](#motion) — no new easing curve). The active-side label reads `--accent`/600 weight; the inactive side `--text-secondary`. Clicking either end label sets that specific side (not a toggle-flip); clicking the track itself flips it. The clickable track element is a 44×32px hit target even though the visual pill is smaller, per the 32px minimum under [Accessibility](#accessibility) — same "bigger hit area than visual size" convention icon buttons already use. Used for the Edit-mode Pin Path/Pins granularity switch ([26-edit-mode-multi-select.md](./26-edit-mode-multi-select.md)).

### Slider (range input)
A continuous-value control for a bounded numeric parameter (pin counts, ratios, angles, thread width), used wherever a plain number input previously stood. Track: `--bg-panel-2` fill, `--border` outline, `--radius-sm` rounding, full width of its container. Thumb: `--accent` fill (`--accent-strong` on hover), 14px circle — visually smaller than the 32px minimum hit target under [Accessibility](#accessibility), same "bigger hit area than visual size" convention the Labeled slide switch's track already uses. Value is always shown as an adjacent read-only monospace label, right-aligned above the track — a raw `<input type="range">` has no visible numeric readout on its own, and this app's convention (per [Typography](#typography)) is that measurements/counts render in the monospace family. Colour transitions use the standard 120ms ease, no new easing curve. First used by `GeneratorPanel`'s pattern-parameter fields and the shared thread-width control ([32-generator-mode.md](./32-generator-mode.md)), and by `ThreadPropertiesPanel`'s own width field.

### Fieldset/legend grouping
For a sub-group of related controls within a panel that needs its own inline title *and* an action row (e.g. add/remove buttons) sharing that same title line, use a native `<fieldset>`/`<legend>` pair rather than a plain bordered `<div>` with a separate heading `<div>` above it (the latter is still correct for groups with no such action row — see the params block convention already in use). `<fieldset>` styling matches the existing grouped-panel recipe (`--bg-app` fill, `--border` outline, `--radius-md`), with the UA-default border/margin/padding reset to zero and replaced by those same values. `<legend>` is set to `display: flex; justify-content: space-between; align-items: center; width: 100%` so a title and right-aligned action buttons share the first row; contained controls (e.g. colour swatches) render in the rows beneath. A conditionally-available action in the legend (e.g. add/remove at a count cap or floor) stays rendered but `disabled` — never unmounted — per the standard disabled-button treatment below; reaching the limit is a normal, expected state, not something worth changing the control's layout for. First used by `GeneratorPanel`'s colour-swatch group ([32-generator-mode.md](./32-generator-mode.md)).

### Property row
A label/value pair inside a bordered card (`--bg-app`, `--border`, `--radius-md`, 12px padding), label in `--text-secondary` 12px, value in monospace `--text-primary` 12px 600 weight, right-aligned. Used for read-only geometry display when an object is selected ([09-selection-and-editing.md](./09-selection-and-editing.md)) and for statistics ([17-statistics.md](./17-statistics.md)).

### Textarea
Multi-line text entry, styled with the same token recipe as every other inline input in the app: `--bg-panel-2` fill, `--border` outline, `--radius-sm`, `--text-primary` text, 12–12.5px size. Resizable vertically only (`resize: vertical`), never horizontally — the app is not a wide-canvas layout. First used by the About tab's contact form message field ([28-help-about-tab.md](./28-help-about-tab.md)); no earlier component needed multi-line free text.

### Layer row
Full-width row: eye icon button, lock icon button, name (flex-grow), 9px vertical / 16px horizontal padding. Selected row: `--accent-soft` background + 2px `--accent` left border. Hidden layer: name text drops to `--text-tertiary` (still fully legible, never below ~50% lightness, per "hidden layers retain their contents" — dimming signals state, it never implies removal). Locked layer: lock icon rendered in `--accent`; unlocked in `--text-tertiary`. See [13-layers.md](./13-layers.md).

### Radial context menu
A cursor-anchored ring of icon-only slices, opened by right-click ([25-radial-context-menu.md](./25-radial-context-menu.md)) — a floating/overlaid component (it doesn't sit in normal panel document flow the way most other components here do). Slice surface: `--bg-panel-2` fill, `--border` stroke between slices. Hover/active slice: `--accent-soft` fill, `--accent` icon colour — same single-accent-hue rule as every other active-state treatment, no new colour. Icons: 16px (`--icon`), same as everywhere else — no special-cased size for menu slices. Ring inner/outer radius are pixel constants on the component itself, not a new CSS custom property — nothing else in the app needs a "ring radius" token, so it isn't generalized into one. Open/close motion reuses the existing 150ms ease timing (matching the zoom-transition precedent under [Motion](#motion)) — no new easing curve.

It was the first floating component in the app and the first consumer of `--shadow-float` (see [Colour](#colour)); the anchored settings popover below is a second, smaller category of floating component and reuses the same token.

### Anchored settings popover
A small panel of fields/actions anchored to the button that opens it (`Export`, Language switcher, Grid settings — see [05-canvas-and-viewport.md](./05-canvas-and-viewport.md) §Grid Settings UI) rather than a cursor-anchored ring or a full-bleed overlay. `--bg-panel`/`--bg-panel-2` fill, `--border` stroke, `--radius-md` outer / `--radius-sm` inner controls, `--shadow-float` elevation, dismissed by outside click or Escape. See [docs/conventions/ui-patterns.md](../conventions/ui-patterns.md) §Anchored settings popover for the exact implementation shape.

### Icons
All icons are inline SVG, stroke-based (`stroke="currentColor"`, ~1.5px weight, round caps/joins), on a 24×24 viewBox at 16px or 14px display size. **No emoji, no icon-font glyphs, anywhere in the product** — including in place of the spec documents' illustrative `👁`/`🔒`/`◎` notation, which are shorthand in the prose specs only, not a rendering instruction. Define each icon once (a `<symbol>` in a shared defs block or an equivalent icon-component registry) and reference it everywhere it's needed — never inline a redrawn copy of the same icon twice.

### Primary / secondary buttons
Primary action (e.g. Export): `--accent-strong` solid fill, white text, no border. Secondary (e.g. Print, Undo/Redo): transparent or `--bg-panel-2` fill, `--border` outline, `--text-secondary`/`--text-primary`. At most one primary button visible in a given toolbar region at a time — primary emphasis is not spread across multiple simultaneous actions. A disabled `.btn` stays in place (never unmounted) — `--text-tertiary` text, 0.5 opacity, `not-allowed` cursor, hover has no effect — so the control's position and the action it represents remain predictable even when momentarily unavailable, rather than shifting neighbouring layout or requiring the user to relocate it once it becomes available again.

## Canvas Rendering Conventions

- Board fill uses a wood-toned radial gradient by default for the "Painted Wood"/"Wood Texture" appearances ([04-board-appearance.md](./04-board-appearance.md)); other appearance types render literally per user configuration and are not constrained by the app chrome palette.
- Guide lines: black, 80% opacity, dashed, per [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) default — rendered at reduced opacity against the board so pins remain the visual foreground.
- Pins: light neutral fill (`#f2ede4`-class off-white) with a dark stroke, so they read consistently against any board appearance/colour.
- Pin highlight states ([12-thread-editor.md](./12-thread-editor.md) §Pin Highlight States) use three visually distinct treatments and these exact roles, reused nowhere else in the UI:
  - Normal — plain filled pin, no ring.
  - Nearest Candidate — accent-independent warm highlight (amber-class colour) with an outer ring, so it never gets confused with the app's own `--accent` selection colour.
  - Active Thread Origin — solid `--accent` fill, no ring.
- Thread colour rendering ([12-thread-editor.md](./12-thread-editor.md) §Thread Colour Rendering): a two/three-colour thread is drawn as the same geometric line rendered multiple times with offset dash patterns per strand — never as multiple visually separate parallel lines. The geometry is one path regardless of strand count.
- Grid: low-opacity dot pattern (not solid grid lines) so it never competes with guides/pins for attention; opacity toggles fully between the visible/hidden state (no partial "dimmed but always-on" grid).
- Symmetry overlays (axis lines for mirror, spokes + centre marker for radial) render in `--accent` at reduced opacity with a dashed stroke, and only while that symmetry mode is active — they are working aids, not part of the permanent artwork.

## Motion

- Colour/background/border transitions on interactive elements: 120ms ease.
- Zoom changes: 150ms ease transform scale.
- No motion on canvas geometry itself beyond zoom — pins, threads, and guides update instantly when data changes (spacing edits, layer toggles), since animating precision geometry would misrepresent the underlying math.
- No decorative/idle animation anywhere in the editor chrome.

## Accessibility

- Minimum interactive hit target: 32px in the densest chrome (icon buttons, layer-row toggles); never below that even where visual size is smaller.
- Colour is never the only signal for state: locked/unlocked, visible/hidden, and active/inactive all pair a colour change with an icon change (eye ↔ eye-off, lock ↔ unlock) or a text label change (ON/OFF), never colour alone.
- Text contrast: `--text-primary` and `--text-secondary` against every background token in the elevation ladder must meet WCAG AA (4.5:1) for body text; `--text-tertiary` is reserved for non-essential/decorative labels (section headers, muted metadata) where a lower ratio is acceptable.
- Focus states are visible (accent-coloured outline/ring) on every keyboard-focusable control — required regardless of the mouse-first pro-tool aesthetic.

## Theming (Future)

Everything above is expressed as tokens specifically so a future light theme (or user-selectable theme) only requires redefining the token values, never the component markup/logic. A light theme must preserve: the same elevation ordering (canvas darkest/lightest-extreme → panel → panel-2), the single-accent-hue rule, the same two-font pairing, and the same component vocabulary. This is out of MVP scope (see [00-overview-and-scope.md](./00-overview-and-scope.md)) but the token-based architecture here is required from the start so it isn't a rewrite later.

## Test Cases

```gherkin
Feature: Design token consistency

  Scenario: No hardcoded chrome colours
    Given any UI chrome component (panel, button, toolbar, layer row)
    When its styles are inspected
    Then every colour value resolves to one of the defined design tokens
    And no literal hex/oklch colour is hardcoded outside the token definitions

  Scenario: Single accent hue across the app
    Given the active editor mode is "Pin" and a Pin tool is selected
    And a layer row is selected in the Layers panel
    Then the mode switcher's active segment, the selected tool button, and the selected layer row all use the same --accent value

  Scenario: Numeric values render in monospace
    Given the status bar shows pin count, spacing, or zoom percentage
    And a property row shows a coordinate, radius, or spacing value
    Then all of these render in the JetBrains Mono family
    And adjacent labels render in Manrope

Feature: Independent toggle chips stay visually independent

  Scenario: Grid and Snap chips never imply dependency
    Given Grid = OFF and Snap = ON
    Then both chips render with their own independent active/inactive styling
    And neither chip is visually disabled, greyed out, or nested under the other

Feature: State communicated through icon + colour, not colour alone

  Scenario: Locked layer is distinguishable without colour
    Given a colour-blind simulation or grayscale rendering of the Layers panel
    When a layer is locked versus unlocked
    Then the lock icon shape itself (closed shackle vs. open shackle) differs, independent of colour
    And the same holds for eye vs. eye-off on visibility

Feature: Pin highlight states are visually disjoint

  Scenario: Candidate and active-origin pins never share a treatment
    Given a pin in the Nearest Candidate state and a pin in the Active Thread Origin state are both visible at once
    Then their rendered styles (ring presence, fill colour) are distinguishable from each other and from a Normal pin

Feature: Thread colour count changes styling only, not geometry

  Scenario: Switching 1 → 2 → 3 colours preserves the underlying path
    Given a Thread Path rendered with colour count 1
    When colour count is changed to 3
    Then the same line geometry is reused for all three rendered strands (only stroke pattern/colour differs)
    And no additional or offset geometry is introduced

Feature: Component reuse

  Scenario: No duplicate one-off component styles
    Given two different panels each need a "selected row" treatment
    Then both use the same Layer Row / Property Row component definition
    And neither hardcodes a bespoke alternative style for the same semantic state
```
