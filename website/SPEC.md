# Kittens public website specification (W0)

- Status: controlling contract for the first public Kittens website;
  authorized by the operator on 2026-08-09
- Public target: `https://gonzih.github.io/kittens-rs/`
- Parent contracts: root [`SPEC.md`](../SPEC.md), especially sections 2.1,
  37, and 38; [`K0-REPORT.md`](../K0-REPORT.md); and the controlling specs of
  every profile represented on the site
- Normative boundary: this entire file governs the website source, generated
  artifact, and GitHub Pages publication. It does not add to or reinterpret
  any crate's API or semantic guarantees.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative within
this website boundary.

## 1. One-sentence definition

The Kittens website is a fast, accessible, evidence-led introduction to an
agent-first Rust SDK that makes selected async topology and ordering facts
visible to the compiler, while showing the project's cute cat-and-yarn
identity and its honest escape surface.

## 2. Audience and outcome

The primary visitor is a Rust developer or coding-agent author deciding in a
few minutes whether Kittens is relevant to a long-lived async harness,
terminal application, or embedded interface. Secondary visitors are engine
authors, reviewers, and future contributors.

The first viewport MUST answer:

1. what Kittens is;
2. which problem it addresses;
3. that it is experimental;
4. where to inspect or try it.

A visitor who continues MUST be able to recover:

- the three-layer coverage thesis: inexpressibility, static detection, and
  deterministic schedule exploration;
- the implemented kernel and current profile crates on `main`;
- the enforcement layer behind each public claim;
- the negative controls and open gates;
- the project's larger direction for harness builders, engine authors, and
  meta-harnesses;
- the canonical `cargo add kittens` and lean `reactor!` entry points.

## 3. Source and publication topology

The website is documentation, not a Kittens profile crate.

- Canonical editable source MUST live under `website/src/` on `main`.
- The source MUST be plain semantic HTML, CSS, and progressive-enhancement
  JavaScript with no runtime package, framework, remote font, analytics, or
  third-party script dependency.
- `node website/scripts/build.mjs` MUST create the complete publishable tree
  under `website/dist/` using only Node standard-library modules.
- `website/dist/` is generated output and MUST NOT be committed to `main`.
- The `gh-pages` branch root MUST contain only the generated publishable tree
  and a provenance README; it is a deployment artifact, not an editing
  surface.
- The GitHub Pages source MUST be the root of `gh-pages` in branch-publishing
  mode. A `.nojekyll` marker MUST be present.
- All on-site URLs MUST work beneath the `/kittens-rs/` project subpath. Local
  assets and fragments therefore use relative paths rather than domain-root
  paths.
- Publication MUST identify the exact source commit in a machine-readable
  `build.json` file and in the `gh-pages` commit message.
- No custom domain is authorized in W0. The canonical public URL is the
  default GitHub Pages project URL above.

## 4. Content contract

### 4.1 Hero and positioning

The hero MUST describe Kittens as experimental, explicit, compile-checked
async orchestration in Rust. It MAY use concise language such as “make async
orchestration harder to get wrong,” but MUST NOT imply that Kittens proves all
concurrency behavior or replaces Rust, an executor, a scheduler, a HAL, or a
sandbox.

The project-owned cat-and-yarn artwork MUST be visible in the first viewport
on ordinary desktop screens and remain meaningful, non-obstructive content on
small screens.

### 4.2 Coverage thesis

The website MUST present the root SPEC section 2.1 claim as layers, never as
static omniscience:

1. ordinary Rust ownership and single-owner reactor structure make some
   defect classes inexpressible;
2. `reactor!` and sealed source admission reject selected declared hazards;
3. deterministic runtime scenarios cover inherently dynamic protocols;
4. total confidence is bounded by escape surface and the scenario corpus.

The “six nines” question MAY appear only if immediately bounded by that
per-class model. W0 SHOULD avoid a numeric headline because no field baseline
exists for defect-class distribution.

### 4.3 What exists today

The website MUST represent only code present on the deployed `main` source
commit as currently shipped. W0 includes:

- `kittens` and `kittens-macros`: the experimental K0 `no_std` reactor/source
  kernel and compiler;
