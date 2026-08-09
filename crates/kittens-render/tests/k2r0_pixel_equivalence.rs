//! K2R-0 host pixel-equivalence oracles for the optional draw-target slice.
//!
//! The full-frame reference is embedded-graphics' independent framebuffer.
//! Stripe assembly crosses the real Kittens witness chain for every region:
//! target mint → draw → start → poll → recover → owning-sweep settlement.

#![cfg(feature = "embedded-graphics")]
#![allow(missing_docs)]

use core::{convert::Infallible, task::Poll};
use std::task::{Context, Waker};

use embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    framebuffer::Framebuffer,
    geometry::{Dimensions, Point, Size},
    pixelcolor::{
        Rgb565, RgbColor,
        raw::{BigEndian, RawU16},
    },
    prelude::{Drawable, Primitive},
    primitives::{Line, PrimitiveStyle, Rectangle},
};
use kittens_render::{
    demand::{FrameDemand, Tick, WrittenDisposition},
    draw_target::{Rgb565StripeDrawTarget, StripeDrawTargetError},
    geometry::{PanelGeometry, Region},
    sweep::{Sweep, SweepPlan, SweepWritten},
    transfer::{FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome},
};

const WIDTH: usize = 13;
const HEIGHT: usize = 9;
const STRIPE_HEIGHT: usize = 4;
const FRAME_BYTES: usize = WIDTH * HEIGHT * 2;
const MAX_STRIPE_BYTES: usize = WIDTH * STRIPE_HEIGHT * 2;
const PANEL: Region = Region {
    x: 0,
    y: 0,
    width: 13,
    height: 9,
};

type ReferenceFrame = Framebuffer<Rgb565, RawU16, BigEndian, WIDTH, HEIGHT, FRAME_BYTES>;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Scene {
    accent: Rgb565,
    marker: Rgb565,
    marker_x: i32,
}

const SCENE_A: Scene = Scene {
    accent: Rgb565::RED,
    marker: Rgb565::BLUE,
    marker_x: 1,
};
const SCENE_B: Scene = Scene {
    accent: Rgb565::GREEN,
    marker: Rgb565::YELLOW,
    marker_x: 9,
};

fn geometry() -> PanelGeometry {
    PanelGeometry::custom_unvalidated_panel(PANEL)
}

fn plan() -> SweepPlan {
    SweepPlan::for_panel(geometry(), 4).expect("valid test plan")
}

/// Replays a complete ordered scene. Its centered geometry is deliberately
/// derived from the draw target's reported bounds: stripe-local dimensions
/// therefore change pixels and make the equivalence oracle fail.
fn render_scene<T>(scene: Scene, target: &mut T)
where
    T: DrawTarget<Color = Rgb565, Error = Infallible>,
{
    let bounds = target.bounding_box();
    let width = i32::try_from(bounds.size.width).expect("small test width");
    let height = i32::try_from(bounds.size.height).expect("small test height");
    let center = Point::new(
        bounds.top_left.x + width / 2,
        bounds.top_left.y + height / 2,
    );

    target.clear(Rgb565::BLACK).expect("infallible draw");
    Rectangle::with_center(center, Size::new(7, 5))
        .into_styled(PrimitiveStyle::with_fill(scene.accent))
        .draw(target)
        .expect("infallible draw");
    Line::new(
        Point::new(bounds.top_left.x - 2, bounds.top_left.y + 1),
        Point::new(
            bounds.top_left.x + width + 1,
            bounds.top_left.y + height - 2,
        ),
    )
    .into_styled(PrimitiveStyle::with_stroke(Rgb565::WHITE, 1))
    .draw(target)
    .expect("infallible draw");
    Rectangle::new(
        Point::new(bounds.top_left.x + scene.marker_x, bounds.top_left.y - 1),
        Size::new(3, 4),
    )
    .into_styled(PrimitiveStyle::with_fill(scene.marker))
    .draw(target)
    .expect("infallible draw");
}

