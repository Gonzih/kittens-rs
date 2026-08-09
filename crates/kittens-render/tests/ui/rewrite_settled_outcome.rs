use kittens_render::transfer::{Settled, TransferOutcome};

fn rewrite(settled: &mut Settled<(), (), ()>) {
    settled.outcome = TransferOutcome::Completed;
}

fn main() {}
