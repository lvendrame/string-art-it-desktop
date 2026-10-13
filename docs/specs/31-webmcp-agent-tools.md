# 31 — WebMCP Agent Tools

## Purpose

Expose StringArtIt's core authoring actions — board setup, pin layers/paths, thread layers/paths, undo/redo, and read-only stats — as [WebMCP](https://github.com/webmachinelearning/webmcp) tools via `navigator.modelContext.registerTool`, so an AI agent visiting the live site can drive the app the same way a person does through the UI, without simulating pixel-accurate clicks/drags.

This is a genuinely public, stable-contract surface — unlike `window.stringArtItDebug` (`App.tsx`, `.claude/skills/test-with-debug-hook/`), which is explicitly unstable dev/test scaffolding with no compatibility guarantee. WebMCP tools are documented, versioned by this spec, and covered by the exit-criteria tests below like any other milestone.

## Browser support (accepted limitation)

`navigator.modelContext` shipped in Chrome 146 Canary (Feb 2026, W3C Community Group draft) behind the `webmcp` flag — it does not exist in any stable browser release as of this writing. Registration is feature-detected (`"modelContext" in navigator`) and is a total no-op everywhere else — zero behavioural change, zero bundle-size-relevant risk to the existing app when unsupported. This means the real browser dispatch path (an actual agent calling a tool through Chrome's implementation) cannot be live-verified in this environment; see Test Cases' final scenario and the Exit Criteria note in the orchestrator entry.

## Tools

All tools are registered once, for the lifetime of the app, from a single `EditorStore` instance — mirrors where the debug hook is wired (`App.tsx`), but as its own module (`src/infrastructure/webmcp/tools.ts`) so the two concerns stay separably documented. Every mutating tool reuses an existing `EditorStore` method unchanged (no new business logic) and therefore inherits that method's existing undo/locked-layer/validation behaviour for free.

### Read-only

- **`get_document_stats`** — no input. Returns board shape/dimensions/appearance, per-pin-layer summary (id, name, visible, locked, path count, pin count) via `projectTotalPins`, per-thread-layer summary (id, name, visible, locked, path count, total length in cm) via a new `projectThreadTotals` (`src/application/document/statistics.ts`, mirrors the existing `threadPathStatistics`/`projectTotalPins` pattern), and `canUndo`/`canRedo`.

### Board setup

- **`set_board_shape`** — `{ shape: "circle"|"oval"|"rectangle"|"square"|"triangle", triangleType?: "equilateral"|"right-angled" }` → `store.setBoardShape`.
- **`set_board_dimensions`** — partial `{ diameter?, width?, height?, side?, base? }` (cm) → `store.setBoardDimensions`.
- **`set_board_appearance`** — `{ type: "solid", colour: string } | { type: "wood-texture"|"painted-wood", presetId: string }` → `store.setBoardAppearance`. Scope note: gradients and custom-texture-upload are intentionally excluded from v1 — an uploaded image or a multi-stop gradient isn't a sensible thing for an agent to author blind; the existing UI remains the only way to set those.

### Pin layers

- **`add_pin_layer`** — no input → `store.addPinLayer()` (also activates it, per existing behaviour), returns the new layer id (read back from `store.getState().activePinLayerId`).
- **`set_active_pin_layer`** — `{ layerId }` → `store.setActivePinLayer`.
- **`toggle_pin_layer_visible`** / **`toggle_pin_layer_locked`** — `{ layerId }` → the matching existing toggle method.

### Pin paths

- **`add_pin_path`** — `{ layerId, geometry }`, where `geometry` is the existing `PinPathGeometry` discriminated union (`src/application/document/pinPath.ts`) restricted to the 10 shape types that don't need font infrastructure: `line`, `arc`, `ellipse`, `circle`, `rectangle`, `square`, `regular-polygon`, `star`, `polygram`, `freehand`. Scope note: `text` geometry is excluded — it requires `opentype.js` font loading (`src/infrastructure/fonts/`), which is intentionally UI-only per 29-text-pin-path.md, so a WebMCP tool can't produce it without duplicating that infra. → `store.addPinPath(layerId, geometry)`, which already derives `pins[]` from the store's current pin-spacing default and leaves the current mode and selection untouched. Returns the new path id (or `null` if the layer is locked, surfaced as a structured `{ success: false, reason: "layer_locked" }`).
- **`delete_pin_path`** — `{ layerId, pathId }` → `store.deletePinPath`.

### Thread layers

- **`add_thread_layer`** — no input → `store.addThreadLayer()`, returns the new layer id (read back from `activeThreadLayerId`).
- **`set_active_thread_layer`** — `{ layerId }` → `store.setActiveThreadLayer`.
- **`toggle_thread_layer_visible`** / **`toggle_thread_layer_locked`** — `{ layerId }` → the matching existing toggle method.

### Thread paths

- **`add_thread_path`** — `{ layerId, pinIds: string[] }` (2+ pin ids, existing pins on any Pin Layer). Composes the same primitives the UI's click-to-draw uses: `store.cancelThreadDraft()` (discard any stray in-progress draft first — scope note below), then `store.extendThreadDraft(pinId)` for each id in order, then `store.finishThreadDraft(layerId)`. No new store method. Returns the new path id by comparing `threadLayers` before/after, or a structured failure if the layer is locked or fewer than 2 ids resolve to real pins.
- **`delete_thread_path`** — `{ layerId, pathId }` → `store.deleteThreadPath`.

Scope note: `add_thread_path` assumes no user-initiated thread draft is genuinely in progress when the tool fires — it unconditionally clears `threadDraft` first. A human actively mid-draw in the Thread tool at the exact moment an agent calls this tool is an accepted, undocumented-elsewhere edge case (same class of "not engineered for" as this codebase's other concurrent-actor gaps), not a new mechanism.

### History

- **`undo`** / **`redo`** — no input → `store.undo()`/`store.redo()`. Both return the resulting `{ canUndo, canRedo }`.

## Test Cases

```gherkin
Feature: WebMCP agent tools

  Scenario: Tools register only when the browser supports WebMCP
    Given "modelContext" does not exist on navigator
    When registerWebMcpTools(store) runs
    Then no error is thrown and nothing is registered

  Scenario: Registering returns an unregister function
    Given a mocked navigator.modelContext
    When registerWebMcpTools(store) runs
    Then every tool name from this spec is registered exactly once
    And calling the returned cleanup function unregisters every one of them

  Scenario: get_document_stats reflects live document state
    Given a document with 2 pin layers (one with a drawn circle) and 1 thread path
    When get_document_stats executes
    Then the returned board/pin-layer/thread-layer/pin/thread counts match the real state exactly

  Scenario: set_board_dimensions mutates the board and is undoable
    When set_board_dimensions executes with { diameter: 45 }
    Then store.getState().board.dimensions.diameter is 45
    And store.undo() restores the previous diameter

  Scenario: add_pin_path creates a real, selected Pin Path
    When add_pin_path executes with a circle geometry on the active pin layer
    Then the layer gains one Pin Path with pins distributed per the current spacing default
    And the returned id matches the new Pin Path's id

  Scenario: add_pin_path on a locked layer fails structurally, not by throwing
    Given the target pin layer is locked
    When add_pin_path executes
    Then it returns { success: false, reason: "layer_locked" } and the layer is unchanged

  Scenario: add_thread_path composes into one thread and one undo step
    Given 4 existing pins across 2 Pin Paths
    When add_thread_path executes with those 4 pin ids in order
    Then exactly one new Thread Path is created connecting them in that order
    And a single store.undo() removes it entirely

  Scenario: undo/redo report resulting history state
    Given 1 undoable action has just been performed
    When undo executes
    Then it returns canUndo:false, canRedo:true (or the correct values for the document's real history depth)

  Scenario: real Chrome WebMCP dispatch (accepted gap)
    Given no stable browser ships navigator.modelContext as of this spec
    Then this path is not live-verified end-to-end — only feature-detection safety (registration is a no-op in a real, current browser) is live-verified via Playwright; tool-registration and execute-handler correctness are covered by unit tests against a mocked navigator.modelContext instead
```

## Desktop notes

- WebMCP (`navigator.modelContext`) has no desktop equivalent; the same 18 tools — names, input schemas, result shapes and structured failures — are served by a local **MCP server** (`sa-app/src/agent_tools.rs` + `sa-infra/src/mcp.rs`, rmcp Streamable HTTP) at `http://127.0.0.1:<port>/mcp` (default port 47821).
- It is **off by default**; the bot button in the top bar turns it on (remembered in prefs) and shows the URL plus a `claude mcp add --transport http …` command.
- Calls are queued to the UI thread and run against the live document, so the canvas updates and every mutation is a normal undo step.
- Security: loopback bind only, `Host` allow-list (DNS-rebinding guard) and an `Origin` allow-list so web pages cannot call it.
- Tool errors (unknown tool, malformed input, unsupported appearance or `text` geometry) return MCP `isError` results; domain refusals stay `{ success: false, reason }` like the web.
