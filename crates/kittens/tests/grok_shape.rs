#![allow(
    clippy::ignored_unit_patterns,
    clippy::struct_excessive_bools,
    clippy::too_many_lines,
    dead_code,
    missing_docs
)]

use kittens::reactor::Control;
use kittens::source::{self, FixedQueue, Latched, ReactorSource, close};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoopExit {
    Disconnected,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FixtureError;

struct GrokSources {
    connection_cancel: Latched<()>,
    graceful_quit: Latched<()>,
    writer_event: FixedQueue<u8, 4>,
    acp_stream: FixedQueue<u8, 64>,
    task_completion: FixedQueue<u8, 4>,
    restore_progress: FixedQueue<u8, 4>,
    background_update: Latched<u8>,
    terminal_input: FixedQueue<u8, 16>,
    resize_deadline: Latched<()>,
    deferred_render: Latched<()>,
    suspend_retry: Latched<()>,
    scroll_clock: Latched<()>,
    animation_tick: Latched<()>,
    billing_poll: Latched<()>,
    access_gate_poll: Latched<()>,
    subscription_watch: Latched<u8>,
    roster_poll: Latched<u8>,
    away_recap_poll: Latched<u8>,
    config_watch: FixedQueue<u8, 4>,
    appearance_watch: Latched<u8>,
    leader_status: Latched<u8>,
    reconnect_reinit: source::OptionalOneShot<std::future::Ready<u8>>,
    voice_stt: source::OptionalMpsc<u8, close::Dormant>,
}

impl GrokSources {
    fn new() -> Self {
        Self {
            connection_cancel: Latched::new(),
            graceful_quit: Latched::new(),
            writer_event: FixedQueue::new(),
            acp_stream: FixedQueue::new(),
            task_completion: FixedQueue::new(),
            restore_progress: FixedQueue::new(),
            background_update: Latched::new(),
            terminal_input: FixedQueue::new(),
            resize_deadline: Latched::new(),
            deferred_render: Latched::new(),
            suspend_retry: Latched::new(),
            scroll_clock: Latched::new(),
            animation_tick: Latched::new(),
            billing_poll: Latched::new(),
            access_gate_poll: Latched::new(),
            subscription_watch: Latched::new(),
            roster_poll: Latched::new(),
            away_recap_poll: Latched::new(),
            config_watch: FixedQueue::new(),
            appearance_watch: Latched::new(),
            leader_status: Latched::new(),
            reconnect_reinit: source::OptionalOneShot::new(),
            voice_stt: source::OptionalMpsc::new(close::Dormant),
        }
    }
}

#[derive(Default)]
struct App {
    accepts_acp: bool,
    resize_due: bool,
    initialized: bool,
    before_count: usize,
    after_count: usize,
    handled: usize,
    saw_exhausted_budget: bool,
}

impl App {
    async fn before(&mut self, sources: &mut GrokSources) -> Result<(), FixtureError> {
        self.before_count += 1;
        tokio::task::yield_now().await;
        if self.resize_due && sources.resize_deadline.is_dormant() {
            sources.resize_deadline.arm(()).map_err(|_| FixtureError)?;
            self.resize_due = false;
        }
        Ok(())
    }

    async fn handle(&mut self, _value: u8) -> Result<(), FixtureError> {
        self.handled += 1;
        self.saw_exhausted_budget |= !tokio::task::coop::has_budget_remaining();
        tokio::task::yield_now().await;
        Ok(())
    }
}

macro_rules! define_grok_runner {
    ($name:ident, $reactor:ident) => {
        impl App {
            async fn $name(&mut self, sources: &mut GrokSources) -> Result<LoopExit, FixtureError> {
                kittens::__private::$reactor! {
                    policy {
                        selection: biased;
                        required_phases: [initialize, before_poll, after_event];
                    }

                    initialize {
                        self.initialized = true;
                        Ok(())
                    }

                    before_poll {
                        self.before(sources).await?;
                        Ok(())
                    }

                    #[source(connection_cancel)]
                    #[readiness(quiescent)]
                    #[shutdown]
                    _ = sources.connection_cancel => {
                        Ok(LoopExit::Disconnected)
                    }

                    #[source(graceful_quit)]
                    #[readiness(quiescent)]
                    #[shutdown]
                    _ = sources.graceful_quit => {
                        Ok(LoopExit::Quit)
                    }

                    #[source(writer_event)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "writer events may wait behind shutdown")]
                    #[yields_to(terminal_input, when = buffered)]
                    value = sources.writer_event => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(acp_stream)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "model streaming may wait behind control")]
                    #[when(self.accepts_acp)]
                    #[yields_to(terminal_input, when = buffered)]
                    #[drain(max = 32)]
                    #[before(task_completion)]
                    value = sources.acp_stream => {
                        self.handle(value).await?;
                        if value == 32 {
                            sources.graceful_quit.arm(()).map_err(|_| FixtureError)?;
                        }
                        Ok(Control::Continue)
                    }

                    #[source(task_completion)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "task completions may wait behind model output")]
                    #[yields_to(terminal_input, when = buffered)]
                    value = sources.task_completion => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(restore_progress)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "restore progress is informational")]
                    #[yields_to(terminal_input, when = buffered)]
                    value = sources.restore_progress => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(background_update)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "background update completion is best effort")]
                    value = sources.background_update => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(terminal_input)]
                    #[readiness(may_remain_ready)]
                    value = sources.terminal_input => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(resize_deadline)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "resize debounce is best effort")]
                    _ = sources.resize_deadline => {
                        Ok(Control::Continue)
                    }

                    #[source(deferred_render)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "frame throttling deliberately delays work")]
                    _ = sources.deferred_render => {
                        Ok(Control::Continue)
                    }

                    #[source(suspend_retry)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "terminal suspend retry is best effort")]
                    _ = sources.suspend_retry => {
                        Ok(Control::Continue)
                    }

                    #[source(scroll_clock)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "scroll animation is best effort")]
                    _ = sources.scroll_clock => {
                        Ok(Control::Continue)
                    }

                    #[source(animation_tick)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "animation recovery is periodic")]
                    _ = sources.animation_tick => {
                        Ok(Control::Continue)
                    }

                    #[source(billing_poll)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "billing refresh is periodic")]
                    _ = sources.billing_poll => {
                        Ok(Control::Continue)
                    }

                    #[source(access_gate_poll)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "access gate refresh is periodic")]
                    _ = sources.access_gate_poll => {
                        Ok(Control::Continue)
                    }

                    #[source(subscription_watch)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "subscription refresh is background work")]
                    value = sources.subscription_watch => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(roster_poll)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "roster refresh is dashboard only")]
                    value = sources.roster_poll => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(away_recap_poll)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "away recap refresh is periodic")]
                    value = sources.away_recap_poll => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(config_watch)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "configuration reload is background work")]
                    value = sources.config_watch => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(appearance_watch)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "appearance changes are background work")]
                    value = sources.appearance_watch => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(leader_status)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "leader status is informational")]
                    value = sources.leader_status => {
                        self.handle(value).await?;
                        Ok(Control::Continue)
                    }

                    #[source(reconnect_reinit)]
                    #[readiness(quiescent)]
                    #[starvation(allowed, reason = "reconnect completion is generation checked")]
                    value = sources.reconnect_reinit => {
                        self.handle(value).await?;
                        sources.graceful_quit.arm(()).map_err(|_| FixtureError)?;
                        Ok(Control::Continue)
                    }

                    #[source(voice_stt)]
                    #[readiness(may_remain_ready)]
                    #[starvation(allowed, reason = "interim voice transcripts are best effort")]
                    #[last]
                    value = sources.voice_stt => {
                        self.handle(value).await?;
                        sources.graceful_quit.arm(()).map_err(|_| FixtureError)?;
                        Ok(Control::Continue)
                    }

                    after_event {
                        self.after_count += 1;
                        Ok(())
                    }
                }
            }
        }
    };
}

