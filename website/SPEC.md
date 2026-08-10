# Kittens public website specification (W0.2)

- Status: controlling contract for the public Kittens marketing website;
  authorized by the operator on 2026-08-09
- Public target: `https://gonzih.github.io/kittens-rs/`
- Parent contracts: root [`SPEC.md`](../SPEC.md), [`K0-REPORT.md`](../K0-REPORT.md),
  and the controlling specs of products represented on the site
- Normative boundary: this file governs website source, generated artifacts,
  and GitHub Pages publication. It does not change crate APIs or guarantees.
- Revision 1 (2026-08-09): publication may proceed without browser review when
  browser discovery yields no controllable browser, provided structural,
  standards, HTTP, and live-deployment checks pass and no browser-conformance
  claim is made.
- Revision 2 (2026-08-09): `kittens-code` became the flagship demo.
- Revision 3 (2026-08-09): the operator rejected release-review language and
  implementation exposition on the marketing surface. W0.2 supersedes W0.1's
  exhaustive flagship-content requirement with a concise, benefit-led product
  story. Development branches, gate identifiers, research archives, spec
  process, and deferred implementation topology belong in repository
  documentation, not landing-page copy.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative within
this website boundary.

## 1. Product definition

Kittens is an experimental Rust SDK for making async orchestration explicit
and harder to get wrong, demonstrated by `kittens-code`: a coding-agent harness
that keeps work durable, bounded, and recoverable.

## 2. Audience and conversion

The primary visitor is a Rust developer or agent builder deciding quickly
whether Kittens is worth trying. The page MUST optimize for comprehension and
curiosity, not completeness.

Within ten seconds a visitor MUST understand:

1. Kittens tackles tangled async coordination;
2. `kittens-code` is the flagship demonstration;
3. the practical payoff is durable state, explicit control, and safer change;
4. the project can be tried or inspected immediately.

The primary conversion is installing `kittens-code-cli`. The secondary
conversion is opening the GitHub repository.

## 3. Marketing voice and density

The public page MUST lead with outcomes, then offer only enough mechanism to
make those outcomes credible.

- Visible body copy SHOULD remain below 700 words, excluding navigation,
  commands, and footer labels.
- The page MUST contain no more than five primary content sections beneath the
  header, including the hero and final call to action.
- The page MUST contain no more than four `h2` headings.
- A normal paragraph SHOULD contain no more than three sentences.
- Technical terms MUST earn their place by clarifying a user benefit.
- Repeated qualifications, duplicate install surfaces, and exhaustive lists
  MUST be removed.
- One compact phrase MAY communicate experimental status:
  “Experimental. APIs may change.”

The visible marketing page MUST NOT expose internal development or release
mechanics, including:

- source or deployment branch names;
- commit topology or merge status;
- internal gate identifiers such as E1, G10, KC0, or K0;
- “status boundary,” “deferred scope,” or release-review framing;
- research-input counts or evidence-archive navigation;
- spec revision state, frozen-contract status, or implementation backlog;
- source-vs-deployed-tree comparisons.

Machine-readable provenance in `build.json` remains required and is not
marketing copy.

## 4. Content contract

### 4.1 Header and hero

The header MUST contain the Kittens brand, no more than three short navigation
links, and one GitHub action. A release bar MUST NOT sit above the navigation.

The hero MUST:

- state the benefit in plain language;
- identify `kittens-code` as the flagship;
- use one short supporting paragraph;
- offer one primary try/install path and one GitHub path;
- show the canonical kittens-and-yarn artwork;
- carry at most three short proof chips.

The hero MUST NOT enumerate internal subsystems, protocol types, or negative
controls.

### 4.2 kittens-code product story

The flagship section MUST sell three visitor-facing outcomes:

1. **Keep the thread:** work survives restarts through a durable transcript.
2. **Recall what matters:** old context remains queryable without keeping every
   token in the live model window.
3. **Fail predictably:** bounded work and deterministic recovery turn common
   async edge cases into behavior that can be inspected and tested.

These outcomes MAY mention replay, recall, cancellation, budgets, or an offline
demo. The section MUST NOT teach the engine/driver protocol, list all wire
types, publish the RLM grammar, diagram the crate topology, or enumerate
deferred work.

The section MUST include one copyable install command:

`cargo install kittens-code-cli --version 0.0.1`

It SHOULD link to crates.io and MAY link to docs.rs. Links to internal source
branches, research archives, specs, or release-review documents MUST NOT appear
in the primary marketing journey.

### 4.3 Why Kittens

One compact section MUST explain the underlying value in three points:

1. important ordering rules sit beside the code;
2. selected topology mistakes become compiler feedback;
3. dynamic races remain covered by deterministic scenarios.

This section MUST NOT include a full reactor example, enforcement-layer table,
compile-fail inventory, gate count, or taxonomy of every checked declaration.
One sentence MUST preserve the honest boundary: Kittens checks declared
orchestration, not arbitrary Rust or the outside world.

### 4.4 Product family and direction

The page MAY include one compact ecosystem line naming `kittens`,
`kittens-code`, `kittens-tui`, and `kittens-render`. Each product receives at
most one short phrase. No boundary matrix or per-crate evidence block is
allowed.

