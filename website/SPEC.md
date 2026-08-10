# Kittens public website specification (W0.4)

- Status: controlling contract for the public Kittens marketing website;
  authorized by the operator on 2026-08-10
- Public target: `https://gonzih.github.io/kittens-rs/`
- Parent contracts: root [`SPEC.md`](../SPEC.md), [`K0-REPORT.md`](../K0-REPORT.md),
  the published [`kittens-code` contract](https://github.com/Gonzih/kittens-rs/blob/2a2fb0d63e817515bc17514c197260af14046a16/docs/kittens-code/SPEC.md),
  and its [FRONTMATTER orientation](https://github.com/Gonzih/kittens-rs/blob/2a2fb0d63e817515bc17514c197260af14046a16/docs/kittens-code/FRONTMATTER.md)
- Normative boundary: this file governs website source, generated artifacts,
  and publication. It does not change crate APIs or guarantees.
- Revision 1 (2026-08-09): publication may proceed without browser review when
  no controllable browser exists, provided structural, standards, HTTP, and
  live-deployment checks pass and no browser-conformance claim is made.
- Revision 2 (2026-08-09): `kittens-code` became the flagship demo.
- Revision 3 (2026-08-09): internal release and implementation exposition was
  removed from the marketing surface.
- Revision 4 (2026-08-10): explanatory chips, cards, captions, and repeated
  benefit copy were removed.
- Revision 5 (2026-08-10): the operator rejected W0.3's generic restart,
  search, and offline-agent positioning. W0.4 restores the repository's actual
  thesis: Kittens is the constraint language for the whole agent system;
  FRONTMATTER, HARNESS, and COGNITION are separate layers; `kittens-code` is the
  HARNESS profile, not the definition of Kittens.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative within
this website boundary.

## 1. Product definition

Kittens is an agent-first, Rust-embedded constraint language and compiler for
long-lived async orchestration. It exists to make the architecture of an agent
system explicit and locally checkable instead of leaving critical relations in
convention, comments, or working memory.

The public layer model comes from `docs/kittens-code/FRONTMATTER.md`:

- **FRONTMATTER** — what the human sees and touches;
- **HARNESS** — the turn loop, tools, context law, persistence, budgets, and
  cancellation;
- **COGNITION** — the model and token generation.

The full agent is FRONTMATTER + HARNESS + COGNITION. Each layer may live on a
different device. Kittens supplies shared orchestration law; it does not erase
the boundaries between the layers.

`kittens-code` is the flagship HARNESS profile and the main public demo of this
architecture.

## 2. Page and conversion contract

W0.4 is one idea on one page.

- Visible copy MUST remain below 160 words, including navigation and footer.
- The page MUST contain exactly one `section` and no `h2` headings.
- The header MUST contain only the brand and one GitHub action.
- The body MUST contain one `h1`, one supporting paragraph, one three-layer
  definition list, one `kittens-code` sentence, and one honest boundary.
- The canonical two-kitten yarn image MUST be the only visible illustration.
- The page MUST contain no button cluster, install command, copy control,
  product banner, feature list, secondary pitch section, or repeated action.
- The primary conversion is GitHub. The only product link MAY be the
  `kittens-code` crates.io page.

The visible page MUST NOT contain the W0.3 positioning or close variants:

- “Async Rust. Fewer surprises.”;
- “picks up where it left off”;
- “work survives restarts”;
- “long conversations stay searchable”;
- “default demo runs offline”;
- generic productivity, memory, reliability, or autonomy claims that do not
  explain the system architecture.

It also MUST NOT contain development branches, commit state, gate identifiers,
deferred-work lists, research archives, release-review language, crate
topology, wire types, or RLM grammar.

## 3. Exact content contract

The hero MUST use this `h1`:

> Make the whole agent explicit.

Its single supporting paragraph MUST say, without adding a second pitch:

> Kittens is a Rust constraint language for expressing FRONTMATTER, HARNESS,
> COGNITION, and the async law between them—then checking declared
> orchestration at compile time.

The layer definition list MUST contain exactly these three terms and meanings:

- `FRONTMATTER` — what the human sees and touches;
- `HARNESS` — the loop, tools, context, and control;
- `COGNITION` — the model and token generation.

The flagship sentence MUST say:

> `kittens-code` is the HARNESS profile built on Kittens.

The honest boundary MUST say:

> It checks declared structure. Handler behavior, raw Rust, and external event
> order remain outside that boundary.

The footer MUST say “Experimental. APIs may change.” and “No cookies. No
analytics.”

## 4. Truth boundary

The website MAY say that selected declared orchestration becomes compiler
input. It MAY present the three-layer decomposition as the stable architecture
orientation.

It MUST NOT claim:

- universal compile-time verification;
- universal race freedom;
- formal verification;
- handler termination or handler-interior analysis;
- control of external event order;
- completed on-device composition of all three layers;
- stable production APIs.

The high-coverage thesis remains layered: ordinary Rust and system architecture
make some defects inexpressible, Kittens statically rejects selected declared
relations, and deterministic tests cover named runtime schedules. The page
expresses only the compile-time slice and its boundary; it MUST NOT compress the
three mechanisms into “Kittens verifies everything.”

## 5. Visual system

The page MUST be quiet and architectural rather than launch-theater:

- one warm cream field;
- charcoal type with coral used only as a restrained accent;
- no dark product panel, gradient field, glow, floating label, card, pill,
  caption, statistic, numbered feature, or decorative microcopy;
- no decorative motion;
- the layer model rendered as plain typography and rules, not three cards;
- generous whitespace, but no headline treatment whose scale obscures the
  meaning.

The canonical visible asset is
[`assets/kittens-logo.webp`](../assets/kittens-logo.webp). The yarn banner MUST
NOT appear on the page. The social card is metadata-only and MUST use the W0.4
headline without additional product claims.

## 6. Source and publication

- Editable source MUST live under `website/src/` on `main`.
- Source MUST remain semantic HTML and CSS with no framework, remote font,
  analytics, third-party script, or unnecessary client JavaScript.
- `node website/scripts/build.mjs` MUST create ignored `website/dist/` using
  only Node standard-library modules.
- Relative local URLs MUST work beneath `/kittens-rs/`.
- `build.json` MUST record the exact merged source commit.
- The generated root of `gh-pages` MUST include `.nojekyll`.
- GitHub Pages MUST publish `gh-pages:/` with HTTPS.

## 7. Accessibility, privacy, and performance

- The page MUST have one `h1`, semantic landmarks, a visible-on-focus skip
  link, a valid definition list, and useful alternative text.
- Links MUST be keyboard reachable with visible focus and SHOULD provide at
  least 44 by 44 CSS-pixel targets where presented as controls.
- Text and meaningful boundaries MUST meet WCAG 2.2 AA contrast targets.
- The layout MUST work at 320 CSS pixels and 200% zoom without horizontal page
  scrolling.
- The page MUST set no cookies, collect no analytics, and make no background
  network requests.
- The metadata-only social card MUST NOT load into the visible page.

## 8. Enforcement ledger

| Property | Enforcement layer | Oracle | Negative control |
|---|---|---|---|
| one architectural idea | structural checker + review | one section, no h2, word ceiling | low word count alone does not make copy true |
| three-layer decomposition | semantic definition list | exactly three required terms and meanings | the page does not claim all layers already run together on hardware |
| honest compile boundary | documentation review + required copy | declared-structure sentence | raw Rust and external order remain legal and unchecked |
| quiet visual surface | semantic HTML + forbidden-class scan | one visible image, no cards/banner/controls | metadata-only social art may remain richer |
| accessible static page | semantic HTML/CSS + structural checker | standards and accessibility markers | automation is not full WCAG conformance |
| exact deployment | `build.json` + Pages commit | live SHA equals merged source SHA | GitHub availability is external |
| reproducible output | standard-library build | repeated tree hashes match | deployment history is not editable source |

## 9. Required oracles and publication gate

Before implementation commit or publication:

1. build, structural check, and reproducibility scripts pass;
2. HTML and sitemap validation pass;
3. repository format, Clippy, and all-feature tests pass;
4. a local server returns the page and every local asset successfully;
5. a PR enters `main` after this spec-first commit;
6. the exact merged build is pushed to `gh-pages`;
7. Pages reports success and live `build.json` names the merged source commit;
8. [`CHANGELOG.md`](../CHANGELOG.md) records the positioning correction.

The structural checker MUST reject extra sections/headings/images, the W0.3
copy, cards, product banners, install/copy UI, and internal development
language. It MUST scan both the landing page and custom 404.

## 10. Non-goals

The public page is not documentation, a tutorial, a feature catalog, a product
demo, an architecture whitepaper, an evidence ledger, or a roadmap. It names
the system clearly and earns one click.
