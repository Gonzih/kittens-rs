# Kittens public website

The editable GitHub Pages source lives in [`src/`](src). The controlling
contract is [`SPEC.md`](SPEC.md), and [`RESEARCH.md`](RESEARCH.md) records the
claim, visual, accessibility, and deployment evidence used for W0.

Build and verify from the repository root:

```console
node website/scripts/build.mjs
node website/scripts/check.mjs website/dist
node website/scripts/repro.mjs
```

The build uses only Node standard-library modules. It copies the canonical
project artwork from root `assets/`, injects the current source commit into
`build.json` and the footer, and writes ignored output to `website/dist/`.

`gh-pages` is generated publication state. Edit `website/src/` on a branch,
merge through `main`, rebuild from that merged commit, and publish the exact
`website/dist/` tree. Do not hand-edit the deployment branch.

The site intentionally has no analytics, forms, cookies, remote fonts,
framework, or background network requests.
