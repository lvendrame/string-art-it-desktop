# 24 — Internationalization

## Purpose

StringArtIt ships in fourteen languages: English (default), Portuguese (pt-BR), Spanish (es), French (fr), German (de), Italian (it), Dutch (nl), Polish (pl), Japanese (ja), Russian (ru), Ukrainian (uk), Turkish (tr), Simplified Chinese (zh-CN), and Korean (ko). A user switches between them via a language dropdown present on both the Board Setup screen and the main Editor's top bar; the choice applies immediately across the whole UI and persists across reloads. This spec covers the language-switching mechanism, translation coverage, and the explicit scope boundaries of what does and does not get translated.

Language selection is ephemeral UI/application preference, not document state — it never routes through `EditorStore`'s `HistoryStack`/`Command` machinery and is never undoable, matching how `overlay` (Stats/Print/Help) and Play mode's local transport state are already treated ([01-architecture.md](./01-architecture.md)).

## Supported Languages & Detection

Fourteen languages: `en` (English, default), `pt-BR` (Portuguese), `es` (Spanish), `fr` (French), `de` (German), `it` (Italian), `nl` (Dutch), `pl` (Polish), `ja` (Japanese), `ru` (Russian), `uk` (Ukrainian), `tr` (Turkish), `zh-CN` (Simplified Chinese), and `ko` (Korean). On first visit, with no saved preference, the initial language is detected from `navigator.language` by case-insensitive prefix match: `pt*` (`pt`, `pt-BR`, `pt-PT`, …) resolves to `pt-BR`; `es*` resolves to `es`; `fr*` resolves to `fr`; `de*` resolves to `de`; `it*` resolves to `it`; `nl*` resolves to `nl`; `pl*` resolves to `pl`; `ja*` resolves to `ja`; `ru*` resolves to `ru`; `uk*` resolves to `uk`; `tr*` resolves to `tr`; `zh*` (including Traditional `zh-TW`/`zh-HK`, until a Traditional locale exists) resolves to `zh-CN`; `ko*` resolves to `ko`; anything else falls back to `en`. Once the user makes an explicit choice via the Language Switcher, it is persisted to `localStorage` under `stringartit:language:v1` (mirroring the `stringartit:autosave:v1` convention in `src/infrastructure/persistence/autosave.ts`) and that stored choice always wins over browser-locale detection on subsequent visits.

## Language Switcher

A shared component (`src/ui/LanguageSwitcher.tsx`), mounted identically in two places — the Board Setup screen's header row (next to the "New Board" title) and the Editor Shell's top bar (after the Print button) — both reading and writing the same global `i18next` language state, so they always agree with no prop threading between them.

