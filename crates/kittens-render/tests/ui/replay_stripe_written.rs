use kittens_render::sweep::{StripeSettlement, StripeWritten, Sweep};

fn replay<S>(sweep: &mut Sweep<S>, witness: StripeWritten) {
    let _first = sweep.settle(StripeSettlement::Written(witness));
    let _second = sweep.settle(StripeSettlement::Written(witness));
}

fn main() {}
