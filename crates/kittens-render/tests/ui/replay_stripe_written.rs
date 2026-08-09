use kittens_render::sweep::{StripeWritten, Sweep};

fn replay<S>(sweep: &mut Sweep<S>, witness: StripeWritten) {
    let _first = sweep.mark_written(witness);
    let _second = sweep.mark_written(witness);
}

fn main() {}
