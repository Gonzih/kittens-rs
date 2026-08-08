//! Procedural-macro compiler for the Kittens K0 reactor grammar.

#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Attribute, Block, Error, Expr, Ident, LitInt, LitStr, Pat, Result, Token, braced, bracketed,
};

const MAX_DRAIN: usize = 4096;

/// Expands the selected K0 implementation: direct core polling with an owned
/// private event enum.
#[proc_macro]
pub fn reactor(input: TokenStream) -> TokenStream {
    expand_entry(input, Backend::Core, Transfer::Event)
}

/// Retained comparison: direct core polling with a private event enum.
#[doc(hidden)]
#[proc_macro]
pub fn reactor_event(input: TokenStream) -> TokenStream {
    expand_entry(input, Backend::Core, Transfer::Event)
}

/// Retained comparison: direct core polling with a selected tag and per-arm
/// item slots.
#[doc(hidden)]
#[proc_macro]
pub fn reactor_slots(input: TokenStream) -> TokenStream {
    expand_entry(input, Backend::Core, Transfer::Slots)
}

/// Retained control: direct biased Tokio selection with a private event enum.
#[doc(hidden)]
#[proc_macro]
pub fn reactor_tokio_event(input: TokenStream) -> TokenStream {
    expand_entry(input, Backend::Tokio, Transfer::Event)
}

/// Retained control: direct biased Tokio selection with a selected tag and
/// per-arm item slots.
#[doc(hidden)]
#[proc_macro]
pub fn reactor_tokio_slots(input: TokenStream) -> TokenStream {
    expand_entry(input, Backend::Tokio, Transfer::Slots)
}

#[derive(Clone, Copy)]
enum Backend {
    Core,
    Tokio,
}

#[derive(Clone, Copy)]
enum Transfer {
    Event,
    Slots,
}