fn reference_pixels(scene: Scene) -> [u8; FRAME_BYTES] {
    let mut reference = ReferenceFrame::new();
    render_scene(scene, &mut reference);
    let mut bytes = [0; FRAME_BYTES];
    bytes.copy_from_slice(reference.data());
    bytes
}

#[derive(Debug)]
struct StripeBuffer {
    bytes: [u8; MAX_STRIPE_BYTES],
    rendered_len: usize,
}

impl StripeBuffer {
    const fn new() -> Self {
        Self {
            bytes: [0xA5; MAX_STRIPE_BYTES],
            rendered_len: 0,
        }
    }
}

#[derive(Debug)]
struct ModelPanel {
    bytes: [u8; FRAME_BYTES],
}

impl ModelPanel {
    const fn new(fill: u8) -> Self {
        Self {
            bytes: [fill; FRAME_BYTES],
        }
    }

    fn write_prefix(&mut self, region: Region, sent: &StripeBuffer, pixels: usize) {
        assert_eq!(region.x, PANEL.x);
        assert_eq!(region.width, PANEL.width);
        assert!(region.y < PANEL.height);
        assert!(region.height <= PANEL.height - region.y);
        let expected = region_byte_len(region);
        assert_eq!(sent.rendered_len, expected);

        let pixel_count = pixels.min(expected / 2);
        let destination_pixel =
            (usize::from(region.y - PANEL.y) * WIDTH) + usize::from(region.x - PANEL.x);
        let destination = destination_pixel * 2;
        let byte_count = pixel_count * 2;
        self.bytes[destination..destination + byte_count]
            .copy_from_slice(&sent.bytes[..byte_count]);
    }
}

struct Resources {
    panel: ModelPanel,
    ready: StripeBuffer,
    spare: StripeBuffer,
}

impl Resources {
    const fn new(fill: u8) -> Self {
        Self {
            panel: ModelPanel::new(fill),
            ready: StripeBuffer::new(),
            spare: StripeBuffer::new(),
        }
    }
}

#[derive(Clone, Copy)]
enum ModelBehavior {
    Complete,
    FailAfterPixels(usize),
}

struct ModelStart {
    panel: ModelPanel,
    sent: StripeBuffer,
    behavior: ModelBehavior,
}

struct ModelTransfer {
    panel: ModelPanel,
    sent: StripeBuffer,
    region: Region,
    behavior: ModelBehavior,
    settled: Option<TransferOutcome>,
}

impl FlightStarter for ModelStart {
    type Transfer = ModelTransfer;
    type Error = Infallible;

    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        assert_eq!(self.sent.rendered_len, region_byte_len(region));
        Ok(ModelTransfer {
            panel: self.panel,
            sent: self.sent,
            region,
            behavior: self.behavior,
            settled: None,
        })
    }
}

impl OwnedTransfer for ModelTransfer {
    type Transport = ModelPanel;
    type Buffer = StripeBuffer;

    fn poll_done(&mut self, _cx: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_none() {
            self.settled = Some(match self.behavior {
                ModelBehavior::Complete => {
                    self.panel
                        .write_prefix(self.region, &self.sent, self.sent.rendered_len / 2);
                    TransferOutcome::Completed
                }
                ModelBehavior::FailAfterPixels(pixels) => {
                    self.panel.write_prefix(self.region, &self.sent, pixels);
                    TransferOutcome::Failed
                }
            });
        }
        Poll::Ready(())
    }

    fn cancel(&mut self) {
        if self.settled.is_none() {
            self.settled = Some(TransferOutcome::Cancelled);
        }
    }

    fn recover(self) -> Recovered<Self::Transport, Self::Buffer> {
        Recovered {
            transport: self.panel,
            buffer: self.sent,
            outcome: self
                .settled
                .expect("model transfer was polled to settlement"),
        }
    }
}

struct StripeLocalDrawTarget<'a> {
    inner: Rgb565StripeDrawTarget<'a>,
    stripe: Region,
}

impl Dimensions for StripeLocalDrawTarget<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(
            Point::new(i32::from(self.stripe.x), i32::from(self.stripe.y)),
            Size::new(u32::from(self.stripe.width), u32::from(self.stripe.height)),
        )
    }
}

