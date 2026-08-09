use kittens_render::demand::{FrameDemand, Tick};
use kittens_render::sweep::SweepWritten;

fn replay(demand: &mut FrameDemand, witness: SweepWritten) {
    let _first = demand.finish_written(witness, Tick(1));
    let _second = demand.finish_written(witness, Tick(2));
}

fn main() {}