fn expand_entry(input: TokenStream, backend: Backend, transfer: Transfer) -> TokenStream {
    let result = syn::parse::<Reactor>(input).and_then(|reactor| {
        validate(&reactor)?;
        Ok(expand(&reactor, backend, transfer))
    });
    match result {
        Ok(tokens) => tokens.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

struct Reactor {
    policy: Policy,
    initialize: Option<Phase>,
    before_poll: Option<Phase>,
    after_event: Option<Phase>,
    arms: Vec<Arm>,
}

struct Policy {
    required_phases: Vec<Ident>,
}

struct Phase {
    name: Ident,
    block: Block,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ReadinessKind {
    MayRemainReady,
    Quiescent,
}

struct Relation {
    target: Ident,
    span: Span,
}

struct YieldRelation {
    target: Ident,
    span: Span,
}

struct Drain {
    max: usize,
    span: Span,
}

struct Arm {
    id: Ident,
    id_span: Span,
    readiness: ReadinessKind,
    readiness_span: Span,
    shutdown_span: Option<Span>,
    terminal_span: Option<Span>,
    before: Vec<Relation>,
    last_span: Option<Span>,
    starvation_reason: Option<LitStr>,
    starvation_span: Option<Span>,
    when: Option<Expr>,
    yields_to: Option<YieldRelation>,
    drain: Option<Drain>,
    binding: Pat,
    source: Expr,
    handler: Block,
}

impl Arm {
    fn is_terminal(&self) -> bool {
        self.shutdown_span.is_some() || self.terminal_span.is_some()
    }

    fn is_shutdown(&self) -> bool {
        self.shutdown_span.is_some()
    }

    fn is_protected(&self) -> bool {
        self.starvation_reason.is_none()
    }
}

impl Parse for Reactor {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let policy_name: Ident = input.parse()?;
        if policy_name != "policy" {
            return Err(ktr(
                policy_name.span(),
                "KTR000",
                "a reactor expression must begin with `policy { ... }`",
                "use the lean expression grammar from the reactor reference",
            ));
        }
        let policy = parse_policy(input)?;

        let mut initialize = None;
        let mut before_poll = None;
        let mut after_event = None;
        let mut arms = Vec::new();

        while !input.is_empty() {
            if input.peek(Token![#]) {
                arms.push(parse_arm(input)?);
                continue;
            }

            let name: Ident = input.parse()?;
            let block: Block = input.parse().map_err(|_| {
                ktr(
                    name.span(),
                    "KTR000",
                    "expected a phase block or an attributed source arm",
                    "write `initialize { ... }`, `before_poll { ... }`, `after_event { ... }`, or start an arm with `#[source(...)]`",
                )
            })?;
            let phase = Phase {
                name: name.clone(),
                block,
            };
            let slot = match name.to_string().as_str() {
                "initialize" => &mut initialize,
                "before_poll" => &mut before_poll,
                "after_event" => &mut after_event,
                _ => {
                    return Err(ktr(
                        name.span(),
                        "KTR000",
                        &format!("unknown reactor phase `{name}`"),
                        "use only `initialize`, `before_poll`, or `after_event`",
                    ));
                }
            };
            if slot.is_some() {
                return Err(ktr(
                    name.span(),
                    "KTR011",
                    &format!("phase `{name}` appears more than once"),
                    "keep exactly one block for each required phase",
                ));
            }
            *slot = Some(phase);
        }

        Ok(Self {
            policy,
            initialize,
            before_poll,
            after_event,
            arms,
        })
    }
}

fn parse_policy(input: ParseStream<'_>) -> Result<Policy> {
    let content;
    braced!(content in input);
    let mut selection_seen = false;
    let mut phases: Option<Vec<Ident>> = None;

    while !content.is_empty() {
        let field: Ident = content.parse()?;
        content.parse::<Token![:]>()?;
        match field.to_string().as_str() {
            "selection" => {
                if selection_seen {
                    return Err(ktr(
                        field.span(),
                        "KTR012",
                        "`selection` is declared more than once",
                        "keep exactly `selection: biased;`",
                    ));
                }
                let value: Ident = content.parse()?;
                content.parse::<Token![;]>()?;
                if value != "biased" {
                    return Err(ktr(
                        value.span(),
                        "KTR012",
                        &format!("unsupported selection policy `{value}`"),
                        "use `selection: biased;`",
                    ));
                }
                selection_seen = true;
            }
            "required_phases" => {
                if phases.is_some() {
                    return Err(ktr(
                        field.span(),
                        "KTR011",
                        "`required_phases` is declared more than once",
                        "keep one exact phase list",
                    ));
                }
                let list;
                bracketed!(list in content);
                let parsed = list
                    .parse_terminated(Ident::parse, Token![,])?
                    .into_iter()
                    .collect();
                content.parse::<Token![;]>()?;
                phases = Some(parsed);
            }
            _ => {
                return Err(ktr(
                    field.span(),
                    "KTR012",
                    &format!("unsupported policy field `{field}`"),
                    "use only `selection: biased;` and `required_phases: [...]`",
                ));
            }
        }
    }

    if !selection_seen {
        return Err(ktr(
            Span::call_site(),
            "KTR012",
            "missing `selection: biased;`",
            "declare the lexical biased selection policy explicitly",
        ));
    }
    let required_phases = phases.ok_or_else(|| {
        ktr(
            Span::call_site(),
            "KTR011",
            "missing `required_phases: [...]`",
            "declare the exact phase set, using `[]` when no phases are required",
        )
    })?;
    Ok(Policy { required_phases })
}

#[allow(clippy::too_many_lines)]
fn parse_arm(input: ParseStream<'_>) -> Result<Arm> {
    let attrs = Attribute::parse_outer(input)?;
    let binding = Pat::parse_single(input)?;
    match &binding {
        Pat::Ident(ident)
            if ident.by_ref.is_none() && ident.mutability.is_none() && ident.subpat.is_none() => {}
        Pat::Wild(_) => {}
        _ => {
            return Err(ktr(
                binding.span(),
                "KTR000",
                "a source binding must be `_` or one immutable identifier",
                "move destructuring into the handler body",
            ));
        }
    }
    input.parse::<Token![=]>()?;
    let source = Expr::parse_without_eager_brace(input)?;
    input.parse::<Token![=>]>()?;
    let handler: Block = input.parse()?;

    let mut id: Option<(Ident, Span)> = None;
    let mut readiness: Option<(ReadinessKind, Span)> = None;
    let mut shutdown_span = None;
    let mut terminal_span = None;
    let mut before = Vec::new();
    let mut last_span = None;
    let mut starvation_reason = None;
    let mut starvation_span = None;
    let mut when = None;
    let mut yields_to = None;
    let mut drain = None;

    for attr in attrs {
        let Some(name) = attr.path().get_ident().map(ToString::to_string) else {
            return Err(unsupported_attr(&attr));
        };
        match name.as_str() {
            "source" => {
                unique(id.as_ref(), &attr, "source")?;
                let parsed: Ident = attr.parse_args()?;
                id = Some((parsed, attr.span()));
            }
            "readiness" => {
                unique(readiness.as_ref(), &attr, "readiness")?;
                let parsed: Ident = attr.parse_args()?;
                let kind = match parsed.to_string().as_str() {
                    "may_remain_ready" => ReadinessKind::MayRemainReady,
                    "quiescent" => ReadinessKind::Quiescent,
                    _ => {
                        return Err(ktr(
                            parsed.span(),
                            "KTR017",
                            &format!("unsupported lean readiness `{parsed}`"),
                            "use exactly `may_remain_ready` or `quiescent`",
                        ));
                    }
                };
                readiness = Some((kind, attr.span()));
            }
            "shutdown" => set_flag(&mut shutdown_span, &attr, "shutdown")?,
            "terminal" => set_flag(&mut terminal_span, &attr, "terminal")?,
            "last" => set_flag(&mut last_span, &attr, "last")?,
            "before" => {
                let target: Ident = attr.parse_args()?;
                before.push(Relation {
                    target,
                    span: attr.span(),
                });
            }
            "starvation" => {
                if starvation_reason.is_some() {
                    return Err(duplicate_attr(&attr, "starvation"));
                }
                let parsed: StarvationArgs = attr.parse_args()?;
                if parsed.reason.value().trim().is_empty() {
                    return Err(ktr(
                        parsed.reason.span(),
                        "KTR018",
                        "a starvation waiver requires a nonempty reason",
                        "state the accepted policy risk or keep the source protected",
                    ));
                }
                starvation_span = Some(attr.span());
                starvation_reason = Some(parsed.reason);
            }
            "when" => {
                if when.is_some() {
                    return Err(duplicate_attr(&attr, "when"));
                }
                when = Some(attr.parse_args()?);
            }
            "yields_to" => {
                if yields_to.is_some() {
                    return Err(duplicate_attr(&attr, "yields_to"));
                }
                let parsed: YieldArgs = attr.parse_args()?;
                yields_to = Some(YieldRelation {
                    target: parsed.target,
                    span: attr.span(),
                });
            }
            "drain" => {
                if drain.is_some() {
                    return Err(duplicate_attr(&attr, "drain"));
                }
                let parsed: DrainArgs = attr.parse_args()?;
                let suffix = parsed.max.suffix();
                let value = parsed.max.base10_parse::<usize>().map_err(|_| {
                    ktr(
                        parsed.max.span(),
                        "KTR008",
                        "drain max is not a supported positive integer literal",
                        &format!("use an unsuffixed literal from 1 through {MAX_DRAIN}"),
                    )
                })?;
                if !suffix.is_empty() || !(1..=MAX_DRAIN).contains(&value) {
                    return Err(ktr(
                        parsed.max.span(),
                        "KTR008",
                        &format!(
                            "drain max must be an unsuffixed literal from 1 through {MAX_DRAIN}"
                        ),
                        "choose a positive bounded service window",
                    ));
                }
                drain = Some(Drain {
                    max: value,
                    span: attr.span(),
                });
            }
            _ => return Err(unsupported_attr(&attr)),
        }
    }

    let (id, id_span) = id.ok_or_else(|| {
        ktr(
            source.span(),
            "KTR017",
            "source arm is missing `#[source(id)]`",
            "give the arm one stable source identifier",
        )
    })?;
    let (readiness, readiness_span) = readiness.ok_or_else(|| {
        ktr(
            id_span,
            "KTR017",
            &format!("source `{id}` is missing `#[readiness(...)]`"),
            "declare exactly `may_remain_ready` or `quiescent`",
        )
    })?;

    Ok(Arm {
        id,
        id_span,
        readiness,
        readiness_span,
        shutdown_span,
        terminal_span,
        before,
        last_span,
        starvation_reason,
        starvation_span,
        when,
        yields_to,
        drain,
        binding,
        source,
        handler,
    })
}

fn unique<T>(slot: Option<&T>, attr: &Attribute, name: &str) -> Result<()> {
    if slot.is_some() {
        Err(duplicate_attr(attr, name))
    } else {
        Ok(())
    }
}

fn set_flag(slot: &mut Option<Span>, attr: &Attribute, name: &str) -> Result<()> {
    if slot.is_some() {
        Err(duplicate_attr(attr, name))
    } else if !matches!(attr.meta, syn::Meta::Path(_)) {
        Err(ktr(
            attr.span(),
            "KTR000",
            &format!("`#[{name}]` takes no arguments"),
            &format!("write exactly `#[{name}]`"),
        ))
    } else {
        *slot = Some(attr.span());
        Ok(())
    }
}

fn duplicate_attr(attr: &Attribute, name: &str) -> Error {
    ktr(
        attr.span(),
        "KTR017",
        &format!("source arm declares `#[{name}]` more than once"),
        "keep one canonical declaration",
    )
}

fn unsupported_attr(attr: &Attribute) -> Error {
    ktr(
        attr.span(),
        "KTR000",
        &format!(
            "unsupported lean reactor attribute `{}`",
            attr.path().to_token_stream()
        ),
        "use the section-37 lean grammar; maximal lifecycle, close, cancellation, priority-class, and batch attributes are not supported",
    )
}

struct StarvationArgs {
    reason: LitStr,
}

impl Parse for StarvationArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let allowed: Ident = input.parse()?;
        if allowed != "allowed" {
            return Err(ktr(
                allowed.span(),
                "KTR018",
                "the lean grammar supports only an explicit `allowed` starvation waiver",
                "remove the attribute to keep default protection, or write `allowed, reason = \"...\"`",
            ));
        }
        input.parse::<Token![,]>()?;
        let reason: Ident = input.parse()?;
        if reason != "reason" {
            return Err(ktr(
                reason.span(),
                "KTR018",
                "expected `reason = \"...\"` in starvation waiver",
                "supply a static architectural rationale",
            ));
        }
        input.parse::<Token![=]>()?;
        let reason = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after starvation reason"));
        }
        Ok(Self { reason })
    }
}