Long-term direction MAY be summarized in one sentence. Meta-harnesses, future
drivers, and unshipped subsystems MUST NOT receive separate marketing cards.

### 4.5 Final action

The final section MUST repeat a single install command and provide one GitHub
link. It MUST NOT introduce new concepts or link to internal process documents.

## 5. Truth boundary

Marketing compression MUST remain accurate.

- The page MAY say that Kittens makes selected async ordering mistakes harder
  to express or easier to catch.
- It MUST NOT claim universal race freedom, production stability, formal
  verification, guaranteed handler termination, or control of external event
  order.
- “Durable,” “recoverable,” “bounded,” and “deterministic” MUST refer only to
  implemented kittens-code behavior documented by its controlling contract.
- “Portable core” MAY refer to the `no_std + alloc` center and its link gates,
  but the marketing page SHOULD prefer the simpler phrase “small Rust core.”
- Experimental status MUST remain visible without dominating the story.

## 6. Visual system

The canonical assets remain:

- [`assets/kittens-logo.webp`](../assets/kittens-logo.webp);
- [`assets/kittens-yarn-banner.webp`](../assets/kittens-yarn-banner.webp);
- the inspected `kittens-code` social card under `website/src/assets/`.

The page MUST preserve the warm cream, coral, teal, lavender, orange, and
charcoal palette. Kittens and yarn remain the central visual metaphor. The
design SHOULD feel joyful and technically credible, with generous whitespace
and fewer cards than W0.1.

Decorative motion MUST be subtle and disabled under
`prefers-reduced-motion: reduce`. Content MUST remain complete with CSS or
JavaScript disabled.

## 7. Source and publication topology

- Editable source MUST live under `website/src/` on `main`.
- Source MUST remain semantic HTML, CSS, and progressive-enhancement JavaScript
  with no runtime framework, remote font, analytics, or third-party scripts.
- `node website/scripts/build.mjs` MUST create `website/dist/` using only Node
  standard-library modules.
- `website/dist/` MUST NOT be committed to `main`.
- The `gh-pages` branch root MUST contain the generated tree and `.nojekyll`.
- Relative local URLs MUST work beneath `/kittens-rs/`.
- `build.json` MUST record the exact merged source commit.
- GitHub Pages MUST publish `gh-pages:/` with HTTPS.

## 8. Accessibility, privacy, and performance

- The page MUST have one `h1`, logical heading order, semantic landmarks, a
  visible-on-focus skip link, and useful alternative text.
- Interactive elements MUST be keyboard reachable with visible focus.
- Text and meaningful boundaries MUST meet WCAG 2.2 AA contrast targets.
- Pointer targets SHOULD be at least 44 by 44 CSS pixels.
- The layout MUST work at 320 CSS pixels and 200% zoom without horizontal page
  scrolling.
- Motion MUST honor reduced-motion preferences.
- Below-fold imagery MUST declare dimensions and lazy-load where appropriate.
- The page MUST set no cookies, collect no analytics, and make no background
  network requests.
- Local system fonts and optimized assets MUST keep initial page resources
  lean; the social card is metadata-only and MUST NOT load into the visible
  page.

## 9. Enforcement ledger

| Property | Enforcement layer | Oracle | Negative control |
|---|---|---|---|
| concise marketing density | structural checker + documentation review | word, section, and heading limits | line count is not a substitute for copy quality |
| internal development mechanics stay private | forbidden-marker scan | generated HTML contains none of the W0.1 process phrases | external GitHub pages may expose repository history after a visitor leaves the site |
| product claims stay honest | documentation review against controlling contracts | required benefit and experimental markers | concise copy does not prove implementation behavior |
| accessible static document | semantic HTML/CSS + structural checker | HTML validation and accessibility markers | automated checks are not full WCAG conformance |
| local assets resolve | build-time checker + served-tree scan | every generated file returns successfully | external links may later move |
| exact deployment is recoverable | `build.json` + Pages commit | live SHA equals merged source SHA | provenance remains machine-facing |
| deterministic publication | standard-library build | two builds hash identically | GitHub availability is external |

## 10. Required oracles and publication gate

Before the implementation commit or publication:

1. `node website/scripts/build.mjs` succeeds;
2. `node website/scripts/check.mjs website/dist` succeeds;
3. `node website/scripts/repro.mjs` proves byte-identical output;
4. HTML and sitemap validation pass;
5. the repository format, Clippy, and all-feature test gates pass;
6. local serving returns the page and every asset successfully;
7. a PR enters `main` after this spec-first commit;
8. the exact merged build is pushed to `gh-pages`;
9. Pages reports success and live `build.json` names the merged source commit;
10. the implementation adds a user-visible [`CHANGELOG.md`](../CHANGELOG.md)
    entry.

The structural checker MUST require the flagship name, all three product
outcomes, the install command, the honest declared-orchestration boundary, and
the experimental label. It MUST reject branch/process/gate language from the
generated page.

## 11. Explicit non-goals

The marketing site is not API documentation, a harness tutorial, a release
review, an evidence archive, a roadmap ledger, a benchmark dashboard, or a
hosted coding agent. Repository docs retain the exhaustive technical truth;
the landing page earns the next click.
