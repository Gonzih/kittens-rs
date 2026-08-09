//! The RLM continuation executor (SPEC Q4/Q5): a pure, suspendable query
//! interpreter over the typed IR.
//!
//! The executor never blocks and never touches IO. Verbs that need
//! transcript data emit a [`StepOutcome::NeedPages`] request; `ask`/
//! `ask-each` emit [`StepOutcome::NeedAsk`]; the driver supplies results
//! through [`Executor::provide_pages`] / [`Executor::provide_ask`] and steps
//! again. Every line binds a typed value to its one-based `%N` slot; an
//! aggregate-meter exhaustion binds an inline `verb_error{cause: budget}`
//! and the script continues (SPEC Q9), while query-level exhaustion
//! terminates the query with a budget error.
//!
//! This is the sans-io heart of the RLM bet; the driver renders records to
//! [`PageRecord`] text and runs the sub-model calls.

use alloc::string::String;
use alloc::vec::Vec;

use kittens_code_protocol::budgets::{BudgetKind, Budgets};
use kittens_code_protocol::error::VerbErrorCause;

use crate::rlm::ir::{BoundValue, By, FinalValue, Instr, Out, Query, Sel};

/// One transcript record rendered to text for the executor (the driver
/// produces these from the store; their shape is opaque to verbs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRecord {
    /// The record's log sequence.
    pub seq: u64,
    /// The record rendered to a single text line.
    pub text: String,
}

/// A bounded page of rendered records with an optional continuation cursor.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Page {
    /// Records in this page, in sequence order.
    pub records: Vec<PageRecord>,
    /// The cursor to resume from, or `None` when the selection is exhausted.
    pub next_cursor: Option<u64>,
}

/// A store page request the driver must satisfy (SPEC S7 read effects).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PageRequest {
    /// The selection whose records are wanted.
    pub sel: Sel,
    /// Resume cursor, `None` for the first page of this selection.
    pub cursor: Option<u64>,
}

/// One sub-model call the driver must run (SPEC Q4 `ask`/`ask-each`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskRequest {
    /// Index within the batch (0 for a plain `ask`; partition index for
    /// `ask-each`), used to rejoin out-of-order results.
    pub index: u32,
    /// The question posed to the sub-model.
    pub question: String,
    /// The rendered selection text supplied as context.
    pub context: String,
    /// Deterministic self-consistency sample count, when set.
    pub sample_k: Option<u8>,
}

/// One resolved sub-model answer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AskResult {
    /// The [`AskRequest::index`] this answers.
    pub index: u32,
    /// The sub-model's answer text (the executor caps it).
    pub answer: String,
}

/// A typed value bound to a `%N` slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Bound {
    /// A selection of rendered records.
    Records(Vec<PageRecord>),
    /// A partition into chunks of records.
    Chunks(Vec<Vec<PageRecord>>),
    /// An integer count.
    Count(u64),
    /// A capped sub-model digest.
    Digest(String),
    /// Capped digests, ordered by partition index.
    DigestList(Vec<String>),
    /// An inline verb error (script continues; SPEC Q9).
    Error(VerbErrorCause),
}

impl Bound {
    /// The declared output kind, or `None` for an error binding.
    #[must_use]
    fn out(&self) -> Option<Out> {
        match self {
            Self::Records(_) => Some(Out::Records),
            Self::Chunks(_) => Some(Out::Chunks),
            Self::Count(_) => Some(Out::Count),
            Self::Digest(_) => Some(Out::Digest),
            Self::DigestList(_) => Some(Out::DigestList),
            Self::Error(_) => None,
        }
    }
}

/// What one [`Executor::step`] produced.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StepOutcome {
    /// The current line needs a store page before it can proceed.
    NeedPages(PageRequest),
    /// The current line needs these sub-model answers (bounded batch).
    NeedAsk(Vec<AskRequest>),
    /// A line completed; its value is bound to `slot` (one-based).
    Line {
        /// The one-based slot the value was bound to.
        slot: u32,
        /// The bound value (may be an inline error).
        bound: Bound,
    },
    /// The query terminated; `answer` is the final answer.
    Done {
        /// The final answer.
        answer: String,
    },
    /// A partial `ask-each` batch was accepted; more results are still
    /// outstanding. The driver keeps calling [`Executor::provide_ask`] with
    /// the remaining answers — no new requests are dispatched. Distinct from
    /// an empty [`StepOutcome::NeedAsk`], which would ambiguously read as
    /// "dispatch nothing."
    AwaitingMore,
    /// The query terminated on a query-level budget or structural error.
    Failed {
        /// Which budget or condition ended it.
        cause: VerbErrorCause,
    },
}

