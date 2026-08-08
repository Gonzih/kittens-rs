#![allow(clippy::ignored_unit_patterns, dead_code, missing_docs)]

use std::convert::Infallible;

use kittens::reactor::Control;
use kittens::source::{BacklogSource, FixedQueue, Latched};

#[derive(Clone, Copy)]
enum Mode {
    Off,
    Aod,
    Interactive,
    Game,
}

impl Mode {
    fn next_frame_at(self, now_ms: u64) -> u64 {
        now_ms
            + match self {
                Self::Off => 30_000,
                Self::Aod => 1_000,
                Self::Interactive => 33,
                Self::Game => 16,
            }
    }
}

#[derive(Debug)]
struct Display;
#[derive(Debug)]
struct Framebuffer([u8; 256]);
#[derive(Debug)]
struct TransferDone {
    display: Display,
    framebuffer: Framebuffer,
}
struct InFlight {
    display: Display,
    framebuffer: Framebuffer,
}

struct Sources {
    stop: Latched<()>,
    frame_deadline: Latched<u64>,
    touch: Latched<u8>,
    button: Latched<u8>,
    optional_control: Latched<u8>,
    transfer_done: Latched<TransferDone>,
    sensor: FixedQueue<u16, 8>,
}

impl Sources {
    fn new() -> Self {
        Self {
            stop: Latched::new(),
            frame_deadline: Latched::new(),
            touch: Latched::new(),
            button: Latched::new(),
            optional_control: Latched::new(),
            transfer_done: Latched::new(),
            sensor: FixedQueue::new(),
        }
    }
}

struct Firmware {
    mode: Mode,
    now_ms: u64,
    frame_due: Option<u64>,
    dirty: bool,
    idle_display: Option<(Display, Framebuffer)>,
    in_flight: Option<InFlight>,
    after_count: usize,
    stop_after_event: bool,
}

impl Firmware {
    fn new() -> Self {
        Self {
            mode: Mode::Off,
            now_ms: 0,
            frame_due: Some(30_000),
            dirty: false,
            idle_display: None,
            in_flight: None,
            after_count: 0,
            stop_after_event: true,
        }
    }

    fn submit_frame(&mut self, display: Display, framebuffer: Framebuffer) {
        self.in_flight = Some(InFlight {
            display,
            framebuffer,
        });
    }

    async fn run(&mut self, sources: &mut Sources) -> Result<(), Infallible> {
        kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [before_poll, after_event];
            }

            before_poll {
                if let Some(due) = self.frame_due {
                    if self.now_ms >= due && sources.frame_deadline.is_dormant() {
                        sources.frame_deadline.arm(due).unwrap();
                        self.frame_due = None;
                    }
                }
                Ok(())
            }

            #[source(stop)]
            #[readiness(quiescent)]
            #[shutdown]
            _ = sources.stop => {
                Ok(())
            }

            #[source(frame_deadline)]
            #[readiness(quiescent)]
            fired_at = sources.frame_deadline => {
                self.now_ms = fired_at;
                self.dirty = true;
                self.frame_due = Some(self.mode.next_frame_at(fired_at));
                Ok(Control::Continue)
            }

            #[source(touch)]
            #[readiness(quiescent)]
            event = sources.touch => {
                self.mode = Mode::Interactive;
                self.now_ms += u64::from(event);
                self.frame_due = Some(self.mode.next_frame_at(self.now_ms));
                self.dirty = true;
                Ok(Control::Continue)
            }

            #[source(button)]
            #[readiness(quiescent)]
            event = sources.button => {
                self.mode = Mode::Game;
                self.now_ms += u64::from(event);
                self.frame_due = Some(self.mode.next_frame_at(self.now_ms));
                self.dirty = true;
                Ok(Control::Continue)
            }

            #[source(optional_control)]
            #[readiness(quiescent)]
            event = sources.optional_control => {
                self.now_ms += u64::from(event);
                Ok(Control::Continue)
            }

