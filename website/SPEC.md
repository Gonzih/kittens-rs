# Kittens public website specification (W0.3)

- Status: controlling contract for the public Kittens marketing website;
  authorized by the operator on 2026-08-10
- Public target: `https://gonzih.github.io/kittens-rs/`
- Parent contracts: root [`SPEC.md`](../SPEC.md), [`K0-REPORT.md`](../K0-REPORT.md),
  and the controlling specs of represented products
- Normative boundary: this file governs website source, generated artifacts,
  and publication. It does not change crate APIs or guarantees.
- Revision 1 (2026-08-09): publication may proceed without browser review when
  no controllable browser exists, provided structural, standards, HTTP, and
  live-deployment checks pass and no browser-conformance claim is made.
- Revision 2 (2026-08-09): `kittens-code` became the flagship demo.
- Revision 3 (2026-08-09): internal release and implementation exposition was
  removed from the marketing surface.
- Revision 4 (2026-08-10): the operator rejected the remaining explanatory
  chips, proof rows, feature-card grids, captions, and repeated benefit copy as
  visual noise. W0.3 adopts the dominant current developer-tool pattern: one
  category claim, one payoff sentence, one action, and one strong visual before
  minimal product detail.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative within
this website boundary.

## 1. Product definition

Kittens gives async Rust clearer coordination rules. `kittens-code` is the
flagship: an AI coding agent that can keep its work and pick up where it left
off.

## 2. Audience and conversion

The primary visitor is a Rust developer or AI-agent builder. The page MUST be
understood before it is studied.

The first viewport MUST communicate:

1. the category: async Rust;
2. the payoff: fewer coordination surprises;
3. the flagship: `kittens-code`;
4. one obvious next action.

The primary conversion is trying `kittens-code`. GitHub is the secondary path.

## 3. Density and voice

W0.3 is a launch page, not an explainer.

- Visible body copy MUST remain below 300 words, excluding commands and footer
  labels.
- The page MUST contain no more than three `section` elements and two `h2`
  headings.
- The header MUST contain only the brand and one GitHub action.
- Each section MUST have one idea and at most one action group.
- Headlines MUST use plain words and short sentences.
- Paragraphs SHOULD remain under 35 words.
- One install command is enough; it MUST NOT be repeated.
- Experimental status belongs quietly in the footer.

The visible page MUST NOT contain:

- floating image labels, image captions, proof chips, stat rows, numbered
  cards, feature grids, ecosystem matrices, or decorative microcopy;
- the words `harness`, `topology`, `sans-IO`, `RLM`, `CoreInput`, or
  `CoreAction`;
- development branches, commit state, gate identifiers, deferred-work lists,
  research archives, specs, or release-review language;
- exhaustive feature lists or multiple restatements of the same benefit.

Machine-readable provenance in `build.json` remains required and is not
marketing copy.

## 4. Content contract

### 4.1 Header and hero

The header MUST contain the Kittens mark/name and one GitHub link. It MUST NOT
contain a navigation-link cluster, release bar, badge, or status label.

The hero MUST contain:

- one short category-defining `h1`;
- one supporting paragraph;
- one primary action leading to `kittens-code`;
- the canonical two-kitten yarn artwork, large and unobstructed.

Nothing may overlay, label, caption, or annotate the hero artwork.

### 4.2 kittens-code

The flagship section MUST contain one outcome-led `h2`, one short paragraph,
one copyable command, and one crates.io link:

`cargo install kittens-code-cli --version 0.0.1`

The paragraph MAY say that work survives restarts, long conversations remain
searchable, and the default demo runs offline. These MUST read as one coherent
payoff, not three cards or slogans.

The canonical yarn-banner artwork MAY appear once, without caption, labels, or
overlaid copy.

### 4.3 Why Kittens

The final section MUST contain one `h2`, one short explanation, the honest
boundary in a single sentence, and one GitHub action.

The explanation MAY say that important coordination rules become compiler
feedback. It MUST NOT teach syntax, list checked declarations, or enumerate
product-family crates.

## 5. Truth boundary

