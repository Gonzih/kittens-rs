use kittens_render::sweep::AbortedSweep;

fn clone_witness(witness: AbortedSweep) {
    let _copy: AbortedSweep = witness.clone();
}

fn main() {}