impl DrawTarget for StripeLocalDrawTarget<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.inner.draw_iter(pixels)
    }
}

fn region_byte_len(region: Region) -> usize {
    usize::from(region.width) * usize::from(region.height) * 2
}

fn transfer_next(
    sweep: &mut Sweep<Scene>,
    resources: Resources,
    behavior: ModelBehavior,
    stripe_local_negative_control: bool,
) -> (Resources, TransferOutcome) {
    let target = sweep.next_target().expect("stripe remains");
    let region = target.region();
    let rendered_len = region_byte_len(region);
    let Resources {
        panel,
        mut ready,
        spare,
    } = resources;
    ready.rendered_len = rendered_len;
    ready.bytes[..rendered_len].fill(0xA5);

    {
        let draw = Rgb565StripeDrawTarget::new(sweep, &target, &mut ready.bytes[..rendered_len])
            .expect("owning target and exact buffer");
        if stripe_local_negative_control {
            let mut broken = StripeLocalDrawTarget {
                inner: draw,
                stripe: region,
            };
            render_scene(*sweep.snapshot(), &mut broken);
        } else {
            let mut draw = draw;
            render_scene(*sweep.snapshot(), &mut draw);
        }
    }

    let mut flight = target
        .start_flight(
            spare,
            ModelStart {
                panel,
                sent: ready,
                behavior,
            },
        )
        .expect("infallible honest model start");
    let waker = Waker::noop().clone();
    let mut cx = Context::from_waker(&waker);
    let settled = match flight.poll_complete(&mut cx) {
        Poll::Ready(settled) => settled,
        Poll::Pending => panic!("model transfer settles immediately"),
    };
    let (panel, sent, spare, settlement) = settled.into_parts();
    let outcome = sweep
        .settle(settlement)
        .expect("model settlement belongs to this sweep");
    (
        Resources {
            panel,
            ready: spare,
            spare: sent,
        },
        outcome,
    )
}

fn complete_sweep(
    mut sweep: Sweep<Scene>,
    mut resources: Resources,
    stripe_local_negative_control: bool,
) -> (SweepWritten, Scene, Resources) {
    while !sweep.is_complete() {
        let (next, outcome) = transfer_next(
            &mut sweep,
            resources,
            ModelBehavior::Complete,
            stripe_local_negative_control,
        );
        resources = next;
        assert_eq!(outcome, TransferOutcome::Completed);
    }
    let (written, scene) = sweep.finish().expect("all stripes were written");
    (written, scene, resources)
}

fn begin_scene(demand: &mut FrameDemand, now: Tick, scene: Scene) -> Sweep<Scene> {
    demand.request();
    demand.begin_sweep(now, scene).expect("requested sweep")
}

#[test]
fn rgb565_bytes_are_high_byte_first() {
    const LOCAL_PANEL: Region = Region {
        x: 5,
        y: 7,
        width: 3,
        height: 4,
    };
    let plan = SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(LOCAL_PANEL), 2)
        .expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    let mut sweep = begin_scene(&mut demand, Tick(0), SCENE_A);
    let target = sweep.next_target().expect("first stripe");
    let mut bytes = [0xA5; 12];

    {
        let mut draw =
            Rgb565StripeDrawTarget::new(&sweep, &target, &mut bytes).expect("exact stripe buffer");
        draw.draw_iter([
            Pixel(Point::new(5, 7), Rgb565::RED),
            Pixel(Point::new(6, 7), Rgb565::GREEN),
            Pixel(Point::new(7, 7), Rgb565::BLUE),
        ])
        .expect("infallible draw");
    }

    assert_eq!(&bytes[..6], &[0xF8, 0x00, 0x07, 0xE0, 0x00, 0x1F]);
}

