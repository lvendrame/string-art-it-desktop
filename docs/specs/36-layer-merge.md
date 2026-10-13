# 36 — Layer Merge

## Purpose

Define a "Merge with layer above" action for the Layers panel: the fastest way to fold two layers into one without manually re-drawing or re-parenting content. The operation is identical in shape for Pin Layers and Thread Layers, mirroring how every other layer operation in [13-layers.md](./13-layers.md) is shared across both kinds.

This is unrelated to the Edit-mode **Merge** tool in [09-selection-and-editing.md](./09-selection-and-editing.md), which combines individual *pins* into one averaged pin. This spec merges whole *layers*.

## Functional Requirements

- A "Merge with layer above" action is available from the Layers panel, acting on the currently active layer (same active-layer targeting convention as Duplicate/Delete/Move — see [13-layers.md](./13-layers.md) §Layer Operations).
- "Above" means the row immediately preceding the active layer in the Layers panel list — the same target the existing Move Up action operates toward.
- Merging moves every object the active layer contains (its Pin Paths, or its Thread Paths) into the target layer, appended **after** the target layer's existing content (renders/stacks on top of what was already there), then deletes the active layer.
- The merged layer keeps the **target (above) layer's** name, id, visibility, and locked state — the active (source) layer's own name/id disappear along with it.
- Pin Layer merge does not regenerate pin IDs (unlike Duplicate) — existing Thread Paths that reference a moved pin keep resolving to the same pin, since the pin's identity, not its layer, is what a Thread Path references (per [02-document-model.md](./02-document-model.md)).
- Thread Layer merge does not rewrite `pinIds` on any moved Thread Path — Thread Paths reference pins by stable document-wide ID, independent of which Thread Layer they live in.
- The whole operation (content move + source layer deletion) is a single undoable step.

## Guarantees

- **Disabled on the topmost layer.** A layer with no layer above it (first row in the panel list) cannot be merged — there is nothing to merge into.
- **Disabled when either layer is locked.** If the active layer or the target (above) layer is locked, the action does not run — locking either side of a merge blocks it, distinct from the lighter-touch guarantee in [13-layers.md](./13-layers.md) that locking only blocks *content* edits for other layer operations; merge is content-moving, so both layers' lock state is checked here specifically.
- **Hidden layers can still be merged.** Visibility, unlike lock state, does not block merge (consistent with "locked layer still allows visibility toggling" in [13-layers.md](./13-layers.md) — the inverse also holds: hidden doesn't block content operations either).
- If the active layer being merged away was the panel's active layer, the target layer becomes active afterward.

## Test Cases

```gherkin
Feature: Merge a layer into the layer above

  Scenario: Merging a Pin Layer moves its content into the layer above
    Given Pin Layers in order [A, B] where B is active and contains 2 Pin Paths, and A contains 1 Pin Path
    When the user merges B into the layer above
    Then only layer A remains
    And A contains 3 Pin Paths: its original Pin Path followed by B's 2 Pin Paths
    And A's name and id are unchanged

  Scenario: Merging a Thread Layer moves its content into the layer above
    Given Thread Layers in order [A, B] where B is active and contains 1 Thread Path, and A contains 2 Thread Paths
    When the user merges B into the layer above
    Then only layer A remains
    And A contains 3 Thread Paths: its original 2 followed by B's 1

  Scenario: Merging a Pin Layer preserves existing thread references
    Given a Pin Layer B (active) with pin-1, referenced by a Thread Path in a Thread Layer, and a Pin Layer A above it
    When the user merges B into A
    Then pin-1 now lives inside A
    And the Thread Path still references pin-1 and renders unchanged

  Scenario: Merge is unavailable on the topmost layer
    Given a Pin Layer "A" is the topmost layer (no layer above it)
    Then the Merge action is disabled for "A"

  Scenario: Merge is blocked when the active layer is locked
    Given Pin Layers in order [A, B], B is active and locked
    Then the Merge action is disabled for B

  Scenario: Merge is blocked when the target layer is locked
    Given Pin Layers in order [A, B], B is active and unlocked, A is locked
    Then the Merge action is disabled for B

  Scenario: Merging a hidden layer is allowed
    Given Pin Layers in order [A, B], B is active, unlocked, and hidden
    When the user merges B into A
    Then the merge succeeds and A contains both layers' content

  Scenario: Merge is a single undo step
    Given Pin Layers in order [A, B], B is active and contains 2 Pin Paths
    When the user merges B into A
    Then A contains its original content plus B's 2 Pin Paths, and B no longer exists
    When the user invokes Undo once
    Then B is restored with its original 2 Pin Paths, and A is restored to its pre-merge content

  Scenario: Merging reassigns the active layer
    Given Pin Layers in order [A, B], B is active
    When the user merges B into A
    Then A becomes the active Pin Layer
```
