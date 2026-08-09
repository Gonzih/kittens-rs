# Kittens public website specification (W0.1)

- Status: controlling contract for the first public Kittens website;
  authorized by the operator on 2026-08-09
- Public target: `https://gonzih.github.io/kittens-rs/`
- Parent contracts: root [`SPEC.md`](../SPEC.md), especially sections 2.1,
  37, and 38; [`K0-REPORT.md`](../K0-REPORT.md); and the controlling specs of
  every profile represented on the site
- Normative boundary: this entire file governs the website source, generated
  artifact, and GitHub Pages publication. It does not add to or reinterpret
  any crate's API or semantic guarantees.
- Revision 1 (2026-08-09): the initial publication gate assumed a controllable
  browser would be available. The environment's required browser discovery
  and troubleshooting found none. W0 may therefore publish after its
  structural, standards, HTTP, and live-deployment oracles pass, but MUST
  retain manual browser review as an open QA gate and MUST NOT claim that
  browser or full WCAG conformance review passed. This records the contract
  drift instead of silently weakening the evidence claim.
- Revision 2 (2026-08-09): the operator designated `kittens-code` as the
  project's flagship demo and required the public site to include its complete
  product story. W0.1 therefore admits the published `0.0.1` family as shipped
  evidence even though its source remains on the repository's `kc0` branch,
  provided that branch boundary and the deferred full `reactor!` driver wiring
  remain adjacent to every flagship-status claim.

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** are normative within
this website boundary.

## 1. One-sentence definition

The Kittens website is a fast, accessible, evidence-led introduction to an
agent-first Rust SDK that makes selected async topology and ordering facts
visible to the compiler, with `kittens-code` as its flagship coding-harness
demo, the project's cute cat-and-yarn identity, and its honest escape surface.

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

The first two viewports MUST identify `kittens-code` as the flagship demo and
offer a direct path to its installable CLI, architecture, and evidence.

A visitor who continues MUST be able to recover:

- the three-layer coverage thesis: inexpressibility, static detection, and
  deterministic schedule exploration;
- the implemented kernel and current profile crates on `main`;
- the enforcement layer behind each public claim;
- the negative controls and open gates;
- the project's larger direction for harness builders, engine authors, and
  meta-harnesses;
- the canonical `cargo add kittens` and lean `reactor!` entry points;
- the `kittens-code` install/demo path, four-crate topology, end-to-end turn
  flow, context/RLM law, durability law, portability claim, and open gates.

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

### 4.3 Flagship demo: kittens-code

The site MUST present `kittens-code` as the main end-to-end demonstration of
the application shape Kittens exists to support: a headless coding-agent
harness with a typed wire, sans-IO state machine, effect-owning host driver,
and JSONL composition-root CLI.

The flagship surface MUST include all of the following:

1. the four published `0.0.1` crates and their boundaries:
   `kittens-code-protocol`, `kittens-code-core`,
   `kittens-code-driver-tokio`, and `kittens-code-cli`;
2. the flow from JSONL `Op` through `CoreInput → Transition → CoreAction`,
   durable append/effect discharge, and authoritative `Event` publication;
3. the append-only checksummed transcript, resume-as-replay, torn-tail repair,
   and one-writer boundary;
4. the live-window/complete-log split, budgeted RLM `recall` continuation, and
   canonical verbs `grep`, `slice`, `head`, `tail`, `count`, `partition`,
   `ask`, `ask-each`, and `final`;
5. branded value caps, aggregate budget meters, exactly-once effect terminal
   ledger, deterministic offline jail, optional `live` Anthropic-dialect
   client, and path-constrained atomic filesystem tools;
6. the proven `thumbv7em-none-eabi` and `wasm32-unknown-unknown` link boundary
   for protocol/core, with IO remaining driver effects;
7. the DISPLAY / ORCHESTRATION / COGNITION decomposition, with
   `kittens-tui`/`kittens-render`, `kittens-code`, and the model kept visibly
   separable;
8. a copyable `cargo install kittens-code-cli --version 0.0.1` command and a
   deterministic jail scenario plus JSONL input example that is valid for the
   published CLI;
9. direct links to every crate on crates.io and docs.rs, plus the `kc0` source
   branch, frozen SPEC, research synthesis, FRONTMATTER architecture note,
   release changelog, and research-input archive.

The flagship copy MUST carry this status boundary beside the first
`kittens-code` heading:

- the four crates are real, published, unyanked experimental evidence
  releases;