#[test]
fn pixels_are_clipped_and_translated_into_the_stripe() {
    const LOCAL_PANEL: Region = Region {
        x: 5,
        y: 7,
        width: 3,
        height: 4,
    };
    let plan = SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(LOCAL_PANEL), 2)
        .expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    let mut sweep = begin_scene(&mut demand, Tick(0), SCENE_A);
    let target = sweep.next_target().expect("first stripe");
    let mut bytes = [0xA5; 12];

    {
        let mut draw =
            Rgb565StripeDrawTarget::new(&sweep, &target, &mut bytes).expect("exact stripe buffer");
        draw.draw_iter([
            Pixel(Point::new(6, 8), Rgb565::WHITE),
            Pixel(Point::new(7, 8), Rgb565::BLUE),
            Pixel(Point::new(6, 9), Rgb565::RED),
            Pixel(Point::new(4, 7), Rgb565::RED),
            Pixel(Point::new(i32::MIN, i32::MIN), Rgb565::RED),
            Pixel(Point::new(i32::MAX, i32::MAX), Rgb565::RED),
        ])
        .expect("infallible draw");
    }

    let mut expected = [0xA5; 12];
    expected[8..10].copy_from_slice(&[0xFF, 0xFF]);
    expected[10..12].copy_from_slice(&[0x00, 0x1F]);
    assert_eq!(bytes, expected);
}

#[test]
fn bounding_box_preserves_global_layout_for_nonzero_panel_origin() {
    const LOCAL_PANEL: Region = Region {
        x: 5,
        y: 7,
        width: 3,
        height: 4,
    };
    let plan = SweepPlan::for_panel(PanelGeometry::custom_unvalidated_panel(LOCAL_PANEL), 2)
        .expect("valid plan");
    let mut demand = FrameDemand::new(0, plan);
    let mut sweep = begin_scene(&mut demand, Tick(0), SCENE_A);
    let target = sweep.next_target().expect("first stripe");
    let mut bytes = [0; 12];
    let draw =
        Rgb565StripeDrawTarget::new(&sweep, &target, &mut bytes).expect("exact stripe buffer");

    assert_eq!(
        draw.bounding_box(),
        Rectangle::new(Point::new(5, 7), Size::new(3, 4))
    );
    assert_ne!(
        draw.bounding_box(),
        Rectangle::new(Point::new(5, 7), Size::new(3, 2)),
        "the draw target is a panel layout viewport, not stripe-local bounds"
    );
}

#[test]
fn constructor_rejects_foreign_target_and_wrong_buffer_length() {
    let mut left_demand = FrameDemand::new(0, plan());
    let mut right_demand = FrameDemand::new(0, plan());
    let mut left = begin_scene(&mut left_demand, Tick(0), SCENE_A);
    let mut right = begin_scene(&mut right_demand, Tick(0), SCENE_A);
    let _left_target = left.next_target().expect("left target");
    let right_target = right.next_target().expect("right target");
    let mut exact = [0; MAX_STRIPE_BYTES];
    assert!(matches!(
        Rgb565StripeDrawTarget::new(&left, &right_target, &mut exact),
        Err(StripeDrawTargetError::TargetMismatch)
    ));

    let mut short = [0; MAX_STRIPE_BYTES - 1];
    assert!(matches!(
        Rgb565StripeDrawTarget::new(&right, &right_target, &mut short),
        Err(StripeDrawTargetError::WrongBufferLength {
            expected: MAX_STRIPE_BYTES,
            actual,
        }) if actual == MAX_STRIPE_BYTES - 1
    ));
}

