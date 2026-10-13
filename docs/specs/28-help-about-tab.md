# 28 — Help: About Tab

## Purpose

Extend the in-app Help overlay ([20-help.md](./20-help.md)) with an 8th tab, **About**, giving users static app/author information and a way to contact the author without leaving the app.

## Entry Point

Same Help overlay as spec 20 — no new entry point. About is appended as the 8th and last tab, after Keyboard & Mouse. Like Layers and Keyboard & Mouse, it never becomes the default tab: it has no corresponding Editor mode, so it is not a target in `MODE_TO_HELP_TAB`.

## Tab content

### App info

Read-only property rows:
- **Name** — "StringArtIt"
- **Description** — a short one-line summary of the app
- **Version** — read live from `package.json`, not hand-maintained
- **Author** — "Luís Fernando Saquetim Vendrame"

### Contact form

- **Name** — free text.
- **Subject** — a dropdown restricted to exactly four options: Question, Support, Issue, Other. No free-text subject.
- **Message** — free text (multi-line).
- **Send** — builds and opens a `mailto:lfsvendrame@gmail.com` link. Disabled (non-navigating) until Name, Subject, and Message are all non-empty. This app is a static, backend-less browser app, so a `mailto:` link — opening the user's own configured mail client — is the only way to "send" anything; the button never itself transmits data anywhere.

**Mailto composition:**
- **To:** `lfsvendrame@gmail.com`
- **Subject:** `"StringArtIt - <Subject>"`, where `<Subject>` is always the canonical English token (`Question`/`Support`/`Issue`/`Other`) regardless of the UI's current locale — only the dropdown's visible option *labels* are translated, so the recipient (a human, not a parser) always sees a consistent, recognizable subject line no matter which language the sender's UI was in.
- **Body:** the entered Name and Message composed into a locale-translated template.

## Test Cases

```gherkin
Feature: About tab

  Scenario: About is the 8th and last tab
    Given the Help overlay is open
    Then the tab order ends with Keyboard & Mouse, About

  Scenario: App info is shown
    Given the Help overlay is open on the "About" tab
    Then the app name, description, version, and author are all shown

  Scenario: Send is disabled until all fields are filled
    Given the Help overlay is open on the "About" tab
    And the contact form's Name, Subject, and Message are all empty
    Then the "Send" control is disabled

  Scenario: Send composes a mailto link
    Given the "About" tab's contact form has Name "Ana", Subject "Support", Message "Help"
    Then the "Send" control's href starts with "mailto:lfsvendrame@gmail.com"
    And its subject parameter is "StringArtIt - Support"
    And its body parameter contains "Ana" and "Help"

  Scenario: Subject is restricted to four options
    Given the Help overlay is open on the "About" tab
    Then the Subject dropdown offers exactly Question, Support, Issue, and Other
```

## Desktop notes

- "Send" opens the `mailto:` URL with the OS default mail app (`open` crate); subject/body encoding matches the web (`encodeURIComponent`).
- The description reads "desktop design tool" (`desktop:about.descriptionValue`, ×14) and the version is the crate version.