define_grok_runner!(run_core_event, reactor_event);
define_grok_runner!(run_core_slots, reactor_slots);
define_grok_runner!(run_tokio_control, reactor_tokio_event);
define_grok_runner!(run_tokio_slots, reactor_tokio_slots);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RawSourceId {
    ConnectionCancel,
    GracefulQuit,
    WriterEvent,
    AcpStream,
    TaskCompletion,
    RestoreProgress,
    BackgroundUpdate,
    TerminalInput,
    ResizeDeadline,
    DeferredRender,
    SuspendRetry,
    ScrollClock,
    AnimationTick,
    BillingPoll,
    AccessGatePoll,
    SubscriptionWatch,
    RosterPoll,
    AwayRecapPoll,
    ConfigWatch,
    AppearanceWatch,
    LeaderStatus,
    ReconnectReinit,
    VoiceStt,
}

async fn raw_tokio_select_once(sources: &mut GrokSources) -> RawSourceId {
    tokio::select! {
        biased;
        _ = core::future::poll_fn(|cx| sources.connection_cancel.poll_next(cx)) => RawSourceId::ConnectionCancel,
        _ = core::future::poll_fn(|cx| sources.graceful_quit.poll_next(cx)) => RawSourceId::GracefulQuit,
        _ = core::future::poll_fn(|cx| sources.writer_event.poll_next(cx)) => RawSourceId::WriterEvent,
        _ = core::future::poll_fn(|cx| sources.acp_stream.poll_next(cx)) => RawSourceId::AcpStream,
        _ = core::future::poll_fn(|cx| sources.task_completion.poll_next(cx)) => RawSourceId::TaskCompletion,
        _ = core::future::poll_fn(|cx| sources.restore_progress.poll_next(cx)) => RawSourceId::RestoreProgress,
        _ = core::future::poll_fn(|cx| sources.background_update.poll_next(cx)) => RawSourceId::BackgroundUpdate,
        _ = core::future::poll_fn(|cx| sources.terminal_input.poll_next(cx)) => RawSourceId::TerminalInput,
        _ = core::future::poll_fn(|cx| sources.resize_deadline.poll_next(cx)) => RawSourceId::ResizeDeadline,
        _ = core::future::poll_fn(|cx| sources.deferred_render.poll_next(cx)) => RawSourceId::DeferredRender,
        _ = core::future::poll_fn(|cx| sources.suspend_retry.poll_next(cx)) => RawSourceId::SuspendRetry,
        _ = core::future::poll_fn(|cx| sources.scroll_clock.poll_next(cx)) => RawSourceId::ScrollClock,
        _ = core::future::poll_fn(|cx| sources.animation_tick.poll_next(cx)) => RawSourceId::AnimationTick,
        _ = core::future::poll_fn(|cx| sources.billing_poll.poll_next(cx)) => RawSourceId::BillingPoll,
        _ = core::future::poll_fn(|cx| sources.access_gate_poll.poll_next(cx)) => RawSourceId::AccessGatePoll,
        _ = core::future::poll_fn(|cx| sources.subscription_watch.poll_next(cx)) => RawSourceId::SubscriptionWatch,
        _ = core::future::poll_fn(|cx| sources.roster_poll.poll_next(cx)) => RawSourceId::RosterPoll,
        _ = core::future::poll_fn(|cx| sources.away_recap_poll.poll_next(cx)) => RawSourceId::AwayRecapPoll,
        _ = core::future::poll_fn(|cx| sources.config_watch.poll_next(cx)) => RawSourceId::ConfigWatch,
        _ = core::future::poll_fn(|cx| sources.appearance_watch.poll_next(cx)) => RawSourceId::AppearanceWatch,
        _ = core::future::poll_fn(|cx| sources.leader_status.poll_next(cx)) => RawSourceId::LeaderStatus,
        _ = core::future::poll_fn(|cx| sources.reconnect_reinit.poll_next(cx)) => RawSourceId::ReconnectReinit,
        _ = core::future::poll_fn(|cx| sources.voice_stt.poll_next(cx)) => RawSourceId::VoiceStt,
    }
}