struct YieldArgs {
    target: Ident,
}

impl Parse for YieldArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let target = input.parse()?;
        input.parse::<Token![,]>()?;
        let when: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let buffered: Ident = input.parse()?;
        if when != "when" || buffered != "buffered" || !input.is_empty() {
            return Err(ktr(
                when.span(),
                "KTR010",
                "buffered yield must use `target, when = buffered`",
                "use the canonical buffered-yield spelling",
            ));
        }
        Ok(Self { target })
    }
}

struct DrainArgs {
    max: LitInt,
}

impl Parse for DrainArgs {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let max_name: Ident = input.parse()?;
        if max_name != "max" {
            return Err(ktr(
                max_name.span(),
                "KTR008",
                "drain requires `max = N`",
                &format!("use an unsuffixed literal from 1 through {MAX_DRAIN}"),
            ));
        }
        input.parse::<Token![=]>()?;
        let max = input.parse().map_err(|_| {
            ktr(
                input.span(),
                "KTR008",
                "drain max must be an unsuffixed integer literal",
                &format!("use a literal from 1 through {MAX_DRAIN}"),
            )
        })?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after drain max"));
        }
        Ok(Self { max })
    }
}

#[allow(clippy::too_many_lines)]
fn validate(reactor: &Reactor) -> Result<()> {
    if reactor.arms.is_empty() {
        return Err(ktr(
            Span::call_site(),
            "KTR000",
            "a reactor requires at least one source arm",
            "add one admitted persistent source",
        ));
    }

    validate_phases(reactor)?;

    let mut ids = HashMap::new();
    let mut places = HashMap::<String, (String, Span)>::new();
    for (index, arm) in reactor.arms.iter().enumerate() {
        let id = arm.id.to_string();
        if ids.insert(id.clone(), index).is_some() {
            return Err(ktr(
                arm.id_span,
                "KTR001",
                &format!("duplicate reactor source id `{id}`"),
                "rename one source declaration and update its relations",
            ));
        }
        if !is_persistent_place(&arm.source) {
            return Err(ktr(
                arm.source.span(),
                "KTR015",
                &format!(
                    "source `{id}` must be a persistent path or field, not a temporary expression"
                ),
                "construct a reviewed persistent source before the reactor, or isolate the producer behind an explicitly owned task/signal and admitted channel source",
            ));
        }
        let place = normalize_place(&arm.source);
        if let Some((first, _)) = places.insert(place, (id.clone(), arm.source.span())) {
            return Err(ktr(
                arm.source.span(),
                "KTR020",
                &format!("source place is declared under both `{first}` and `{id}`"),
                "keep one source ID for this exact persistent place; distinct-place aliasing remains a rustc backstop",
            ));
        }

        if arm.is_shutdown() {
            if arm.when.is_some() || arm.yields_to.is_some() || arm.drain.is_some() {
                return Err(ktr(
                    arm.shutdown_span.expect("checked"),
                    "KTR005",
                    &format!(
                        "shutdown source `{id}` must be unguarded, undrained, and unable to yield"
                    ),
                    "remove `when`, `yields_to`, and `drain`; keep long cleanup in the enclosing owner",
                ));
            }
            if let Some(last_span) = arm.last_span {
                return Err(ktr(
                    last_span,
                    "KTR005",
                    &format!("shutdown source `{id}` cannot be `last`"),
                    "keep shutdown in the leading lexical prefix",
                ));
            }
            if arm.starvation_reason.is_some() {
                return Err(ktr(
                    arm.starvation_span.expect("checked"),
                    "KTR005",
                    &format!("shutdown source `{id}` cannot waive starvation protection"),
                    "remove the waiver and keep shutdown in the leading prefix",
                ));
            }
        }
        if let (true, Some(drain)) = (arm.is_terminal(), arm.drain.as_ref()) {
            return Err(ktr(
                drain.span,
                "KTR008",
                &format!(
                    "terminal source `{id}` cannot be drained because its first successful item exits"
                ),
                "remove `drain`",
            ));
        }
    }

    for arm in &reactor.arms {
        for relation in &arm.before {
            if !ids.contains_key(&relation.target.to_string()) {
                return Err(unknown_relation(&arm.id, &relation.target, relation.span));
            }
        }
        if let Some(relation) = &arm.yields_to {
            if !ids.contains_key(&relation.target.to_string()) {
                return Err(unknown_relation(&arm.id, &relation.target, relation.span));
            }
            if relation.target == arm.id {
                return Err(ktr(
                    relation.span,
                    "KTR010",
                    &format!("source `{}` cannot yield to itself", arm.id),
                    "target a distinct backlog-probeable source",
                ));
            }
        }
    }

    let last: Vec<_> = reactor
        .arms
        .iter()
        .enumerate()
        .filter_map(|(index, arm)| arm.last_span.map(|span| (index, arm, span)))
        .collect();
    if last.len() > 1 {
        let (_, arm, span) = last[1];
        return Err(ktr(
            span,
            "KTR004",
            &format!(
                "source `{}` conflicts with earlier global `last` source `{}`",
                arm.id, last[0].1.id
            ),
            "keep exactly one global last source",
        ));
    }
    if let Some((index, arm, span)) = last.first().copied() {
        if index + 1 != reactor.arms.len() {
            return Err(ktr(
                span,
                "KTR004",
                &format!(
                    "source `{}` is declared `last`, but source `{}` follows it",
                    arm.id,
                    reactor.arms[index + 1].id
                ),
                "move the complete last source arm to the end without changing its attributes",
            ));
        }
    }

    let mut edges = Vec::new();
    for (from, arm) in reactor.arms.iter().enumerate() {
        for relation in &arm.before {
            edges.push(Edge {
                from,
                to: ids[&relation.target.to_string()],
                span: relation.span,
                reason: format!("`#[before({})]`", relation.target),
            });
        }
    }
    for (from, arm) in reactor.arms.iter().enumerate() {
        if arm.is_shutdown() {
            for (to, target) in reactor.arms.iter().enumerate() {
                if !target.is_shutdown() {
                    edges.push(Edge {
                        from,
                        to,
                        span: arm.shutdown_span.expect("checked"),
                        reason: "the shutdown leading-prefix rule".to_owned(),
                    });
                }
            }
        }
    }
    if let Some((last_index, arm, span)) = last.first().copied() {
        for from in 0..reactor.arms.len() {
            if from != last_index {
                edges.push(Edge {
                    from,
                    to: last_index,
                    span,
                    reason: format!("`#[last]` on `{}`", arm.id),
                });
            }
        }
    }

    validate_cycles(reactor, &edges, "KTR003", "scheduling")?;
    validate_yield_cycles(reactor, &ids)?;

    for edge in &edges {
        if edge.from >= edge.to {
            let predecessor = &reactor.arms[edge.from].id;
            let successor = &reactor.arms[edge.to].id;
            return Err(ktr(
                edge.span,
                "KTR016",
                &format!(
                    "source `{predecessor}` must precede `{successor}` because of {}",
                    edge.reason
                ),
                "move the complete source arm so lexical order matches the declared relation",
            ));
        }
    }

    for (victim_index, victim) in reactor.arms.iter().enumerate() {
        if !victim.is_protected() {
            continue;
        }
        for dominant in &reactor.arms[..victim_index] {
            if dominant.readiness != ReadinessKind::MayRemainReady {
                continue;
            }
            let yields = dominant
                .yields_to
                .as_ref()
                .is_some_and(|relation| relation.target == victim.id);
            if !yields {
                return Err(ktr(
                    dominant.readiness_span,
                    "KTR007",
                    &format!(
                        "may-remain-ready source `{}` can starve protected source `{}`",
                        dominant.id, victim.id
                    ),
                    &format!(
                        "move `{}` above `{}`, or add `#[yields_to({}, when = buffered)]` when `{}` is backlog-probeable; a starvation waiver changes policy",
                        victim.id, dominant.id, victim.id, victim.id
                    ),
                ));
            }
        }
    }

    Ok(())
}

