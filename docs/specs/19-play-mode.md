# 19 — Play Mode

## Purpose

Define a fifth Editor mode, **Play**, that replays how the current document's Thread Paths were drawn — one segment at a time — with transport controls (play/pause, frame stepping, an editable frame number, a read-only total frame count, a per-frame interval) and a button to export the animation as a video file.

## Frame Model

A **frame** is one thread segment: one edge between two consecutive pins in a `ThreadPath.pinIds` array. Frames are numbered **0..totalFrames**, where frame *N* means "the first *N* segments (in draw order) are drawn." Frame **0 is the true starting state — nothing drawn yet** — and is always valid, even when `totalFrames` is 0. `totalFrames` is the sum of segment counts across every Thread Path in every Thread Layer.

**Draw-order reconstruction**: no timestamp or insertion-index field exists anywhere in the document model. Draw order is reconstructed from the current document structure: Thread Layers in their current array order → each layer's `threadPaths` in array order (append-only — nothing ever reorders paths within a layer) → each path's `pinIds` in order. This is a deterministic approximation, not a literal edit log — reordering Thread Layers (a z-order/stacking operation, see [13-layers.md](./13-layers.md)) changes replay order along with the visual stacking order. This trade-off is accepted because no ground-truth timestamp exists in the model.

## Rendering

- **All pins render unconditionally at every frame**, including frame 0 — pins are never animated in.
- Only Thread Path segments are frame-gated: at frame *N*, exactly the first *N* segments (in draw order) are visible; any path/layer beyond the frame budget is entirely absent, and the path straddling the boundary shows only its completed segments.
- The board is framed statically to show the whole board (same `fitViewportForBoard` used to fit the interactive editor's viewport) — there is no pan/zoom interaction in Play mode.
- Every other rendering concern (board fill/outline, grid, pin styling, multi-colour twisted thread strands) is identical to the interactive editor — Play mode reuses the same rendering components unchanged.

## Transport Controls

- **Layout**: a single row of five controls, matching the standard media-player convention (YouTube/Spotify/VLC) — First, Previous, **Play/Pause** (visually dominant — larger, accent-coloured), Next, Last. There is no separate Stop button: First already does exactly what Stop would (pause and reset to frame 0), so a distinct Stop control would just duplicate it.
- **Play/Pause** — a single toggle button. Play starts/resumes auto-advancing one frame every "time between frames" interval; reaching the last frame **stops advancing (does not loop)**. Pressing Play again while already at the last frame **restarts from frame 0**, matching standard media-player convention.
- **First / Previous / Next / Last** — jump to frame 0, `frame - 1`, `frame + 1`, or `totalFrames` respectively (each clamped to `[0, totalFrames]`); any of these pauses playback if it was running.
- **Frame number** — a directly editable numeric field; typing a value jumps to that frame (clamped) and pauses playback.
- **Total frames** — a read-only field showing `totalFrames`.
- **Time between frames (ms)** — a numeric input controlling the auto-advance interval.
- Entering Play mode always resets to frame 0, paused — "each time you open the player it starts from the beginning," regardless of where playback was left off on a previous visit.
- If `totalFrames === 0` (no threads drawn yet), all transport controls are disabled; the canvas shows only pins.

## Video Export

- One button, "Export to Video," available in the Play-mode panel.
- Records the animation by drawing each frame onto a hidden `<canvas>` (sized from the board's physical dimensions at a fixed internal resolution) and using `HTMLCanvasElement.captureStream()` + `MediaRecorder` to capture it — the standard browser technique for recording canvas animations to video.
- Each frame is held on the recording canvas for exactly the configured "time between frames" interval before advancing, so **the exported video's duration equals `totalFrames × intervalMs`** — the same pacing pressing Play would show.
- Each frame is rasterized by serializing the live Play-mode `<svg>` DOM node (not by re-deriving markup through the static SVG/PNG/PDF exporter), so the video matches the on-screen preview exactly, including multi-colour/twisted thread rendering.
- **Output is `.webm` only** — `MediaRecorder` has no reliably cross-browser MP4 encoder. If the browser has no `MediaRecorder` or no supported `video/webm` MIME type, the Export button is disabled and a short message explains video export isn't supported in this browser.
- While exporting, the transport controls are disabled (the export routine drives the frame position itself) and the button shows live progress (`Exporting… N/totalFrames`).

## Test Cases

```gherkin
Feature: Frame count and draw order

  Scenario: Total frame count matches the segment count across multiple layers and paths
    Given Thread Layer A has one Thread Path with 3 segments and another with 2 segments
    And Thread Layer B has one Thread Path with 4 segments
    Then the total frame count is 9

  Scenario: Scrubbing to frame N shows exactly N segments and all pins
    Given a document with 9 total thread-drawing frames
    When the user sets the frame to 4
    Then exactly 4 segments (in draw order) are visible
    And every pin in the document is visible, regardless of frame

Feature: Playback controls

  Scenario: Play auto-advances and stops at the last frame without looping
    Given the current frame is 0 and there are 5 total frames
    When the user presses Play and waits long enough for 5 interval ticks
    Then the frame advances to 5 and playback stops
    And a further interval tick does not wrap back to frame 0

  Scenario: Pressing Play again at the last frame restarts from the beginning
    Given playback has stopped at the last frame (frame equals totalFrames)
    When the user presses Play
    Then the frame resets to 0 and playback resumes advancing from there

  Scenario: First resets to frame 0 and pauses; Pause does not reset
    Given playback is running at frame 3
    When the user presses Pause
    Then playback halts and the frame remains 3
    When the user presses Play then First
    Then playback halts and the frame resets to 0

  Scenario: Editing the frame number field clamps into range and pauses playback
    Given playback is running and there are 10 total frames
    When the user types 999 into the frame number field
    Then the frame is clamped to 10
    And playback pauses

  Scenario: Entering Play mode always resets to frame 0
    Given the user previously left Play mode at frame 7
    When the user switches back into Play mode
    Then the frame is 0 (nothing drawn) and playback is paused

Feature: Video export

  Scenario: Export is disabled with no threads drawn
    Given the document has zero thread segments
    Then the Export to Video button is disabled

  Scenario: Export is disabled when the browser lacks MediaRecorder support
    Given the browser has no MediaRecorder implementation or no supported video/webm MIME type
    Then the Export to Video button is disabled and an explanatory message is shown
```

## Desktop notes

- Video export pipes raw RGBA frames rendered with tiny-skia to the system FFmpeg (found on `PATH` or the usual Homebrew / `/usr` locations): `.webm` → VP9, `.mp4` → H.264, 30 fps, each step held for the playback interval. It runs on a worker thread with progress and cancel; when FFmpeg is missing, a translated message explains how to install it.