#[derive(Clone, Copy, Debug)]
enum PresenterCommand {
    Request,
    Draw(u8),
    Ack(u64),
    Deadline,
    Finish,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct Presenter {
    pending: bool,
    in_flight: Option<u64>,
    next_sequence: u64,
    emitted_payloads: usize,
    coalesced_requests: usize,
    blocked_draws: usize,
    no_payload_draws: usize,
    stale_acks: usize,
    deadline_armed: bool,
    deadline_fires: usize,
}

impl Presenter {
    fn request(&mut self) {
        if self.pending {
            self.coalesced_requests += 1;
        }
        self.pending = true;
    }

    fn draw(&mut self, payloads: u8) {
        if !self.pending {
            return;
        }
        if self.in_flight.is_some() {
            self.blocked_draws += 1;
            return;
        }

        self.pending = false;
        if payloads == 0 {
            self.no_payload_draws += 1;
            return;
        }

        self.next_sequence += u64::from(payloads);
        self.emitted_payloads += usize::from(payloads);
        self.in_flight = Some(self.next_sequence);
        self.deadline_armed = true;
    }

    fn acknowledge(&mut self, sequence: u64) {
        match self.in_flight {
            Some(target) if sequence >= target => {
                self.in_flight = None;
                self.deadline_armed = false;
            }
            Some(_) => self.stale_acks += 1,
            None => {}
        }
    }