It is a **custom-built dropdown**, not a native `<select>`: a `.btn`-styled trigger (Globe icon + current language's native name + chevron) opens an absolutely-positioned menu (`role="listbox"`) listing every supported language as `role="option"` buttons, the active one marked with a checkmark. This is a deliberate new UI pattern — `18-design-system.md`'s Component Vocabulary previously had no Select/Dropdown entry beyond the segmented mode switcher (scoped to ≤4 options as a tab-like control, not a real dropdown) and the plain `<select>` elements used for paper size / board texture presets. The switcher closes on outside click (`pointerdown` outside its container) and on `Escape`, and supports `ArrowUp`/`ArrowDown` to move focus between its options.

## Translation Coverage

Every user-facing string across `src/ui/` is sourced from `react-i18next`, split into namespaces mirroring the UI's existing directory structure:

| Namespace | Covers |
|---|---|
| `common` | Shared chrome reused in ≥2 places: the "Close" button word, the five Editor-mode words (also used by `ModeSwitcher` and the Help tab labels), and the Language Switcher's own strings |
| `boardSetup` | Board Setup screen + Board Appearance panel |
| `editorShell` | Editor Shell's own top-bar labels (Undo/Redo/Stats/Help/Print) |
| `menus` | File menu, Export menu |
| `toolbars` | Select/Pin/Thread/Play toolbars, Canvas toolbar |
| `panels` | Pin Properties, Selection, Symmetry, Layers, Statistics panels |
| `printPreview` | Print Preview panel (its own namespace — the single largest non-Help surface) |
| `canvas` | Status bar text, the board canvas's accessible name |
| `help` | The full in-app Help panel content |
| `errors` | User-visible error alert copy (Open Project, Export) |

**Explicit scope-outs** (deliberately never translated):
- The "StringArtIt" brand name in the Editor Shell's top bar.
- Internal invariant-guard error messages thrown by `src/domain/` (`distribution.ts`, `polygonFamily.ts`, `radial.ts`) — these are programmer-error guards never surfaced through a UI catch block, and importing i18next into the framework-free domain layer would violate the layer boundary `01-architecture.md` establishes (enforced by ESLint since M0), even though the boundary rule's literal package-ban only names `react`/`react-dom`.
- The raw `err.message` from raster/PDF export failures (`ExportMenu.tsx`) when an `Error` is actually thrown — this is unscripted diagnostic detail from `svg2pdf.js`/canvas internals, not authored UX copy. Only the *fallback* copy shown when no `Error` message exists is translated.
- Internal tool-id/discriminant strings (e.g. the raw pin-tool name shown in the Pin-mode status bar hint) that were already displayed as raw lowercase identifiers before this milestone — this preserves pre-existing behavior rather than inventing new display names as part of an unrelated change.

## Error Message Translation

`src/application/document/projectFile.ts`'s three failure modes are each a named `Error` subclass (`InvalidProjectFileError`, `NoMigrationPathError`, `IncompatibleProjectVersionError`) carrying structured data (e.g. `foundVersion`) rather than only an English message. `src/ui/toolbars/openFileErrors.ts`'s `mapOpenFileError()` discriminates on `instanceof` and returns translated, interpolated copy from the `errors` namespace — translation never depends on matching against the English `.message` text, which stays as internal/diagnostic content.

## Help Panel Translation

`src/ui/panels/help/helpContent.ts` holds structural data only — `HelpTab`/`HelpSection`/`HelpItem` carry `labelKey`/`descriptionKey`/`headingKey` translation keys (hand-authored per item, not auto-slugged from English text) rather than literal strings, resolved via `t()` in `HelpPanel.tsx`/`HelpSectionView` at render time. This keeps the existing data-driven-content convention (`docs/conventions/ui-patterns.md`) intact while making every tab's content fully translatable, including the ~240 lines of tool/control descriptions. The five tabs that mirror Editor modes (Edit/Pin/Thread/Pan/Play) reuse `common:modes.*` for their tab label instead of duplicating the translation.

## Testing Architecture

`react-i18next`'s `useTranslation()` hook falls back to a module-level singleton instance when no `<I18nextProvider>` wraps the component tree. `src/i18n/index.ts` calls `i18n.use(initReactI18next).init({...})` as an import side-effect, imported once in `src/main.tsx` (the app) and once in `src/test/setup.ts` (the global Vitest setup) — so every existing component test continues to `render()` directly with no wrapper, and resolves real translated text synchronously (English JSON bundled into the main chunk, `initAsync: false`, no Suspense). This holds only as long as every English JSON value is byte-for-byte identical to the literal string it replaced — the discipline followed throughout this milestone's implementation, verified by running the full existing test suite unmodified after each batch of extraction.

## Test Cases

```gherkin
Feature: Language switcher

  Scenario: Dropdown lists every supported language
    Given the Language Switcher is closed
    When the user opens it
    Then "English", "Português (BR)", "Español", "Français", "Deutsch", "Italiano", "Nederlands", "Polski", "日本語", "Русский", "Українська", "Türkçe", "简体中文", and "한국어" are all listed as options

  Scenario: Selecting a language updates visible text immediately
    Given the app is showing English text in Board Setup
    When the user selects "Português (BR)" from the Language Switcher
    Then the Board Setup screen's visible text is now in Portuguese
    And opening the main editor also shows Portuguese text in its top bar

  Scenario: Language choice persists across reload
    Given the user has selected "Português (BR)"
    When the page is reloaded
    Then the app opens in Portuguese without the user re-selecting it

  Scenario: First visit auto-detects from the browser locale
    Given no language preference has been saved
    And the browser's language starts with "pt"
    When the app loads
    Then the UI renders in Portuguese

  Scenario: First visit falls back to English for a non-Portuguese browser locale
    Given no language preference has been saved
    And the browser's language does not start with "pt"
    When the app loads
    Then the UI renders in English

  Scenario: Help panel is fully translated
    Given the language is "Português (BR)"
    When the Help panel is opened on any tab
    Then every section heading and item label/description renders in Portuguese

  Scenario: Error alert text is translated
    Given the language is "Português (BR)"
    When the user attempts to open an invalid project file
    Then the alert text shown is the Portuguese translation of the invalid-file message

  Scenario: The switcher closes on outside click or Escape
    Given the Language Switcher's menu is open
    When the user clicks outside it, or presses Escape
    Then the menu closes without changing the selected language
```

## Notes

- Packages: `i18next` + `react-i18next` only — no `i18next-http-backend` (translations are JSON modules loaded through the bundler, see Language loading below) and no `i18next-browser-languagedetector` (the prefix-based detection rule above is simple enough to hand-write in `src/i18n/detectLanguage.ts`, a pure function unit-tested independently of the i18next instance).
- Language loading: only English (the fallback) is bundled into the main chunk. Every other language is a lazily-loaded chunk of its own (`locale-<code>`, grouped by `localeChunkName` in `vite.config.ts`, loaded via `import.meta.glob` in `src/i18n/resources.ts`). `src/main.tsx` awaits `ensureLanguageLoaded()` for the detected language before the first render — so the page never flashes English first — then preloads the remaining languages with `preloadAllLanguages()` once the browser is idle. The Language Switcher goes through `switchLanguage()`, which loads a language before changing to it, so the UI switches in one step even if the chunk hasn't arrived yet. Startup never calls `changeLanguage`, so browser-locale detection is still never persisted as an explicit choice. If the detected language's chunk fails to load, the app still renders, falling back to English text.
- `src/i18n/keyParity.test.ts` walks every namespace's `en` vs. each other supported language's JSON pair and asserts identical leaf-key sets, catching translation drift as new strings are added in future milestones.

## Desktop notes

- Locale JSON files are embedded at build time; a key-parity test covers all 14 languages including the desktop-only `desktop` namespace.
- CJK: the bundled UI fonts have no CJK glyphs (the web relied on browser fallback). For ja / zh-CN / ko the app loads an installed system face via fontdb (Hiragino / Yu Gothic / Meiryo, PingFang / Hiragino Sans GB / Microsoft YaHei, Apple SD Gothic Neo / Malgun Gothic, or Noto Sans CJK on Linux), only while that language is active, shifted onto Manrope's baseline. A 10-glyph Noto Sans CJK subset (OFL, `assets/fonts/ui/`) always renders the native names 日本語 / 简体中文 / 한국어 in the language switcher.