- their repository source and controlling documents currently live on `kc0`,
  not on deployed `main`;
- the current Tokio runner demonstrates the core/driver/CLI spine, while the
  full `kittens::reactor!` driver topology and the E1 evaluation rig remain
  deferred KC0 scope;
- no MCU driver, on-silicon agent, production stability, or universal agent
  correctness is claimed.

Calling `kittens-code` the flagship demo describes product priority and
architectural coverage; it MUST NOT be used as evidence that every imported
KC0 gate is closed or that the current runner already exercises the kernel
macro.

### 4.4 What exists today

The website MUST distinguish code present on the deployed `main` source commit
from separately published evidence whose source is on a named repository
branch. W0.1 includes on `main`:

- `kittens` and `kittens-macros`: the experimental K0 `no_std` reactor/source
  kernel and compiler;
- `kittens-tui`: the terminal orchestration profile;
- `kittens-render`: the embedded rendering/interaction evidence profile.

The published `kittens-code` family is the sole W0.1 exception to the
main-source rule under section 4.3. Other unmerged work, candidate drivers, and
superseded root SPEC sections 11–36 MUST NOT be described as shipped. The
Embassy/web/WASI drivers, swarm, L2 implementation, MCU composition, full
reactor driver topology, and eval rigs remain explicitly labeled direction or
open gates.

Every crate card MUST link to its repository contract or README. Published
crates MAY also link to crates.io and docs.rs.

### 4.5 Evidence and honest boundary

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

### 4.6 Code and calls to action

The canonical code example MUST use the lean grammar from root SPEC section 38
and `docs/agent-guide.md`, not the superseded maximal grammar. It MUST include
load-bearing `///` rationale at an orchestration boundary and a nearby note
that handlers remain ordinary unchecked Rust.

Primary calls to action MUST lead to the GitHub repository and installation.
Secondary calls MAY lead to the agent guide, diagnostics, evidence report,
crate docs, and specs. No call to action may imply production stability.

The flagship installation action MUST lead with `kittens-code-cli`; the kernel
installation and lean `reactor!` example MUST remain available as the lower-
level path.

### 4.7 Vision

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

For W0.1, the social card MUST identify `kittens-code` as the flagship demo
without replacing the Kittens master brand or canonical kitten-and-yarn art.

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
| semantic document and keyboard/focus/reduced-motion affordances | semantic HTML/CSS + structural checker + browser review when available | automated structure/standards scan; manual keyboard/reduced-motion pass remains open when the environment has no browser | automated checks are not full WCAG conformance |
| no remote runtime dependencies or tracking | source allowlist + network/browser review | structural URL scan and clean-load request inspection | clicking an external link leaves the site boundary |
| exact deployed source is recoverable | generated `build.json` + deployment commit | compare live `build.json` to merged source SHA | GitHub Pages availability remains external |
| project claims match shipped evidence | documentation review against controlling contracts | source-linked research ledger and PR review | linked reports retain their own open gates |
| `kittens-code` flagship scope stays complete and honest | content assertions + review against the frozen `kc0` contracts and published registry state | four-crate/link/flow/install/status marker scan | flagship priority does not close deferred reactor, eval, MCU, or stability gates |
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
   focus are reviewed in a real browser when one is available; if mandatory
   browser discovery and troubleshooting returns no browser, publication MAY
   proceed only after the nonvisual gates pass, the gap is recorded, and no
   browser-review or WCAG-conformance claim is made;
7. the implementation enters through a PR to `main` after this spec-first
   commit;
8. the generated tree is committed and pushed to a new `gh-pages` branch;
9. the GitHub Pages repository setting names `gh-pages` and `/`;
10. the deployment reports success, the public URL returns HTTP 200, and the
    live `build.json` names the merged source commit.

The implementation MUST add one user-visible website entry to
[`CHANGELOG.md`](../CHANGELOG.md).

The structural checker MUST fail if the flagship section loses any published
crate name, the install command, the source-on-`kc0` boundary, the deferred
full-reactor/E1 boundary, or links to the SPEC, research synthesis,
FRONTMATTER, release changelog, and research inputs.

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
- an in-browser coding-agent runtime, hosted model endpoint, benchmark result,
  or claim that `kittens-code` is production-ready.

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
- **Merge `kc0` into `main`:** outside the website change. The site links the
  published source branch until a separately reviewed code merge lands; a
  future site build MUST update the status boundary when that repository fact
  changes.