- `kittens-tui`: the terminal orchestration profile;
- `kittens-render`: the embedded rendering/interaction evidence profile.

Unmerged work, candidate drivers, and superseded root SPEC sections 11–36 MUST
NOT be described as shipped. Broader coding-harness and frontmatter composition
may appear only as explicitly labeled direction.

Every crate card MUST link to its repository contract or README. Published
crates MAY also link to crates.io and docs.rs.

### 4.4 Evidence and honest boundary

The website MUST distinguish “checked,” “runtime-tested,” “ordinary Rust,” and
“not guaranteed.” At minimum it MUST communicate:

- checked declarations: shutdown prefix, precedence and cycle relationships,
  global `last`, bounded macro-managed draining, buffered yield relationships,
  readiness compatibility, persistent/admitted sources, and required phases;
- runtime/profile protocols: TUI acknowledgement/presenter state and render
  settlement/demand/touch state;
- negative controls: raw runtime bypasses and handler-interior loops or awaits
  compile; external event order and hardware timing are not proven;
- status: K0 formal closure and stable API are not claimed, and profile open
  gates stay linked rather than summarized away.

Evidence numbers MUST come from versioned repository reports on the deployed
source commit. Measurements MUST carry enough context not to masquerade as
general performance claims.

### 4.5 Code and calls to action

The canonical code example MUST use the lean grammar from root SPEC section 38
and `docs/agent-guide.md`, not the superseded maximal grammar. It MUST include
load-bearing `///` rationale at an orchestration boundary and a nearby note
that handlers remain ordinary unchecked Rust.

Primary calls to action MUST lead to the GitHub repository and installation.
Secondary calls MAY lead to the agent guide, diagnostics, evidence report,
crate docs, and specs. No call to action may imply production stability.

### 4.6 Vision

The vision section MUST separate current evidence from direction. It SHOULD
show three consumer horizons:

1. agents building one harness;
2. component and engine authors building domain profiles;
3. meta-harnesses generating and supervising other harnesses.

It SHOULD describe the long-term aim as shrinking escape surface through one
declared orchestration vocabulary across runtimes and domains. Candidate
simulation, schedule exploration, machine-readable topology, and future
harness/frontmatter composition MUST be labeled as direction or gated work.

## 5. Visual system

The repository's existing artwork is canonical for W0:

- [`assets/kittens-logo.webp`](../assets/kittens-logo.webp): orange tabby and
  tuxedo kitten playing around a coral yarn ball;
- [`assets/kittens-yarn-banner.webp`](../assets/kittens-yarn-banner.webp):
  three kittens and interlaced coral, teal, and lavender yarn.

The site MUST use those assets rather than substitute stock art or a
third-party logo. The visual language SHOULD derive from their warm cream,
coral, teal, lavender, orange, and charcoal palette. Typography MUST remain
high-contrast and technical enough that the theme feels warm rather than
juvenile.

CSS shapes, borders, and yarn-line motifs MAY support the composition. New
model-authored SVG illustration MUST NOT ship. A bespoke raster social card
MAY be generated from the canonical visual brief; it MUST be inspected for
text accuracy and brand consistency before use.

Decorative motion MUST be subtle, transform/opacity-only, and disabled under
`prefers-reduced-motion: reduce`. Content MUST remain complete with CSS or
JavaScript disabled.

## 6. Accessibility, privacy, and performance

W0 targets WCAG 2.2 AA within the limits of automated and manual review.

- The page MUST have one descriptive `h1`, logical heading order, semantic
  landmarks, a visible-on-focus skip link, and useful alternative text for
  meaningful art.
- Every interactive element MUST be keyboard reachable with a clearly visible
  focus indicator that is not obscured by sticky content.
- Body text and interactive labels MUST meet at least 4.5:1 contrast; large
  text and meaningful non-text boundaries MUST meet at least 3:1.
- Pointer targets SHOULD be at least 44 by 44 CSS pixels.
- The layout MUST not require horizontal scrolling at 320 CSS pixels and MUST
  remain usable at 200% zoom.
