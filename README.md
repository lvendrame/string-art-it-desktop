# StringArtIt Desktop

A native desktop app for designing physical string-art boards: lay out pins on a board, weave threads between them,
generate patterns, then print 1:1 templates, export images/PDF/SVG or render a video of the weaving order.

It is a Rust + [egui](https://github.com/emilk/egui) port of the StringArtIt web editor. Project files are the same
JSON format, so designs move freely between the web and desktop apps.

## Features

- **Board setup** — circle, oval, rectangle, square, triangle (equilateral / right-angled); solid, gradient, wood-grain,
  painted or custom-image surface.
- **Pin mode** — line, arc, ellipse, circle, rectangle, square, polygon / star / polygram, freehand, multi-segment path
  and text (20 bundled fonts) tools; pins distributed at an exact spacing; horizontal / vertical / radial symmetry;
  pin and path erasers.
- **Edit mode** — select / multi-select at pin, path or layer granularity; move, rotate, scale, merge; keyboard
  transforms with repeat; pin distance read-out.
- **Thread mode** — click-to-weave, pattern-follow, zig-zag, parabolic curve, radial and repeat tools, thread select,
  eraser and segment eraser; multi-colour twisted threads.
- **Generate mode** — 19 parametric patterns (mandala, star, spirals, wave, lotus, flower of life, …) with live preview,
  confirmed into new layers as one undo step.
- **Layers** — pin and thread layers with visibility, lock, rename, duplicate, reorder and merge.
- **Play mode** — step through the thread order and export it as a WebM/MP4 video (system FFmpeg).
- **Print** — actual-size or fit-to-page, paper sizes, overlapping tiles with trim/alignment marks, calibration page;
  sends a vector PDF to a system printer or opens it in the PDF viewer.
- **Export** — SVG, PDF (vector), PNG, JPEG at a chosen DPI.
- **Statistics, Help** (8 tabs) and **14 UI languages** (en, pt-BR, es, fr, de, it, nl, pl, ja, ru, uk, tr, zh-CN, ko).
- **Agent server (MCP)** — optional local Model Context Protocol server so AI agents can build designs in the open
  document (see below).
- Autosave with restore prompt, unsaved-changes guard, undo/redo, copy/paste (compatible with the web app), radial
  context menu and keyboard shortcuts.

## Requirements

- Rust **1.97** (pinned in `rust-toolchain.toml`; `rustup` installs it automatically).
- macOS 11+, Windows 10+, or Linux with a Vulkan/Metal/DX12-capable GPU driver (wgpu) or OpenGL fallback.
- Optional: **FFmpeg** on `PATH` for video export (`brew install ffmpeg`, `winget install ffmpeg`, `apt install ffmpeg`).
- Linux: the usual winit/wgpu development packages (e.g. `libxkbcommon-dev`, `libwayland-dev`) and `libcups2-dev`
  for printing (Debian/Ubuntu names).

## Build and run

```sh
cargo run --release -p string-art-it
```

Set `RUST_LOG=info` to see log output (font fallback, agent server, exports).

## Tests

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

UI tests use `egui_kittest`. To save rendered screenshots of the main screens for review, run them with
`STRINGARTIT_SNAPSHOT_DIR=/some/dir cargo test --test render`.

## Packaging

Bundles are built with [cargo-packager](https://github.com/crabnebula-dev/cargo-packager) (configured in
`apps/string-art-it/Cargo.toml`):

```sh
cargo install cargo-packager --locked
cd apps/string-art-it
cargo packager --release            # default formats for the host OS
cargo packager --release -f app     # macOS .app only (also: dmg, wix, nsis, deb, appimage, pacman)
```

Output goes to `target/packages/`. Code signing and notarization are not configured.

App icons in `assets/app-icon/` are generated from `favicon.svg`:

```sh
cargo run -p string-art-it --example gen_icons
cd assets/app-icon
iconutil -c icns StringArtIt.iconset -o StringArtIt.icns && rm -r StringArtIt.iconset   # macOS
magick png/16x16.png png/24x24.png png/32x32.png png/48x48.png png/64x64.png png/256x256.png StringArtIt.ico
```

## Agent server (MCP)

Click the bot button in the top bar and turn on **Let AI agents edit this document**. The app then serves a
[Model Context Protocol](https://modelcontextprotocol.io) endpoint at `http://127.0.0.1:47821/mcp` (Streamable HTTP).
It is off by default, listens on this computer only, and rejects requests from web pages.

Add it to an MCP client, for example Claude Code:

```sh
claude mcp add --transport http string-art-it http://127.0.0.1:47821/mcp
```

Tools: `get_document_stats`, `set_board_shape`, `set_board_dimensions`, `set_board_appearance`, `add_pin_layer`,
`set_active_pin_layer`, `toggle_pin_layer_visible`, `toggle_pin_layer_locked`, `add_pin_path`, `delete_pin_path`,
`add_thread_layer`, `set_active_thread_layer`, `toggle_thread_layer_visible`, `toggle_thread_layer_locked`,
`add_thread_path`, `delete_thread_path`, `undo`, `redo`. Changes appear live on the canvas and can be undone like any
edit. The port can be changed with `agentPort` in the preferences file.

## Where files go

| What | macOS | Windows | Linux |
|---|---|---|---|
| Preferences (`prefs.json`) | `~/Library/Application Support/com.StringArtIt.StringArtIt/` | `%APPDATA%\StringArtIt\StringArtIt\config\` | `~/.config/stringartit/` |
| Autosave (`autosave.v1.json`) | `~/Library/Application Support/com.StringArtIt.StringArtIt/` | `%APPDATA%\StringArtIt\StringArtIt\data\` | `~/.local/share/stringartit/` |

(Paths come from the [`directories`](https://crates.io/crates/directories) crate.)

## Project layout

```
crates/sa-domain   pure geometry: segments, paths, pin distribution, shapes, symmetry, snapping, generator maths
crates/sa-app      document model + web-compatible project file, undo history, editor store, tools' logic,
                   generator patterns, statistics, playback, print settings, agent tools
crates/sa-render   backend-neutral scene + tiny-skia raster, SVG and PDF (krilla) backends, board fills
crates/sa-infra    i18n, fonts, export, print, video (FFmpeg), persistence, system CJK fonts, MCP server
apps/string-art-it eframe/egui app: theme, icons, widgets, canvas, tools, panels, overlays
docs/specs         functional specs (from the web app, with "Desktop notes" where behaviour differs)
docs/plan          milestone tracker
```

`sa-domain` and `sa-app` never depend on egui or IO crates.

## Third-party assets

- Icons: [Lucide](https://lucide.dev) 1.48.0 (ISC, `assets/icons/lucide/LICENSE`).
- UI fonts: Manrope and JetBrains Mono (SIL OFL 1.1); a 10-glyph subset of Noto Sans CJK (SIL OFL 1.1) for language
  names. Text-tool fonts: 20 OFL families listed in `assets/fonts/text/SOURCES.md`.
- On ja / zh-CN / ko, CJK text uses fonts already installed on the system.