fn validate_phases(reactor: &Reactor) -> Result<()> {
    let mut required = HashSet::new();
    for phase in &reactor.policy.required_phases {
        let name = phase.to_string();
        if !matches!(name.as_str(), "initialize" | "before_poll" | "after_event") {
            return Err(ktr(
                phase.span(),
                "KTR011",
                &format!("unknown required phase `{name}`"),
                "use only `initialize`, `before_poll`, and `after_event`",
            ));
        }
        if !required.insert(name.clone()) {
            return Err(ktr(
                phase.span(),
                "KTR011",
                &format!("required phase `{name}` is listed more than once"),
                "keep one occurrence in the exact phase set",
            ));
        }
    }

    for (name, present, span) in [
        (
            "initialize",
            reactor.initialize.is_some(),
            reactor
                .initialize
                .as_ref()
                .map_or(Span::call_site(), |p| p.name.span()),
        ),
        (
            "before_poll",
            reactor.before_poll.is_some(),
            reactor
                .before_poll
                .as_ref()
                .map_or(Span::call_site(), |p| p.name.span()),
        ),
        (
            "after_event",
            reactor.after_event.is_some(),
            reactor
                .after_event
                .as_ref()
                .map_or(Span::call_site(), |p| p.name.span()),
        ),
    ] {
        let wanted = required.contains(name);
        if wanted && !present {
            return Err(ktr(
                Span::call_site(),
                "KTR011",
                &format!("policy requires phase `{name}` exactly once, but its block is missing"),
                &format!(
                    "add one `{name} {{ ... }}` block or remove it from `required_phases` only if the application no longer needs that position"
                ),
            ));
        }
        if present && !wanted {
            return Err(ktr(
                span,
                "KTR011",
                &format!("phase `{name}` has a block but is absent from `required_phases`"),
                "add it to the required set or remove the block",
            ));
        }
    }
    Ok(())
}

