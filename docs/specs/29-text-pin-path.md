# 29 — Text Pin Path

## Purpose

Define a Text drawing tool for Pin mode: the user places a single origin point, then types a string that is rasterised via a real font into pin geometry — every letter's outline becomes pins, using the same distribution rules as every other shape ([07-pin-geometry-engine.md](./07-pin-geometry-engine.md)). Unlike every other Pin tool, Text is click-once-then-type rather than click-drag-release.

## Text Tool

Added to the Pin toolbar's basic tools ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md) §Pin Drawing Tools), next to Freehand.

1. The user selects the Text tool and clicks once on the board — this places the origin and creates an empty Text Pin Path (no pins yet, since no text has been typed).
2. The editor immediately switches to Edit ("Select") mode with the new Pin Path selected — a Text-specific hand-off (`EditorStore.editPinPath`), since the Text field lives in the Edit-mode Selection panel. Every other Pin tool stays in Pin mode on creation ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)).
3. The Selection panel shows a Text field group, **Text field first**, then Font, Weight, Italic, Size, Letter spacing — and the Text field is focused automatically the moment the panel shows it, so the user can start typing immediately with no extra click. Focus happens on that same selection-becomes-a-Text-Pin-Path transition (whether from the tool's placement click or from re-selecting an existing Text Pin Path later), not on every keystroke — typing itself never steals focus back.
4. Every change to any of these six fields regenerates the Pin Path's contours from the font and recomputes pins, live — the same per-keystroke commit behaviour every other Selection panel field already has ([09-selection-and-editing.md](./09-selection-and-editing.md)'s geometry fields, [08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)'s Spacing field).

Cancelling: since the Pin Path is created (empty) on the first click rather than after a multi-step gesture, there is no "cancel before commit" state — the empty Text Pin Path can be removed like any other Pin Path (Path Eraser, or Delete Pin Path in the Selection panel) if the user changes their mind before typing anything.

## Text Configuration Fields

Listed in the order they appear in the Selection panel:

| Field | Description |
|---|---|
| Text | The string itself. First field in the group, and the one that receives focus automatically when a Text Pin Path becomes selected |
| Font | One of 20 bundled font families (see Font Library below), spanning serif, sans-serif, cursive, and monospace |
| Weight | Regular or Bold — only the weights that actually exist as files for the selected font are offered (most script/cursive fonts ship Regular only) |
| Italic | On/off. Disabled for a font with no italic file |
| Size | Font size, in the app's physical document units (cm) — same convention as every other length field (Circle radius, Line length) |
| Letter spacing | Extra physical gap added between letters, in cm, beyond the font's own advance width |

Weight and Italic never combine into a single "Bold Italic" file — Italic always resolves to the font's plain italic file regardless of the Weight selection, since combined Bold+Italic variants are not part of the bundled font set (kept intentionally small — see Font Library).

## Geometry Model — Multi-Contour Text

Every other `PinPathGeometry` variant maps to exactly one continuous guide path. A letter can require more than one closed loop — an outer ring plus a hole ("o", "e", "a"), or a disconnected piece ("i"'s dot, "!"'s dot) — so Text introduces the first multi-contour geometry:

```text
PinPathGeometry (type: "text")
├── origin            // where the user clicked
├── text, fontId, weight, italic, size, letterSpacing  // the editable fields
├── rotation           // bumped by the Rotation tool; unused by layout
└── contours: Point[][]  // DERIVED CACHE — already flattened, already positioned
```

`contours` is produced by walking the string one character at a time through the selected font (so each letter advances by its own real width plus `letterSpacing`, keeping "each letter is a different shape" true rather than merging every letter's outline into one combined path), flattening every glyph's Bezier curves into a fixed-density polyline per closed contour, and shifting every point by `origin`. This keeps the geometry a **single Pin Path** — one selectable, movable, undoable object — per [02-document-model.md](./02-document-model.md)'s "a geometric pin drawing remains a Pin Path instead of decomposing into unrelated individual points."

`contours` is a derived cache exactly the way `pins[]` already is a derived cache of every other shape's `geometry` — it is recomputed by the UI layer (the only layer that talks to the font-parsing library) whenever any of the six text fields changes, and consumed read-only by pin distribution and rendering. It also round-trips through Save/Open like `pins[]` does, so a saved project renders correctly without re-resolving the source font at load time.

## Pin Distribution — Multi-Contour Closed-Path Rule

See [07-pin-geometry-engine.md](./07-pin-geometry-engine.md#multi-contour-closed-path-distribution-text) for the full rule. In short: each contour is distributed independently using the existing **closed-path** algorithm (the same one Circle/Ellipse use — continuous accumulation around one perimeter, closest-integer-interval-count spacing), because a flattened glyph contour is a compound curved path, not a shape defined by real straight-edge vertices. Pins from every contour concatenate into one flat `pins[]`, same shape as every other Pin Path. `actualSpacing` reports the first contour's value as a representative summary (the same "one number even though it varies per piece" precedent `distributePathPerVertex` already uses for per-edge shapes).

## Generic Tool Support (Move / Rotate / Scale / Merge)

`contours` are absolute points, so Text gets the exact same treatment Freehand's `points[]` already gets in every generic geometry function:

- **Move** shifts `origin` and every contour point by the drag delta — pins keep their ids, exactly like every other Move.
- **Rotation** / **Scale** rotate/scale `origin` and every contour point about the tool's pivot, directly — they do **not** re-lay-out the font at an angle or a new size.
- **Merge** needs no Text-specific code — it already operates purely on `pins[]`, agnostic of geometry type.

**Scope note:** because Rotate/Scale transform the baked points rather than the font layout, if the user rotates or scales a Text Pin Path and *then* edits any of the six text fields again, the contours are regenerated from scratch at the field's literal (unrotated, unscaled) values — the manual transform is discarded. This is the same "a typed edit regenerates from scratch, a manual point-level edit doesn't survive it" precedent already documented for the Pin Eraser vs. geometry edits elsewhere in this app — not a new class of gap.

## Font Library

20 bundled font families under `public/fonts/`, all SIL Open Font License 1.1 (Google Fonts), each with Regular + Bold (where the family ships both) + Italic (regular weight only). Covers all four required categories:

| Category | Fonts |
|---|---|
| Sans-serif | Lato, Poppins, PT Sans, Barlow, Hind, Titillium Web |
| Serif | PT Serif, Crimson Text, Arvo, Cardo, Tinos |
| Cursive | Pacifico, Sacramento, Great Vibes, Kaushan Script, Alex Brush (Regular only — no bold/italic variants exist for these script faces) |
| Monospace | Courier Prime, Space Mono, IBM Plex Mono, Anonymous Pro |

Full provenance and per-font weight availability: `public/fonts/SOURCES.md`.

## Font Loading

Fonts are fetched and parsed (via `opentype.js`) on demand, not bundled into the JS build. Once a (font, weight, italic) combination has been loaded, it is cached in memory for the rest of the session — switching back to it, or typing further text against it, never re-fetches. Only a Font/Weight/Italic **switch** to a not-yet-loaded combination pays a network+parse cost; plain typing never does, since it reuses whichever font is already loaded.

## Test Cases

```gherkin
Feature: Text tool placement

  Scenario: One click places an empty Text Pin Path and switches to Edit mode
    Given the editor is in Pin mode with the Text tool active
    When the user clicks a point on the board
    Then a Text Pin Path is created at that point with no pins yet
    And the editor mode switches to Edit (Select)
    And the new Pin Path is selected

  Scenario: The empty Text Pin Path can be removed before typing anything
    Given a Text Pin Path was just placed with no text typed
    When the user deletes it via the Selection panel or the Path Eraser
    Then no Text Pin Path remains

Feature: Text field order and focus

  Scenario: The Text field is first in the field group
    Given a Text Pin Path is selected
    Then the Text field appears before Font, Weight, Italic, Size, and Letter spacing

  Scenario: Placing a Text Pin Path focuses the Text field immediately
    Given the editor is in Pin mode with the Text tool active
    When the user clicks a point on the board
    Then the Selection panel's Text field has keyboard focus
    And the user can type without first clicking into the field

  Scenario: Re-selecting an existing Text Pin Path also focuses its Text field
    Given a Text Pin Path exists and something else is currently selected
    When the user selects that Text Pin Path with the Select tool
    Then its Text field receives focus

  Scenario: Typing does not repeatedly steal focus back
    Given a Text Pin Path is selected and the user has since moved focus away from the Text field
    When the user edits the Text field, triggering a contour regeneration and re-render
    Then focus is not reset back to the Text field by that re-render — it only moves on an actual selection change, never on every keystroke

Feature: Multi-contour geometry

  Scenario: A hole letter produces more than one contour
    Given the user types a letter with an enclosed counter, e.g. "o"
    Then the Pin Path's geometry has at least 2 contours
    And pins are distributed independently on both the outer ring and the inner hole

  Scenario: Every letter advances by its own real width plus letter spacing
    Given the user types two letters
    Then the second letter's contours are positioned to the right of the first by that letter's advance width plus the configured Letter spacing
    And the two letters' contours do not overlap

Feature: Multi-contour pin distribution

  Scenario: Each contour is distributed independently using the closed-path rule
    Given a Text Pin Path with two contours of different perimeters
    When pins are distributed
    Then each contour's pin count and actual spacing are computed independently via the closed-path algorithm
    And the Pin Path's pins[] is the concatenation of both contours' pins
    And no pin is duplicated at either contour's start/end seam

  Scenario: An empty string produces zero pins without error
    Given a Text Pin Path with an empty text field
    Then it has zero contours and zero pins

Feature: Live field editing

  Scenario: Typing updates the pin path live
    Given a Text Pin Path is selected
    When the user types an additional character into the Text field
    Then the geometry's contours are regenerated to include the new character
    And pins are recomputed to match

  Scenario: Changing Font, Weight, Italic, Size, or Letter spacing regenerates contours
    Given a Text Pin Path is selected with existing text
    When the user changes any of Font, Weight, Italic, Size, or Letter spacing
    Then the contours are regenerated from the (possibly newly loaded) font at the new field value
    And the text content itself is unchanged

  Scenario: Italic is unavailable for a font with no italic file
    Given a script font that ships only a Regular file (e.g. Pacifico)
    Then the Italic control is disabled

Feature: Generic tool support

  Scenario: Move translates the whole Text Pin Path
    Given a Text Pin Path is selected
    When the user drags it with the Move tool
    Then the origin and every contour point shift by the drag delta
    And pin ids remain stable

  Scenario: Editing text after a manual Scale discards the scale
    Given a Text Pin Path was scaled up with the Scale tool
    When the user then edits the Text field
    Then the contours are regenerated from the font at the field's literal (unscaled) size
    And the manual scale is not preserved

  Scenario: Merge treats Text Pin Path pins like any other pins
    Given a Text Pin Path and a Circle Pin Path with pins selected from each
    When the user commits a Merge
    Then the merge proceeds exactly as it would for two non-text shapes

Feature: Persistence

  Scenario: A saved project renders its text without re-fetching the font
    Given a project containing a Text Pin Path is saved and reloaded
    Then the Pin Path's contours and pins render immediately from the saved data
    And no font is re-fetched to display it
```
