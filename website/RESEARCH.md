# Kittens website research ledger

Date: 2026-08-09

This ledger records the evidence used to shape W0. It is not a new product
contract; [`SPEC.md`](SPEC.md) controls the website and each linked crate spec
controls its own semantics.

## 1. Repository and product evidence

- **Fact —** Root SPEC section 2.1 defines exactly three coverage mechanisms:
  inexpressibility, static detection, and deterministic schedule exploration.
  It also makes total confidence a function of escape surface and explicitly
  forbids presenting a six-nines ambition as static omniscience.
- **Fact —** Root SPEC section 3 names three consumer tiers: agents building
  one harness, engine/component authors, and meta-harnesses that generate and
  supervise other harnesses.
- **Fact —** Root SPEC section 37 is the controlling K0 implementation
  contract; sections 11–36 are retained superseded/candidate hypotheses.
  Website copy therefore cannot mine those sections for shipped APIs.
- **Fact —** [`K0-REPORT.md`](../K0-REPORT.md) selects direct lexical core
  polling with an owned event enum as the implemented candidate, while leaving
  formal K0 closure, agent ablation, diagnostic-mechanism comparison, and
  pinned performance work open.
- **Fact —** The K0 report records a 23-arm Grok-shape fixture, a 2,496-byte
  `.text` / 304-byte `.rodata` bare-metal model delta, no writable static data,
  and a 68.8% generated-future-size increase over its Tokio/event comparison.
  These figures are evidence with local test conditions, not general
  performance claims.
- **Fact —** The repository's compile-pass negative controls show that raw
  selection/spawning, removal of declarations, starvation waivers, and
  handler-interior unbounded loops remain legal. Those controls must sit near
  positive claims on the site.
- **Fact —** `kittens-tui` publishes runtime protocols for input isolation,
  ordered frame writing/acknowledgement, presenter gating, and terminal
  lifecycle. Its README explicitly refuses widget/layout/rendering ownership
  and raw-write prevention.
- **Fact —** `kittens-render` is an experimental K2R-0 host evidence release,
  not a display driver or physical-presentation proof. Its board-HIL,
  interrupt-delivery, kernel-admission, bilateral seam, `write_region`, and
  sealing gates remain open.
- **Observation —** The most differentiated public story is not “another async
  runtime.” It is that selected orchestration intent becomes local compiler
  input while handlers remain ordinary Rust and runtimes remain external.
- **Recommendation —** Lead with the edit-time outcome (“make orchestration
  harder to get wrong”), then immediately qualify it with “selected declared
  hazards” and “experimental.” Explain mechanism after relevance is clear.

## 2. Async landscape and positioning