struct Edge {
    from: usize,
    to: usize,
    span: Span,
    reason: String,
}

fn validate_cycles(reactor: &Reactor, edges: &[Edge], id: &str, kind: &str) -> Result<()> {
    let count = reactor.arms.len();
    let mut adjacency = vec![Vec::new(); count];
    for edge in edges {
        adjacency[edge.from].push(edge.to);
    }
    let mut state = vec![0_u8; count];
    let mut stack = Vec::new();
    if let Some(cycle) =
        (0..count).find_map(|node| dfs_cycle(node, &adjacency, &mut state, &mut stack))
    {
        let names = cycle
            .iter()
            .map(|index| reactor.arms[*index].id.to_string())
            .collect::<Vec<_>>()
            .join(" -> ");
        let closing = cycle[cycle.len() - 2];
        let first = *cycle.last().expect("cycle nonempty");
        let span = edges
            .iter()
            .find(|edge| edge.from == closing && edge.to == first)
            .map_or(reactor.arms[closing].id_span, |edge| edge.span);
        return Err(ktr(
            span,
            id,
            &format!("reactor {kind} cycle: {names}"),
            "remove or reverse one listed relation without deleting an unrelated constraint",
        ));
    }
    Ok(())
}

fn dfs_cycle(
    node: usize,
    adjacency: &[Vec<usize>],
    state: &mut [u8],
    stack: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    match state[node] {
        1 => {
            let start = stack.iter().position(|candidate| *candidate == node)?;
            let mut cycle = stack[start..].to_vec();
            cycle.push(node);
            return Some(cycle);
        }
        2 => return None,
        _ => {}
    }
    state[node] = 1;
    stack.push(node);
    for &next in &adjacency[node] {
        if let Some(cycle) = dfs_cycle(next, adjacency, state, stack) {
            return Some(cycle);
        }
    }
    stack.pop();
    state[node] = 2;
    None
}