            #[source(transfer_done)]
            #[readiness(quiescent)]
            done = sources.transfer_done => {
                self.idle_display = Some((done.display, done.framebuffer));
                self.dirty = true;
                Ok(Control::Continue)
            }

            #[source(sensor)]
            #[readiness(may_remain_ready)]
            #[starvation(allowed, reason = "sensor telemetry is best effort")]
            #[drain(max = 4)]
            #[last]
            sample = sources.sensor => {
                self.now_ms += u64::from(sample);
                Ok(Control::Continue)
            }

            after_event {
                self.after_count += 1;
                if self.dirty {
                    if let Some((display, framebuffer)) = self.idle_display.take() {
                        self.submit_frame(display, framebuffer);
                    }
                    self.dirty = false;
                }
                if self.stop_after_event || self.in_flight.is_some() {
                    let _ = sources.stop.arm(());
                }
                Ok(())
            }
        }
    }
}

#[tokio::test]
async fn ownership_returning_completion_reenables_submission_in_after_event() {
    let mut firmware = Firmware::new();
    let mut sources = Sources::new();
    sources
        .transfer_done
        .arm(TransferDone {
            display: Display,
            framebuffer: Framebuffer([0; 256]),
        })
        .unwrap();

    assert_eq!(firmware.run(&mut sources).await, Ok(()));
    assert!(firmware.in_flight.is_some());
    assert_eq!(firmware.after_count, 1);
}

#[tokio::test]
async fn protected_touch_precedes_ready_sensor_and_sensor_remains_buffered() {
    struct PrioritySources {
        touch: Latched<u8>,
        sensor: FixedQueue<u16, 4>,
    }
    let mut sources = PrioritySources {
        touch: Latched::new(),
        sensor: FixedQueue::new(),
    };
    sources.touch.arm(7).unwrap();
    sources.sensor.push(99).unwrap();
    let mut selected = 0;

    let result: Result<(), Infallible> = kittens::reactor! {
        policy { selection: biased; required_phases: []; }

        #[source(touch)]
        #[readiness(quiescent)]
        value = sources.touch => {
            selected = value;
            Ok(Control::Stop(()))
        }

        #[source(sensor)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "sensor telemetry is best effort")]
        #[last]
        _ = sources.sensor => {
            Ok(Control::Continue)
        }
    };

    assert_eq!(result, Ok(()));
    assert_eq!(selected, 7);
    assert!(sources.sensor.has_backlog());
}

#[tokio::test]
async fn dynamic_absolute_deadline_rearms_after_each_fire() {
    let mut firmware = Firmware::new();
    let mut sources = Sources::new();
    firmware.mode = Mode::Interactive;
    firmware.now_ms = 133;
    firmware.frame_due = Some(133);

    assert_eq!(firmware.run(&mut sources).await, Ok(()));
    assert_eq!(firmware.now_ms, 133);
    assert_eq!(firmware.frame_due, Some(166));

    firmware.now_ms = 166;
    assert_eq!(firmware.run(&mut sources).await, Ok(()));
    assert_eq!(firmware.frame_due, Some(199));
    assert_eq!(firmware.after_count, 2);
}

#[test]
fn dormant_optional_host_source_does_not_self_wake() {
    let mut source = Latched::<u8>::new();
    assert!(source.is_dormant());
    source.arm(1).unwrap();
    assert_eq!(source.disarm(), Some(1));
    assert!(source.is_dormant());
}

#[test]
fn embedded_future_stays_within_predeclared_host_model_budget() {
    // Predeclared in docs/fixture-manifest.md before expansion measurement.
    const MAX_FUTURE_BYTES: usize = 16 * 1024;
    let mut firmware = Firmware::new();
    let mut sources = Sources::new();
    let future = firmware.run(&mut sources);
    let size = std::mem::size_of_val(&future);
    eprintln!("embedded future size: {size} bytes");
    assert!(size <= MAX_FUTURE_BYTES, "embedded future is {size} bytes");
}
