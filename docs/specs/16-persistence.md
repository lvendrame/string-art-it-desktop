# 16 — Persistence

## Purpose

Define project save/load lifecycle and the versioned file format.

## Functional Requirements

Projects support:

- New
- Save
- Open
- Duplicate
- Autosave (Phase 2)

Project files are versioned.

```json
{
  "version": 1,
  "board": {},
  "settings": {},
  "pinLayers": [],
  "threadLayers": []
}
```

## Interaction Rules

- `version` is checked on load; a loader for an older version runs a migration step before the file is handed to the Document Engine (see [01-architecture.md](./01-architecture.md), "Document Engine Responsibilities" → version migration).
- "Duplicate" creates a fully independent copy — mutating the duplicate must never affect the original (no shared references to arrays/objects between the two in-memory documents).
- "New" discards the current in-memory document (the UI layer, not this spec, is responsible for prompting to save unsaved changes first).
- Save persists the current Document Engine state as-is; it must round-trip losslessly (see [02-document-model.md](./02-document-model.md) test case for round-trip integrity).

## Test Cases

```gherkin
Feature: Project lifecycle

  Scenario: New creates an empty project
    Given no project is currently open
    When the user selects "New"
    Then a fresh Project is created with an empty pinLayers[] and threadLayers[] and a default board

  Scenario: Save persists the current document state
    Given a Project with 2 Pin Layers and 1 Thread Layer
    When the user saves
    Then the saved file contains 2 pinLayers and 1 threadLayers matching current state

  Scenario: Open restores an exact document state
    Given a previously saved project file
    When the user opens it
    Then the resulting in-memory Project exactly matches the saved state (board, layers, pins, threads)

  Scenario: Duplicate creates an independent copy
    Given an open Project "Design A"
    When the user duplicates it to "Design A copy"
    Then "Design A copy" has identical content to "Design A"
    When the user modifies a Pin Path in "Design A copy"
    Then "Design A" remains unchanged

Feature: Versioned file format

  Scenario: Saved file includes a version field
    Given the current schema version is 1
    When a project is saved
    Then the file's top-level "version" field equals 1

  Scenario: Opening an older version triggers migration
    Given a saved file with "version": 0 using an older schema shape
    When the user opens it
    Then a migration step transforms the data to the current schema before it is loaded into the Document Engine
    And the resulting in-memory Project is a valid current-version document

  Scenario: Opening a file with a newer, unsupported version is rejected safely
    Given a saved file with "version": 999 (newer than supported)
    When the user attempts to open it
    Then the application reports an incompatible-version error
    And does not attempt to partially load malformed data
```

## Desktop notes

- Native Open / Save / Save As dialogs (`rfd`); saves are atomic (temp file + rename). Files stay web-compatible JSON v1.
- The window title shows the file name with `•` when dirty; New / Open / window close prompt for unsaved changes.
- Autosave goes to `<OS data dir>/autosave.v1.json` (2 s debounce) with a restore/discard prompt at startup; preferences (language, calibration, last folder, agent server) go to `<OS config dir>/prefs.json`.
