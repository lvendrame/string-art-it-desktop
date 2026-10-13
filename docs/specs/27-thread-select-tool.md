# 27 — Thread Select Tool

## Purpose

Add a fourth Thread-mode tool, **Select**, for picking a single existing Thread Path and editing its colours (including how many), width, and twist pitch. Complements the existing Draw/Eraser/Segment Eraser tools ([12-thread-editor.md](./12-thread-editor.md), [11-erasers.md](./11-erasers.md)).

These fields are shown through the **same** properties panel used to edit the drawing defaults for the *next* thread (previously split across `ThreadToolbar` and a separate selected-thread panel) — a single dual-context panel, same pattern `PinPropertiesPanel` already uses for Pin properties, so the same set of fields never appears twice on screen at once (found live: showing both left the user unable to tell which control edited what, and the selected-thread copy didn't expose the colour-count buttons at all, so an existing thread's colour count could never be changed).

## Scope

This is a first pass, deliberately narrow:

- Single-selection only — selecting a Thread Path replaces any previously selected one. There is no Thread-mode multi-select (the multi-select model in [26-edit-mode-multi-select.md](./26-edit-mode-multi-select.md) is Edit-mode/Pin-Path-and-Pin-scoped only and does not extend to threads).
- Editable properties: **colour count** (1/2/3) and each colour, **width**, and **twist pitch** (shown only when 2+ colours, same as the drawing-defaults panel already did).
- **Out of scope for this pass**: thread geometry/`pinIds[]` editing and multi-thread selection. These may be added later but are not part of this spec.

## Selection

Clicking a Thread Path with the Select tool selects it; clicking empty canvas (no thread within hit tolerance) clears the selection. Switching to another Thread tool (Draw/Eraser/Segment Eraser) also clears the selection — a tool switch means the user is about to act with a different tool, not continue editing the selected Thread Path, so it's cleared exactly as picking any Pin tool clears a Pin Path/Pins selection ([08-pin-tools-and-properties.md](./08-pin-tools-and-properties.md)).

Hit-testing reuses the existing nearest-thread-path lookup already used by the Thread Eraser tools, within the same screen-space tolerance convention.

## Properties Panel

`ThreadPropertiesPanel` is **dual-context**, exactly like `PinPropertiesPanel`: with a Thread Path selected, its fields read and write that Thread Path; with none selected, the same fields read and write the drawing defaults used by the next drawn thread. It always renders (there is no separate "empty" state) — only the section title changes ("Thread Properties (selected)" vs "Thread Properties (defaults)") to say which context is active. `ThreadToolbar` itself holds only the Draw/Select/Eraser/Segment tool buttons and the pin-highlight-state legend — no colour/width/twist-pitch fields, so there is exactly one place these fields ever appear.

Fields:

- **Colour count** — three buttons (1/2/3); clicking one resizes `colours[]` to that length, reusing existing colour values where possible and falling back to the app's default palette for any new slots. This applies to the selected Thread Path's own `colours[]` when one is selected, not just the drawing defaults.
- **Colours** — one colour swatch per current colour.
- **Width** — a numeric field.
- **Twist pitch** — a numeric field, shown only when the current colour count is 2 or more (a 1-colour thread has nothing to twist).

Every edit writes to whichever context is active (the selected Thread Path, or the drawing defaults) and, when a Thread Path is selected, is undoable — each edit commits as one undo step.

## Locked Layers

If the selected Thread Path's layer is locked, its colour/width fields do not take effect — no change occurs, consistent with every other locked-layer editing rule ([13-layers.md](./13-layers.md)).

## Test Cases

```gherkin
Feature: Selecting a Thread Path

  Scenario: Clicking a Thread Path with the Select tool selects it
    Given the Thread Select tool is active
    And a Thread Path exists on canvas
    When the user clicks on the Thread Path
    Then that Thread Path becomes selected
    And its properties panel shows its colours and width

  Scenario: Clicking empty canvas clears the selection
    Given a Thread Path is selected
    When the user clicks on empty canvas with the Select tool
    Then no Thread Path is selected
    And the properties panel switches back to showing the drawing defaults

  Scenario: Selecting a new Thread Path replaces the previous selection
    Given Thread Path A is selected
    When the user clicks Thread Path B
    Then only Thread Path B is selected

  Scenario: Switching Thread tools clears the selection
    Given Thread Path A is selected with the Select tool
    When the user switches to the Draw tool
    Then no Thread Path is selected

Feature: Editing a selected Thread Path's properties

  Scenario: Changing a colour swatch updates the Thread Path
    Given a one-colour Thread Path is selected
    When the user changes its colour swatch
    Then the Thread Path's colours[] reflects the new colour
    And the change is undoable in one step

  Scenario: Changing width updates the Thread Path
    Given a Thread Path is selected with width 1.5
    When the user sets width to 2.0 in the properties panel
    Then the Thread Path's width becomes 2.0
    And the change is undoable in one step

  Scenario: Editing colours/width does not affect the drawing defaults
    Given a Thread Path is selected and edited to a new colour
    When the user starts drawing a new Thread Path
    Then the new Thread Path uses the existing drawing defaults, not the edited path's colour

  Scenario: The colour-count buttons change the selected Thread Path's own colour count
    Given a one-colour Thread Path is selected
    When the user clicks the "3" colour-count button
    Then the Thread Path's colours[] grows to 3 entries
    And the change is undoable in one step

  Scenario: Twist pitch only appears once the selected Thread Path has 2+ colours
    Given a one-colour Thread Path is selected
    Then no twist pitch field is shown
    When the user changes its colour count to 2
    Then a twist pitch field appears, editing that Thread Path's own twist pitch

  Scenario: Only one set of colour/width/twist-pitch fields is ever shown
    Given a Thread Path is selected
    Then exactly one Width field and one set of colour-count buttons are visible
    And ThreadToolbar shows no colour/width/twist-pitch fields of its own

Feature: Locked layer blocks editing

  Scenario: Cannot edit a Thread Path on a locked layer
    Given a selected Thread Path belongs to a locked Thread Layer
    When the user attempts to change its colour or width
    Then no change occurs
```
