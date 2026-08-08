# Reported diagnostic-only repair smoke exercise (2026-08-07)

Method, results, and limitations are in `K0-REPORT.md`. This directory
retains four reported mutated inputs and eight final patch excerpts (a/b are
reported as separate attempts of the same mutation). It does not retain exact
prompts, model identifiers, transcripts, rustc JSON, test/oracle output,
token/tool/time counts, or network-isolation evidence. These files therefore
do not establish that the attempts were blind or independent, verify the
reported iteration counts, or satisfy SPEC sections 27.4 and 37.11.

## Trial m1a

```diff
22a23,29
>         #[source(stop)]
>         #[readiness(quiescent)]
>         #[shutdown]
>         _ = sources.stop => {
>             Ok(0)
>         }
>
33,39d39
<
<         #[source(stop)]
<         #[readiness(quiescent)]
<         #[shutdown]
<         _ = sources.stop => {
<             Ok(0)
<         }
```

## Trial m1b

```diff
22a23,29
>         #[source(stop)]
>         #[readiness(quiescent)]
>         #[shutdown]
>         _ = sources.stop => {
>             Ok(0)
>         }
>
33,39d39
<
<         #[source(stop)]
<         #[readiness(quiescent)]
<         #[shutdown]
<         _ = sources.stop => {
<             Ok(0)
<         }
```

## Trial m2a

```diff
32a33
>         #[yields_to(input, when = buffered)]
```

## Trial m2b

```diff
32a33
>         #[yields_to(input, when = buffered)]
```

## Trial m3a

```diff
19a20,23
>     // KTR015 canonical repair: retain the one-time probe as a persistent
>     // one-shot source constructed before entering the reactor.
>     let mut job = kittens::source::one_shot(probe_job());
>
36c40
<         result = probe_job() => {
---
>         result = job => {
```

## Trial m3b

```diff
19a20
>     let mut job = kittens::source::one_shot(probe_job());
36c37
<         result = probe_job() => {
---
>         result = job => {
```

## Trial m4a

```diff
32d31
<         #[drain(max = 4)]
```

## Trial m4b

```diff
32d31
<         #[drain(max = 4)]
```