/// The Q5 meter set for one query, all clamped to `Budgets`.
#[derive(Clone, Debug)]
struct Meters {
    budgets: Budgets,
    verbs: u16,
    subcalls: u16,
    partitions: u16,
    selected_bytes: u32,
    scanned_pages: u32,
    scanned_bytes: u64,
    page_effects: u32,
}

impl Meters {
    fn new(budgets: Budgets) -> Self {
        Self {
            budgets,
            verbs: 0,
            subcalls: 0,
            partitions: 0,
            selected_bytes: 0,
            scanned_pages: 0,
            scanned_bytes: 0,
            page_effects: 0,
        }
    }

    /// Charges an aggregate meter, returning `Err` when it would exceed its
    /// budget (the caller binds an inline `verb_error{cause: budget}`).
    ///
    /// Charges saturate rather than wrap: a charge past a counter's width is
    /// pinned to its maximum, which can only push a meter further over its
    /// (smaller) budget — never falsely under it.
    fn charge(&mut self, kind: BudgetKind, amount: u64) -> Result<(), ()> {
        let amount32 = u32::try_from(amount).unwrap_or(u32::MAX);
        let amount16 = u16::try_from(amount).unwrap_or(u16::MAX);
        match kind {
            BudgetKind::ScannedPages => {
                self.scanned_pages = self.scanned_pages.saturating_add(amount32);
                over(
                    u64::from(self.scanned_pages),
                    u64::from(self.budgets.scanned_pages),
                )
            }
            BudgetKind::ScannedBytes => {
                self.scanned_bytes = self.scanned_bytes.saturating_add(amount);
                over(self.scanned_bytes, self.budgets.scanned_bytes)
            }
            BudgetKind::PageEffects => {
                self.page_effects = self.page_effects.saturating_add(amount32);
                over(
                    u64::from(self.page_effects),
                    u64::from(self.budgets.page_effects),
                )
            }
            BudgetKind::SelectedBytes => {
                self.selected_bytes = self.selected_bytes.saturating_add(amount32);
                over(
                    u64::from(self.selected_bytes),
                    u64::from(self.budgets.selected_bytes),
                )
            }
            BudgetKind::PartitionCount => {
                self.partitions = self.partitions.saturating_add(amount16);
                over(
                    u64::from(self.partitions),
                    u64::from(self.budgets.partition_count),
                )
            }
            BudgetKind::TotalSubcalls => {
                self.subcalls = self.subcalls.saturating_add(amount16);
                over(
                    u64::from(self.subcalls),
                    u64::from(self.budgets.total_subcalls),
                )
            }
            _ => Ok(()),
        }
    }
}

fn over(used: u64, limit: u64) -> Result<(), ()> {
    if used > limit { Err(()) } else { Ok(()) }
}

/// One-based `%N` slot for a zero-based program counter (queries cannot be
/// longer than `u32::MAX` lines under the verb-count budget).
fn slot_of(pc: usize) -> u32 {
    u32::try_from(pc + 1).unwrap_or(u32::MAX)
}

/// Zero-based slot index for a one-based `%N` line, or `None` for the
/// invalid `%0` reference (review input 19 #17).
fn slot_index(line: u32) -> Option<usize> {
    (line != 0).then(|| (line - 1) as usize)
}