#[test]
fn full_frame_and_witnessed_stripe_sweep_are_pixel_equivalent() {
    let mut demand = FrameDemand::new(0, plan());
    let sweep = begin_scene(&mut demand, Tick(0), SCENE_A);
    assert!(sweep.full_repaint());

    let (written, returned_scene, resources) = complete_sweep(sweep, Resources::new(0xCC), false);
    assert_eq!(returned_scene, SCENE_A);
    assert_eq!(
        demand.finish_written(written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    assert_eq!(resources.panel.bytes, reference_pixels(SCENE_A));
}

#[test]
fn negative_control_stripe_local_bounds_changes_centered_layout() {
    let mut demand = FrameDemand::new(0, plan());
    let sweep = begin_scene(&mut demand, Tick(0), SCENE_A);
    let (written, _, resources) = complete_sweep(sweep, Resources::new(0xCC), true);
    assert_eq!(
        demand.finish_written(written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    assert_ne!(
        resources.panel.bytes,
        reference_pixels(SCENE_A),
        "the oracle must detect the forbidden stripe-local Dimensions implementation"
    );
}

#[test]
fn mid_sweep_scene_change_is_rendered_only_by_next_epoch() {
    let mut demand = FrameDemand::new(0, plan());
    let mut live_scene = SCENE_A;
    let mut first = begin_scene(&mut demand, Tick(0), live_scene);
    assert_eq!(first.epoch().get(), 0);
    let resources = Resources::new(0xCC);
    let (mut resources, outcome) =
        transfer_next(&mut first, resources, ModelBehavior::Complete, false);
    assert_eq!(outcome, TransferOutcome::Completed);

    live_scene = SCENE_B;
    demand.request();
    while !first.is_complete() {
        let (next, outcome) = transfer_next(&mut first, resources, ModelBehavior::Complete, false);
        resources = next;
        assert_eq!(outcome, TransferOutcome::Completed);
    }
    let (first_written, returned_first) = first.finish().expect("first epoch complete");
    assert_eq!(returned_first, SCENE_A);
    assert_eq!(resources.panel.bytes, reference_pixels(SCENE_A));
    assert_eq!(
        demand.finish_written(first_written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );

    let second = demand
        .begin_sweep(Tick(1), live_scene)
        .expect("mid-sweep request survived settlement");
    assert_eq!(second.epoch().get(), 1);
    let (second_written, returned_second, resources) = complete_sweep(second, resources, false);
    assert_eq!(returned_second, SCENE_B);
    assert_eq!(resources.panel.bytes, reference_pixels(SCENE_B));
    assert_eq!(
        demand.finish_written(second_written, Tick(2)),
        Ok(WrittenDisposition::Effective)
    );
}

#[test]
fn post_failure_full_repaint_restores_pixel_equivalence() {
    let mut demand = FrameDemand::new(0, plan());
    let initial = begin_scene(&mut demand, Tick(0), SCENE_A);
    let (initial_written, _, resources) = complete_sweep(initial, Resources::new(0xCC), false);
    assert_eq!(
        demand.finish_written(initial_written, Tick(1)),
        Ok(WrittenDisposition::Effective)
    );
    assert!(!demand.full_repaint_required());

    let mut failed = begin_scene(&mut demand, Tick(1), SCENE_B);
    assert!(!failed.full_repaint());
    let (resources, first_outcome) =
        transfer_next(&mut failed, resources, ModelBehavior::Complete, false);
    assert_eq!(first_outcome, TransferOutcome::Completed);
    let (resources, failed_outcome) = transfer_next(
        &mut failed,
        resources,
        ModelBehavior::FailAfterPixels(5),
        false,
    );
    assert_eq!(failed_outcome, TransferOutcome::Failed);
    assert!(failed.is_poisoned());
    assert_ne!(resources.panel.bytes, reference_pixels(SCENE_B));

    let (aborted, returned_failed) = failed.abort().expect("failed transfer settled");
    assert_eq!(returned_failed, SCENE_B);
    demand
        .finish_failed(aborted, Tick(2))
        .expect("owning failed epoch");
    assert!(demand.full_repaint_required());

    let replacement = demand
        .begin_sweep(Tick(2), SCENE_B)
        .expect("failed sweep retains demand");
    assert!(replacement.full_repaint());
    let (replacement_written, returned_replacement, resources) =
        complete_sweep(replacement, resources, false);
    assert_eq!(returned_replacement, SCENE_B);
    assert_eq!(resources.panel.bytes, reference_pixels(SCENE_B));
    assert_eq!(
        demand.finish_written(replacement_written, Tick(3)),
        Ok(WrittenDisposition::Effective)
    );
    assert!(!demand.full_repaint_required());
}