    fn fire_deadline(&mut self) {
        if self.deadline_armed {
            self.deadline_fires += 1;
        }
    }

    fn apply(&mut self, command: PresenterCommand) -> bool {
        match command {
            PresenterCommand::Request => self.request(),
            PresenterCommand::Draw(payloads) => self.draw(payloads),
            PresenterCommand::Ack(sequence) => self.acknowledge(sequence),
            PresenterCommand::Deadline => self.fire_deadline(),
            PresenterCommand::Finish => return true,
        }
        false
    }

    fn initial_presentation(&mut self) {
        self.request();
        self.draw(1);
    }
}

fn presenter_commands() -> FixedQueue<PresenterCommand, 16> {
    let mut commands = FixedQueue::new();
    for command in [
        PresenterCommand::Request,
        PresenterCommand::Request,
        PresenterCommand::Draw(3),
        PresenterCommand::Ack(0),
        PresenterCommand::Deadline,
        PresenterCommand::Ack(1),
        PresenterCommand::Draw(0),
        PresenterCommand::Request,
        PresenterCommand::Draw(3),
        PresenterCommand::Ack(3),
        PresenterCommand::Deadline,
        PresenterCommand::Ack(4),
        PresenterCommand::Finish,
    ] {
        commands.push(command).unwrap();
    }
    commands
}

async fn run_raw_presenter(mut commands: FixedQueue<PresenterCommand, 16>) -> Presenter {
    let mut presenter = Presenter::default();
    presenter.initial_presentation();
    loop {
        let command = tokio::select! {
            biased;
            command = core::future::poll_fn(|cx| commands.poll_next(cx)) => command,
            _ = core::future::pending::<()>() => unreachable!(),
        };
        if presenter.apply(command) {
            return presenter;
        }
    }
}

async fn run_kittens_presenter(mut commands: FixedQueue<PresenterCommand, 16>) -> Presenter {
    let mut presenter = Presenter::default();
    let result: Result<(), core::convert::Infallible> = kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [initialize];
        }

        initialize {
            presenter.initial_presentation();
            Ok(())
        }

        #[source(commands)]
        #[readiness(may_remain_ready)]
        #[drain(max = 16)]
        #[last]
        command = commands => {
            if presenter.apply(command) {
                Ok(Control::Stop(()))
            } else {
                Ok(Control::Continue)
            }
        }
    };
    result.unwrap();
    presenter
}

#[test]
fn twenty_three_arm_forms_compile_without_custom_type_or_recursion_limits() {
    let mut app = App::default();
    let mut sources = GrokSources::new();
    let core = app.run_core_event(&mut sources);
    let core_size = std::mem::size_of_val(&core);
    drop(core);

    let mut control_app = App::default();
    let mut control_sources = GrokSources::new();
    let control = control_app.run_tokio_control(&mut control_sources);
    let control_size = std::mem::size_of_val(&control);
    drop(control);

    let mut core_slots_app = App::default();
    let mut core_slots_sources = GrokSources::new();
    let core_slots = core_slots_app.run_core_slots(&mut core_slots_sources);
    let core_slots_size = std::mem::size_of_val(&core_slots);
    drop(core_slots);

    let mut tokio_slots_app = App::default();
    let mut tokio_slots_sources = GrokSources::new();
    let tokio_slots = tokio_slots_app.run_tokio_slots(&mut tokio_slots_sources);
    let tokio_slots_size = std::mem::size_of_val(&tokio_slots);
    drop(tokio_slots);

    eprintln!(
        "grok future sizes: core-event={core_size} core-slots={core_slots_size} \
         tokio-event={control_size} tokio-slots={tokio_slots_size} bytes"
    );

    assert!(
        core_size < 128 * 1024,
        "core future unexpectedly large: {core_size}"
    );
    assert!(
        control_size < 128 * 1024,
        "Tokio control future unexpectedly large: {control_size}"
    );
    assert!(
        core_slots_size < 128 * 1024,
        "core slots future unexpectedly large: {core_slots_size}"
    );
    assert!(
        tokio_slots_size < 128 * 1024,
        "Tokio slots future unexpectedly large: {tokio_slots_size}"
    );
}