fn validate_yield_cycles(reactor: &Reactor, ids: &HashMap<String, usize>) -> Result<()> {
    let edges = reactor
        .arms
        .iter()
        .enumerate()
        .filter_map(|(from, arm)| {
            arm.yields_to.as_ref().map(|relation| Edge {
                from,
                to: ids[&relation.target.to_string()],
                span: relation.span,
                reason: "buffered yield".to_owned(),
            })
        })
        .collect::<Vec<_>>();
    validate_cycles(reactor, &edges, "KTR010", "buffered-yield")
}

fn unknown_relation(owner: &Ident, target: &Ident, span: Span) -> Error {
    ktr(
        span,
        "KTR002",
        &format!("source `{owner}` references unknown source `{target}`"),
        "correct the identifier or declare the target source",
    )
}

fn is_persistent_place(expr: &Expr) -> bool {
    match expr {
        Expr::Path(_) => true,
        Expr::Field(field) => is_persistent_place(&field.base),
        Expr::Paren(paren) => is_persistent_place(&paren.expr),
        _ => false,
    }
}

fn normalize_place(expr: &Expr) -> String {
    match expr {
        Expr::Paren(paren) => normalize_place(&paren.expr),
        Expr::Field(field) => format!(
            "{}.{}",
            normalize_place(&field.base),
            field.member.to_token_stream()
        ),
        Expr::Path(path) => path
            .to_token_stream()
            .to_string()
            .replace(char::is_whitespace, ""),
        _ => expr
            .to_token_stream()
            .to_string()
            .replace(char::is_whitespace, ""),
    }
}

fn expand(reactor: &Reactor, backend: Backend, transfer: Transfer) -> TokenStream2 {
    let kittens = kittens_path();
    let assertions = expand_assertions(reactor, &kittens);
    let initialize = reactor
        .initialize
        .as_ref()
        .map(|phase| expand_phase(phase, &kittens));
    let before_poll = reactor
        .before_poll
        .as_ref()
        .map(|phase| expand_phase(phase, &kittens));
    let after_event = reactor
        .after_event
        .as_ref()
        .map(|phase| expand_phase(phase, &kittens));
    let guards = expand_guards(reactor, &kittens);
    let selection = match transfer {
        Transfer::Event => expand_event_selection(reactor, &kittens, backend),
        Transfer::Slots => expand_slot_selection(reactor, &kittens, backend),
    };
    let handlers = match transfer {
        Transfer::Event => expand_event_handlers(reactor, &kittens),
        Transfer::Slots => expand_slot_handlers(reactor, &kittens),
    };

    quote! {{
        async {
            #assertions
            #initialize
            '__kittens_reactor: loop {
                #before_poll
                #guards
                #selection
                #handlers
                #after_event
            }
        }
        .await
    }}
}

