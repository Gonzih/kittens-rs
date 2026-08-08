# Diagnostic-only repair pilot artifacts (2026-08-07)

Method, results, and limitations are in `K0-REPORT.md`. The four mutated
fixtures in this directory are the exact inputs; the verified repair diffs
per trial follow (a/b are independent blind trials of the same mutation).

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