#[tokio::test]
async fn raw_tokio_oracle_preserves_the_declared_twenty_three_source_order() {
    let mut sources = GrokSources::new();
    sources.writer_event.push(3).unwrap();
    sources.acp_stream.push(4).unwrap();
    let (voice_sender, voice_receiver) = tokio::sync::mpsc::unbounded_channel();
    voice_sender.send(23).unwrap();
    sources.voice_stt.arm(voice_receiver).unwrap();
    assert_eq!(
        raw_tokio_select_once(&mut sources).await,
        RawSourceId::WriterEvent
    );
    assert_eq!(
        raw_tokio_select_once(&mut sources).await,
        RawSourceId::AcpStream
    );
    assert_eq!(
        raw_tokio_select_once(&mut sources).await,
        RawSourceId::VoiceStt
    );
}

#[tokio::test]
async fn grok_dynamic_voice_arm_and_reconnect_replacement_are_persistent() {
    let mut voice_app = App::default();
    let mut voice_sources = GrokSources::new();
    let (voice_sender, voice_receiver) = tokio::sync::mpsc::unbounded_channel();
    voice_sender.send(23).unwrap();
    voice_sources.voice_stt.arm(voice_receiver).unwrap();
    assert_eq!(
        voice_app.run_core_event(&mut voice_sources).await,
        Ok(LoopExit::Quit)
    );
    assert_eq!(voice_app.handled, 1);

    let mut reconnect_app = App::default();
    let mut reconnect_sources = GrokSources::new();
    reconnect_sources
        .reconnect_reinit
        .arm(std::future::ready(1))
        .unwrap();
    assert!(
        reconnect_sources
            .reconnect_reinit
            .cancel_and_replace(std::future::ready(2))
    );
    assert_eq!(
        reconnect_app.run_core_event(&mut reconnect_sources).await,
        Ok(LoopExit::Quit)
    );
    assert_eq!(reconnect_app.handled, 1);
}

#[tokio::test]
async fn grok_scripted_thirty_two_item_drain_does_not_reach_a_budget_boundary() {
    let mut app = App {
        accepts_acp: true,
        ..App::default()
    };
    let mut sources = GrokSources::new();
    for value in 1..=32 {
        sources.acp_stream.push(value).unwrap();
    }

    assert_eq!(app.run_core_event(&mut sources).await, Ok(LoopExit::Quit));
    assert_eq!(app.handled, 32);
    assert_eq!(app.after_count, 1);
    assert!(!app.saw_exhausted_budget);

    let mut control_app = App {
        accepts_acp: true,
        ..App::default()
    };
    let mut control_sources = GrokSources::new();
    for value in 1..=32 {
        control_sources.acp_stream.push(value).unwrap();
    }
    assert_eq!(
        control_app.run_tokio_control(&mut control_sources).await,
        Ok(LoopExit::Quit)
    );
    assert_eq!(control_app.handled, app.handled);
    assert_eq!(control_app.after_count, app.after_count);
    assert_eq!(control_app.saw_exhausted_budget, app.saw_exhausted_budget);
}

#[tokio::test]
async fn raw_and_kittens_forms_preserve_application_owned_presenter_scenarios() {
    let raw = run_raw_presenter(presenter_commands()).await;
    let generated = run_kittens_presenter(presenter_commands()).await;
    assert_eq!(generated, raw);
    assert_eq!(
        generated,
        Presenter {
            pending: false,
            in_flight: None,
            next_sequence: 4,
            emitted_payloads: 4,
            coalesced_requests: 1,
            blocked_draws: 1,
            no_payload_draws: 1,
            stale_acks: 2,
            deadline_armed: false,
            deadline_fires: 2,
        }
    );
}