- Motion MUST honor reduced-motion preferences. No animation may flash, auto-
  advance content, or gate comprehension.
- Below-fold imagery MUST declare dimensions and use lazy decoding/loading
  where appropriate to prevent layout shift.
- The page MUST not set cookies, fingerprint visitors, collect analytics,
  submit forms, or make background network requests.
- System fonts and optimized local WebP/PNG assets MUST keep first-load weight
  bounded. The W0 target is under 500 KiB transferred for the complete page and
  under 250 KiB before scrolling, measured from an empty cache without browser
  extensions.

## 7. Enforcement-layer ledger

| Public property | Enforcement layer | Required oracle | Negative control |
|---|---|---|---|
| local URLs and assets resolve below the project path | build-time structural checker | build plus link/asset scan | external destinations may later move |
| required claims and boundary language remain present | build-time content assertions + documentation review | marker assertions against rendered HTML | prose cannot prove crate behavior |
| semantic document and keyboard/focus/reduced-motion affordances | semantic HTML/CSS + structural checker + browser review | automated structure scan and manual keyboard/reduced-motion pass | automated checks are not full WCAG conformance |
| no remote runtime dependencies or tracking | source allowlist + network/browser review | structural URL scan and clean-load request inspection | clicking an external link leaves the site boundary |
| exact deployed source is recoverable | generated `build.json` + deployment commit | compare live `build.json` to merged source SHA | GitHub Pages availability remains external |
| project claims match shipped evidence | documentation review against controlling contracts | source-linked research ledger and PR review | linked reports retain their own open gates |
| `gh-pages` is reproducible output | deterministic standard-library build | two clean builds have identical file hashes except the injected source SHA | generated branch history is not canonical source history |

## 8. Required oracles and publication gate

The implementation MUST provide standard-library-only build and check scripts.
Before the website implementation commit or publication:

1. `node website/scripts/build.mjs` succeeds from the repository root;
2. `node website/scripts/check.mjs website/dist` succeeds;
3. two builds for the same source SHA produce the same publishable bytes;
4. the repository's normal formatting, lint, and test gates pass;
5. a local HTTP server returns the page and every local asset with no 404;
6. keyboard navigation, narrow and wide layouts, reduced motion, and readable
   focus are reviewed in a real browser;
7. the implementation enters through a PR to `main` after this spec-first
   commit;
8. the generated tree is committed and pushed to a new `gh-pages` branch;
9. the GitHub Pages repository setting names `gh-pages` and `/`;
10. the deployment reports success, the public URL returns HTTP 200, and the
    live `build.json` names the merged source commit.

The implementation MUST add one user-visible website entry to
[`CHANGELOG.md`](../CHANGELOG.md).

## 9. Explicit non-goals and negative controls

W0 is not:

- API documentation, a blog, a playground, a benchmark dashboard, or a docs
  search engine;
- a claim that experimental crate APIs are stable or that any open evidence
  gate is closed;
- an enforcement mechanism for Kittens code;
- a ban on raw Rust concurrency or handler-side escape surfaces;
- a custom-domain or analytics rollout;
- a replacement for repository specs, reports, compile-fail fixtures, or
  crate documentation.

A beautiful site can make an incorrect claim more persuasive. For that reason,
visual polish never outranks evidence provenance or adjacent non-guarantees.

## 10. Deferred work, with gates

- **Custom domain:** deferred until the operator supplies a domain, verifies
  ownership, authorizes DNS changes, and selects the canonical host.
- **Analytics:** deferred until a concrete product question, privacy policy,
  retention limit, and consent/legal assessment exist. W0 collects nothing.
- **Versioned API docs or playground:** deferred until an API-stability and
  maintenance contract exists; docs.rs remains canonical meanwhile.
- **Multi-page docs/blog:** deferred until two distinct information journeys
  cannot remain legible on the single landing page and navigation/link oracles
  are added.
- **Localization:** deferred until an owner and source-of-truth translation
  workflow exist.
- **Automated Pages publishing from `main`:** deferred until the operator
  chooses whether generated-branch history or GitHub Actions artifacts are the
  long-term publication record. W0 performs one explicit, auditable publish.
