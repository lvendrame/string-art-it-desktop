# StringArtIt Desktop

Native Rust + egui port of the StringArtIt web editor (`~/projects/pocs/string_art_it_2`).
Functional specs: [`docs/specs/`](docs/specs/) (copied from web; spec text is normative, the TS code wins where a spec is stale).
Design system: [`docs/specs/18-design-system.md`](docs/specs/18-design-system.md).

## Start here

Read [`docs/plan/orchestrator.md`](docs/plan/orchestrator.md). Pick the first milestone not `Done`, read its specs and the
listed web TS sources, port their vitest cases to Rust tests, implement, verify, then update the status row.

## Layout

- `crates/sa-domain` — pure geometry (no IO, no egui).
- `crates/sa-app` — document model, project file v1 (web-compatible JSON), commands/undo, editor state, algorithms.
- `crates/sa-render` — backend-neutral Scene + tiny-skia/SVG/PDF backends.
- `crates/sa-infra` — fonts, export, video (FFmpeg), print, autosave/prefs, i18n, MCP server.
- `apps/string-art-it` — eframe/egui binary (theme, icons, widgets, panels, canvas).

`sa-domain` and `sa-app` must never depend on egui or IO crates.

## Assets

- Icons: `assets/icons/lucide/*.svg` pinned to lucide-static **1.48.0** (same glyphs as web `lucide-react@1.48.0`) +
  `assets/icons/custom/` (Layers eye/lock). Add new icons by copying from lucide-static 1.48.0 and adding a row to `icons!` in `apps/string-art-it/src/icons.rs`.
- UI fonts: Manrope + JetBrains Mono static instances (`assets/fonts/ui`) + a 10-glyph Noto Sans CJK subset for native language names; CJK UI text uses system fonts (`sa-infra/src/system_fonts.rs`). Text-tool fonts: `assets/fonts/text` (20 OFL families).
- App icons: `assets/app-icon/` generated from `favicon.svg` (`cargo run -p string-art-it --example gen_icons`, see README).
- Locales: `assets/locales/<lng>/<ns>.json` — i18next JSON copied verbatim from web, plus the desktop-only `desktop.json` (keep all 14 in sync; key-parity test enforces it).

## Before considering any change done

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