fn expand_assertions(reactor: &Reactor, kittens: &TokenStream2) -> TokenStream2 {
    let assertions = reactor.arms.iter().map(|arm| {
        let source = &arm.source;
        let readiness = match arm.readiness {
            ReadinessKind::MayRemainReady => quote!(#kittens::source::readiness::MayRemainReady),
            ReadinessKind::Quiescent => quote!(#kittens::source::readiness::Quiescent),
        };
        let drain = arm.drain.as_ref().map(|_| {
            quote! {
                #kittens::__private::assert_KTR009_source_is_drainable(&(#source));
            }
        });
        quote! {
            #kittens::__private::assert_SRC001_reactor_source_is_admitted__repair_use_retained_or_channel(&(#source));
            #kittens::__private::assert_KTR006_declared_readiness_matches::<#readiness, _>(&(#source));
            #drain
        }
    });
    let backlog = reactor.arms.iter().filter_map(|arm| {
        arm.yields_to.as_ref().map(|relation| {
            let target = source_for(reactor, &relation.target);
            quote! {
                #kittens::__private::assert_KTR010_yield_target_has_backlog_probe(&(#target));
            }
        })
    });
    quote! {
        if false {
            #(#assertions)*
            #(#backlog)*
        }
    }
}

fn expand_phase(phase: &Phase, kittens: &TokenStream2) -> TokenStream2 {
    let block = &phase.block;
    quote! {
        match #kittens::__private::assert_KTR013_phase_result((async #block).await) {
            core::result::Result::Ok(()) => {}
            core::result::Result::Err(__kittens_error) => {
                return core::result::Result::Err(__kittens_error);
            }
        }
    }
}

fn expand_guards(reactor: &Reactor, kittens: &TokenStream2) -> TokenStream2 {
    let guards = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let guard = format_ident!("__kittens_enabled_{index}");
        let user = arm.when.as_ref().map_or_else(
            || quote!(true),
            |expr| quote!(#kittens::__private::assert_KTR019_guard_result_is_bool(#expr)),
        );
        let yield_probe = arm.yields_to.as_ref().map(|relation| {
            let target = source_for(reactor, &relation.target);
            quote! {
                if #kittens::source::BacklogSource::has_backlog(&(#target)) {
                    false
                } else {
                    true
                }
            }
        });
        if let Some(probe) = yield_probe {
            quote! {
                let #guard: bool = {
                    let __kittens_user_guard = #user;
                    if __kittens_user_guard { #probe } else { false }
                };
            }
        } else {
            quote! {
                let #guard: bool = #user;
            }
        }
    });
    quote!(#(#guards)*)
}

fn expand_event_selection(
    reactor: &Reactor,
    kittens: &TokenStream2,
    backend: Backend,
) -> TokenStream2 {
    let variants = (0..reactor.arms.len())
        .map(|index| format_ident!("Source{index}"))
        .collect::<Vec<_>>();
    let generics = (0..reactor.arms.len())
        .map(|index| format_ident!("T{index}"))
        .collect::<Vec<_>>();
    let poll_arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let guard = format_ident!("__kittens_enabled_{index}");
        let variant = &variants[index];
        let source = &arm.source;
        quote! {
            if #guard {
                match #kittens::source::ReactorSource::poll_next(&mut (#source), __kittens_cx) {
                    core::task::Poll::Ready(__kittens_item) => {
                        return core::task::Poll::Ready(__KittensEvent::#variant(__kittens_item));
                    }
                    core::task::Poll::Pending => {}
                }
            }
        }
    });
    let tokio_arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let guard = format_ident!("__kittens_enabled_{index}");
        let variant = &variants[index];
        let source = &arm.source;
        quote! {
            __kittens_item = core::future::poll_fn(|__kittens_cx| {
                #kittens::source::ReactorSource::poll_next(&mut (#source), __kittens_cx)
            }), if #guard => __KittensEvent::#variant(__kittens_item),
        }
    });

    let selection = match backend {
        Backend::Core => quote! {
            let __kittens_event = core::future::poll_fn(|__kittens_cx| {
                #(#poll_arms)*
                core::task::Poll::Pending
            })
            .await;
        },
        Backend::Tokio => quote! {
            let __kittens_event = #kittens::__private::tokio::select! {
                biased;
                #(#tokio_arms)*
                _ = core::future::pending::<()>() => unreachable!("pending sentinel cannot complete"),
            };
        },
    };

    quote! {
        #[allow(non_camel_case_types)]
        enum __KittensEvent<#(#generics),*> {
            #(#variants(#generics)),*
        }
        #selection
    }
}

fn expand_slot_selection(
    reactor: &Reactor,
    kittens: &TokenStream2,
    backend: Backend,
) -> TokenStream2 {
    let variants = (0..reactor.arms.len())
        .map(|index| format_ident!("Source{index}"))
        .collect::<Vec<_>>();
    let slots = (0..reactor.arms.len())
        .map(|index| format_ident!("__kittens_slot_{index}"))
        .collect::<Vec<_>>();
    let slot_declarations = slots.iter().map(|slot| quote!(let mut #slot = None;));
    let poll_arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let guard = format_ident!("__kittens_enabled_{index}");
        let variant = &variants[index];
        let slot = &slots[index];
        let source = &arm.source;
        quote! {
            if #guard {
                match #kittens::source::ReactorSource::poll_next(&mut (#source), __kittens_cx) {
                    core::task::Poll::Ready(__kittens_item) => {
                        #slot = Some(__kittens_item);
                        return core::task::Poll::Ready(__KittensTag::#variant);
                    }
                    core::task::Poll::Pending => {}
                }
            }
        }
    });
    let tokio_arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let guard = format_ident!("__kittens_enabled_{index}");
        let variant = &variants[index];
        let slot = &slots[index];
        let source = &arm.source;
        quote! {
            __kittens_item = core::future::poll_fn(|__kittens_cx| {
                #kittens::source::ReactorSource::poll_next(&mut (#source), __kittens_cx)
            }), if #guard => {
                #slot = Some(__kittens_item);
                __KittensTag::#variant
            },
        }
    });

    let selection = match backend {
        Backend::Core => quote! {
            let __kittens_tag = core::future::poll_fn(|__kittens_cx| {
                #(#poll_arms)*
                core::task::Poll::Pending
            })
            .await;
        },
        Backend::Tokio => quote! {
            let __kittens_tag = #kittens::__private::tokio::select! {
                biased;
                #(#tokio_arms)*
                _ = core::future::pending::<()>() => unreachable!("pending sentinel cannot complete"),
            };
        },
    };

    quote! {
        enum __KittensTag { #(#variants),* }
        #(#slot_declarations)*
        #selection
    }
}

