# 30 — SEO

## Purpose

Make the deployed app (https://stringartit.com/) discoverable and correctly represented by search engines and social-media crawlers. Before this spec, `index.html` had no meta description, canonical link, Open Graph/Twitter tags, favicon, or structured data, and no `robots.txt`/`sitemap.xml` existed — and even the rendered app carried no descriptive text at all beyond a bare "New Board" setup form. This spec covers the technical SEO tags/assets plus a set of inline landing-content sections that give the app's single URL real, crawlable text about what it does.

## Entry Point

No new entry point or route. Everything here lives at `/`, either in `index.html`'s static `<head>`, in `public/` static assets, or as new sections rendered inline around the existing `BoardSetup` screen (shown when the editor has not yet been entered).

## Technical SEO — `index.html`

Static `<head>` tags (present in the raw HTML, not dependent on JS execution, since social-media crawlers and most SEO tooling don't run JS):
- `<title>` and `<meta name="description">` — keyword-rich, truthful to shipped features.
- `<link rel="canonical">` pointing at `https://stringartit.com/`.
- Open Graph tags (`og:type`, `og:site_name`, `og:title`, `og:description`, `og:url`, `og:image`, `og:locale`).
- Twitter Card tags (`summary_large_image`, title/description/image).
- `<meta name="theme-color">` matching `--bg-app`.
- Favicon (`favicon.svg`), `apple-touch-icon.png`, and `manifest.webmanifest` links.
- A `SoftwareApplication` JSON-LD block (name, url, description, applicationCategory, operatingSystem, `offers` price 0 USD). No `aggregateRating`/`review` fields — those must never be fabricated (a real Google structured-data spam policy violation).

## Static assets (`public/`)

- `robots.txt` — allows all crawlers, points at the sitemap.
- `sitemap.xml` — lists the one real indexable URL (`https://stringartit.com/`).
- `manifest.webmanifest` — name/short_name/description/icons/theme_color/background_color/display.
- `favicon.svg` — a simple pin-and-thread motif using only `--bg-app`/`--accent`.
- `apple-touch-icon.png` (180×180) and `og-image.png` (1200×630) — rasterized from the same motif.

These are plain static files; nginx's existing SPA fallback (`try_files $uri $uri/ /index.html`) serves any of them directly since they exist as real files, with no config change needed.

## `<html lang>` sync

`document.documentElement.lang` is set once on i18n init and kept in sync on every subsequent language change, so the document's declared language always matches the active i18next locale instead of being hardcoded to `"en"`.

## Landing content

Four new sections render inline in the `!entered` branch of `App.tsx`, around the existing `BoardSetup` screen:

- **Hero** (`LandingHero`) — the page's one `<h1>` plus a one-paragraph subhead. `BoardSetup`'s own former `<h1>` ("New Board") is demoted to `<h2>` so the page has exactly one `<h1>`.
- **Features** (`LandingFeatures`) — a card grid of real shipped capabilities (shapes, pin placement, multi-layer threading, layers, undo, print), each reusing an existing icon already used elsewhere in the app.
- **How it works** (`LandingHowItWorks`) — a 4-step numbered process (set up board → place pins → connect threads → export/print) matching the real app flow.
- **FAQ** (`LandingFAQ`) — a data-driven Q&A list (`faqContent.ts`, following the same typed-keys + generic-renderer split as the in-app Help content's `HelpSection`/`HelpItem` pattern), including a purely definitional "What is string art?" entry.

All four reuse only documented design-system tokens (`18-design-system.md`) — no new colors, radii, fonts, or animation. All copy lives in a new `landing` i18n namespace, authored in full for all 5 locales (`en`/`pt-BR`/`es`/`fr`/`de`).

## Test Cases

```gherkin
Feature: Technical SEO tags

  Scenario: Static head carries full metadata
    Given the built index.html is inspected directly (no JS execution)
    Then it contains a non-empty <title> and meta description
    And it contains a canonical link to https://stringartit.com/
    And it contains Open Graph and Twitter card tags
    And it contains a SoftwareApplication JSON-LD block with no aggregateRating or review fields

  Scenario: robots.txt and sitemap.xml are served
    Given the app is deployed
    Then GET /robots.txt returns 200 and references /sitemap.xml
    And GET /sitemap.xml returns 200 and lists https://stringartit.com/

Feature: <html lang> sync

  Scenario: Initial lang matches the detected locale
    Given the app loads with a stored or detected locale
    Then document.documentElement.lang equals that locale

  Scenario: Switching language updates <html lang>
    Given the app is loaded
    When the user switches the UI language via the language switcher
    Then document.documentElement.lang updates to match the new locale

Feature: Landing content

  Scenario: The page has exactly one h1
    Given the app loads on the BoardSetup screen
    Then there is exactly one level-1 heading, rendered by LandingHero
    And BoardSetup's own heading is a level-2 heading

  Scenario: Landing sections render around BoardSetup without breaking it
    Given the app loads on the BoardSetup screen
    Then LandingHero renders above BoardSetup
    And LandingFeatures, LandingHowItWorks, and LandingFAQ render below BoardSetup
    And BoardSetup's shape/dimension/appearance controls and "Continue to Editor" still work exactly as before

  Scenario: Landing content is i18n-driven
    Given the app loads with a non-English locale selected
    Then BoardSetup's own strings are translated
    And the landing sections render fully translated copy in that locale
```