- **Fact —** [Tokio's official `select!` documentation](https://docs.rs/tokio/latest/tokio/macro.select.html)
  says biased selection polls top-to-bottom and makes fairness the author's
  responsibility; it specifically warns that a busy stream can prevent a
  shutdown branch from being polled if ordered poorly. It also documents that
  losing branches are cancelled and that cancellation safety depends on the
  future being raced.
- **Fact —** [Tokio's homepage](https://tokio.rs/) leads with an outcome,
  follows with four short properties, and then shows the ecosystem stack.
  Tokio owns an async runtime and scheduler; Kittens explicitly does not.
- **Fact —** [Embassy's homepage](https://embassy.dev/) leads with the embedded
  outcome, places a realistic code example near the top, and then enumerates
  domain features. Embassy owns an executor/HAL ecosystem; Kittens' kernel
  instead aims to keep topology semantics executor-neutral.
- **Fact —** [Rust's official homepage](https://rust-lang.org/) organizes its
  case around performance, reliability, and productivity, then shows distinct
  domains including networking and embedded systems. This validates using
  familiar Rust outcomes before introducing Kittens-specific vocabulary.
- **Observation —** The comparable sites are strongest when a one-sentence
  promise, small proof-oriented feature set, concrete code, and ecosystem map
  appear in that order. None requires a complex application shell.
- **Hypothesis —** One short landing page will produce better first
  understanding than either a multi-page launch or a long-form technical
  essay, because the first job is earning curiosity rather than transferring
  the repository's full contract.
- **Recommendation —** Use one compressed journey: product promise → three
  flagship outcomes → three reasons the underlying approach matters → compact
  family view → install. Keep exhaustive truth in repository documentation.

## 3. Brand and visual research

- **Fact —** Commit `b6460bf` added two project-owned assets:
  `assets/kittens-logo.webp` (640×640, 69,830 bytes) and
  `assets/kittens-yarn-banner.webp` (1536×648, 140,934 bytes).
- **Observation —** The logo is already the requested mark: an orange tabby
  and a tuxedo kitten playing around a coral ball of yarn. The banner extends
  the system with a calico kitten and interlaced coral, teal, and lavender
  yarn on warm cream.
- **Observation —** A warm cream base with dark charcoal typography allows the
  illustrations to stay playful while code blocks, evidence labels, and
  enforcement tables retain technical authority.
- **Recommendation —** Treat the local art as canonical instead of sourcing a
  web image. This avoids licensing/provenance ambiguity, preserves continuity
  with crates.io and the README, and satisfies the requested theme exactly.
- **Recommendation —** Derive a restrained palette from the assets: cream
  surfaces, charcoal text, coral actions, teal evidence accents, lavender
  direction accents, and orange highlights. Use system sans/mono fonts and
  rounded geometry without novelty display type.
- **Hypothesis —** A yarn strand that visually passes through the coverage
  layers can make “many async strands, one declared topology” memorable without
  pretending the art is a technical diagram.

## 4. Accessibility and interaction evidence

- **Fact —** [WCAG 2.2](https://www.w3.org/TR/WCAG22/) is the current W3C
  Recommendation and adds criteria including focus not obscured and target
  size. W0 targets Level AA within the limits stated in the website spec.
- **Fact —** W3C's [contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum)
  sets 4.5:1 for ordinary text and 3:1 for large text. W0 uses those as hard
  palette gates rather than relying on aesthetic judgment.
- **Fact —** W3C's [target-size guidance](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum)
  sets a 24×24 CSS-pixel Level-AA minimum with exceptions and recommends larger
  important targets. W0 aims for 44×44 primary controls.
- **Fact —** W3C lists a `prefers-reduced-motion` technique for preventing
  unnecessary motion. All W0 motion is decorative and disabled under that
  preference.
- **Observation —** A sticky header can obscure keyboard focus and anchored
  headings unless scroll padding is reserved. W0 must pair stickiness with
  `scroll-padding-top` and strong `:focus-visible` treatment.
- **Recommendation —** Keep navigation short enough to wrap cleanly without a
  mobile menu. This preserves full function when JavaScript is disabled and
  eliminates a disclosure-widget accessibility surface.

## 5. Performance, privacy, and search evidence

- **Fact —** [Core Web Vitals](https://web.dev/articles/vitals) currently use
  LCP, INP, and CLS as the stable user-experience metrics. Lab tools can
  measure LCP and CLS; a static load cannot directly establish field INP.
- **Observation —** The two canonical WebP assets total about 206 KiB. Loading
  only the 640×640 logo in the first viewport and lazy-loading the banner keeps
  the initial image budget beneath the W0 250 KiB target before HTML/CSS.
- **Recommendation —** Use explicit image dimensions, no remote fonts, one
  small deferred script, and no framework. This minimizes layout shift,
  blocking work, supply-chain surface, and privacy questions simultaneously.
- **Fact —** [Google's structured-data guidance](https://developers.google.com/search/docs/appearance/structured-data/software-app)
  requires rating or review data for a SoftwareApplication rich result.
- **Recommendation —** Do not invent reviews or ratings merely to qualify for
  a rich result. W0 should ship accurate title/description/canonical/Open Graph
  metadata and a bespoke social card; richer app markup can wait for a valid
  evidence set.
- **Fact —** W0 has no form, account, cookie, analytics, or background API.
  External network requests occur only when a visitor deliberately follows an
  external link.

## 6. GitHub Pages publication evidence

- **Fact —** [GitHub's publishing-source documentation](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)
  supports publishing the root or `/docs` directory of any branch. It notes
  that externally generated static output commonly lands on `gh-pages` with a
  `.nojekyll` file.
- **Fact —** The [GitHub Pages REST API](https://docs.github.com/en/rest/pages/pages)
  can create a Pages site with a branch and `/` source. The repository returned
  HTTP 404 for the Pages resource before W0, confirming no site was configured.
- **Fact —** `Gonzih/kittens-rs` is public, its default branch is `main`, and no
  local or remote `gh-pages` branch existed at the start of W0.
- **Recommendation —** Keep editable source and the research/spec ledger on
  `main`, publish deterministic output to a new `gh-pages` root, configure the
  source explicitly through the API, and verify the live provenance marker.

## 7. kittens-code flagship audit

- **Fact —** The operator designated `kittens-code` as Kittens' main demo on
  2026-08-09. The website therefore treats flagship status as product priority,
  not as a new semantic guarantee.
- **Fact —** The family has four published crates at unyanked version `0.0.1`:
  `kittens-code-protocol`, `kittens-code-core`,
  `kittens-code-driver-tokio`, and `kittens-code-cli`. The crates.io API was
  checked directly on 2026-08-09; each corresponding docs.rs page returned
  HTTP 200.
- **Fact —** The exact published repository state is branch `kc0` at
  `2a2fb0d63e817515bc17514c197260af14046a16`. It is not an ancestor of the
  deployed `main`: both branches carry unique work after merge base
  `185d009f79f2e97a3b58c7150c2d87cdf14c7661`. Website links therefore name
  `kc0` explicitly instead of constructing broken `main` links.
- **Fact —** The frozen KC0 contract names a four-layer crate path: serde-only
  no_std wire data; a no_std + alloc sans-IO engine; a Tokio effect driver; and
  the `kittens-code` JSONL composition-root binary. The CLI's indexed call
  graph confirms that its protocol loop submits typed Ops through `Runner`,
  drives to quiescence, and writes newly published Events exactly once.
- **Fact —** The core contract is `handle(CoreInput) -> Transition`, with one
  bounded owned action batch. The appender is the sole write path; authoritative
  Events follow durability. Model, store-page, VFS, timer, and sub-model work
  remain driver effects correlated by `EffectId` and `TurnEpoch`.
- **Fact —** KC0 implements an append-only checksummed JSONL transcript,
  physical torn-tail truncation before later appends, persisted crash repair,
  exclusive per-log writer locking, resume-as-replay, checked monotonic ids,
  and an ownership/epoch terminal ledger. These were explicit release-review
  blockers and the release changelog records their closure before publication.
- **Fact —** The context engine keeps the full transcript queryable while the
  live model window compacts. The RLM executor lowers `grep`, `slice`, `head`,
  `tail`, `count`, `partition`, `ask`, `ask-each`, and `final` into a typed,
  budgeted continuation; a successful `recall` returns through the ordinary
  capped tool-result path.
- **Fact —** Core law includes branded caps for model-visible values, compiled
  aggregate-meter ceilings, budget update Events, checked/saturating integer
  behavior, exactly-once effect terminals, and deterministic replay oracles.
- **Fact —** The default `JailClient` is scripted and offline. The optional
  `live` feature admits an Anthropic-dialect SSE client with bounded retry,
  `Retry-After`, cancellation, and a circuit breaker. Default builds do not
  pull the HTTP/TLS tree.
- **Fact —** `kittens-code-protocol` and `kittens-code-core` were release-gated
  on both `thumbv7em-none-eabi` and `wasm32-unknown-unknown`. This proves the
  center links under those targets; it does not prove a working MCU/Web driver
  or an on-silicon agent.
- **Fact —** The current driver README says the full `kittens::reactor!`
  topology and E1 evaluation rig remain deferred KC0 scope. The website must
  not transform the architectural target in the SPEC/FRONTMATTER note into a
  shipped-current-runner claim.
- **Observation —** `kittens-code` is the clearest public explanation of why
  the kernel exists: one application forces streaming, interrupt/shutdown,
  effect completion, persistence, context pressure, tools, budgets, and
  frontmatter to share an inspectable vocabulary.
- **Fact —** After reviewing the published W0.1 page, the operator rejected
  visible branch topology, gate identifiers, release-review framing, and the
  exhaustive architecture walkthrough as inappropriate for marketing.
- **Observation —** W0.1 preserved technical truth but made visitors process
  the internal development structure before reaching the product payoff. The
  volume turned evidence into friction.
- **Recommendation —** After the operator's W0.2 review, remove even the
  three-outcome cards and explanatory image overlays. Keep `kittens-code`
  immediately after the hero, state its payoff once, and move implementation
  vocabulary back to repository docs.

## 8. Current developer-tool landing pattern audit

- **Fact —** [Cursor](https://www.cursor.com/) leads with “The AI Code Editor,”
  one short payoff, one primary download action, and a dominant product visual.
- **Fact —** [Resend](https://resend.com/home) leads with “Email for
  developers,” one payoff sentence, and two actions before product detail.
- **Fact —** [Linear](https://linear.app/homepage) opens with a category claim,
  a short AI-era payoff, and a dominant product visual before deeper material.
- **Fact —** [Vercel](https://vercel.com/home) uses a bold category line, one
  supporting sentence, and a very small action set before elaboration.
- **Observation —** Their impact comes from hierarchy, restraint, and the
  product visual. Floating explanations and repeated benefit cards are not what
  makes the first viewport feel current.
- **Recommendation —** W0.3 should use one headline, one sentence, one action,
  and unannotated art in the hero, followed only by a minimal flagship block and
  a minimal statement of the underlying idea.

## 9. Gaps and decisions deliberately not invented

- **Gap: no visitor research, search-query data, or analytics baseline exists
  for Kittens (no data exists).** W0 uses the audiences already named in the
  root SPEC and collects no behavioral data.
- **Gap: no field baseline exists for the distribution of async race/ordering
  defects across Kittens' coverage classes (no data exists).** The website
  avoids numeric total-coverage claims.
- **Gap: no custom domain, DNS ownership proof, or domain preference was
  supplied (no data exists).** W0 uses GitHub's project URL.
- **Gap: no formal public trademark or logo-usage policy exists (no data
  exists).** W0 uses only repository-owned art inside the repository's own
  website and does not claim a transferable brand license.
- **Gap: no production-user testimonials or adoption figures exist in the
  repository (no data exists).** W0 invents neither.
- **Gap: the publication environment exposed no controllable browser after the
  required discovery and troubleshooting flow (no browser exists).** W0 can
  establish semantic structure, standards validation, local HTTP behavior,
  reproducibility, and live deployment, but manual keyboard, zoom, responsive,
  reduced-motion, and visual browser review remains an explicitly open QA gate.