/// Truncates text to a byte cap on a UTF-8 boundary (value-cap law:
/// truncate, never error).
fn cap(text: &str, limit: u32) -> String {
    let limit = limit as usize;
    if text.len() <= limit {
        return String::from(text);
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    String::from(&text[..end])
}

/// Per-line execution progress for verbs that stream store pages.
#[derive(Clone, Debug)]
struct PageWalk {
    sel: Sel,
    cursor: Option<u64>,
    collected: Vec<PageRecord>,
    started: bool,
}

/// The pending `ask-each` join state.
#[derive(Clone, Debug)]
struct AskJoin {
    slots: Vec<Option<String>>,
    outstanding: usize,
}

/// What the current line is waiting on between steps.
#[derive(Clone, Debug)]
enum Pending {
    /// Nothing; ready to execute the next instruction.
    Idle,
    /// Walking store pages for the instruction at `pc`.
    Pages(PageWalk),
    /// Awaiting a single `ask` answer.
    Ask,
    /// Awaiting `ask-each` answers, rejoining by index.
    AskEach(AskJoin),
}

/// The suspendable query executor (SPEC Q4 `QueryCont`).
pub struct Executor {
    query: Query,
    pc: usize,
    slots: Vec<Option<Bound>>,
    meters: Meters,
    pending: Pending,
    finished: bool,
}

impl Executor {
    /// Builds an executor for a lowered query under a budget set.
    ///
    /// Slots for lines the lowerer already flagged as inline errors are
    /// pre-bound (SPEC Q9: a bad line still occupies its `%N`).
    #[must_use]
    pub fn new(query: Query, budgets: Budgets) -> Self {
        let len = query.len();
        let mut slots = alloc::vec![None; len];
        for (i, binding) in query.iter().enumerate() {
            if let BoundValue::Error(err) = &binding.value {
                slots[i] = Some(Bound::Error(err.cause));
            }
        }
        Self {
            query,
            pc: 0,
            slots,
            meters: Meters::new(budgets),
            pending: Pending::Idle,
            finished: false,
        }
    }

    /// The instruction at the current program counter, or `None` when the
    /// current binding is a pre-lowered error to skip.
    fn current_instr(&self) -> Option<Instr> {
        match &self.query.get(self.pc)?.value {
            BoundValue::Instr(instr) => Some(instr.clone()),
            BoundValue::Error(_) => None,
        }
    }

    /// The store pages or ask results currently outstanding, for the engine
    /// to `CancelEffect` on discard (SPEC Q4 termination/discard).
    #[must_use]
    pub fn is_suspended(&self) -> bool {
        !matches!(self.pending, Pending::Idle)
    }

    /// Advances the query until it needs input, produces a line binding, or
    /// terminates. Call repeatedly, supplying inputs between calls.
    pub fn step(&mut self) -> StepOutcome {
        if self.finished || self.pc >= self.query.len() {
            self.finished = true;
            return StepOutcome::Done {
                answer: String::new(),
            };
        }
        if !matches!(self.pending, Pending::Idle) {
            // Stepping while suspended is a driver contract error; results
            // arrive through provide_pages / provide_ask instead.
            return StepOutcome::Failed {
                cause: VerbErrorCause::Parse,
            };
        }
        if let Some(instr) = self.current_instr() {
            self.begin_instruction(&instr)
        } else {
            // A pre-lowered error binding: re-surface it and advance so the
            // caller sees every line, then keep going (Q9 continue).
            let slot = slot_of(self.pc);
            let bound = self.slots[self.pc]
                .clone()
                .unwrap_or(Bound::Error(VerbErrorCause::Parse));
            self.pc += 1;
            StepOutcome::Line { slot, bound }
        }
    }

    /// Supplies a store page for the suspended page-walking line.
    ///
    /// Returns the next step outcome: another page request, a completed line
    /// binding, or (for a subsequent instruction) whatever that yields.
    pub fn provide_pages(&mut self, page: Page) -> StepOutcome {
        let Pending::Pages(mut walk) = core::mem::replace(&mut self.pending, Pending::Idle) else {
            return StepOutcome::Failed {
                cause: VerbErrorCause::Parse,
            };
        };
        walk.started = true;
        // Charge the page effect and the scanned volume.
        let scanned: u64 = page.records.iter().map(|r| r.text.len() as u64).sum();
        if self.meters.charge(BudgetKind::PageEffects, 1).is_err()
            || self.meters.charge(BudgetKind::ScannedPages, 1).is_err()
            || self
                .meters
                .charge(BudgetKind::ScannedBytes, scanned)
                .is_err()
        {
            return self.bind_error(VerbErrorCause::Budget);
        }
        walk.collected.extend(page.records);
        if let Some(cursor) = page.next_cursor {
            walk.cursor = Some(cursor);
            let request = PageRequest {
                sel: walk.sel.clone(),
                cursor: walk.cursor,
            };
            self.pending = Pending::Pages(walk);
            return StepOutcome::NeedPages(request);
        }
        // Selection exhausted: finish this instruction with its records.
        self.finish_page_instruction(walk.collected)
    }

    /// Supplies one or more `ask` results for the suspended ask line.
    pub fn provide_ask(&mut self, results: Vec<AskResult>) -> StepOutcome {
        match core::mem::replace(&mut self.pending, Pending::Idle) {
            Pending::Ask => {
                let answer = results
                    .into_iter()
                    .next()
                    .map(|r| cap(&r.answer, self.meters.budgets.ask_digest_bytes))
                    .unwrap_or_default();
                self.bind(Bound::Digest(answer))
            }
            Pending::AskEach(mut join) => {
                for result in results {
                    if let Some(slot) = join.slots.get_mut(result.index as usize) {
                        if slot.is_none() {
                            *slot = Some(cap(&result.answer, self.meters.budgets.ask_digest_bytes));
                            join.outstanding = join.outstanding.saturating_sub(1);
                        }
                    }
                }
                if join.outstanding == 0 {
                    let digests = join
                        .slots
                        .into_iter()
                        .map(Option::unwrap_or_default)
                        .collect();
                    self.bind(Bound::DigestList(digests))
                } else {
                    self.pending = Pending::AskEach(join);
                    StepOutcome::AwaitingMore
                }
            }
            _ => StepOutcome::Failed {
                cause: VerbErrorCause::Parse,
            },
        }
    }

    fn begin_instruction(&mut self, instr: &Instr) -> StepOutcome {
        // Verb-count is a query-level meter (SPEC Q5): exceeding it
        // terminates the whole query rather than binding an inline error.
        self.meters.verbs = self.meters.verbs.saturating_add(1);
        if self.meters.verbs > self.meters.budgets.verb_count {
            return StepOutcome::Failed {
                cause: VerbErrorCause::Budget,
            };
        }
        match instr {
            // Every record-reading and partitioning verb begins by walking
            // the store pages for its selection; the transform is applied
            // once the walk finishes (`finish_page_instruction`).
            Instr::Grep { sel, .. }
            | Instr::Slice { sel }
            | Instr::Head { sel, .. }
            | Instr::Tail { sel, .. }
            | Instr::Count { sel, .. }
            | Instr::Partition { sel, .. } => self.start_page_walk(sel.clone()),
            Instr::Ask {
                sel,
                question,
                sample_k,
            } => {
                if self.meters.charge(BudgetKind::TotalSubcalls, 1).is_err() {
                    return self.bind_error(VerbErrorCause::Budget);
                }
                let context = self.render_sel(sel);
                if self
                    .meters
                    .charge(BudgetKind::SelectedBytes, context.len() as u64)
                    .is_err()
                {
                    return self.bind_error(VerbErrorCause::Budget);
                }
                self.pending = Pending::Ask;
                StepOutcome::NeedAsk(alloc::vec![AskRequest {
                    index: 0,
                    question: question.clone(),
                    context,
                    sample_k: *sample_k,
                }])
            }
            Instr::AskEach { chunks, question } => self.begin_ask_each(chunks.line(), question),
            Instr::Final { value } => self.finish(value),
        }
    }

    fn start_page_walk(&mut self, sel: Sel) -> StepOutcome {
        let request = PageRequest {
            sel: sel.clone(),
            cursor: None,
        };
        self.pending = Pending::Pages(PageWalk {
            sel,
            cursor: None,
            collected: Vec::new(),
            started: false,
        });
        StepOutcome::NeedPages(request)
    }

    fn finish_page_instruction(&mut self, records: Vec<PageRecord>) -> StepOutcome {
        // The instruction at `pc` is one of the record verbs (we only walk
        // pages for a real instruction), so `current_instr` is Some.
        let Some(instr) = self.current_instr() else {
            return self.bind_error(VerbErrorCause::Parse);
        };
        let bound = match instr {
            Instr::Slice { .. } => Bound::Records(records),
            Instr::Head { n, .. } => Bound::Records(records.into_iter().take(n as usize).collect()),
            Instr::Tail { n, .. } => {
                let skip = records.len().saturating_sub(n as usize);
                Bound::Records(records.into_iter().skip(skip).collect())
            }
            Instr::Grep { pattern, ctx, .. } => Bound::Records(grep(&records, &pattern, ctx)),
            Instr::Count { pattern, .. } => {
                let count = match pattern {
                    Some(p) => records.iter().filter(|r| r.text.contains(&p)).count(),
                    None => records.len(),
                };
                Bound::Count(count as u64)
            }
            Instr::Partition {
                by, size, pattern, ..
            } => {
                return self.finish_partition(&records, by, size, pattern);
            }
            _ => Bound::Error(VerbErrorCause::Parse),
        };
        self.bind(bound)
    }

    fn finish_partition(
        &mut self,
        records: &[PageRecord],
        by: By,
        size: Option<u32>,
        pattern: Option<String>,
    ) -> StepOutcome {
        let chunks = partition(records, by, size, pattern);
        if self
            .meters
            .charge(BudgetKind::PartitionCount, chunks.len() as u64)
            .is_err()
        {
            return self.bind_error(VerbErrorCause::Budget);
        }
        self.bind(Bound::Chunks(chunks))
    }

    fn begin_ask_each(&mut self, chunks_line: u32, question: &str) -> StepOutcome {
        // `Ref(0)` is invalid; a zero line has no slot (review input 19 #17).
        let Some(index) = slot_index(chunks_line) else {
            return self.bind_error(VerbErrorCause::BadRef);
        };
        let Some(Some(Bound::Chunks(chunks))) = self.slots.get(index) else {
            return self.bind_error(VerbErrorCause::BadRef);
        };
        let chunks = chunks.clone();
        // An empty partition resolves immediately to an empty digest list.
        if chunks.is_empty() {
            return self.bind(Bound::DigestList(Vec::new()));
        }
        if self
            .meters
            .charge(BudgetKind::TotalSubcalls, chunks.len() as u64)
            .is_err()
        {
            return self.bind_error(VerbErrorCause::Budget);
        }
        let mut requests = Vec::with_capacity(chunks.len());
        for (index, chunk) in chunks.iter().enumerate() {
            let context = render_records(chunk);
            // Selected-bytes exhaustion aborts the whole ask-each with a
            // budget error rather than silently over-spending (input 19 #11).
            if self
                .meters
                .charge(BudgetKind::SelectedBytes, context.len() as u64)
                .is_err()
            {
                return self.bind_error(VerbErrorCause::Budget);
            }
            requests.push(AskRequest {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                question: String::from(question),
                context,
                sample_k: None,
            });
        }
        self.pending = Pending::AskEach(AskJoin {
            slots: alloc::vec![None; chunks.len()],
            outstanding: chunks.len(),
        });
        StepOutcome::NeedAsk(requests)
    }

    fn finish(&mut self, value: &FinalValue) -> StepOutcome {
        self.finished = true;
        match value {
            FinalValue::Literal(text) => StepOutcome::Done {
                answer: text.clone(),
            },
            FinalValue::Ref(r) => {
                let answer = slot_index(r.line())
                    .and_then(|i| self.slots.get(i))
                    .and_then(Option::as_ref)
                    .map(render_bound)
                    .unwrap_or_default();
                StepOutcome::Done { answer }
            }
        }
    }

    /// Binds a value to the current slot and advances the program counter.
    fn bind(&mut self, bound: Bound) -> StepOutcome {
        let slot = slot_of(self.pc);
        let capped = self.cap_bound(bound);
        self.slots[self.pc] = Some(capped.clone());
        self.pc += 1;
        self.pending = Pending::Idle;
        StepOutcome::Line {
            slot,
            bound: capped,
        }
    }

    fn bind_error(&mut self, cause: VerbErrorCause) -> StepOutcome {
        self.bind(Bound::Error(cause))
    }

    /// Applies value caps to a binding (truncate, never error).
    fn cap_bound(&self, bound: Bound) -> Bound {
        match bound {
            Bound::Digest(text) => Bound::Digest(cap(&text, self.meters.budgets.ask_digest_bytes)),
            Bound::DigestList(list) => Bound::DigestList(
                list.into_iter()
                    .map(|d| cap(&d, self.meters.budgets.ask_digest_bytes))
                    .collect(),
            ),
            other => other,
        }
    }

    fn render_sel(&self, sel: &Sel) -> String {
        match sel {
            Sel::Ref(r) => slot_index(r.line())
                .and_then(|i| self.slots.get(i))
                .and_then(Option::as_ref)
                .map(render_bound)
                .unwrap_or_default(),
            // Range/Whole context is materialized by the driver via pages;
            // for `ask` over a raw range the executor renders empty context
            // and relies on the driver having pre-supplied it as a Ref.
            _ => String::new(),
        }
    }

    /// Whether a reference names a slot with the required output kind
    /// (used by tests and, later, richer validation).
    #[must_use]
    pub fn slot_out(&self, line: u32) -> Option<Out> {
        slot_index(line)
            .and_then(|i| self.slots.get(i))
            .and_then(Option::as_ref)
            .and_then(Bound::out)
    }
}

/// Selects records matching `pattern`, keeping `ctx` records of context on
/// each side of every hit.
///
/// KC0 matches by literal substring: the pinned `no_std` regex dialect (Q6,
/// decision D7) is not yet available in core, and the std driver will supply
/// the real Q6 engine through the search port (review input 19 #13). Until
/// then callers get literal semantics, which is a subset of the eventual
/// dialect and never over-matches.
fn grep(records: &[PageRecord], pattern: &str, ctx: u16) -> Vec<PageRecord> {
    let ctx = ctx as usize;
    let mut keep = alloc::vec![false; records.len()];
    for (i, r) in records.iter().enumerate() {
        if r.text.contains(pattern) {
            let lo = i.saturating_sub(ctx);
            let hi = (i + ctx + 1).min(records.len());
            for slot in keep.iter_mut().take(hi).skip(lo) {
                *slot = true;
            }
        }
    }
    records
        .iter()
        .zip(keep)
        .filter(|&(_, k)| k)
        .map(|(r, _)| r.clone())
        .collect()
}

fn partition(
    records: &[PageRecord],
    by: By,
    size: Option<u32>,
    pattern: Option<String>,
) -> Vec<Vec<PageRecord>> {
    match by {
        // `--by=turns --size=N`: N records per chunk. Distinguishing
        // user-turn *boundaries* would require the driver to tag records
        // with their originating turn; that record-rendering contract is
        // deferred with D7, so KC0's turn partition is record-count based
        // and documented as such (review input 19 #13, partial).
        By::Turns => {
            let n = size.unwrap_or(1).max(1) as usize;
            records.chunks(n).map(<[PageRecord]>::to_vec).collect()
        }
        // `--by=bytes --size=N`: fill each chunk until its rendered byte
        // total would exceed N, then start a new chunk. A single record
        // larger than N occupies its own chunk (never dropped).
        By::Bytes => {
            let budget = size.unwrap_or(1).max(1) as usize;
            let mut chunks: Vec<Vec<PageRecord>> = Vec::new();
            let mut current: Vec<PageRecord> = Vec::new();
            let mut used = 0usize;
            for r in records {
                let len = r.text.len();
                if !current.is_empty() && used + len > budget {
                    chunks.push(core::mem::take(&mut current));
                    used = 0;
                }
                used += len;
                current.push(r.clone());
            }
            if !current.is_empty() {
                chunks.push(current);
            }
            chunks
        }
        // `--by=regex "pat"`: start a new chunk at each record matching the
        // separator. KC0 uses the literal-substring fallback pending the
        // pinned no_std regex dialect (D7 / review input 19 #13); the std
        // driver will supply the real Q6 engine through the search port.
        By::Regex => {
            let sep = pattern.unwrap_or_default();
            let mut chunks = Vec::new();
            let mut current = Vec::new();
            for r in records {
                if !sep.is_empty() && r.text.contains(&sep) && !current.is_empty() {
                    chunks.push(core::mem::take(&mut current));
                }
                current.push(r.clone());
            }
            if !current.is_empty() {
                chunks.push(current);
            }
            chunks
        }
    }
}

fn render_records(records: &[PageRecord]) -> String {
    let mut out = String::new();
    for r in records {
        out.push_str(&r.text);
        out.push('\n');
    }
    out
}

fn render_bound(bound: &Bound) -> String {
    match bound {
        Bound::Records(records) => render_records(records),
        Bound::Chunks(chunks) => {
            let mut out = String::new();
            for chunk in chunks {
                out.push_str(&render_records(chunk));
            }
            out
        }
        Bound::Count(n) => {
            let mut s = String::new();
            let mut buf = *n;
            if buf == 0 {
                s.push('0');
            } else {
                let mut digits = Vec::new();
                while buf > 0 {
                    digits.push(b'0' + (buf % 10) as u8);
                    buf /= 10;
                }
                digits.reverse();
                s.push_str(core::str::from_utf8(&digits).unwrap_or("0"));
            }
            s
        }
        Bound::Digest(d) => d.clone(),
        Bound::DigestList(list) => list.join("\n"),
        Bound::Error(_) => String::new(),
    }
}
