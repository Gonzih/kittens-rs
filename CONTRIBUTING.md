# Contributing

Kittens is still an evidence-driven K0 API. Changes to source admission,
readiness, poll order, drain semantics, phase behavior, or diagnostics need a
positive fixture, a failing mutation, an honest compiling negative control, and
an update to `K0-REPORT.md`.

Run formatting, clippy with warnings denied, all workspace tests, rustdoc, the
renamed-dependency fixture, and the feature-unified bare-metal gate before a
pull request. Do not add deferred scope, rendering, capability, simulation, or
hardware APIs without a separately authorized specification slice.
