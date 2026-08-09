use kittens_render::transfer::{Settled, TransferOutcome};

fn main() {
    let _forged: Settled<(), (), ()> = Settled {
        transport: (),
        buffer: (),
        spare: (),
        outcome: TransferOutcome::Completed,
        target: None,
    };
}