fn expand_event_handlers(reactor: &Reactor, kittens: &TokenStream2) -> TokenStream2 {
    let arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let variant = format_ident!("Source{index}");
        let body = expand_service_window(reactor, arm, kittens, &quote!(__kittens_selected_item));
        quote! {
            __KittensEvent::#variant(__kittens_selected_item) => { #body }
        }
    });
    quote! {
        match __kittens_event { #(#arms),* }
    }
}

fn expand_slot_handlers(reactor: &Reactor, kittens: &TokenStream2) -> TokenStream2 {
    let arms = reactor.arms.iter().enumerate().map(|(index, arm)| {
        let variant = format_ident!("Source{index}");
        let slot = format_ident!("__kittens_slot_{index}");
        let body = expand_service_window(
            reactor,
            arm,
            kittens,
            &quote!(#slot.take().expect("selected source slot contains its item")),
        );
        quote! {
            __KittensTag::#variant => { #body }
        }
    });
    quote! {
        match __kittens_tag { #(#arms),* }
    }
}

fn expand_service_window(
    reactor: &Reactor,
    arm: &Arm,
    kittens: &TokenStream2,
    initial_item: &TokenStream2,
) -> TokenStream2 {
    let binding = &arm.binding;
    let handler = &arm.handler;
    if arm.is_terminal() {
        return quote! {
            let #binding = #initial_item;
            let __kittens_exit = match #kittens::__private::assert_KTR013_terminal_handler_result(
                (async #handler).await
            ) {
                core::result::Result::Ok(__kittens_exit) => __kittens_exit,
                core::result::Result::Err(__kittens_error) => {
                    return core::result::Result::Err(__kittens_error);
                }
            };
            break '__kittens_reactor core::result::Result::Ok(__kittens_exit);
        };
    }

    if arm.drain.is_none() {
        return quote! {
            let #binding = #initial_item;
            let __kittens_control = match #kittens::__private::assert_KTR013_continuing_handler_result(
                (async #handler).await
            ) {
                core::result::Result::Ok(__kittens_control) => __kittens_control,
                core::result::Result::Err(__kittens_error) => {
                    return core::result::Result::Err(__kittens_error);
                }
            };
            match __kittens_control {
                #kittens::reactor::Control::Continue => {}
                #kittens::reactor::Control::Stop(__kittens_exit) => {
                    break '__kittens_reactor core::result::Result::Ok(__kittens_exit);
                }
            }
        };
    }

    let max = arm.drain.as_ref().expect("checked above").max;
    let source = &arm.source;
    let yield_check = arm.yields_to.as_ref().map(|relation| {
        let target = source_for(reactor, &relation.target);
        quote! {
            if #kittens::source::BacklogSource::has_backlog(&(#target)) {
                break;
            }
        }
    });

    quote! {
        let mut __kittens_item = #initial_item;
        let mut __kittens_handled = 0usize;
        loop {
            __kittens_handled += 1;
            let #binding = __kittens_item;
            let __kittens_control = match #kittens::__private::assert_KTR013_continuing_handler_result(
                (async #handler).await
            ) {
                core::result::Result::Ok(__kittens_control) => __kittens_control,
                core::result::Result::Err(__kittens_error) => {
                    return core::result::Result::Err(__kittens_error);
                }
            };
            match __kittens_control {
                #kittens::reactor::Control::Continue => {}
                #kittens::reactor::Control::Stop(__kittens_exit) => {
                    break '__kittens_reactor core::result::Result::Ok(__kittens_exit);
                }
            }
            if __kittens_handled >= #max {
                break;
            }
            #yield_check
            match #kittens::source::DrainableSource::try_next(&mut (#source)) {
                #kittens::source::TryNext::Item(__kittens_next) => {
                    __kittens_item = __kittens_next;
                }
                #kittens::source::TryNext::Empty | #kittens::source::TryNext::Dormant => break,
            }
        }
    }
}

fn source_for<'a>(reactor: &'a Reactor, id: &Ident) -> &'a Expr {
    &reactor
        .arms
        .iter()
        .find(|arm| arm.id == *id)
        .expect("validated source relation")
        .source
}

fn kittens_path() -> TokenStream2 {
    match crate_name("kittens") {
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Ok(FoundCrate::Itself) | Err(_) => quote!(::kittens),
    }
}

fn ktr(span: Span, id: &str, consequence: &str, repair: &str) -> Error {
    Error::new(span, format!("{id} {consequence}. Repair: {repair}."))
}