- The page MAY say that Kittens turns selected coordination rules into compiler
  feedback.
- It MUST NOT claim universal race freedom, production stability, formal
  verification, handler termination, or control of external event order.
- `kittens-code` durability, searchability, restart recovery, and offline demo
  claims MUST remain within its implemented contract.
- The honest sentence MUST make clear that Kittens checks declared
  coordination, not arbitrary Rust or the outside world.
- The footer MUST say: “Experimental. APIs may change.”

## 6. Visual system

The canonical visible assets remain:

- [`assets/kittens-logo.webp`](../assets/kittens-logo.webp);
- [`assets/kittens-yarn-banner.webp`](../assets/kittens-yarn-banner.webp).

The inspected social card remains metadata-only. Visible page art MUST contain
no added text.

The visual hierarchy MUST do the work:

- oversized typography;
- generous negative space;
- one warm cream field and one dark product field;
- the coral, teal, lavender, orange, and charcoal palette;
- very few borders, shadows, pills, or containers.

Decorative motion MUST be subtle and disabled under
`prefers-reduced-motion: reduce`. Content MUST remain complete without CSS or
JavaScript.

## 7. Source and publication

- Editable source MUST live under `website/src/` on `main`.
- Source MUST remain semantic HTML, CSS, and progressive-enhancement JavaScript
  with no framework, remote font, analytics, or third-party script.
- `node website/scripts/build.mjs` MUST create ignored `website/dist/` using
  only Node standard-library modules.
- Relative local URLs MUST work beneath `/kittens-rs/`.
- `build.json` MUST record the exact merged source commit.
- The generated root of `gh-pages` MUST include `.nojekyll`.
- GitHub Pages MUST publish `gh-pages:/` with HTTPS.

## 8. Accessibility, privacy, and performance

- The page MUST have one `h1`, logical headings, semantic landmarks, a
  visible-on-focus skip link, and useful alternative text.
- Controls MUST be keyboard reachable with visible focus and SHOULD provide at
  least 44 by 44 CSS-pixel targets.
- Text and meaningful boundaries MUST meet WCAG 2.2 AA contrast targets.
- The layout MUST work at 320 CSS pixels and 200% zoom without horizontal page
  scrolling.
- Motion MUST honor reduced-motion preferences.
- Below-fold imagery MUST declare dimensions and lazy-load.
- The page MUST set no cookies, collect no analytics, and make no background
  network requests.
- The metadata-only social card MUST NOT load into the visible page.

## 9. Enforcement ledger

| Property | Enforcement layer | Oracle | Negative control |
|---|---|---|---|
| radical copy restraint | structural checker + review | word, section, heading, and forbidden-pattern limits | low word count alone does not create good hierarchy |
| unannotated visual | semantic HTML + forbidden-class scan | hero art has no sibling labels or caption | image alt text remains available to assistive technology |
| honest claims | documentation review | required boundary and experimental phrase | concise copy does not prove implementation behavior |
| accessible static page | semantic HTML/CSS + structural checker | standards and accessibility-marker checks | automation is not full WCAG conformance |
| exact deployment | `build.json` + Pages commit | live SHA equals merged source SHA | GitHub availability is external |
| reproducible output | standard-library build | repeated tree hashes match | deployment history is not editable source |

## 10. Required oracles and publication gate

Before implementation commit or publication:

1. build, structural check, and reproducibility scripts pass;
2. HTML and sitemap validation pass;
3. repository format, Clippy, and all-feature tests pass;
4. a local server returns the page and all local assets successfully;
5. a PR enters `main` after this spec-first commit;
6. the exact merged build is pushed to `gh-pages`;
7. Pages reports success and live `build.json` names the merged source commit;
8. [`CHANGELOG.md`](../CHANGELOG.md) records the user-visible reduction.

The structural checker MUST require the hero, flagship, install command,
declared-coordination boundary, and experimental footer. It MUST reject extra
sections, headings, chips, captions, grids, technical jargon, and internal
development language.

## 11. Non-goals

The marketing site is not documentation, a tutorial, an architecture map, a
feature catalog, an evidence archive, or a roadmap. It earns one click.
